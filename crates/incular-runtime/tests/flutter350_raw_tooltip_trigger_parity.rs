//! Exact trigger intents from Flutter 3.50.0 widgets/raw_tooltip_test.dart.
use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, RawTooltip, RawTooltipController, TooltipComponentBuilder, TooltipTriggerMode, Widget,
};
use std::{cell::Cell, rc::Rc, time::Duration};

struct Fixture {
    runtime: Runtime,
    start: Instant,
    controller: RawTooltipController,
    triggers: Rc<Cell<u32>>,
}

impl Fixture {
    fn new(mode: TooltipTriggerMode, delay: Duration) -> Self {
        Self::with_hover_delays(mode, delay, Duration::ZERO, Duration::ZERO)
    }

    fn with_hover_delays(
        mode: TooltipTriggerMode,
        delay: Duration,
        enter: Duration,
        exit: Duration,
    ) -> Self {
        let controller = RawTooltipController::new();
        let triggers = Rc::new(Cell::new(0));
        let capture = triggers.clone();
        let source = Widget::box_(Size::new(100., 100.), Color::WHITE);
        let tooltip = RawTooltip::with_builder(
            source,
            TooltipComponentBuilder::new(|| Widget::box_(Size::new(40., 20.), Color::BLACK)),
        )
        .controller(controller.clone())
        .trigger_mode(mode)
        .vertical_offset(24.)
        .touch_delay(delay)
        .hover_delay(enter)
        .dismiss_delay(exit)
        .on_triggered(move || capture.set(capture.get() + 1));
        let mut fixture = Self {
            runtime: Runtime::new(Center::new(tooltip).into()).unwrap(),
            start: Instant::now(),
            controller,
            triggers,
        };
        assert!(!fixture.frame(0), "first frame hidden");
        fixture
    }

    fn frame(&mut self, at: u64) -> bool {
        self.runtime
            .run_frame_at(
                Constraints::tight(Size::new(800., 600.)),
                self.start + Duration::from_millis(at),
            )
            .unwrap();
        self.runtime.tree().transient_surfaces().len() == 1
    }

    fn pointer(&mut self, pointer: u64, kind: PointerDeviceKind, phase: PointerPhase, at: u64) {
        self.pointer_at(pointer, kind, phase, at, Offset::new(400., 300.));
    }

    fn pointer_at(
        &mut self,
        pointer: u64,
        kind: PointerDeviceKind,
        phase: PointerPhase,
        at: u64,
        position: Offset,
    ) {
        self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                device: pointer,
                kind,
                buttons: if phase == PointerPhase::Down {
                    PRIMARY_POINTER_BUTTON
                } else {
                    0
                },
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::EMPTY,
                phase,
                position,
            },
            self.start + Duration::from_millis(at),
        );
    }

    fn tap(&mut self, id: u64, at: u64) -> bool {
        self.pointer(id, PointerDeviceKind::Touch, PointerPhase::Down, at);
        self.pointer(id, PointerDeviceKind::Touch, PointerPhase::Up, at + 10);
        self.frame(at + 20)
    }

    fn long_press(&mut self, id: u64, at: u64) -> (bool, bool) {
        self.pointer(id, PointerDeviceKind::Touch, PointerPhase::Down, at);
        self.frame(at);
        self.frame(at + 500);
        let activated = self.frame(at + 510);
        self.pointer(id, PointerDeviceKind::Touch, PointerPhase::Up, at + 520);
        let released = self.frame(at + 530);
        (activated, released)
    }
}

const NORMAL_DELAY: Duration = Duration::from_millis(1500);

#[test]
fn tap_mode_shows_the_overlay_after_touch_tap() {
    let mut probe = Fixture::new(TooltipTriggerMode::Tap, NORMAL_DELAY);
    assert!(probe.tap(1, 10));
    assert_eq!(probe.triggers.get(), 1);
}

