use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PointerDeviceKind, PointerPhase, PointerSampleMetadata, Size,
    time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Positioned, RawTooltip, RawTooltipController, SizedBox, Stack, Widget};
use std::time::Duration;

const WAIT: u64 = 700;
const OUTSIDE: Offset = Offset::new(0., 0.);
const FIRST: Offset = Offset::new(400., 50.);
const SECOND: Offset = Offset::new(400., 150.);
const FIRST_KEY: &str = "first tooltip";
const SECOND_KEY: &str = "last tooltip";

struct Fixture {
    now: Instant,
    runtime: Runtime,
    first: RawTooltipController,
    second: RawTooltipController,
}

impl Fixture {
    fn new(wait: u64, exit: u64, animated: bool) -> Self {
        let first = RawTooltipController::new();
        let second = RawTooltipController::new();
        let anchor = |y: f32, key: &'static str, controller: &RawTooltipController| {
            let child = SizedBox::square(100.);
            let popup = move || Widget::box_(Size::new(70., 20.), Color::BLACK).with_key(key);
            let tooltip = if animated {
                RawTooltip::with_animated_builder(child, move |_| popup())
            } else {
                RawTooltip::with_builder(
                    child,
                    incular_widgets::TooltipComponentBuilder::new(popup),
                )
            };
            Positioned::builder()
                .left(350.)
                .top(y)
                .child(
                    tooltip
                        .hover_delay(Duration::from_millis(wait))
                        .ignore_pointer(!animated)
                        .dismiss_delay(Duration::from_millis(exit))
                        .controller(controller.clone()),
                )
                .build()
        };
        let mut tooltip = Self {
            now: Instant::now(),
            runtime: Runtime::new(
                Stack::new([
                    anchor(0., FIRST_KEY, &first),
                    anchor(100., SECOND_KEY, &second),
                ])
                .into(),
            )
            .unwrap(),
            first,
            second,
        };
        tooltip.frame(0);
        tooltip
    }

    fn frame(&mut self, ms: u64) {
        self.runtime
            .run_frame_at(
                Constraints::tight(Size::new(800., 600.)),
                self.now + Duration::from_millis(ms),
            )
            .unwrap();
    }

    fn move_to(&mut self, device: u64, position: Offset, ms: u64) {
        self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: device,
                device,
                kind: PointerDeviceKind::Mouse,
                buttons: 0,
                button: None,
                sample: PointerSampleMetadata::EMPTY,
                phase: PointerPhase::Move,
                position,
            },
            self.now + Duration::from_millis(ms),
        );
        self.frame(ms);
    }

    fn visibility(&self, first: bool, second: bool) {
        let has = |key: &str| self.runtime.tree().element_with_key(&key.into()).is_some();
        assert_eq!(has(FIRST_KEY), first, "first tooltip");
        assert_eq!(has(SECOND_KEY), second, "last tooltip");
    }
}

#[test]
fn moving_directly_between_targets_waives_only_the_second_hover_wait() {
    let mut probe = Fixture::new(WAIT, 100, true);
    probe.move_to(1, OUTSIDE, 1);
    probe.move_to(1, FIRST, 10);
    probe.frame(709);
    probe.visibility(false, false);
    probe.frame(710);
    probe.visibility(true, false);

    probe.move_to(1, SECOND, 710);
    probe.visibility(false, true);
}

#[test]
fn another_mouse_can_hold_the_first_tooltip_without_waiving_the_second_wait() {
    let mut probe = Fixture::new(WAIT, 100, true);
    probe.move_to(1, FIRST, 10);
    probe.frame(710);
    probe.move_to(2, SECOND, 900);
    assert_eq!(probe.first.hovering_devices(), 1);
    assert!(probe.second.is_waiting());
    probe.frame(1599);
    probe.visibility(true, false);
    probe.frame(1600);
    probe.visibility(true, true);
}

#[test]
fn an_old_dismissed_tooltip_does_not_waive_a_cold_hover_wait() {
    let mut probe = Fixture::new(WAIT, 100, true);
    probe.move_to(1, FIRST, 10);
    probe.frame(710);
    probe.frame(861);
    probe.move_to(1, OUTSIDE, 900);
    probe.frame(1000);
    probe.frame(1076);
    probe.visibility(false, false);
    probe.move_to(1, SECOND, 2000);
    assert!(probe.second.is_waiting());
    probe.frame(2699);
    probe.visibility(false, false);
    probe.frame(2700);
    probe.visibility(false, true);
}

#[test]
fn legacy_immediate_tooltips_can_handoff_after_exit_has_already_hidden_the_first() {
    let mut probe = Fixture::new(WAIT, 0, false);
    probe.move_to(1, FIRST, 10);
    probe.frame(710);
    probe.visibility(true, false);
    probe.move_to(1, SECOND, 740);
    probe.visibility(false, true);
}

#[test]
fn an_exit_then_quick_reentry_cancels_each_scheduled_hide() {
    let mut probe = Fixture::new(WAIT, WAIT, true);
    let mut time = 10;
    for reentries in 0..=2 {
        probe.move_to(1, FIRST, time);
        probe.frame(time + WAIT + 151);
        probe.visibility(true, false);
        time += WAIT + 151;
        for _ in 0..reentries {
            probe.move_to(1, OUTSIDE, time + 10);
            probe.move_to(1, FIRST, time + 20);
            probe.frame(time + WAIT + 20);
            probe.visibility(true, false);
            time += WAIT + 30;
        }
        probe.move_to(1, OUTSIDE, time + 10);
        let reset = time + 3_600_000;
        probe.frame(reset);
        probe.frame(reset + 76);
        probe.visibility(false, false);
        time = reset + 100;
    }
}

#[test]
fn reentering_mid_reverse_resumes_the_fade_without_the_cold_wait() {
    let mut probe = Fixture::new(WAIT, 100, true);
    probe.move_to(1, FIRST, 10);
    probe.frame(710);
    probe.frame(861);
    probe.move_to(1, OUTSIDE, 900);
    probe.frame(1000);
    probe.frame(1035);
    assert!(!probe.first.is_visible());
    probe.visibility(true, false);
    probe.move_to(1, FIRST, 1040);
    assert!(probe.first.is_visible());
    assert_eq!(probe.first.pending_deadline(), None);
    probe.frame(1200);
    probe.visibility(true, false);
}
