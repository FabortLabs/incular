//! Flutter's `AnimatedBuilder`, `ListenableBuilder` and the transitions that
//! follow an animation (`widgets/transitions.dart`): builders rerun when the
//! animation changes, compositor transitions update only their layer, and
//! `SizeTransition`, `AlignTransition`, `PositionedTransition`,
//! `RelativePositionedTransition` and `MatrixTransition` follow their
//! animation instead of a fixed value.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_animation::{AnimationController, Curve, CurvedAnimation, ReverseAnimation};
use incular_config::{Alignment, Constraints};
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{DisplayList, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    AlignTransition, AnimatedBuilder, FadeTransition, ListenableBuilder, MatrixTransition,
    PositionedTransition, RelativePositionedTransition, RelativeRect, SizeTransition, SizedBox,
    Stack, TweenAnimation, Widget,
};

fn mount(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).expect("mount");
    layout(&mut tree);
    tree
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
}

fn red_box(width: f32, height: f32) -> Widget {
    Widget::box_(Size::new(width, height), Color::rgba(255, 0, 0, 255))
}

/// The rectangle of the red box in the frame's paint order.
fn red_rect(list: &DisplayList) -> Option<Rect> {
    let mut translation = Offset::ZERO;
    let mut stack = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                stack.push(translation);
                translation = translation + transform.transform_point(Offset::ZERO);
            }
            PaintCommand::PopTransform => translation = stack.pop().unwrap_or(Offset::ZERO),
            PaintCommand::Rect { rect, color } if color.red == 255 && color.green == 0 => {
                return Some(Rect::from_origin_size(rect.origin + translation, rect.size));
            }
            _ => {}
        }
    }
    None
}

#[test]
fn animated_builder_reruns_when_the_animation_changes() {
    let controller = AnimationController::new(Duration::from_millis(100));
    let builds = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new(-1.0));
    let mut tree = mount(
        AnimatedBuilder::new(controller.clone(), {
            let (builds, seen, controller) = (builds.clone(), seen.clone(), controller.clone());
            move |_, child| {
                builds.set(builds.get() + 1);
                seen.set(controller.value());
                child.expect("the child is passed through")
            }
        })
        .child(red_box(10., 10.)),
    );
    assert_eq!((builds.get(), seen.get()), (1, 0.0));
    controller.set_value(0.5);
    layout(&mut tree);
    assert_eq!(seen.get(), 0.5);
    assert_eq!(builds.get(), 2);
}

#[test]
fn listenable_builder_follows_a_curved_animation() {
    let controller = AnimationController::new(Duration::from_millis(100));
    let curved = incular_animation::DoubleAnimation::from(CurvedAnimation::new(
        controller.clone(),
        Curve::EaseIn,
    ));
    let seen = Rc::new(Cell::new(-1.0));
    let mut tree = mount(ListenableBuilder::new(curved.clone(), {
        let (seen, curved) = (seen.clone(), curved.clone());
        move |_, _| {
            seen.set(curved.value());
            red_box(10., 10.)
        }
    }));
    controller.set_value(0.5);
    layout(&mut tree);
    assert!((seen.get() - Curve::EaseIn.apply(0.5)).abs() < 1e-6);
}

#[test]
fn fade_transition_from_an_animation_updates_only_its_layer() {
    let controller = AnimationController::new(Duration::from_millis(100));
    let builds = Rc::new(Cell::new(0));
    let child = incular_widgets::Builder::new({
        let builds = builds.clone();
        move |_| {
            builds.set(builds.get() + 1);
            red_box(10., 10.)
        }
    });
    let mut tree = mount(FadeTransition::from_animation(
        ReverseAnimation::new(controller.clone()),
        child,
    ));
    let opacity = |tree: &mut WidgetTree| {
        tree.paint()
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::PushOpacity { alpha, .. } => Some(*alpha),
                _ => None,
            })
    };
    // `1 - 0`: fully opaque, so no opacity layer may be needed.
    assert!(opacity(&mut tree).is_none_or(|value| value == 1.0));
    let start = incular_core::time::Instant::now();
    controller.forward(start);
    tree.update_compositor(start + Duration::from_millis(25))
        .unwrap();
    layout(&mut tree);
    let value = opacity(&mut tree).expect("an opacity layer while fading");
    assert!((value - 0.75).abs() < 1e-3, "opacity {value}");
    assert_eq!(builds.get(), 1, "the child is not rebuilt");
}

