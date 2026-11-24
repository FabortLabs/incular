//! The pinned Flutter gestures/tap_and_drag_test.dart callback sequences.
//! These are sole-member recognizer cases, not competition fixtures.
use incular_core::{DragStartBehavior, Offset, PointerPhase, time::Instant};
use incular_gestures::{PointerDeviceKind, RawPointerEvent, TapAndDragGestureRecognizer};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use PointerPhase::{Cancel, Down, Move, Up};

type Point = (f32, f32);
const A: Point = (10., 10.);
const A_UP: Point = (11., 9.);
const B: Point = (12., 12.);
const B_UP: Point = (13., 11.);

fn event(base: Instant, ms: u64, id: u64, phase: PointerPhase, xy: Point) -> RawPointerEvent {
    RawPointerEvent::new(
        id,
        0,
        PointerDeviceKind::Touch,
        u32::from(matches!(phase, Down | Move)),
        Offset::new(xy.0, xy.1),
        phase,
        base + Duration::from_millis(ms),
    )
}

#[derive(Debug, PartialEq, Eq)]
enum Callback {
    Press(u32),
    Lift(u32),
    Start(u32),
    Update(u32),
    End(u32),
    Stop,
}
use Callback::*;
type Log = Rc<RefCell<Vec<Callback>>>;

struct Drag {
    recognizer: TapAndDragGestureRecognizer,
    log: Log,
    base: Instant,
}

impl Drag {
    fn pan() -> Self {
        let log = Log::default();
        let recognizer = TapAndDragGestureRecognizer::pan()
            .drag_start_behavior(DragStartBehavior::Down)
            .max_consecutive_tap(3)
            .on_tap_down({
                let l = log.clone();
                move |d| l.borrow_mut().push(Press(d.consecutive_tap_count))
            })
            .on_tap_up({
                let l = log.clone();
                move |d| l.borrow_mut().push(Lift(d.consecutive_tap_count))
            })
            .on_drag_start({
                let l = log.clone();
                move |d| l.borrow_mut().push(Start(d.consecutive_tap_count))
            })
            .on_drag_update({
                let l = log.clone();
                move |d| l.borrow_mut().push(Update(d.consecutive_tap_count))
            })
            .on_drag_end({
                let l = log.clone();
                move |d| l.borrow_mut().push(End(d.consecutive_tap_count))
            })
            .on_cancel({
                let l = log.clone();
                move || l.borrow_mut().push(Stop)
            });
        Self {
            recognizer,
            log,
            base: Instant::now(),
        }
    }
    fn send(&mut self, ms: u64, id: u64, phase: PointerPhase, xy: Point) {
        self.recognizer
            .handle_raw(event(self.base, ms, id, phase, xy));
    }
    fn tap(&mut self, ms: u64, id: u64, down: Point, up: Point) {
        self.send(ms, id, Down, down);
        self.send(ms, id, Up, up);
    }
    fn expect(&self, sequence: &[Callback]) {
        assert_eq!(self.log.take(), sequence);
    }
}

#[test]
fn three_consecutive_taps_report_each_count() {
    let mut g = Drag::pan();
    for (id, ms, from, to) in [(1, 0, A, A_UP), (2, 150, B, B_UP), (3, 300, B, B_UP)] {
        g.tap(ms, id, from, to);
        g.expect(&[Press(id as u32), Lift(id as u32)]);
    }
}

#[test]
fn consecutive_count_resets_after_timeout() {
    let mut g = Drag::pan();
    g.tap(0, 1, A, A_UP);
    g.expect(&[Press(1), Lift(1)]);
    g.tap(1000, 2, B, B_UP);
    g.expect(&[Press(1), Lift(1)]);
}

#[test]
fn consecutive_count_resets_after_distant_tap() {
    let mut g = Drag::pan();
    g.tap(0, 1, A, A_UP);
    g.expect(&[Press(1), Lift(1)]);
    g.tap(100, 4, (130., 130.), (131., 129.));
    g.expect(&[Press(1), Lift(1)]);
}

#[test]
fn fourth_close_tap_restarts_a_series_capped_at_three() {
    let mut g = Drag::pan();
    for (id, count, from, to) in [
        (1, 1, A, A_UP),
        (2, 2, B, B_UP),
        (3, 3, B, B_UP),
        (3, 1, B, B_UP),
    ] {
        g.tap(0, id, from, to);
        g.expect(&[Press(count), Lift(count)]);
    }
}

#[test]
fn large_motion_reports_start_update_and_end_once() {
    let mut g = Drag::pan();
    g.send(0, 5, Down, A);
    g.send(0, 5, Move, (40., 45.));
    g.send(0, 5, Up, (40., 45.));
    g.expect(&[Press(1), Start(1), Update(1), End(1)]);
}

#[test]
fn third_tap_can_become_a_drag_without_losing_its_count() {
    let mut g = Drag::pan();
    g.tap(0, 5, A, A);
    g.tap(150, 5, A, A);
    g.send(300, 5, Down, A);
    g.send(300, 5, Move, (40., 45.));
    g.send(300, 5, Up, (40., 45.));
    g.expect(&[
        Press(1),
        Lift(1),
        Press(2),
        Lift(2),
        Press(3),
        Start(3),
        Update(3),
        End(3),
    ]);
}

