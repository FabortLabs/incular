//! Retained render samples from Flutter 3.50.0 implicit animations.
use incular_config::Constraints;
use incular_core::{Color, Offset, Size, Transform as Affine};
use incular_rendering::PaintCommand;
use incular_widgets::{
    AnimatedOpacity, AnimatedRotation, AnimatedScale, AnimatedSlide, Center, CustomScrollView,
    Sliver, SliverAnimatedOpacity, SliverToBoxAdapter, Widget,
    internal::{RenderKind, WidgetTree},
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};
const DURATION: Duration = Duration::from_secs(1);
const MARKER: Color = Color::rgba(22, 133, 244, 255);

#[derive(Clone, Copy)]
enum Case {
    Slide,
    Scale,
    Rotation,
    Alpha,
    SliverAlpha,
}
fn scene(case: Case, next: bool) -> Widget {
    let marker = Widget::box_(Size::new(40., 40.), MARKER);
    match case {
        Case::Slide => Center::new(AnimatedSlide::new(
            if next {
                Offset::new(1., 1.)
            } else {
                Offset::ZERO
            },
            DURATION,
            marker,
        ))
        .into(),
        Case::Scale => Center::new(AnimatedScale::new(
            if next { 2. } else { 1. },
            DURATION,
            marker,
        ))
        .into(),
        Case::Rotation => Center::new(AnimatedRotation::new(
            if next { 1.5 } else { 0. },
            DURATION,
            marker,
        ))
        .into(),
        Case::Alpha => Center::new(AnimatedOpacity::new(
            if next { 1. } else { 0. },
            DURATION,
            marker,
        ))
        .into(),
        Case::SliverAlpha => {
            let sample: Box<dyn Sliver> = Box::new(SliverAnimatedOpacity::new(
                if next { 1. } else { 0. },
                DURATION,
                SliverToBoxAdapter::new(marker),
            ));
            CustomScrollView::new([sample]).into()
        }
    }
}

fn alpha(tree: &WidgetTree) -> f32 {
    let mut pending = vec![tree.root().unwrap()];
    while let Some(id) = pending.pop() {
        if let Some(RenderKind::Opacity { alpha, .. }) = tree
            .render_id(id)
            .and_then(|id| tree.render_object_kind(id))
        {
            return *alpha;
        }
        pending.extend(tree.children(id).unwrap_or_default());
    }
    panic!("animation must retain an opacity node");
}
fn painted(tree: &mut WidgetTree) -> (Offset, Offset) {
    let mut stack = vec![Affine::IDENTITY];
    for command in tree.paint().commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                stack.push(stack.last().unwrap().then(*transform))
            }
            PaintCommand::PopTransform => {
                stack.pop();
            }
            PaintCommand::Rect { rect, color } if *color == MARKER => {
                let map = stack.last().unwrap();
                let start = map.transform_point(rect.origin);
                let right = map.transform_point(rect.origin + Offset::new(rect.size.width, 0.));
                return (start, right - start);
            }
            _ => {}
        }
    }
    panic!("paint marker must be recorded");
}
fn test(case: Case) {
    let mut tree = WidgetTree::new();
    let revision = Rc::new(Cell::new(0));
    let next = Rc::new(Cell::new(false));
    let builds = Rc::new(Cell::new(0));
    let read_next = next.clone();
    let count_builds = builds.clone();
    tree.mount(Widget::stateful_builder(revision.clone(), move |_| {
        count_builds.set(count_builds.get() + 1);
        scene(case, read_next.get())
    }))
    .unwrap();
    let origin = Instant::now();
    let frame = |tree: &mut WidgetTree, ms| {
        tree.update_compositor(origin + Duration::from_millis(ms))
            .unwrap();
        tree.layout(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
    };
    frame(&mut tree, 0);
    assert_eq!(builds.get(), 1);
    next.set(true);
    revision.set(1);
    assert_eq!(builds.get(), 1);
    frame(&mut tree, 16);
    frame(&mut tree, 16);
    let first_pos = if matches!(case, Case::Slide) {
        painted(&mut tree).0
    } else {
        Offset::ZERO
    };
    for (ms, t) in [(0, 0.), (500, 0.5), (750, 0.75), (1000, 1.)] {
        frame(&mut tree, 16 + ms);
        assert_eq!(builds.get(), 2, "no rebuild of the caller at {ms} ms");
        let (actual, expected) = match case {
            Case::Slide => (
                painted(&mut tree).0 - first_pos,
                Offset::new(40. * t, 40. * t),
            ),
            Case::Scale => (painted(&mut tree).1, Offset::new(40. * (1. + t), 0.)),
            Case::Rotation => {
                let a = 1.5 * t * std::f32::consts::TAU;
                (
                    painted(&mut tree).1,
                    Offset::new(40. * a.cos(), 40. * a.sin()),
                )
            }
            Case::Alpha | Case::SliverAlpha => (Offset::new(alpha(&tree), 0.), Offset::new(t, 0.)),
        };
        assert!(
            (actual.x - expected.x).abs() < 0.001 && (actual.y - expected.y).abs() < 0.001,
            "elapsed {ms} ms: actual={actual:?} expected={expected:?}"
        );
    }
}

#[test]
fn slide_samples_follow_child_fraction() {
    test(Case::Slide);
}
#[test]
fn scale_samples_preserve_linear_progress() {
    test(Case::Scale);
}
#[test]
fn rotation_samples_preserve_full_turns() {
    test(Case::Rotation);
}
#[test]
fn box_alpha_samples_preserve_linear_progress() {
    test(Case::Alpha);
}
#[test]
fn sliver_alpha_samples_preserve_linear_progress() {
    test(Case::SliverAlpha);
}
