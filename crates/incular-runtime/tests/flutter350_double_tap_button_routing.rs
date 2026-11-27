use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::{Alignment, Constraints};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, SECONDARY_POINTER_BUTTON, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Align, GestureDetector, Widget};

type Trace = Rc<RefCell<Vec<&'static str>>>;
const HIT: Offset = Offset::new(80., 80.);
const MISS: Offset = Offset::new(220., 220.);
const PRIMARY: u32 = PRIMARY_POINTER_BUTTON;
const SECONDARY: u32 = SECONDARY_POINTER_BUTTON;

struct Fixture {
    runtime: Runtime,
    trace: Trace,
    base: Instant,
}

impl Fixture {
    fn new(single: bool, secondary: bool) -> Self {
        Self::with_details(single, secondary, false)
    }

    fn with_details(single: bool, secondary: bool, details: bool) -> Self {
        let trace = Trace::default();
        let record = |name| {
            let trace = trace.clone();
            move || trace.borrow_mut().push(name)
        };
        let mut detector = GestureDetector::new(Widget::box_(Size::new(200., 200.), Color::WHITE))
            .on_double_tap(record("double"));
        if single {
            detector = detector.on_tap(record("single"));
        }
        if details {
            let down = record("down");
            detector = detector
                .on_tap_down(move |_| down())
                .on_tap_cancel(record("cancel"));
        }
        if secondary {
            detector = detector.on_secondary_tap(record("secondary"));
        }
        let mut runtime = Runtime::new(Align::new(Alignment::TOP_LEFT, detector).into()).unwrap();
        let base = Instant::now();
        runtime
            .run_frame_at(Constraints::tight(Size::new(300., 300.)), base)
            .unwrap();
        Self {
            runtime,
            trace,
            base,
        }
    }

    fn frame(&mut self, time: u64) {
        self.runtime
            .run_frame_at(
                Constraints::tight(Size::new(300., 300.)),
                self.base + Duration::from_millis(time),
            )
            .unwrap();
    }

    fn event(
        &mut self,
        time: u64,
        pointer: u64,
        phase: PointerPhase,
        button: u32,
        position: Offset,
    ) {
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                device: 1,
                kind: PointerDeviceKind::Mouse,
                button: Some(button),
                phase,
                buttons: if phase == PointerPhase::Down {
                    button
                } else {
                    0
                },
                position,
                sample: PointerSampleMetadata::EMPTY,
            },
            self.base + Duration::from_millis(time),
        );
    }

    fn tap(&mut self, time: u64, pointer: u64, button: u32, pos: Offset) {
        self.event(time, pointer, PointerPhase::Down, button, pos);
        self.event(time + 1, pointer, PointerPhase::Up, button, pos);
    }
}

#[test]
fn an_in_route_secondary_press_interrupts_a_mounted_double_tap() {
    let mut f = Fixture::new(false, false);
    f.tap(0, 1, PRIMARY, HIT);
    f.tap(100, 2, SECONDARY, HIT);
    f.tap(200, 3, PRIMARY, HIT);
    assert!(f.trace.borrow().is_empty());
    f.tap(270, 4, PRIMARY, HIT);
    assert_eq!(&*f.trace.borrow(), &["double"]);
}

#[test]
fn an_allowed_secondary_press_replaces_the_held_primary_single() {
    let mut f = Fixture::new(true, true);
    f.tap(0, 1, PRIMARY, HIT);
    assert!(f.trace.borrow().is_empty());
    f.event(100, 2, PointerPhase::Down, SECONDARY, HIT);
    assert!(f.trace.borrow().is_empty());
    f.event(101, 2, PointerPhase::Up, SECONDARY, HIT);
    assert_eq!(&*f.trace.borrow(), &["secondary"]);
    f.runtime
        .run_frame_at(
            Constraints::tight(Size::new(300., 300.)),
            f.base + Duration::from_millis(402),
        )
        .unwrap();
    assert_eq!(&*f.trace.borrow(), &["secondary"]);
}

#[test]
fn a_secondary_press_without_a_callback_leaves_no_phantom_single() {
    let mut f = Fixture::new(true, false);
    f.tap(0, 1, PRIMARY, HIT);
    assert!(f.trace.borrow().is_empty());
    f.event(100, 2, PointerPhase::Down, SECONDARY, HIT);
    assert!(f.trace.borrow().is_empty());
    f.event(101, 2, PointerPhase::Up, SECONDARY, HIT);
    f.runtime
        .run_frame_at(
            Constraints::tight(Size::new(300., 300.)),
            f.base + Duration::from_millis(402),
        )
        .unwrap();
    assert!(f.trace.borrow().is_empty());
}

#[test]
fn a_secondary_press_off_the_detector_does_not_interrupt_the_pair() {
    let mut f = Fixture::new(false, false);
    f.tap(0, 1, PRIMARY, HIT);
    f.tap(100, 2, SECONDARY, MISS);
    f.tap(200, 3, PRIMARY, HIT);
    assert_eq!(&*f.trace.borrow(), &["double"]);
}

#[test]
fn rejecting_an_already_reported_primary_tap_cancels_exactly_once() {
    let mut f = Fixture::with_details(true, false, true);
    f.event(0, 1, PointerPhase::Down, PRIMARY, HIT);
    f.frame(105);
    assert_eq!(&*f.trace.borrow(), &["down"]);
    f.event(105, 1, PointerPhase::Up, PRIMARY, HIT);
    assert_eq!(&*f.trace.borrow(), &["down"]);
    f.tap(205, 2, SECONDARY, HIT);
    assert_eq!(&*f.trace.borrow(), &["down", "cancel"]);
    f.frame(506);
    assert_eq!(&*f.trace.borrow(), &["down", "cancel"]);
}
