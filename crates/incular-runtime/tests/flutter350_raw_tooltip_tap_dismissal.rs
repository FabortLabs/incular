use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Center, RawTooltip, SizedBox, TooltipTriggerMode, Widget};
use std::time::Duration;

const ANCHOR: Offset = Offset::new(400., 300.);
const KEY: &str = "tap dismissal tooltip";

struct Fixture {
    runtime: Runtime,
    start: Instant,
}

impl Fixture {
    fn new(mode: TooltipTriggerMode, child_size: f32, dismiss: bool) -> Self {
        let tooltip = RawTooltip::with_animated_builder(SizedBox::square(child_size), |_| {
            Widget::box_(Size::new(60., 14.), Color::BLACK).with_key(KEY)
        })
        .hover_delay(Duration::from_millis(700))
        .vertical_offset(24.)
        .touch_delay(Duration::from_millis(1500))
        .trigger_mode(mode)
        .ignore_pointer(false)
        .enable_tap_to_dismiss(dismiss);
        let mut probe = Self {
            runtime: Runtime::new(Center::new(tooltip).into()).unwrap(),
            start: Instant::now(),
        };
        probe.frame(0);
        probe
    }

    fn frame(&mut self, ms: u64) {
        self.runtime
            .run_frame_at(
                Constraints::tight(Size::new(800., 600.)),
                self.start + Duration::from_millis(ms),
            )
            .unwrap();
    }

    fn pointer(
        &mut self,
        id: u64,
        kind: PointerDeviceKind,
        phase: PointerPhase,
        at: Offset,
        ms: u64,
    ) {
        self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: id,
                device: id,
                kind,
                buttons: if phase == PointerPhase::Down {
                    PRIMARY_POINTER_BUTTON
                } else {
                    0
                },
                button: (kind != PointerDeviceKind::Mouse).then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::EMPTY,
                phase,
                position: at,
            },
            self.start + Duration::from_millis(ms),
        );
    }

    fn tap(&mut self, id: u64, at: Offset, ms: u64) {
        self.pointer(id, PointerDeviceKind::Touch, PointerPhase::Down, at, ms);
        self.pointer(id, PointerDeviceKind::Touch, PointerPhase::Up, at, ms + 1);
        self.frame(ms + 10);
    }

    fn hover(&mut self, id: u64, ms: u64) {
        self.pointer(id, PointerDeviceKind::Mouse, PointerPhase::Move, ANCHOR, ms);
        self.frame(ms);
        self.frame(ms + 700);
        assert!(self.bubble().is_some());
    }

    fn bubble(&self) -> Option<Offset> {
        let tree = self.runtime.tree();
        let bounds = tree.element_bounds(tree.element_with_key(&KEY.into())?)?;
        Some(Offset::new(
            bounds.origin.x + bounds.size.width * 0.5,
            bounds.origin.y + bounds.size.height * 0.5,
        ))
    }
}

#[test]
fn tapping_the_interactive_bubble_dismisses_a_tooltip_opened_by_tap() {
    let mut probe = Fixture::new(TooltipTriggerMode::Tap, 100., true);
    probe.tap(1, ANCHOR, 10);
    let bubble = probe.bubble().unwrap();
    probe.tap(2, bubble, 30);
    probe.frame(50);
    assert!(probe.bubble().is_none());
}

#[test]
fn disabling_tap_to_dismiss_keeps_the_interactive_bubble_when_tapped() {
    let mut probe = Fixture::new(TooltipTriggerMode::Tap, 100., false);
    probe.tap(1, ANCHOR, 10);
    let bubble = probe.bubble().unwrap();
    probe.tap(2, bubble, 30);
    probe.frame(50);
    assert_eq!(probe.bubble(), Some(bubble));
}

#[test]
fn touch_tap_on_the_anchor_and_outside_dismiss_a_mouse_hovered_long_press_tooltip() {
    let mut probe = Fixture::new(TooltipTriggerMode::LongPress, 10., true);
    probe.hover(100, 10);
    probe.tap(1, ANCHOR, 720);
    probe.frame(860);
    assert!(
        probe.bubble().is_none(),
        "tapping the long-press anchor must dismiss"
    );

    probe.pointer(
        100,
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        Offset::ZERO,
        870,
    );
    probe.frame(870);
    probe.hover(100, 880);
    probe.tap(2, Offset::ZERO, 1590);
    probe.frame(1730);
    assert!(probe.bubble().is_none(), "tapping elsewhere must dismiss");
}

#[test]
fn tapping_elsewhere_cancels_a_pending_hover_before_it_can_show() {
    let mut probe = Fixture::new(TooltipTriggerMode::LongPress, 10., true);
    probe.pointer(
        100,
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        ANCHOR,
        10,
    );
    probe.frame(10);
    probe.tap(1, Offset::ZERO, 20);
    probe.frame(1_000);
    assert!(probe.bubble().is_none());
}
