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
