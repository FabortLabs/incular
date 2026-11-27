//! Flutter's `DecoratedBoxTransition`, `DefaultTextStyleTransition`
//! (`widgets/transitions.dart`) and `DualTransitionBuilder`
//! (`widgets/dual_transition_builder.dart`): each rebuilds whenever its
//! animation changes.

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_animation::{AnimationController, AnimationStatus, Curve};
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::{Brush, DisplayList, PaintCommand};
use incular_text::TextStyle;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    BoxDecoration, Builder, DecoratedBoxTransition, DecorationPosition, DefaultTextStyleTransition,
    DualTransitionBuilder, SizedBox, TweenAnimation, Widget,
};

const DURATION: Duration = Duration::from_millis(100);

fn mount(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).expect("mount");
    layout(&mut tree);
    tree
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .expect("layout");
}

/// Solid colors in paint order.
fn painted_colors(list: &DisplayList) -> Vec<Color> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { color, .. } => Some(*color),
            PaintCommand::RRect {
                brush: Brush::Solid(color),
                ..
            }
            | PaintCommand::FillPath {
                brush: Brush::Solid(color),
                ..
            } => Some(*color),
            _ => None,
        })
        .collect()
}

const BLACK: Color = Color::rgba(0, 0, 0, 255);
const ORANGE: Color = Color::rgba(200, 100, 0, 255);

fn decoration_animation(controller: &AnimationController) -> TweenAnimation<BoxDecoration> {
    TweenAnimation::new(
        controller.clone(),
        BoxDecoration::new().color(BLACK),
        BoxDecoration::new().color(ORANGE),
    )
}

#[test]
fn tween_animation_evaluates_the_tween_at_the_curved_value() {
    let controller = AnimationController::new(DURATION);
    let animation = decoration_animation(&controller);
    assert_eq!(animation.value().color, Some(BLACK));
    controller.set_value(0.25);
    // 0 + (200 - 0) * 0.25 = 50 and 0 + (100 - 0) * 0.25 = 25.
    assert_eq!(animation.value().color, Some(Color::rgba(50, 25, 0, 255)));
    controller.set_value(1.0);
    assert_eq!(animation.value().color, Some(ORANGE));

    // The curve applies before the tween: 100 * easeIn(0.5).
    let eased = TweenAnimation::new(controller.clone(), 0.0_f32, 100.0).curve(Curve::EaseIn);
    controller.set_value(0.5);
    assert!((eased.value() - Curve::EaseIn.apply(0.5) * 100.0).abs() < 1e-4);
}

#[test]
fn decorated_box_transition_repaints_when_the_animation_changes() {
    let controller = AnimationController::new(DURATION);
    let mut tree = mount(DecoratedBoxTransition::new(
        decoration_animation(&controller),
        SizedBox::new().width(40.).height(40.),
    ));
    assert_eq!(painted_colors(&tree.paint()), vec![BLACK]);

    controller.set_value(0.5);
    layout(&mut tree);
    // 200 * 0.5 = 100 and 100 * 0.5 = 50.
    assert_eq!(
        painted_colors(&tree.paint()),
        vec![Color::rgba(100, 50, 0, 255)]
    );

    let start = Instant::now();
    controller.forward(start);
    controller.tick(start + DURATION);
    layout(&mut tree);
    assert_eq!(painted_colors(&tree.paint()), vec![ORANGE]);
}

#[test]
fn decorated_box_transition_paints_a_foreground_decoration_over_the_child() {
    let controller = AnimationController::new(DURATION);
    let child = Widget::box_(Size::new(40., 40.), Color::WHITE);
    let mut tree = mount(
        DecoratedBoxTransition::new(decoration_animation(&controller), child.clone())
            .position(DecorationPosition::Foreground),
    );
    assert_eq!(painted_colors(&tree.paint()), vec![Color::WHITE, BLACK]);

    let mut background = mount(DecoratedBoxTransition::new(
        decoration_animation(&controller),
        child,
    ));
    assert_eq!(
        painted_colors(&background.paint()),
        vec![BLACK, Color::WHITE]
    );
}