fn overlapping_after_acceptance(first_lift: u64) {
    let mut g = Drag::pan();
    g.send(0, 1, Down, A);
    g.send(0, 2, Down, B);
    for id in [first_lift, 3 - first_lift] {
        g.send(0, id, Up, if id == 1 { A_UP } else { B_UP });
    }
    g.expect(&[Press(1), Lift(1)]);
}
#[test]
fn accepted_primary_persists_when_it_lifts_first() {
    overlapping_after_acceptance(1);
}
#[test]
fn accepted_primary_persists_when_it_lifts_last() {
    overlapping_after_acceptance(2);
}

fn overlapping_registered_before_acceptance(first_lift: u64) {
    let mut g = Drag::pan();
    // The externally driven arena delivers acceptance for pointer 1 only
    // after both registrations. The pointer events are routed separately.
    g.recognizer = g.recognizer.with_external_arena(|_, _| None);
    let a = event(g.base, 0, 1, Down, A);
    let b = event(g.base, 0, 2, Down, B);
    g.recognizer.add_pointer(a);
    g.recognizer.add_pointer(b);
    g.recognizer.accept_gesture(1);
    g.recognizer.handle_event(a);
    g.recognizer.handle_event(b);
    for id in [first_lift, 3 - first_lift] {
        let up = if id == 1 { A_UP } else { B_UP };
        g.recognizer.handle_event(event(g.base, 0, id, Up, up));
    }
    g.expect(&[Press(1), Lift(1)]);
}

#[test]
fn primary_can_lift_first_when_both_registered_before_acceptance() {
    overlapping_registered_before_acceptance(1);
}

#[test]
fn primary_can_lift_last_when_both_registered_before_acceptance() {
    overlapping_registered_before_acceptance(2);
}

#[test]
fn acceptance_before_routing_the_down_event_still_delivers_tap_up() {
    let mut g = Drag::pan();
    g.recognizer = g.recognizer.with_external_arena(|_, _| None);
    let down = event(g.base, 0, 1, Down, A);
    g.recognizer.add_pointer(down);
    g.recognizer.accept_gesture(1);
    g.recognizer.handle_event(down);
    g.recognizer.handle_event(event(g.base, 0, 1, Up, A_UP));
    g.expect(&[Press(1), Lift(1)]);
}

#[test]
fn a_release_inside_tap_slop_is_not_a_drag() {
    let mut g = Drag::pan();
    g.tap(0, 1, A, A_UP);
    g.expect(&[Press(1), Lift(1)]);
}

fn intermediate_drag(g: &mut Drag) {
    g.send(0, 5, Down, A);
    g.send(0, 5, Move, (25., 25.));
    g.send(0, 5, Up, (25., 25.));
    g.expect(&[Press(1), Start(1), End(1)]);
}
#[test]
fn motion_between_tap_and_pan_slop_starts_drag_on_release() {
    intermediate_drag(&mut Drag::pan());
}
#[test]
fn a_drag_past_tap_slop_does_not_poison_the_next_series() {
    let mut g = Drag::pan();
    intermediate_drag(&mut g);
    g.tap(1000, 1, A, A_UP);
    g.expect(&[Press(1), Lift(1)]);
}

#[test]
fn pointer_cancel_has_no_lift_and_resets_count() {
    let mut g = Drag::pan();
    g.send(0, 1, Down, A);
    g.send(0, 1, Cancel, (0., 0.));
    g.expect(&[Press(1), Stop]);
    g.tap(100, 2, B, B_UP);
    g.expect(&[Press(1), Lift(1)]);
}

#[test]
fn absent_drag_end_handler_does_not_break_the_next_down() {
    let log = Log::default();
    let mut g = TapAndDragGestureRecognizer::pan()
        .drag_start_behavior(DragStartBehavior::Down)
        .max_consecutive_tap(3)
        .on_tap_down({
            let l = log.clone();
            move |d| l.borrow_mut().push(Press(d.consecutive_tap_count))
        });
    let base = Instant::now();
    for (ms, id, phase, at) in [
        (0, 5, Down, A),
        (0, 5, Move, (25., 25.)),
        (0, 5, Up, (25., 25.)),
        (1000, 1, Down, A),
        (1000, 1, Up, A_UP),
    ] {
        g.handle_raw(event(base, ms, id, phase, at));
    }
    assert_eq!(log.take(), [Press(1), Press(1)]);
}

#[test]
fn horizontal_drag_end_reports_the_actual_release_position() {
    let point = Rc::new(Cell::new(None));
    let mut g = TapAndDragGestureRecognizer::horizontal_drag()
        .drag_start_behavior(DragStartBehavior::Down)
        .max_consecutive_tap(3)
        .on_drag_end({
            let point = point.clone();
            move |d| point.set(Some(d.global_position))
        });
    let base = Instant::now();
    for (phase, at) in [
        (Down, A),
        (Move, (50., 20.)),
        (Move, (90., 30.)),
        (Move, (120., 45.)),
        (Up, (120., 45.)),
    ] {
        g.handle_raw(event(base, 0, 5, phase, at));
    }
    assert_eq!(point.get(), Some(Offset::new(120., 45.)));
}
