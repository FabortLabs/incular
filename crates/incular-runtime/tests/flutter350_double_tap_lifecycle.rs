use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{GestureDetector, Widget};

const POINT: Offset = Offset::new(80., 80.);
const MOVED: Offset = Offset::new(102., 80.);

struct Fixture {
    runtime: Runtime,
    trace: Rc<RefCell<Vec<&'static str>>>,
    base: Instant,
}

impl Fixture {
    fn new(done: bool) -> Self {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let record = |name| {
            let trace = trace.clone();
            move || trace.borrow_mut().push(name)
        };
        let down = record("down");
        let mut detector = GestureDetector::new(Widget::box_(Size::new(200., 200.), Color::WHITE))
            .on_double_tap_down(move |_| down())
            .on_double_tap_cancel(record("cancel"));
        if done {
            detector = detector.on_double_tap(record("double"));
        }
        let mut f = Self {
            runtime: Runtime::new(detector.into()).unwrap(),
            trace,
            base: Instant::now(),
        };
        f.frame(0);
        f
    }

    fn event(&mut self, ms: u64, pointer: u64, phase: PointerPhase, position: Offset) {
        let buttons = if matches!(phase, PointerPhase::Down | PointerPhase::Move) {
            PRIMARY_POINTER_BUTTON
        } else {
            0
        };
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                kind: PointerDeviceKind::Touch,
                device: 1,
                button: Some(PRIMARY_POINTER_BUTTON),
                phase,
                buttons,
                position,
                sample: PointerSampleMetadata::EMPTY,
            },
            self.base + Duration::from_millis(ms),
        );
    }

    fn frame(&mut self, ms: u64) {
        self.runtime
            .run_frame_at(
                Constraints::tight(Size::new(200., 200.)),
                self.base + Duration::from_millis(ms),
            )
            .unwrap();
    }

    fn tap(&mut self, ms: u64, pointer: u64) {
        self.event(ms, pointer, PointerPhase::Down, POINT);
        self.event(ms + 1, pointer, PointerPhase::Up, POINT);
    }

    fn expect(&self, expected: &[&str]) {
        assert_eq!(&*self.trace.borrow(), expected);
    }
}

#[test]
fn a_moved_first_press_cannot_seed_a_later_double_tap() {
    let mut f = Fixture::new(true);
    f.event(0, 1, PointerPhase::Down, POINT);
    f.event(1, 1, PointerPhase::Move, MOVED);
    f.event(2, 1, PointerPhase::Up, MOVED);
    f.tap(100, 2);
    f.expect(&[]);
    f.tap(200, 3);
    f.expect(&["down", "double"]);
}

#[test]
fn a_moved_second_press_cancels_at_the_move_and_is_not_revived() {
    let mut f = Fixture::new(true);
    f.tap(0, 1);
    f.event(100, 2, PointerPhase::Down, POINT);
    f.expect(&["down"]);
    f.event(101, 2, PointerPhase::Move, MOVED);
    f.expect(&["down", "cancel"]);
    f.event(102, 2, PointerPhase::Move, POINT);
    f.event(103, 2, PointerPhase::Up, POINT);
    f.frame(550);
    f.expect(&["down", "cancel"]);
}

#[test]
fn pointer_cancellation_is_reported_only_for_a_qualified_second_press() {
    let mut f = Fixture::new(true);
    f.event(0, 1, PointerPhase::Down, POINT);
    f.event(1, 1, PointerPhase::Cancel, POINT);
    f.expect(&[]);
    f.tap(100, 2);
    f.event(200, 3, PointerPhase::Down, POINT);
    f.expect(&["down"]);
    f.event(201, 3, PointerPhase::Cancel, POINT);
    f.expect(&["down", "cancel"]);
    f.frame(600);
    f.expect(&["down", "cancel"]);
}

#[test]
fn down_and_cancel_callbacks_work_without_a_completion_callback() {
    let mut f = Fixture::new(false);
    f.tap(0, 1);
    f.event(100, 2, PointerPhase::Down, POINT);
    f.expect(&["down"]);
    f.event(101, 2, PointerPhase::Cancel, POINT);
    f.frame(550);
    f.expect(&["down", "cancel"]);
}
