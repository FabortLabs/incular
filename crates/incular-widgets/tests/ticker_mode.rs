//! Flutter's `TickerMode`: a disabled subtree's animations stop ticking but
//! keep time, nested modes combine, and `TickerMode::of` reports the mode a
//! build runs under.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_animation::{AnimationController, AnimationStatus};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Builder, TickerMode, Widget};

fn mount(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    tree
}

fn ticking(controller: &AnimationController) -> Widget {
    Widget::animation_ticker(
        controller.clone(),
        Widget::box_(Size::new(10., 10.), Color::WHITE),
    )
}

#[test]
fn a_disabled_ticker_mode_mutes_its_animations_until_enabled() {
    let controller = AnimationController::new(Duration::from_millis(100));
    let enabled = Rc::new(Cell::new(false));
    let mut tree = mount(Builder::new({
        let (controller, enabled) = (controller.clone(), enabled.clone());
        move |_| TickerMode::new(enabled.get(), ticking(&controller)).into()
    }));
    let start = Instant::now();
    controller.forward(start);
    tree.tick_animations(start + Duration::from_millis(50));
    let _ = tree.update_compositor(start + Duration::from_millis(50));
    assert_eq!(controller.value(), 0.0, "muted: no ticks");
    assert_eq!(controller.status(), AnimationStatus::Forward);

    // Enabled again, the controller catches up with the time that passed.
    enabled.set(true);
    let mut tree = mount(TickerMode::new(true, ticking(&controller)));
    tree.tick_animations(start + Duration::from_millis(60));
    assert!((controller.value() - 0.6).abs() < 1e-5);
}

#[test]
fn nested_ticker_modes_combine() {
    let controller = AnimationController::new(Duration::from_millis(100));
    let mut tree = mount(TickerMode::new(
        false,
        TickerMode::new(true, ticking(&controller)),
    ));
    let start = Instant::now();
    controller.forward(start);
    tree.tick_animations(start + Duration::from_millis(50));
    let _ = tree.update_compositor(start + Duration::from_millis(50));
    assert_eq!(controller.value(), 0.0, "an outer disabled mode wins");
}

#[test]
fn ticker_mode_of_reports_the_enclosing_mode() {
    let seen = Rc::new(Cell::new(None));
    let probe = |seen: &Rc<Cell<Option<bool>>>| {
        let seen = seen.clone();
        Builder::new(move |context| {
            seen.set(Some(TickerMode::of(context)));
            Widget::box_(Size::new(1., 1.), Color::WHITE)
        })
    };
    mount(probe(&seen));
    assert_eq!(seen.get(), Some(true), "enabled without a TickerMode");
    mount(TickerMode::new(false, TickerMode::new(true, probe(&seen))));
    assert_eq!(seen.get(), Some(false));
    mount(TickerMode::new(true, probe(&seen)));
    assert_eq!(seen.get(), Some(true));
}

/// anim_core-005: a self-ticking (`with_vsync`) controller follows the
/// `TickerMode` around the node that ticks it, as Flutter's state-created
/// tickers do (widgets/ticker_provider.dart `_WidgetTickerProviderStateMixin`
/// updates `ticker.muted` from `TickerMode.getNotifier`).
#[test]
fn a_vsync_controller_follows_the_enclosing_ticker_mode() {
    use incular_animation::scheduler;
    let controller = AnimationController::new(Duration::from_millis(100)).with_vsync();
    let start = Instant::now();
    let tree = {
        let mut tree = mount(TickerMode::new(false, ticking(&controller)));
        controller.forward(start);
        let _ = tree.update_compositor(start);
        tree
    };
    scheduler::begin_frame(start + Duration::from_millis(50));
    scheduler::end_frame(start + Duration::from_millis(50));
    assert_eq!(
        controller.value(),
        0.0,
        "the scheduler skips a muted ticker"
    );
    assert!(
        !scheduler::has_scheduled_frame(),
        "a muted ticker asks for no frame"
    );
    drop(tree);

    let mut tree = mount(TickerMode::new(true, ticking(&controller)));
    let _ = tree.update_compositor(start + Duration::from_millis(60));
    scheduler::begin_frame(start + Duration::from_millis(70));
    scheduler::end_frame(start + Duration::from_millis(70));
    assert!(
        (controller.value() - 0.7).abs() < 1e-5,
        "{}",
        controller.value()
    );
}
