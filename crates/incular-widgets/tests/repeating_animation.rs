//! Flutter's `RepeatingAnimationBuilder` (`_RepeatingSimulation` cycles
//! from the current value) and `SliverFadeTransition`.

use std::time::{Duration, Instant};

use incular_animation::TweenValue;
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::PaintCommand;
use incular_widgets::internal::{ElementId, OpacityController, WidgetTree};
use incular_widgets::{
    CustomScrollView, RepeatMode, RepeatingAnimationBuilder, Sliver, SliverFadeTransition,
    SliverToBoxAdapter, Widget,
};

const PERIOD: Duration = Duration::from_millis(200);

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
        // The first frame builds; the second starts the repeat.
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

    fn update(&mut self, widget: Widget, millis: u64) {
        self.tree.update(self.root, widget).expect("update");
        self.frame(millis);
        self.frame(millis);
    }

    fn width(&self) -> f32 {
        self.tree
            .element_bounds(self.root)
            .expect("bounds")
            .size
            .width
    }
}

fn bar(mode: RepeatMode, paused: bool) -> Widget {
    RepeatingAnimationBuilder::new(TweenValue::new(0_f32, 100.), PERIOD, |_, width, _| {
        Widget::box_(Size::new(width, 10.), Color::WHITE)
    })
    .repeat_mode(mode)
    .paused(paused)
    .into()
}

fn approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-3,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn restart_mode_runs_every_cycle_forward() {
    let mut harness = Harness::new(bar(RepeatMode::Restart, false));
    harness.frame(100);
    approx(harness.width(), 50.);
    // A quarter into the second cycle.
    harness.frame(250);
    approx(harness.width(), 25.);
}

#[test]
fn reverse_mode_runs_every_other_cycle_backward() {
    let mut harness = Harness::new(bar(RepeatMode::Reverse, false));
    harness.frame(250);
    // A quarter into the backward cycle: 100 + (0 - 100) * 0.25.
    approx(harness.width(), 75.);
}

#[test]
fn pausing_holds_the_value_and_resuming_continues_from_it() {
    let mut harness = Harness::new(bar(RepeatMode::Restart, false));
    harness.frame(100);
    harness.update(bar(RepeatMode::Restart, true), 100);
    harness.frame(500);
    approx(harness.width(), 50.);
    // Resumed at 1000 ms from 50: `_initialT` is half a period, so 50 ms
    // later the cycle is three quarters through.
    harness.update(bar(RepeatMode::Restart, false), 1_000);
    harness.frame(1_050);
    approx(harness.width(), 75.);
}

#[test]
fn sliver_fade_transition_follows_its_controller() {
    let opacity = OpacityController::new();
    let _ = opacity.set_opacity(0.25);
    let sliver: Box<dyn Sliver> = Box::new(SliverFadeTransition::new(
        opacity.clone(),
        SliverToBoxAdapter::new(Widget::box_(Size::new(50., 50.), Color::WHITE)),
    ));
    let mut harness = Harness::new(CustomScrollView::new([sliver]).into());
    let alphas = |tree: &mut WidgetTree| -> Vec<f32> {
        tree.paint()
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::PushOpacity { alpha, .. } => Some(*alpha),
                _ => None,
            })
            .collect()
    };
    assert_eq!(alphas(&mut harness.tree), [0.25]);
    let _ = opacity.set_opacity(0.75);
    harness.frame(16);
    assert_eq!(alphas(&mut harness.tree), [0.75]);
}
