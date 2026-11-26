//! Flutter 3.50.0 serial_tap_test, tap_test and double_tap_test.
//! Times advance the recognizer clock without sleeping. Coordinates, pointer
//! identity and release/cancel ordering come from the named upstream cases.
use incular_core::{Offset, PointerPhase, time::Instant};
use incular_gestures::{
    ButtonGestureCallbacks, GestureCallbacks, GestureDecision, PointerButtonClass,
    PointerDeviceKind, PointerEvent, PointerGestureRecognizer, RawPointerEvent,
    SerialTapGestureRecognizer, TapDownDetails,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

use PointerPhase::{Cancel, Down, Move, Up};

fn at(base: Instant, ms: u64) -> Instant {
    base + Duration::from_millis(ms)
}

type Point = (f32, f32);
const A: Point = (10., 10.);
const A_UP: Point = (11., 9.);
const B: Point = (12., 12.);
const B_UP: Point = (13., 11.);

fn raw(
    base: Instant,
    ms: u64,
    pointer: u64,
    phase: PointerPhase,
    xy: Point,
    buttons: u32,
) -> RawPointerEvent {
    RawPointerEvent::new(
        pointer,
        0,
        PointerDeviceKind::Touch,
        buttons,
        Offset::new(xy.0, xy.1),
        phase,
        at(base, ms),
    )
}

type SerialLog = Rc<RefCell<Vec<(&'static str, u32)>>>;
struct Serial {
    gesture: SerialTapGestureRecognizer,
    log: SerialLog,
    base: Instant,
}

impl Serial {
    fn new() -> Self {
        let log = SerialLog::default();
        let gesture = SerialTapGestureRecognizer::new()
            .on_serial_tap_down({
                let log = log.clone();
                move |d| log.borrow_mut().push(("down", d.count))
            })
            .on_serial_tap_cancel({
                let log = log.clone();
                move |d| log.borrow_mut().push(("cancel", d.count))
            })
            .on_serial_tap_up({
                let log = log.clone();
                move |d| log.borrow_mut().push(("up", d.count))
            });
        Self {
            gesture,
            log,
            base: Instant::now(),
        }
    }
    fn event(&mut self, ms: u64, id: u64, phase: PointerPhase, xy: Point, button: u32) {
        self.gesture
            .handle_raw(raw(self.base, ms, id, phase, xy, button));
    }
    fn tap(&mut self, ms: u64, id: u64, xy: Point, up: Point, button: u32) {
        self.event(ms, id, Down, xy, button);
        self.event(ms, id, Up, up, 0);
    }
    fn expect(&self, expected: &[(&'static str, u32)]) {
        assert_eq!(self.log.take(), expected);
    }
}

#[test]
fn serial_counts_three_completed_taps() {
    let mut s = Serial::new();
    s.tap(0, 1, A, A_UP, 1);
    s.expect(&[("down", 1), ("up", 1)]);
    s.tap(150, 2, B, B_UP, 1);
    s.expect(&[("down", 2), ("up", 2)]);
    s.tap(300, 3, B, B_UP, 1);
    s.expect(&[("down", 3), ("up", 3)]);
}

#[test]
fn serial_pointer_cancel_resets_before_the_next_tap() {
    let mut s = Serial::new();
    s.event(0, 1, Down, A, 1);
    s.event(0, 1, Cancel, (0., 0.), 0);
    s.expect(&[("down", 1), ("cancel", 1)]);
    s.tap(100, 2, B, B_UP, 1);
    s.expect(&[("down", 1), ("up", 1)]);
}

#[test]
fn serial_drag_cancels_without_up_and_resets() {
    let mut s = Serial::new();
    s.event(0, 5, Down, A, 1);
    s.event(0, 5, Move, (25., 25.), 1);
    s.event(0, 5, Up, (25., 25.), 0);
    s.expect(&[("down", 1), ("cancel", 1)]);
    s.tap(1000, 1, A, A_UP, 1);
    s.expect(&[("down", 1), ("up", 1)]);
}

#[test]
fn serial_inter_tap_timeout_restarts_count() {
    let mut s = Serial::new();
    s.tap(0, 1, A, A_UP, 1);
    s.expect(&[("down", 1), ("up", 1)]);
    s.tap(1000, 2, B, B_UP, 1);
    s.expect(&[("down", 1), ("up", 1)]);
}

#[test]
fn serial_distant_tap_restarts_even_inside_timeout() {
    let mut s = Serial::new();
    s.tap(0, 1, A, A_UP, 1);
    s.expect(&[("down", 1), ("up", 1)]);
    s.tap(100, 4, (130., 130.), (131., 129.), 1);
    s.expect(&[("down", 1), ("up", 1)]);
}

#[test]
fn serial_button_change_restarts_count() {
    for delay in [1000, 100] {
        // Original timeout and isolated button-change control.
        let mut s = Serial::new();
        s.tap(0, 1, A, A_UP, 1);
        s.expect(&[("down", 1), ("up", 1)]);
        s.tap(delay, 6, A, A_UP, 2);
        s.expect(&[("down", 1), ("up", 1)]);
    }
}

#[test]
fn serial_interleaved_touches_cancel_the_first_before_second_down() {
    let mut s = Serial::new();
    s.event(0, 1, Down, A, 1);
    s.event(0, 2, Down, B, 1);
    s.event(0, 1, Up, A_UP, 0);
    s.event(0, 2, Up, B_UP, 0);
    s.expect(&[("down", 1), ("cancel", 1), ("down", 1), ("up", 1)]);
}

#[test]
fn serial_without_callbacks_does_not_track() {
    let mut gesture = SerialTapGestureRecognizer::new();
    let base = Instant::now();
    gesture.handle_raw(raw(base, 0, 1, Down, A, 1));
    assert!(!gesture.is_tracking_pointer());
    gesture.handle_raw(raw(base, 0, 1, Up, A_UP, 0));
    assert_eq!(gesture.next_deadline(), None);
}

#[test]
fn serial_secondary_button_tracks_and_reports_one_completion() {
    let mut s = Serial::new();
    s.tap(0, 6, A, A_UP, 2);
    s.expect(&[("down", 1), ("up", 1)]);
}

type TapLog = Rc<RefCell<Vec<&'static str>>>;
struct Tap {
    g: PointerGestureRecognizer,
    log: TapLog,
    base: Instant,
}
fn tag(log: &TapLog, entry: &'static str) -> Rc<dyn Fn()> {
    let log = log.clone();
    Rc::new(move || log.borrow_mut().push(entry))
}
fn detail_tag<T: 'static>(log: &TapLog, entry: &'static str) -> Rc<dyn Fn(T)> {
    let log = log.clone();
    Rc::new(move |_| log.borrow_mut().push(entry))
}
fn button(log: &TapLog, [down, up, cancel]: [&'static str; 3]) -> ButtonGestureCallbacks {
    ButtonGestureCallbacks {
        on_tap_down: Some(detail_tag(log, down)),
        on_tap_up: Some(detail_tag(log, up)),
        on_tap_cancel: Some(tag(log, cancel)),
        ..ButtonGestureCallbacks::default()
    }
}
impl Tap {
    fn new(buttons: u32, details: bool) -> Self {
        let log = TapLog::default();
        let mut callbacks = GestureCallbacks {
            on_tap: Some(tag(&log, "tap")),
            ..GestureCallbacks::default()
        };
        if details {
            callbacks.on_tap_down = Some(detail_tag(&log, "primaryDown"));
            callbacks.on_tap_up = Some(detail_tag(&log, "primaryUp"));
            callbacks.on_tap_cancel = Some(tag(&log, "primaryCancel"));
            callbacks.secondary = button(&log, ["secondaryDown", "secondaryUp", "secondaryCancel"]);
            callbacks.tertiary = button(&log, ["tertiaryDown", "tertiaryUp", "tertiaryCancel"]);
        } else {
            callbacks.on_tap_cancel = Some(tag(&log, "cancel"));
        }
        Self {
            g: PointerGestureRecognizer::new(callbacks.for_button_class(
                PointerButtonClass::of_press(buttons).expect("recognized button mask"),
            )),
            log,
            base: Instant::now(),
        }
    }
    fn primary() -> Self {
        Self::new(1, false)
    }
    fn event(&mut self, ms: u64, pointer: u64, phase: PointerPhase, xy: Point) {
        self.g
            .handle(pointer_event(self.base, ms, pointer, phase, xy));
    }
    fn poll(&mut self, ms: u64) {
        if let GestureDecision::Accept(action) = self.g.poll(at(self.base, ms)) {
            self.g.dispatch(action);
        }
    }
    fn expect(&self, expected: &[&str]) {
        assert_eq!(self.log.take(), expected);
    }
}
fn pointer_event(
    base: Instant,
    ms: u64,
    pointer: u64,
    phase: PointerPhase,
    xy: Point,
) -> PointerEvent {
    PointerEvent {
        pointer,
        position: Offset::new(xy.0, xy.1),
        phase,
        time: at(base, ms),
    }
}

#[test]
fn tap_dispatches_once_on_release_not_press() {
    let mut t = Tap::primary();
    t.event(0, 1, Down, A);
    t.expect(&[]);
    t.event(0, 1, Up, A_UP);
    t.expect(&["tap"]);
}

#[test]
fn two_completed_presses_on_reused_pointer_emit_two_callbacks() {
    let mut t = Tap::primary();
    for ms in [0, 100] {
        t.event(ms, 1, Down, A);
        t.expect(&[]);
        t.event(ms, 1, Up, A_UP);
        t.expect(&["tap"]);
    }
}

fn overlapping_taps(first_release: u64) {
    let mut t = Tap::primary();
    t.event(0, 1, Down, A);
    t.event(0, 2, Down, (30., 30.));
    t.expect(&[]);
    for pointer in [first_release, 3 - first_release] {
        t.event(0, pointer, Up, if pointer == 1 { A_UP } else { (31., 29.) });
        t.expect(if pointer == 1 { &["tap"] } else { &[] });
    }
}

#[test]
fn overlapping_single_taps_fifo_keeps_first_press() {
    overlapping_taps(1);
}
#[test]
fn overlapping_single_taps_filo_keeps_first_press() {
    overlapping_taps(2);
}

#[test]
fn unrelated_move_and_cancel_do_not_clear_the_primary_tap() {
    let mut t = Tap::primary();
    t.event(0, 1, Down, A);
    t.event(20, 2, Down, (50., 50.));
    t.event(40, 2, Move, (90., 90.));
    t.event(60, 2, Cancel, (90., 90.));
    t.expect(&[]);
    t.event(80, 1, Up, A_UP);
    t.expect(&["tap"]);
}

#[test]
fn tap_diagonal_motion_outside_slop_cancels_once() {
    use incular_gestures::{GestureArena, GestureArenaKey, GestureDisposition};

    let mut t = Tap::primary();
    let mut arena = GestureArena::new();
    let key = GestureArenaKey::pointer(0, 3);
    let member = arena.add(key, false);
    t.event(0, 3, Down, A);
    t.expect(&[]);
    let result = arena.close_sole_member(key);
    assert!(result.iter().any(|entry| {
        entry.member == member && entry.disposition == GestureDisposition::Accepted
    }));
    assert!(t.g.accept_tap_on_arena_close(3));
    assert!(!t.g.accept_tap_on_arena_close(3), "no duplicated down");
    assert!(arena.close_sole_member(key).is_empty());
    t.expect(&[]);

    t.event(0, 3, Move, (25., 25.));
    t.expect(&["cancel"]);
    t.event(0, 3, Up, (25., 25.));
    t.expect(&[]);
    let _ = arena.cancel(key);
    t.expect(&[]);
}

#[test]
fn tap_diagonal_motion_inside_slop_keeps_tap_live() {
    let mut t = Tap::primary();
    t.event(0, 4, Down, A);
    t.expect(&[]);
    t.event(0, 4, Move, (22., 22.));
    t.expect(&[]);
    t.event(0, 4, Up, (22., 22.));
    t.expect(&["tap"]);
}

#[test]
fn tap_without_long_press_callback_survives_500ms() {
    let mut t = Tap::primary();
    t.event(0, 1, Down, A);
    t.expect(&[]);
    t.poll(500);
    t.expect(&[]);
    t.event(500, 1, Up, A_UP);
    t.expect(&["tap"]);
}

fn button_trace(
    buttons: u32,
    id: u64,
    xy: Point,
    up: Point,
    ending: PointerPhase,
    down: &str,
    end: &[&str],
) {
    let mut t = Tap::new(buttons, true);
    t.event(0, id, Down, xy);
    t.expect(&[]);
    t.poll(500);
    t.expect(&[down]);
    t.event(500, id, ending, up);
    t.expect(end);
}

#[test]
fn primary_tap_callback_order() {
    button_trace(1, 1, A, A_UP, Up, "primaryDown", &["primaryUp", "tap"]);
}
#[test]
fn primary_tap_cancel_callback_order() {
    button_trace(1, 1, A, (0., 0.), Cancel, "primaryDown", &["primaryCancel"]);
}
#[test]
fn secondary_tap_callback_order() {
    button_trace(
        2,
        5,
        (20., 20.),
        (20., 20.),
        Up,
        "secondaryDown",
        &["secondaryUp"],
    );
}
#[test]
fn tertiary_tap_callback_order() {
    button_trace(
        4,
        6,
        (20., 20.),
        (20., 20.),
        Up,
        "tertiaryDown",
        &["tertiaryUp"],
    );
}
#[test]
fn secondary_tap_cancel_callback_order() {
    button_trace(
        2,
        5,
        (20., 20.),
        (0., 0.),
        Cancel,
        "secondaryDown",
        &["secondaryCancel"],
    );
}
#[test]
fn tertiary_tap_cancel_callback_order() {
    button_trace(
        4,
        6,
        (20., 20.),
        (0., 0.),
        Cancel,
        "tertiaryDown",
        &["tertiaryCancel"],
    );
}

#[derive(Debug, PartialEq)]
enum DoubleEvent {
    Down(Offset, Offset),
    Done,
    Cancel,
}
struct Double {
    g: PointerGestureRecognizer,
    log: Rc<RefCell<Vec<DoubleEvent>>>,
    base: Instant,
}
impl Double {
    fn new() -> Self {
        let log = Rc::new(RefCell::new(Vec::new()));
        let g = PointerGestureRecognizer::new(GestureCallbacks {
            on_double_tap_down: Some(Rc::new({
                let log = log.clone();
                move |d: TapDownDetails| {
                    log.borrow_mut()
                        .push(DoubleEvent::Down(d.global_position, d.local_position))
                }
            })),
            on_double_tap: Some(Rc::new({
                let log = log.clone();
                move || log.borrow_mut().push(DoubleEvent::Done)
            })),
            on_double_tap_cancel: Some(Rc::new({
                let log = log.clone();
                move || log.borrow_mut().push(DoubleEvent::Cancel)
            })),
            ..GestureCallbacks::default()
        });
        Self {
            g,
            log,
            base: Instant::now(),
        }
    }
    fn event(&mut self, ms: u64, pointer: u64, phase: PointerPhase, xy: Point) {
        self.g
            .handle(pointer_event(self.base, ms, pointer, phase, xy));
    }
    fn tap(&mut self, ms: u64, id: u64, xy: Point, up: Point) {
        self.event(ms, id, Down, xy);
        self.event(ms, id, Up, up);
    }
    fn expect(&self, expected: &[DoubleEvent]) {
        assert_eq!(self.log.take(), expected);
    }
    fn completes(&mut self, ms: u64, pointer: u64, xy: Point, up: Point) {
        self.event(ms, pointer, Down, xy);
        let p = Offset::new(xy.0, xy.1);
        self.expect(&[DoubleEvent::Down(p, p)]);
        self.event(ms, pointer, Up, up);
        self.expect(&[DoubleEvent::Done]);
    }
}

#[test]
fn double_tap_down_contains_second_global_and_local_positions() {
    let mut d = Double::new();
    d.tap(0, 1, A, A_UP);
    d.expect(&[]);
    d.completes(100, 2, B, B_UP);
}

#[test]
fn double_tap_rejects_far_second_press_silently() {
    let mut d = Double::new();
    d.tap(0, 1, A, A_UP);
    d.tap(0, 3, (130., 130.), (131., 129.));
    d.expect(&[]);
}
#[test]
fn double_tap_rejects_large_motion_during_first_tap() {
    let mut d = Double::new();
    d.event(0, 4, Down, A);
    d.event(0, 4, Move, (25., 25.));
    d.event(0, 4, Up, (25., 25.));
    d.tap(0, 1, A, A_UP);
    d.expect(&[]);
}
#[test]
fn double_tap_rejects_second_press_after_timeout() {
    let mut d = Double::new();
    d.tap(0, 1, A, A_UP);
    d.tap(5000, 2, B, B_UP);
    d.expect(&[]);
}
#[test]
fn double_tap_after_timeout_uses_second_press_as_new_first() {
    let mut d = Double::new();
    d.tap(0, 1, A, A_UP);
    d.tap(5000, 2, B, B_UP);
    d.expect(&[]);
    d.completes(5100, 5, A, A_UP);
}
#[test]
fn double_tap_gap_is_measured_from_first_release_not_first_press() {
    let mut d = Double::new();
    d.event(0, 1, Down, A);
    d.event(1000, 1, Up, A_UP);
    d.expect(&[]);
    d.completes(1000, 2, B, B_UP);
}
#[test]
fn double_tap_ignores_overlapping_presses() {
    let mut d = Double::new();
    d.event(0, 1, Down, A);
    d.event(0, 2, Down, B);
    d.event(0, 1, Up, A_UP);
    d.event(0, 2, Up, B_UP);
    d.expect(&[]);
}
#[test]
fn double_tap_retains_one_of_overlapping_presses_for_the_next_tap() {
    let mut d = Double::new();
    d.event(0, 1, Down, A);
    d.event(0, 2, Down, B);
    d.event(0, 1, Up, A_UP);
    d.event(0, 2, Up, B_UP);
    d.expect(&[]);
    d.completes(100, 1, A, A_UP);
}
#[test]
fn double_tap_rejects_two_presses_10ms_apart_silently() {
    let mut d = Double::new();
    d.tap(0, 1, A, A_UP);
    d.tap(10, 2, B, B_UP);
    d.expect(&[]);
}
#[test]
fn double_tap_rapid_second_press_becomes_first_tap_of_next_pair() {
    let mut d = Double::new();
    d.tap(0, 1, A, A_UP);
    d.tap(10, 2, B, B_UP);
    d.expect(&[]);
    d.completes(110, 5, A, A_UP);
}