#[test]
fn default_text_style_transition_rebuilds_descendants_with_the_style() {
    let controller = AnimationController::new(DURATION);
    let seen = Rc::new(Cell::new(0.0_f32));
    let reader = {
        let seen = seen.clone();
        Builder::new(move |context| {
            seen.set(context.depend_on::<TextStyle>().expect("style").size);
            SizedBox::new().width(10.).height(10.).into()
        })
    };
    let style = |size: f32| TextStyle {
        size,
        ..TextStyle::default()
    };
    let mut tree = mount(DefaultTextStyleTransition::new(
        TweenAnimation::new(controller.clone(), style(10.), style(20.)),
        reader,
    ));
    assert_eq!(seen.get(), 10.);
    controller.set_value(0.5);
    layout(&mut tree);
    // 10 + (20 - 10) * 0.5.
    assert_eq!(seen.get(), 15.);
}

#[test]
fn an_interrupted_dual_transition_keeps_playing_the_transition_it_started() {
    use AnimationStatus::{Completed, Dismissed, Forward, Reverse};
    let effective = DualTransitionBuilder::effective_status;
    // Settled statuses always win.
    for last in [Dismissed, Forward, Reverse, Completed] {
        assert_eq!(effective(last, Dismissed), Dismissed);
        assert_eq!(effective(last, Completed), Completed);
    }
    assert_eq!(effective(Dismissed, Forward), Forward);
    assert_eq!(effective(Completed, Reverse), Reverse);
    // A turn midway keeps the running transition.
    assert_eq!(effective(Forward, Reverse), Forward);
    assert_eq!(effective(Reverse, Forward), Reverse);

    let values = DualTransitionBuilder::builder_values;
    assert_eq!(values(Dismissed, 0.0), (0.0, 0.0));
    assert_eq!(values(Forward, 0.25), (0.25, 0.0));
    assert_eq!(values(Completed, 1.0), (1.0, 0.0));
    assert_eq!(values(Reverse, 0.25), (1.0, 0.75));
}

#[test]
fn dual_transition_builder_routes_the_animation_to_the_running_side() {
    let controller = AnimationController::new(DURATION);
    let forward_value = Rc::new(Cell::new(f32::NAN));
    let reverse_value = Rc::new(Cell::new(f32::NAN));
    let builder = {
        let forward_value = forward_value.clone();
        let reverse_value = reverse_value.clone();
        DualTransitionBuilder::new(
            controller.clone(),
            move |_, value, child| {
                forward_value.set(value);
                child.expect("the reverse side")
            },
            move |_, value, _| {
                reverse_value.set(value);
                SizedBox::new().width(10.).height(10.).into()
            },
        )
    };
    let mut tree = mount(builder);
    let reverse = || reverse_value.get();
    assert_eq!((forward_value.get(), reverse()), (0.0, 0.0));

    // Forward: the forward side follows the animation.
    let start = Instant::now();
    controller.forward(start);
    controller.tick(start);
    controller.tick(start + Duration::from_millis(50));
    layout(&mut tree);
    assert_eq!(controller.status(), AnimationStatus::Forward);
    assert!((forward_value.get() - 0.5).abs() < 1e-4);
    assert_eq!(reverse(), 0.0);

    // Reversed midway: still the forward transition, now running back.
    let turn = start + Duration::from_millis(50);
    controller.reverse(turn);
    controller.tick(turn);
    controller.tick(turn + Duration::from_millis(20));
    layout(&mut tree);
    assert_eq!(controller.status(), AnimationStatus::Reverse);
    assert!((forward_value.get() - 0.3).abs() < 1e-4);
    assert_eq!(reverse(), 0.0);

    // Completed: the forward side holds at 1 and the reverse side at 0.
    controller.set_value(1.0);
    layout(&mut tree);
    assert_eq!(controller.status(), AnimationStatus::Completed);
    assert_eq!((forward_value.get(), reverse()), (1.0, 0.0));

    // Reverse from the end: the reverse side follows `1 - value`.
    let back = turn + Duration::from_millis(100);
    controller.reverse(back);
    controller.tick(back);
    controller.tick(back + Duration::from_millis(25));
    layout(&mut tree);
    assert_eq!(controller.status(), AnimationStatus::Reverse);
    assert_eq!(forward_value.get(), 1.0);
    assert!((reverse() - 0.25).abs() < 1e-4);
}

