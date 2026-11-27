use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerPhase, SECONDARY_POINTER_BUTTON, time::Instant,
};
use incular_gestures::{
    ButtonGestureCallbacks, DoubleTapGestureRecognizer, GestureArenaClient, GestureArenaKey,
    GestureArenaManager, GestureArenaRegistration, GestureCallbacks, GestureDisposition,
    GestureRecognizer, PointerDeviceKind, PointerGestureRecognizer, RawPointerEvent,
};

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Down(Offset, Offset),
    Double,
    Cancel,
}

#[derive(Default)]
struct Contender {
    accepted: Cell<usize>,
    rejected: Cell<usize>,
}

impl GestureArenaClient for Contender {
    fn accept_gesture(&self, _: GestureArenaKey) {
        self.accepted.set(self.accepted.get() + 1);
    }

    fn reject_gesture(&self, _: GestureArenaKey) {
        self.rejected.set(self.rejected.get() + 1);
    }
}

struct Fixture {
    arena: GestureArenaManager,
    recognizer: DoubleTapGestureRecognizer,
    begin: Instant,
    trace: Rc<RefCell<Vec<Event>>>,
}

impl Fixture {
    fn new() -> Self {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let on_down = trace.clone();
        let on_tap = trace.clone();
        let on_cancel = trace.clone();
        let recognizer = DoubleTapGestureRecognizer::new()
            .on_double_tap_down(move |details| {
                on_down
                    .borrow_mut()
                    .push(Event::Down(details.global_position, details.local_position))
            })
            .on_double_tap(move || on_tap.borrow_mut().push(Event::Double))
            .on_double_tap_cancel(move || on_cancel.borrow_mut().push(Event::Cancel));
        Self {
            arena: GestureArenaManager::new(),
            recognizer,
            begin: Instant::now(),
            trace,
        }
    }

    fn event(&self, pointer: u64, ms: u64, phase: PointerPhase, buttons: u32) -> RawPointerEvent {
        let down = if pointer == 2 {
            Offset::new(12., 12.)
        } else {
            Offset::new(10., 10.)
        };
        RawPointerEvent::new(
            pointer,
            0,
            PointerDeviceKind::Touch,
            buttons,
            if phase == PointerPhase::Up {
                down + Offset::new(1., -1.)
            } else {
                down
            },
            phase,
            self.begin + Duration::from_millis(ms),
        )
    }

    fn key(pointer: u64) -> GestureArenaKey {
        GestureArenaKey::pointer(0, pointer)
    }

    fn down(
        &self,
        pointer: u64,
        ms: u64,
        competitor: bool,
    ) -> Option<(Rc<Contender>, GestureArenaRegistration)> {
        let key = Self::key(pointer);
        let down = self.event(pointer, ms, PointerPhase::Down, PRIMARY_POINTER_BUTTON);
        assert!(self.recognizer.add_pointer(&self.arena, key, down));
        let other = competitor.then(|| {
            let probe = Rc::new(Contender::default());
            let registration = self.arena.add(key, probe.clone());
            (probe, registration)
        });
        self.arena.close(key);
        self.arena.flush_deferred_resolutions();
        self.recognizer.route(key, down);
        other
    }

    fn up(&self, pointer: u64, ms: u64) {
        self.recognizer.route(
            Self::key(pointer),
            self.event(pointer, ms, PointerPhase::Up, 0),
        );
    }

    fn finish(&self, pointer: u64, ms: u64) {
        self.up(pointer, ms);
        self.arena.sweep(Self::key(pointer));
    }

    fn at(&self, ms: u64) -> Instant {
        self.begin + Duration::from_millis(ms)
    }

    fn expect(&self, expected: &[Event]) {
        assert_eq!(&*self.trace.borrow(), expected);
    }
}

#[test]
fn a_competitor_can_win_the_first_arena_before_its_sweep() {
    let f = Fixture::new();
    let (probe, other) = f.down(1, 0, true).unwrap();
    f.up(1, 1);
    other.resolve(GestureDisposition::Accepted);
    assert_eq!(probe.accepted.get(), 1);
    f.arena.sweep(Fixture::key(1));
    f.down(2, 100, false);
    f.finish(2, 101);
    f.expect(&[]);
}

#[test]
fn a_competitor_can_win_the_held_first_arena_between_taps() {
    let f = Fixture::new();
    let (probe, other) = f.down(1, 0, true).unwrap();
    f.finish(1, 1);
    assert_eq!(probe.accepted.get(), 0);
    other.resolve(GestureDisposition::Accepted);
    assert_eq!(probe.accepted.get(), 1);
    f.down(2, 100, false);
    f.finish(2, 101);
    f.expect(&[]);
}

#[test]
fn losing_the_first_arena_during_the_second_press_cancels_once() {
    let f = Fixture::new();
    let (probe, other) = f.down(1, 0, true).unwrap();
    f.finish(1, 1);
    f.expect(&[]);
    f.down(2, 100, false);
    let position = Offset::new(12., 12.);
    f.expect(&[Event::Down(position, position)]);
    other.resolve(GestureDisposition::Accepted);
    assert_eq!(probe.accepted.get(), 1);
    f.expect(&[Event::Down(position, position), Event::Cancel]);
    f.finish(2, 101);
    f.expect(&[Event::Down(position, position), Event::Cancel]);
}