#[test]
fn size_transition_clips_the_child_to_its_factor() {
    let controller = AnimationController::new(Duration::from_millis(100));
    let mut tree = mount(Stack::new([Widget::from(
        incular_widgets::Positioned::new(
            SizeTransition::new(controller.clone(), red_box(40., 60.))
                .fixed_cross_axis_size_factor(1.0),
        )
        .left(0.)
        .top(0.),
    )]));
    controller.set_value(0.5);
    layout(&mut tree);
    let list = tree.paint();
    let clips: Vec<Size> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushClip { rect } => Some(rect.size),
            _ => None,
        })
        .collect();
    // Flutter: `ClipRect(Align(heightFactor: 0.5))` over a 40x60 child.
    assert!(clips.contains(&Size::new(40., 30.)), "clips {clips:?}");
    // A plain factor still works, as a stopped animation.
    let fixed = mount(SizeTransition::new(0.25, red_box(40., 60.)));
    let _ = fixed;
}

#[test]
fn align_and_positioned_transitions_follow_their_animations() {
    let controller = AnimationController::new(Duration::from_millis(100));
    let mut tree = mount(AlignTransition::new(
        TweenAnimation::new(
            controller.clone(),
            Alignment::TOP_LEFT,
            Alignment::BOTTOM_RIGHT,
        ),
        red_box(20., 20.),
    ));
    assert_eq!(
        red_rect(&tree.paint()).map(|rect| rect.origin),
        Some(Offset::ZERO)
    );
    controller.set_value(1.0);
    layout(&mut tree);
    assert_eq!(
        red_rect(&tree.paint()).map(|rect| rect.origin),
        Some(Offset::new(80., 80.))
    );

    let positioned = AnimationController::new(Duration::from_millis(100));
    let mut tree = mount(Stack::new([Widget::from(PositionedTransition::new(
        TweenAnimation::new(
            positioned.clone(),
            RelativeRect::from_ltrb(0., 0., 80., 80.),
            RelativeRect::from_ltrb(50., 50., 30., 30.),
        ),
        SizedBox::expand().child(red_box(1., 1.)),
    ))]));
    positioned.set_value(0.5);
    layout(&mut tree);
    // Each side lerps: left 25, top 25, right 55, bottom 55.
    assert_eq!(
        red_rect(&tree.paint()),
        Some(Rect::from_origin_size(
            Offset::new(25., 25.),
            Size::new(20., 20.)
        ))
    );

    let relative = AnimationController::new(Duration::from_millis(100));
    let mut tree = mount(Stack::new([Widget::from(
        RelativePositionedTransition::new(
            TweenAnimation::new(
                relative.clone(),
                Rect::from_origin_size(Offset::ZERO, Size::new(10., 10.)),
                Rect::from_origin_size(Offset::new(40., 20.), Size::new(30., 30.)),
            ),
            Size::new(100., 100.),
            SizedBox::expand().child(red_box(1., 1.)),
        ),
    )]));
    relative.set_value(1.0);
    layout(&mut tree);
    assert_eq!(
        red_rect(&tree.paint()),
        Some(Rect::from_origin_size(
            Offset::new(40., 20.),
            Size::new(30., 30.)
        ))
    );
}

#[test]
fn matrix_transition_applies_the_transform_of_the_animation() {
    let controller = AnimationController::new(Duration::from_millis(100));
    let mut tree = mount(
        MatrixTransition::new(
            controller.clone(),
            |value| CoreTransform::translation(Offset::new(value * 30., 0.)),
            red_box(10., 10.),
        )
        .alignment(Alignment::TOP_LEFT),
    );
    controller.set_value(0.5);
    layout(&mut tree);
    let translated = tree.paint().commands().iter().any(|command| {
        matches!(command, PaintCommand::PushTransform { transform }
            if (transform.transform_point(Offset::ZERO).x - 15.).abs() < 1e-3)
    });
    assert!(translated, "the child moves by 0.5 * 30");
}

#[test]
fn transition_controllers_can_start_on_the_next_frame() {
    // A slow build between the request and the first frame does not eat
    // into the animation: its clock starts at the first frame.
    let opacity = incular_widgets::OpacityController::new();
    let mut tree = mount(FadeTransition::new(opacity.clone(), red_box(10., 10.)));
    opacity.animate_to_on_next_frame(0.0, Duration::from_millis(100));
    let first = incular_core::time::Instant::now() + Duration::from_secs(5);
    tree.update_compositor(first).unwrap();
    assert_eq!(opacity.opacity(), 1.0);
    tree.update_compositor(first + Duration::from_millis(50))
        .unwrap();
    assert!(
        (opacity.opacity() - 0.5).abs() < 1e-3,
        "{}",
        opacity.opacity()
    );
    opacity.animate_to_curved(
        1.0,
        Duration::from_millis(100),
        Curve::EaseIn,
        Some(first + Duration::from_millis(50)),
    );
    tree.update_compositor(first + Duration::from_millis(100))
        .unwrap();
    let expected = 0.5 + 0.5 * Curve::EaseIn.apply(0.5);
    assert!(
        (opacity.opacity() - expected).abs() < 1e-3,
        "{}",
        opacity.opacity()
    );
}