/// Bounds of the deepest element under `widget`, which is the leaf box.
fn leaf_bounds(widget: Widget) -> incular_core::Rect {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).expect("mount");
    layout(&mut tree);
    let mut leaf = root;
    while let Some(&child) = tree.children(leaf).and_then(|kids| kids.first()) {
        leaf = child;
    }
    tree.element_bounds(leaf).expect("bounds")
}

/// Flutter `SlideTransition(textDirection: TextDirection.rtl)` mirrors the
/// horizontal offset; without a direction it applies as given.
#[test]
fn slide_transition_mirrors_the_horizontal_offset_in_rtl() {
    use incular_animation::{DrivenAnimation, LinearTween};
    use incular_config::TextDirection;
    use incular_core::Offset;
    use incular_widgets::SlideTransition;

    let controller = AnimationController::new(DURATION);
    controller.set_value(1.0);
    let position = |controller: &AnimationController| {
        DrivenAnimation::new(
            controller.clone(),
            LinearTween {
                begin: Offset::ZERO,
                end: Offset::new(0.4, 0.),
            },
        )
    };
    let boxed = || Widget::box_(Size::new(50., 20.), Color::WHITE);
    // Fractions of the 50 px child: 0.4 * 50 = 20.
    let ltr = leaf_bounds(SlideTransition::from_animation_in_direction(
        position(&controller),
        Some(TextDirection::Ltr),
        boxed(),
    ));
    let rtl = leaf_bounds(SlideTransition::from_animation_in_direction(
        position(&controller),
        Some(TextDirection::Rtl),
        boxed(),
    ));
    let unset = leaf_bounds(SlideTransition::from_animation(
        position(&controller),
        boxed(),
    ));
    assert!((ltr.origin.x - 20.).abs() < 1e-3, "{ltr:?}");
    assert!((rtl.origin.x + 20.).abs() < 1e-3, "{rtl:?}");
    assert!((unset.origin.x - 20.).abs() < 1e-3, "{unset:?}");
}

#[test]
fn relative_rect_tween_interpolates_each_side() {
    use incular_widgets::{RelativeRect, RelativeRectTween};

    let begin = RelativeRect::from_ltrb(0., 0., 10., 10.);
    let end = RelativeRect::from_ltrb(20., 40., 30., 50.);
    let tween = RelativeRectTween::new(begin, end);
    assert_eq!(tween.begin(), begin);
    assert_eq!(tween.end(), end);
    assert_eq!(tween.lerp(0.), begin);
    assert_eq!(tween.lerp(1.), end);
    assert_eq!(tween.lerp(0.5), RelativeRect::from_ltrb(10., 20., 20., 30.));

    let controller = AnimationController::new(DURATION);
    controller.set_value(0.5);
    let animation = tween.animate(controller.clone());
    assert_eq!(
        animation.value(),
        RelativeRect::from_ltrb(10., 20., 20., 30.)
    );
}

/// Flutter's `Animation<Rect?>` form: a null endpoint lerps from or to the
/// zero rectangle, as `Rect.lerp` does.
#[test]
fn relative_positioned_transition_scales_a_null_endpoint_from_zero() {
    use incular_core::{Offset, Rect};
    use incular_widgets::{RelativePositionedTransition, Stack};

    let controller = AnimationController::new(DURATION);
    controller.set_value(0.5);
    let end = Rect::from_origin_size(Offset::new(10., 10.), Size::new(30., 20.));
    let transition = RelativePositionedTransition::from_optional_rects(
        controller.clone(),
        None,
        Some(end),
        Size::new(100., 100.),
        Widget::box_(Size::new(1., 1.), Color::WHITE),
    );
    let bounds = leaf_bounds(Stack::new([Widget::from(transition)]).into());
    // Halfway from the zero rectangle to `end`: origin (5, 5).
    assert!((bounds.origin.x - 5.).abs() < 1e-3, "{bounds:?}");
    assert!((bounds.origin.y - 5.).abs() < 1e-3, "{bounds:?}");
}

fn shape_decoration(color: Color) -> incular_widgets::ShapeDecoration {
    use incular_widgets::{BorderSide, CircleBorder, ShapeDecoration};
    ShapeDecoration::new(CircleBorder::new(BorderSide::default())).color(color)
}

