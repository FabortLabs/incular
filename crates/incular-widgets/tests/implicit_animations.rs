//! Flutter's implicit animations (`ImplicitlyAnimatedWidgetState`): a new
//! target animates from the current value over the duration, a target that
//! changes mid-flight restarts from where the animation is, and `onEnd`
//! fires when a run completes.

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Alignment, Constraints, EdgeInsets};
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{DisplayList, PaintCommand};
use incular_widgets::internal::{ElementId, RenderKind, WidgetTree};
use incular_widgets::{
    AnimatedAlign, AnimatedContainer, AnimatedFractionallySizedBox, AnimatedOpacity,
    AnimatedPadding, AnimatedPositioned, AnimatedPositionedDirectional, AnimatedRotation,
    AnimatedScale, AnimatedSlide, CustomScrollView, Directionality, Sliver, SliverAnimatedOpacity,
    SliverToBoxAdapter, Stack, TweenAnimationBuilder, Widget,
};

const DURATION: Duration = Duration::from_millis(200);

struct Harness {
    tree: WidgetTree,
    root: ElementId,
    base: Instant,
}

impl Harness {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).expect("mount");
        let mut harness = Self {
            tree,
            root,
            base: Instant::now(),
        };
        // The first frame builds; the second starts animations requested
        // by that build.
        harness.frame(0);
        harness.frame(0);
        harness
    }

    fn frame(&mut self, millis: u64) {
        self.tree
            .update_compositor(self.base + Duration::from_millis(millis))
            .expect("frame");
        self.tree
            .layout(Constraints::loose(Size::new(200., 200.)))
            .expect("layout");
    }

    /// Rebuilds with `widget` at `millis`: the animation starts on that
    /// frame's tick.
    fn update(&mut self, widget: Widget, millis: u64) {
        self.tree.update(self.root, widget).expect("update");
        self.frame(millis);
        self.frame(millis);
    }

    fn size(&self) -> Size {
        self.tree.element_bounds(self.root).expect("bounds").size
    }

    fn opacities(&self) -> Vec<f32> {
        let mut found = Vec::new();
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            if let Some(RenderKind::Opacity { alpha, .. }) = self
                .tree
                .render_id(id)
                .and_then(|render| self.tree.render_object_kind(render))
            {
                found.push(*alpha);
            }
            stack.extend(self.tree.children(id).unwrap_or_default());
        }
        found
    }

    /// Painted rectangles in root coordinates.
    fn rects(&mut self) -> Vec<Rect> {
        painted_rects(&self.tree.paint())
    }
}

fn painted_rects(list: &DisplayList) -> Vec<Rect> {
    let mut transforms = vec![CoreTransform::IDENTITY];
    let mut rects = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let current = *transforms.last().unwrap();
                transforms.push(current.then(*transform));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Rect { rect, .. } => {
                rects.push(transforms.last().unwrap().transform_rect_bbox(*rect));
            }
            PaintCommand::RRect { rrect, .. } => {
                rects.push(transforms.last().unwrap().transform_rect_bbox(rrect.rect));
            }
            _ => {}
        }
    }
    rects
}

fn approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-3,
        "expected {expected}, got {actual}"
    );
}

fn approx_rect(actual: Rect, expected: Rect) {
    approx(actual.origin.x, expected.origin.x);
    approx(actual.origin.y, expected.origin.y);
    approx(actual.size.width, expected.size.width);
    approx(actual.size.height, expected.size.height);
}

fn square() -> Widget {
    Widget::box_(Size::new(50., 50.), Color::WHITE)
}

#[test]
fn opacity_animates_to_a_new_target_and_reports_the_end() {
    let ended = Rc::new(Cell::new(0));
    let opacity = |value: f32| {
        let ended = ended.clone();
        AnimatedOpacity::new(value, DURATION, square())
            .on_end(move || ended.set(ended.get() + 1))
            .into()
    };
    let mut harness = Harness::new(opacity(1.));
    assert_eq!(harness.opacities(), [1.]);
    harness.update(opacity(0.), 1_000);
    harness.frame(1_100);
    // Half of 200 ms, linear: 1 + (0 - 1) * 0.5.
    assert_eq!(harness.opacities(), [0.5]);
    assert_eq!(ended.get(), 0);
    harness.frame(1_200);
    harness.frame(1_216);
    assert_eq!(harness.opacities(), [0.]);
    assert_eq!(ended.get(), 1);
}