#[test]
fn long_press_mode_rejects_a_tap_then_shows_by_the_release_frame() {
    let mut probe = Fixture::new(TooltipTriggerMode::LongPress, NORMAL_DELAY);
    assert!(!probe.tap(1, 10));
    let (_, visible_after_release) = probe.long_press(2, 50);
    assert!(visible_after_release);
    assert_eq!(probe.triggers.get(), 1);
}

#[test]
fn manual_mode_ignores_tap_and_long_press_but_can_show_programmatically() {
    let mut probe = Fixture::new(TooltipTriggerMode::Manual, NORMAL_DELAY);
    assert!(!probe.tap(1, 10));
    assert_eq!(probe.long_press(2, 50), (false, false));
    probe
        .controller
        .show_at(probe.start + Duration::from_millis(600));
    assert!(probe.frame(610));
}

#[test]
fn long_press_overlay_stays_visible_until_delay_measured_from_release() {
    let mut probe = Fixture::new(TooltipTriggerMode::LongPress, Duration::from_secs(1));
    let far_at = 600 + Duration::from_secs(9999 * 24 * 60 * 60).as_millis() as u64;
    probe.pointer(1, PointerDeviceKind::Touch, PointerPhase::Down, 10);
    assert!(!probe.frame(509));
    let accepted = probe.frame(510);
    let held = probe.frame(far_at);
    let held_again = probe.frame(far_at + 1);
    assert_eq!(probe.controller.pending_deadline(), None);
    probe.pointer(1, PointerDeviceKind::Touch, PointerPhase::Up, far_at + 2);
    let released = probe.frame(far_at + 2);
    probe.frame(far_at + 1002);
    let expired = probe.frame(far_at + 1502);
    assert_eq!(
        [accepted, held, held_again, released, expired],
        [true, true, true, true, false]
    );
}

#[test]
fn mouse_hold_is_not_a_touch_tooltip_trigger() {
    let mut probe = Fixture::new(TooltipTriggerMode::LongPress, NORMAL_DELAY);
    probe.pointer(1, PointerDeviceKind::Mouse, PointerPhase::Down, 10);
    probe.frame(610);
    probe.pointer(1, PointerDeviceKind::Mouse, PointerPhase::Up, 610);
    let mouse = (probe.frame(620), probe.triggers.get());
    let (_, touch) = probe.long_press(2, 650);
    assert!(touch, "same live tooltip accepts a touch hold");
    assert_eq!(mouse, (false, 0), "mouse press without hover-move");
}

#[test]
fn released_long_press_expires_on_its_own_touch_deadline() {
    let mut probe = Fixture::new(TooltipTriggerMode::LongPress, Duration::from_secs(3));
    probe.pointer(1, PointerDeviceKind::Touch, PointerPhase::Down, 10);
    assert!(probe.frame(510));
    // Release outside the anchor; the captured gesture owns this release.
    probe.pointer_at(
        1,
        PointerDeviceKind::Touch,
        PointerPhase::Up,
        520,
        Offset::new(30., 30.),
    );
    assert!(probe.frame(3519));
    assert!(!probe.frame(3520));
    assert_eq!(probe.triggers.get(), 1);
}

#[test]
fn tap_starts_its_deadline_when_released() {
    let mut probe = Fixture::new(TooltipTriggerMode::Tap, Duration::from_secs(3));
    assert!(probe.tap(1, 10));
    assert!(probe.frame(3019));
    assert!(!probe.frame(3020));
}

#[test]
fn hover_stays_visible_until_exit_and_respects_both_scheduled_delays() {
    let mut probe = Fixture::with_hover_delays(
        TooltipTriggerMode::LongPress,
        NORMAL_DELAY,
        Duration::from_millis(100),
        Duration::from_millis(60),
    );
    probe.pointer_at(
        1,
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        0,
        Offset::ZERO,
    );
    probe.pointer(1, PointerDeviceKind::Mouse, PointerPhase::Move, 10);
    assert!(!probe.frame(109));
    assert!(probe.frame(110));
    assert!(probe.frame(86_400_110));
    probe.pointer_at(
        1,
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        86_400_120,
        Offset::ZERO,
    );
    assert!(probe.frame(86_400_179));
    assert!(!probe.frame(86_400_180));
}