/// transitions-005: a `ShapeDecoration` tween animates its color through
/// `DecoratedBoxTransition`, as Flutter's `Animation<Decoration>` does.
#[test]
fn decorated_box_transition_animates_a_shape_decoration() {
    use incular_widgets::Decoration;
    let controller = AnimationController::new(DURATION);
    let tween = TweenAnimation::new(
        controller.clone(),
        Decoration::Shape(shape_decoration(BLACK)),
        Decoration::Shape(shape_decoration(ORANGE)),
    );
    let mut tree = mount(DecoratedBoxTransition::new(
        tween,
        SizedBox::new().width(40.).height(40.),
    ));
    assert_eq!(painted_colors(&tree.paint()), vec![BLACK]);

    controller.set_value(0.5);
    layout(&mut tree);
    assert_eq!(
        painted_colors(&tree.paint()),
        vec![Color::rgba(100, 50, 0, 255)]
    );
}

/// A mixed Flutter decoration tween converts the box to a shape so
/// the mounted decoration's fill interpolates continuously.
#[test]
fn decorated_box_transition_interpolates_the_fill_of_mixed_decoration_kinds() {
    use incular_widgets::Decoration;
    let controller = AnimationController::new(DURATION);
    let tween = TweenAnimation::new(
        controller.clone(),
        Decoration::Box(BoxDecoration::new().color(BLACK)),
        Decoration::Shape(shape_decoration(ORANGE)),
    );
    let mut tree = mount(DecoratedBoxTransition::new(
        tween,
        SizedBox::new().width(40.).height(40.),
    ));
    controller.set_value(0.25);
    layout(&mut tree);
    assert_eq!(painted_colors(&tree.paint()), vec![Color::rgba(50, 25, 0, 255)]);

    controller.set_value(0.75);
    layout(&mut tree);
    assert_eq!(painted_colors(&tree.paint()), vec![Color::rgba(150, 75, 0, 255)]);
}

/// The sampling of every matrix filter stage in paint order
/// (transitions-003).
fn matrix_filter_samplings(list: &DisplayList) -> Vec<incular_rendering::ImageSampling> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushMatrixFilter { sampling, .. } => Some(*sampling),
            _ => None,
        })
        .collect()
}

#[test]
fn transform_filter_quality_resamples_the_child_through_a_matrix_filter() {
    // Flutter `RenderTransform.paint`: a filterQuality paints the child into
    // `ImageFilterLayer(ImageFilter.matrix(transform, filterQuality))`.
    use incular_rendering::{FilterQuality, ImageSampling};
    let child = Widget::box_(Size::new(20., 20.), Color::WHITE);
    let mut plain = mount(incular_widgets::Transform::scale(2., child.clone()));
    assert!(matrix_filter_samplings(&plain.paint()).is_empty());
    let mut nearest = mount(
        incular_widgets::Transform::scale(2., child.clone()).filter_quality(FilterQuality::None),
    );
    assert_eq!(
        matrix_filter_samplings(&nearest.paint()),
        [ImageSampling::Nearest]
    );
    let mut linear =
        mount(incular_widgets::Transform::scale(2., child).filter_quality(FilterQuality::Low));
    assert_eq!(
        matrix_filter_samplings(&linear.paint()),
        [ImageSampling::Linear]
    );
}

#[test]
fn matrix_transition_filter_quality_applies_only_while_animating() {
    // Flutter `MatrixTransition.build`: `filterQuality: animation.isAnimating
    // ? filterQuality : null`.
    use incular_rendering::{FilterQuality, ImageSampling};
    let controller = AnimationController::new(DURATION);
    let mut tree = mount(
        incular_widgets::MatrixTransition::new(
            controller.clone(),
            |value| incular_core::Transform::scale(1. + value),
            Widget::box_(Size::new(20., 20.), Color::WHITE),
        )
        .filter_quality(FilterQuality::None),
    );
    assert!(
        matrix_filter_samplings(&tree.paint()).is_empty(),
        "dismissed"
    );
    let start = Instant::now();
    controller.forward(start);
    controller.tick(start + DURATION / 2);
    layout(&mut tree);
    assert_eq!(
        matrix_filter_samplings(&tree.paint()),
        [ImageSampling::Nearest]
    );
    controller.tick(start + DURATION * 2);
    assert_eq!(controller.status(), AnimationStatus::Completed);
    layout(&mut tree);
    assert!(
        matrix_filter_samplings(&tree.paint()).is_empty(),
        "completed"
    );
}