#[test]
fn a_target_changed_mid_flight_restarts_from_the_current_value() {
    let mut harness = Harness::new(AnimatedOpacity::new(1., DURATION, square()).into());
    harness.update(AnimatedOpacity::new(0., DURATION, square()).into(), 1_000);
    harness.frame(1_100);
    assert_eq!(harness.opacities(), [0.5]);
    // `didUpdateWidget`: begin = 0.5 (the current value), end = 1, and the
    // controller runs again from 0.
    harness.update(AnimatedOpacity::new(1., DURATION, square()).into(), 1_100);
    assert_eq!(harness.opacities(), [0.5]);
    harness.frame(1_150);
    // A quarter of the way: 0.5 + (1 - 0.5) * 0.25.
    assert_eq!(harness.opacities(), [0.625]);
}

#[test]
fn an_unchanged_target_does_not_animate() {
    let mut harness = Harness::new(AnimatedOpacity::new(0.4, DURATION, square()).into());
    harness.update(AnimatedOpacity::new(0.4, DURATION, square()).into(), 1_000);
    harness.frame(1_100);
    assert_eq!(harness.opacities(), [0.4]);
}

#[test]
fn container_size_animates_through_its_constraints() {
    let container = |width: f32| {
        AnimatedContainer::new(DURATION)
            .width(width)
            .height(50.)
            .color(Color::WHITE)
            .into()
    };
    let mut harness = Harness::new(container(100.));
    assert_eq!(harness.size(), Size::new(100., 50.));
    harness.update(container(200.), 1_000);
    harness.frame(1_050);
    // `BoxConstraints.lerp` of tight 100 and tight 200 at 0.25.
    assert_eq!(harness.size(), Size::new(125., 50.));
    harness.frame(1_200);
    assert_eq!(harness.size(), Size::new(200., 50.));
}

#[test]
fn container_transform_pivots_about_its_alignment() {
    let container = |scale: f32| {
        AnimatedContainer::new(DURATION)
            .width(40.)
            .height(40.)
            .color(Color::WHITE)
            .transform(CoreTransform::scale(scale))
            .transform_alignment(Alignment::CENTER)
            .into()
    };
    let mut harness = Harness::new(container(1.));
    harness.update(container(2.), 1_000);
    harness.frame(1_100);
    // Scale 1.5 about the center (20, 20) of the 40 x 40 box.
    approx_rect(
        harness.rects()[0],
        Rect::from_origin_size(Offset::new(-10., -10.), Size::new(60., 60.)),
    );
}

#[test]
fn padding_and_alignment_animate() {
    let padded = |inset: f32| AnimatedPadding::new(EdgeInsets::all(inset), DURATION, square());
    let mut harness = Harness::new(padded(0.).into());
    harness.update(padded(20.).into(), 1_000);
    harness.frame(1_100);
    assert_eq!(harness.size(), Size::new(70., 70.));

    let aligned = |alignment| AnimatedAlign::new(alignment, DURATION, square());
    let mut harness = Harness::new(aligned(Alignment::TOP_LEFT).into());
    harness.update(aligned(Alignment::BOTTOM_RIGHT).into(), 1_000);
    harness.frame(1_100);
    // Alignment (0, 0) in a 200 x 200 box: the square's origin is (75, 75).
    assert_eq!(harness.rects()[0].origin, Offset::new(75., 75.));
}

#[test]
fn fractionally_sized_box_animates_its_factors() {
    let sized = |factor: f32| {
        AnimatedFractionallySizedBox::new(DURATION, Widget::box_(Size::ZERO, Color::WHITE))
            .width_factor(factor)
            .height_factor(factor)
    };
    let mut harness = Harness::new(sized(0.5).into());
    assert_eq!(harness.size(), Size::new(100., 100.));
    harness.update(sized(1.).into(), 1_000);
    harness.frame(1_100);
    assert_eq!(harness.size(), Size::new(150., 150.));
}

