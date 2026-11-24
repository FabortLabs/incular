//! Callback traces from Flutter 3.50.0 test/gestures/long_press_test.dart.
//! Flutter registers, closes, then routes the recognizer directly; these tests
//! enter through Incular's real retained widget and timestamped input path.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, SECONDARY_POINTER_BUTTON, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{GestureDetector, Widget};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Seen {
    Down,
    Start,
    Move(Offset),
    End,
    Cancel,
}

use Seen::{Cancel, Down, End, Move, Start};

const PRESS: Offset = Offset::new(10.0, 10.0);
const RELEASE: Offset = Offset::new(11.0, 9.0);
const FAR: Offset = Offset::new(100.0, 200.0);
const SURFACE: Size = Size {
    width: 300.0,
    height: 300.0,
};

struct Fixture {
    runtime: Runtime,
    now: Instant,
    seen: Rc<RefCell<Vec<Seen>>>,
}

impl Fixture {
    fn new(record_move: bool) -> Self {
        let seen: Rc<RefCell<Vec<Seen>>> = Rc::default();
        let record = |event| {
            let seen = Rc::clone(&seen);
            move || seen.borrow_mut().push(event)
        };
        let detector = GestureDetector::new(Widget::box_(SURFACE, Color::WHITE))
            .on_long_press_down({
                let down = record(Down);
                move |_| down()
            })
            .on_long_press_cancel(record(Cancel));
        let detector = if record_move {
            let seen = Rc::clone(&seen);
            detector
                .on_long_press(record(Start))
                .on_long_press_move_update(move |details| {
                    seen.borrow_mut().push(Move(details.global_position));
                })
                .on_long_press_up(record(End))
        } else {
            detector
                .on_long_press_start({
                    let start = record(Start);
                    move |_| start()
                })
                .on_long_press_end({
                    let end = record(End);
                    move |_| end()
                })
        };
        let mut runtime = Runtime::new(detector.into()).unwrap();
        let now = Instant::now();
        runtime
            .run_frame_at(Constraints::tight(SURFACE), now)
            .unwrap();
        Self { runtime, now, seen }
    }

    fn elapse(&mut self, millis: u64) {
        self.now += Duration::from_millis(millis);
        self.runtime
            .run_frame_at(Constraints::tight(SURFACE), self.now)
            .unwrap();
    }

    fn route(&mut self, pointer: u64, phase: PointerPhase, position: Offset, buttons: u32) {
        let button = matches!(phase, PointerPhase::Down | PointerPhase::Up)
            .then_some(PRIMARY_POINTER_BUTTON);
        self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                device: 0,
                kind: PointerDeviceKind::Touch,
                buttons,
                button,
                sample: PointerSampleMetadata::EMPTY,
                phase,
                position,
            },
            self.now,
        );
    }

    fn down(&mut self, pointer: u64) {
        self.route(pointer, PointerPhase::Down, PRESS, PRIMARY_POINTER_BUTTON);
    }

    fn up(&mut self, pointer: u64) {
        self.route(pointer, PointerPhase::Up, RELEASE, 0);
    }

    fn snapshot(&self) -> Vec<Seen> {
        self.seen.borrow().clone()
    }

    fn dispose(self) -> Vec<Seen> {
        drop(self.runtime);
        self.seen.borrow().clone()
    }
}

#[test]
fn recognizes_long_press_at_its_default_deadline() {
    // Upstream line 61, plus just-before and at-deadline observations.
    let mut fixture = Fixture::new(true);
    assert_eq!(fixture.snapshot(), []);
    fixture.down(5);
    assert_eq!(fixture.snapshot(), [Down]);
    fixture.elapse(300);
    assert_eq!(fixture.snapshot(), [Down]);
    fixture.elapse(199);
    assert_eq!(fixture.snapshot(), [Down]);
    fixture.elapse(1);
    assert_eq!(fixture.snapshot(), [Down, Start]);
    fixture.elapse(500);
    assert_eq!(fixture.snapshot(), [Down, Start]);
    assert_eq!(fixture.dispose(), [Down, Start]);
}

#[test]
fn moving_before_acceptance_cancels_long_press() {
    // Upstream line 106 (Long press group).
    let mut fixture = Fixture::new(true);
    assert_eq!(fixture.snapshot(), []);
    fixture.down(5);
    assert_eq!(fixture.snapshot(), [Down]);
    fixture.elapse(300);
    assert_eq!(fixture.snapshot(), [Down]);
    fixture.route(5, PointerPhase::Move, FAR, PRIMARY_POINTER_BUTTON);
    assert_eq!(fixture.snapshot(), [Down, Cancel]);
    fixture.elapse(1000);
    fixture.up(5);
    fixture.elapse(300);
    assert_eq!(fixture.snapshot(), [Down, Cancel]);
    assert_eq!(fixture.dispose(), [Down, Cancel]);
}

#[test]
fn moving_after_acceptance_reports_the_global_position_and_ends() {
    // Upstream line 365.
    let mut fixture = Fixture::new(true);
    assert_eq!(fixture.snapshot(), []);
    fixture.down(5);
    assert_eq!(fixture.snapshot(), [Down]);
    fixture.elapse(1000);
    assert_eq!(fixture.snapshot(), [Down, Start]);
    fixture.route(5, PointerPhase::Move, FAR, PRIMARY_POINTER_BUTTON);
    assert_eq!(fixture.snapshot(), [Down, Start, Move(FAR)]);
    fixture.up(5);
    fixture.elapse(300);
    assert_eq!(fixture.snapshot(), [Down, Start, Move(FAR), End]);
    assert_eq!(fixture.dispose(), [Down, Start, Move(FAR), End]);
}

#[test]
fn buttons_changed_before_acceptance_cancel_but_next_press_still_works() {
    // Upstream line 427. The primary stream's move changes only its buttons.
    let mut fixture = Fixture::new(false);
    fixture.down(5);
    assert_eq!(fixture.snapshot(), [Down]);
    fixture.elapse(300);
    fixture.route(5, PointerPhase::Move, PRESS, SECONDARY_POINTER_BUTTON);
    let first_after_change = fixture.snapshot();
    fixture.elapse(700);
    fixture.up(5);
    let first_after_release = fixture.snapshot();
    fixture.seen.borrow_mut().clear();

    fixture.down(6);
    assert_eq!(fixture.snapshot(), [Down]);
    fixture.elapse(1000);
    assert_eq!(fixture.snapshot(), [Down, Start]);
    fixture.seen.borrow_mut().clear();
    fixture.up(6);
    assert_eq!(fixture.snapshot(), [End]);
    assert_eq!(
        first_after_change,
        [Down, Cancel],
        "first press after release: {first_after_release:?}",
    );
}

#[test]
fn buttons_changed_after_acceptance_do_not_cancel_long_press() {
    // Upstream line 455.
    let mut fixture = Fixture::new(false);
    fixture.down(5);
    fixture.elapse(1000);
    assert_eq!(fixture.snapshot(), [Down, Start]);
    fixture.seen.borrow_mut().clear();
    fixture.route(5, PointerPhase::Move, PRESS, SECONDARY_POINTER_BUTTON);
    assert_eq!(fixture.snapshot(), []);
    fixture.up(5);
    assert_eq!(fixture.snapshot(), [End]);
}