#[test]
fn scale_transition_filter_quality_applies_only_while_the_scale_animates() {
    // Flutter `ScaleTransition` is a `MatrixTransition`: the filter quality
    // applies while the animation runs and is dropped at rest.
    use incular_rendering::{FilterQuality, ImageSampling};
    let scale = incular_widgets::ScaleController::new();
    let mut tree = mount(
        incular_widgets::ScaleTransition::new(
            scale.clone(),
            Widget::box_(Size::new(20., 20.), Color::WHITE),
        )
        .filter_quality(FilterQuality::Medium),
    );
    let start = Instant::now();
    let _ = tree.update_compositor(start).expect("compositor update");
    assert!(matrix_filter_samplings(&tree.paint()).is_empty(), "at rest");
    scale.animate_to(2., DURATION, start);
    let _ = tree
        .update_compositor(start + DURATION / 2)
        .expect("compositor update");
    assert_eq!(
        matrix_filter_samplings(&tree.paint()),
        [ImageSampling::Linear]
    );
    for step in 1..=4 {
        let _ = tree
            .update_compositor(start + DURATION * step)
            .expect("compositor update");
    }
    assert!((scale.scale() - 2.).abs() < 1e-4);
    assert!(matrix_filter_samplings(&tree.paint()).is_empty(), "settled");
}

#[test]
fn shape_decoration_transition_respects_order_and_keeps_the_child_tappable() {
    use incular_core::{Offset, PointerDeviceKind, PointerPhase};
    use incular_gestures::PointerEvent;
    use incular_widgets::{GestureDetector, HitTestBehavior};

    const RED: Color = Color::rgba(255, 0, 0, 255);
    const GREEN: Color = Color::rgba(0, 255, 0, 255);
    let controller = AnimationController::new(DURATION);
    let taps = Rc::new(Cell::new(0));
    for (position, expected) in [
        (DecorationPosition::Foreground, [RED, GREEN]),
        (DecorationPosition::Background, [GREEN, RED]),
    ] {
        let child = GestureDetector::new(Widget::box_(Size::new(40., 40.), RED))
            .behavior(HitTestBehavior::Opaque)
            .on_tap({
                let taps = Rc::clone(&taps);
                move || taps.set(taps.get() + 1)
            });
        let mut tree = mount(
            DecoratedBoxTransition::new(
                TweenAnimation::new(
                    controller.clone(),
                    shape_decoration(GREEN),
                    shape_decoration(GREEN),
                ),
                child,
            )
            .position(position),
        );
        assert_eq!(painted_colors(&tree.paint()), expected);

        let now = Instant::now();
        for (phase, time) in [
            (PointerPhase::Down, now),
            (PointerPhase::Up, now + Duration::from_millis(10)),
        ] {
            let _ = tree.dispatch_pointer_gesture_in_window(
                0,
                1,
                PointerDeviceKind::Touch,
                PointerEvent {
                    phase,
                    time,
                    pointer: 1,
                    position: Offset::new(20., 20.),
                },
            );
        }
    }
    assert_eq!(taps.get(), 2);
}

#[test]
fn mixed_decoration_honors_foreground_through_both_endpoints_and_midpoints() {
    use incular_widgets::Decoration;
    const RED: Color = Color::rgba(255, 0, 0, 255);
    const GREEN: Color = Color::rgba(0, 255, 0, 255);
    let controller = AnimationController::new(DURATION);
    let tween = TweenAnimation::new(
        controller.clone(),
        Decoration::Box(BoxDecoration::new().color(GREEN)),
        Decoration::Shape(shape_decoration(GREEN)),
    );
    let mut tree = mount(
        DecoratedBoxTransition::new(tween, Widget::box_(Size::new(40., 40.), RED))
            .position(DecorationPosition::Foreground),
    );
    for time in [0., 0.25, 0.5, 0.75, 1.] {
        controller.set_value(time);
        layout(&mut tree);
        assert_eq!(
            painted_colors(&tree.paint()),
            [RED, GREEN],
            "fraction {time}"
        );
    }
}