#[test]
fn positioned_animates_inside_a_stack() {
    let positioned = |left: f32| {
        Stack::new([Widget::from(
            AnimatedPositioned::new(DURATION, Widget::box_(Size::ZERO, Color::WHITE))
                .left(left)
                .top(10.)
                .width(20.)
                .height(20.),
        )])
        .into()
    };
    let mut harness = Harness::new(positioned(0.));
    approx_rect(
        harness.rects()[0],
        Rect::from_origin_size(Offset::new(0., 10.), Size::new(20., 20.)),
    );
    harness.update(positioned(100.), 1_000);
    harness.frame(1_100);
    approx_rect(
        harness.rects()[0],
        Rect::from_origin_size(Offset::new(50., 10.), Size::new(20., 20.)),
    );
}

#[test]
fn directional_positioned_follows_the_text_direction() {
    let positioned = |start: f32| {
        Directionality::new(
            incular_config::TextDirection::Rtl,
            Stack::new([Widget::from(
                AnimatedPositionedDirectional::new(
                    DURATION,
                    Widget::box_(Size::ZERO, Color::WHITE),
                )
                .start(start)
                .top(0.)
                .width(20.)
                .height(20.),
            )]),
        )
        .into()
    };
    let mut harness = Harness::new(positioned(0.));
    harness.update(positioned(100.), 1_000);
    harness.frame(1_100);
    // Right to left, start is the right edge: 200 - 50 - 20 = 130.
    assert_eq!(harness.rects()[0].origin.x, 130.);
}

#[test]
fn scale_rotation_and_slide_transform_the_child() {
    let mut harness = Harness::new(
        AnimatedScale::new(1., DURATION, square())
            .alignment(Alignment::TOP_LEFT)
            .into(),
    );
    harness.update(
        AnimatedScale::new(2., DURATION, square())
            .alignment(Alignment::TOP_LEFT)
            .into(),
        1_000,
    );
    harness.frame(1_200);
    approx_rect(
        harness.rects()[0],
        Rect::from_origin_size(Offset::ZERO, Size::new(100., 100.)),
    );

    let mut harness = Harness::new(AnimatedRotation::new(0., DURATION, square()).into());
    harness.update(
        AnimatedRotation::new(0.25, DURATION, square()).into(),
        1_000,
    );
    harness.frame(1_100);
    // An eighth of a turn about the center (25, 25): the bounding box of
    // the rotated 50 x 50 square is 50·√2 wide.
    let side = 50. * std::f32::consts::SQRT_2;
    approx_rect(
        harness.rects()[0],
        Rect::from_origin_size(
            Offset::new(25. - side / 2., 25. - side / 2.),
            Size::new(side, side),
        ),
    );

    let mut harness = Harness::new(AnimatedSlide::new(Offset::ZERO, DURATION, square()).into());
    harness.update(
        AnimatedSlide::new(Offset::new(1., 0.), DURATION, square()).into(),
        1_000,
    );
    harness.frame(1_100);
    // Half of one child width.
    assert_eq!(harness.rects()[0].origin, Offset::new(25., 0.));
}

#[test]
fn tween_animation_builder_runs_from_begin_on_first_build() {
    let built = |end: f32| {
        TweenAnimationBuilder::new(end, DURATION, |_, width: f32, _| {
            Widget::box_(Size::new(width, 10.), Color::WHITE)
        })
        .begin(0.)
        .into()
    };
    let mut harness = Harness::new(built(100.));
    // `begin != end`, so the first build starts at `begin` and runs forward.
    assert_eq!(harness.size(), Size::new(0., 10.));
    harness.frame(100);
    assert_eq!(harness.size(), Size::new(50., 10.));
    harness.frame(200);
    assert_eq!(harness.size(), Size::new(100., 10.));
    // A new end animates from the current value; `begin` no longer applies.
    harness.update(built(50.), 1_000);
    harness.frame(1_100);
    assert_eq!(harness.size(), Size::new(75., 10.));
}

#[test]
fn sliver_opacity_keeps_animating_across_rebuilds() {
    let view = |opacity: f32| {
        let sliver: Box<dyn Sliver> = Box::new(SliverAnimatedOpacity::new(
            opacity,
            DURATION,
            SliverToBoxAdapter::new(square()),
        ));
        CustomScrollView::new([sliver]).into()
    };
    let mut harness = Harness::new(view(1.));
    harness.update(view(0.), 1_000);
    harness.frame(1_100);
    assert_eq!(harness.opacities(), [0.5]);
    harness.frame(1_200);
    assert_eq!(harness.opacities(), [0.]);
}