#[test]
fn a_passive_member_wins_only_when_the_timeout_releases_the_arena() {
    let f = Fixture::new();
    let (probe, _) = f.down(1, 0, true).unwrap();
    f.finish(1, 1);
    assert_eq!(probe.accepted.get(), 0);
    assert_eq!(f.recognizer.deadline(), Some(f.at(301)));
    f.recognizer.advance_to(f.at(300));
    assert_eq!(probe.accepted.get(), 0);
    f.recognizer.advance_to(f.at(301));
    f.recognizer.advance_to(f.at(5000));
    assert_eq!(probe.accepted.get(), 1);
    f.expect(&[]);
}

struct TapDownClient(RefCell<PointerGestureRecognizer>);

impl GestureArenaClient for TapDownClient {
    fn accept_gesture(&self, key: GestureArenaKey) {
        self.0
            .borrow_mut()
            .accept_tap_on_arena_close(key.pointer_id().unwrap());
    }

    fn reject_gesture(&self, _: GestureArenaKey) {
        self.0.borrow_mut().reject();
    }
}

fn tap_down_client(
    f: &Fixture,
    event: RawPointerEvent,
    record: Rc<RefCell<Vec<&'static str>>>,
) -> Rc<TapDownClient> {
    let primary = record.clone();
    let callbacks = GestureCallbacks {
        on_tap_down: Some(Rc::new(move |_| primary.borrow_mut().push("primary"))),
        secondary: ButtonGestureCallbacks {
            on_tap_down: Some(Rc::new(move |_| record.borrow_mut().push("secondary"))),
            ..Default::default()
        },
        ..Default::default()
    };
    let selected = callbacks.for_button_mask(event.buttons).unwrap();
    let tap = Rc::new(TapDownClient(RefCell::new(PointerGestureRecognizer::new(
        selected,
    ))));
    f.arena.add(Fixture::key(event.pointer), tap.clone());
    tap.0.borrow_mut().observe_raw(event);
    tap
}

#[test]
fn a_primary_double_does_not_delay_a_secondary_tap_down() {
    let f = Fixture::new();
    let key = Fixture::key(6);
    let down = f.event(6, 0, PointerPhase::Down, SECONDARY_POINTER_BUTTON);
    assert!(!f.recognizer.add_pointer(&f.arena, key, down));
    let read = Rc::new(RefCell::new(Vec::new()));
    let _tap = tap_down_client(&f, down, read.clone());
    f.arena.close(key);
    f.arena.flush_deferred_resolutions();
    assert_eq!(&*read.borrow(), &["secondary"]);
    f.expect(&[]);
}

#[test]
fn a_primary_double_delays_the_competing_primary_tap_down() {
    let f = Fixture::new();
    let key = Fixture::key(1);
    let down = f.event(1, 0, PointerPhase::Down, PRIMARY_POINTER_BUTTON);
    assert!(f.recognizer.add_pointer(&f.arena, key, down));
    let read = Rc::new(RefCell::new(Vec::new()));
    let tap = tap_down_client(&f, down, read.clone());
    f.arena.close(key);
    f.arena.flush_deferred_resolutions();
    assert!(read.borrow().is_empty());
    tap.0.borrow_mut().poll(f.at(300));
    assert_eq!(&*read.borrow(), &["primary"]);
    f.expect(&[]);
}

#[test]
fn two_completed_presses_accept_both_contests_and_only_emit_one_double_tap() {
    let f = Fixture::new();
    let (first, _) = f.down(1, 0, true).unwrap();
    f.finish(1, 1);
    let (second, _) = f.down(2, 100, true).unwrap();
    assert_eq!(f.recognizer.deadline(), None);
    f.recognizer.advance_to(f.at(900));
    f.finish(2, 901);
    assert_eq!((first.rejected.get(), second.rejected.get()), (1, 1));
    let position = Offset::new(12., 12.);
    f.expect(&[Event::Down(position, position), Event::Double]);
}

#[test]
fn dropping_a_waiting_recognizer_releases_the_still_live_arena() {
    let f = Fixture::new();
    let (probe, _) = f.down(1, 0, true).unwrap();
    f.finish(1, 1);
    assert_eq!(probe.accepted.get(), 0);
    let Fixture {
        arena, recognizer, ..
    } = f;
    drop(recognizer);
    assert_eq!(probe.accepted.get(), 1);
    arena.flush_deferred_resolutions();
    assert_eq!(probe.accepted.get(), 1);
}

#[test]
fn a_cancel_callback_panic_cannot_leave_a_passive_contender_held() {
    let mut f = Fixture::new();
    f.recognizer = DoubleTapGestureRecognizer::new()
        .on_double_tap_cancel(|| panic!("application cancel handler"));
    let (probe, _) = f.down(1, 0, true).unwrap();
    f.finish(1, 1);
    f.down(2, 100, false);

    let callback_error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        f.recognizer
            .route(Fixture::key(2), f.event(2, 101, PointerPhase::Cancel, 0));
    }));
    assert!(callback_error.is_err());
    assert_eq!(probe.accepted.get(), 1);
    assert_eq!(f.recognizer.deadline(), None);
}
