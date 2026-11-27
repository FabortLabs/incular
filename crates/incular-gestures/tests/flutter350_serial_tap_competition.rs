use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_core::{Offset, PointerPhase, time::Instant};
use incular_gestures::{
    DoubleTapGestureRecognizer, GestureAction, GestureArenaClient, GestureArenaKey,
    GestureArenaManager, GestureArenaRegistration, GestureCallbacks, GestureDisposition,
    GestureRecognizer, PointerDeviceKind, PointerGestureRecognizer, RawPointerEvent,
    SerialTapGestureRecognizer,
};

type Log = Rc<RefCell<Vec<(&'static str, u32)>>>;

fn serial(log: &Log) -> SerialTapGestureRecognizer {
    SerialTapGestureRecognizer::new()
        .on_serial_tap_down({
            let log = log.clone();
            move |detail| log.borrow_mut().push(("down", detail.count))
        })
        .on_serial_tap_up({
            let log = log.clone();
            move |detail| log.borrow_mut().push(("up", detail.count))
        })
        .on_serial_tap_cancel({
            let log = log.clone();
            move |detail| log.borrow_mut().push(("cancel", detail.count))
        })
}

fn events(base: Instant, pointer: u64, ms: u64) -> (RawPointerEvent, RawPointerEvent) {
    let time = base + Duration::from_millis(ms);
    let (down, up) = if pointer == 1 {
        ((10., 10.), (11., 9.))
    } else {
        ((12., 12.), (13., 11.))
    };
    let event = |phase, point: (f32, f32), buttons| {
        RawPointerEvent::new(
            pointer,
            0,
            PointerDeviceKind::Touch,
            buttons,
            Offset::new(point.0, point.1),
            phase,
            time,
        )
    };
    (
        event(PointerPhase::Down, down, 1),
        event(PointerPhase::Up, up, 0),
    )
}

struct SerialClient {
    serial: RefCell<SerialTapGestureRecognizer>,
    requests: Rc<RefCell<Vec<(u64, GestureDisposition)>>>,
}

impl SerialClient {
    fn new(log: &Log) -> Rc<Self> {
        let requests = Rc::<RefCell<Vec<(u64, GestureDisposition)>>>::default();
        let queue = requests.clone();
        Rc::new(Self {
            serial: RefCell::new(
                serial(log).with_external_arena(move |pointer, disposition| {
                    queue.borrow_mut().push((pointer, disposition));
                    None
                }),
            ),
            requests,
        })
    }

    fn add(
        self: &Rc<Self>,
        arena: &GestureArenaManager,
        key: GestureArenaKey,
        down: RawPointerEvent,
    ) -> GestureArenaRegistration {
        let entry = arena.add(key, self.clone());
        self.serial.borrow_mut().poll(down.time);
        self.serial.borrow_mut().add_pointer(down);
        entry
    }

    fn route(&self, entry: &GestureArenaRegistration, event: RawPointerEvent) {
        self.serial.borrow_mut().handle_event(event);
        for (pointer, resolution) in self.requests.take() {
            assert_eq!(pointer, event.pointer);
            entry.resolve(resolution);
        }
    }
}

impl GestureArenaClient for SerialClient {
    fn accept_gesture(&self, key: GestureArenaKey) {
        self.serial
            .borrow_mut()
            .accept_gesture(key.pointer_id().unwrap());
    }

    fn reject_gesture(&self, key: GestureArenaKey) {
        self.serial
            .borrow_mut()
            .reject_gesture(key.pointer_id().unwrap());
    }
}

struct SingleTap {
    recognizer: RefCell<PointerGestureRecognizer>,
    routed: Cell<bool>,
}

impl SingleTap {
    fn route(&self, event: RawPointerEvent) {
        if self.routed.get() {
            // Tap records the release but does not claim an unresolved
            // arena here. It dispatches only if the arena chooses it.
            let _ = self.recognizer.borrow_mut().observe_raw(event);
        }
    }
}

impl GestureArenaClient for SingleTap {
    fn accept_gesture(&self, _: GestureArenaKey) {
        self.recognizer.borrow_mut().dispatch(GestureAction::Tap);
    }

    fn reject_gesture(&self, _: GestureArenaKey) {
        self.routed.set(false);
        self.recognizer.borrow_mut().cancel();
    }
}

fn serial_wins_over_single_tap(serial_first: bool) {
    let log = Log::default();
    let serial = SerialClient::new(&log);
    let winner = Rc::new(Cell::new(false));
    let canceled = Rc::new(Cell::new(false));
    let peers = Rc::new(SingleTap {
        recognizer: RefCell::new(PointerGestureRecognizer::new(GestureCallbacks {
            on_tap: Some({
                let winner = winner.clone();
                Rc::new(move || winner.set(true))
            }),
            on_tap_cancel: Some({
                let canceled = canceled.clone();
                Rc::new(move || canceled.set(true))
            }),
            ..GestureCallbacks::default()
        })),
        routed: Cell::new(true),
    });
    let arena = GestureArenaManager::new();
    let base = Instant::now();
    let (down, mut up) = events(base, 1, 0);
    let key = GestureArenaKey::pointer(1, 1);
    let add_serial = || serial.add(&arena, key, down);
    let entry = if serial_first {
        let entry = add_serial();
        let _peer = arena.add(key, peers.clone());
        entry
    } else {
        let _peer = arena.add(key, peers.clone());
        add_serial()
    };
    arena.close(key);
    if serial_first {
        serial.route(&entry, down);
        peers.route(down);
    } else {
        peers.route(down);
        serial.route(&entry, down);
    }
    up.time += Duration::from_millis(100);
    let _ = peers.recognizer.borrow_mut().poll(up.time);
    if serial_first {
        serial.route(&entry, up);
        peers.route(up);
    } else {
        peers.route(up);
        serial.route(&entry, up);
    }
    arena.sweep(key);
    assert_eq!(log.take(), [("down", 1), ("up", 1)]);
    assert!(!winner.get());
    assert!(canceled.get());
}

#[test]
fn serial_wins_over_the_single_tap_below_it() {
    serial_wins_over_single_tap(false);
}

#[test]
fn serial_wins_over_the_single_tap_above_it() {
    serial_wins_over_single_tap(true);
}

#[derive(Default)]
struct Eager {
    rejected: Cell<bool>,
    released: Cell<bool>,
}

impl GestureArenaClient for Eager {
    fn accept_gesture(&self, _: GestureArenaKey) {}

    fn reject_gesture(&self, _: GestureArenaKey) {
        self.rejected.set(true);
    }
}

impl Eager {
    fn release(&self, entry: &GestureArenaRegistration) {
        if !self.rejected.get() {
            entry.resolve(GestureDisposition::Accepted);
            self.released.set(true);
        }
    }
}

fn serial_competes_with_eager(serial_first: bool, accept_on_down: bool) {
    let log = Log::default();
    let serial = SerialClient::new(&log);
    let peer = Rc::new(Eager::default());
    let arena = GestureArenaManager::new();
    let (down, up) = events(Instant::now(), 1, 0);
    let key = GestureArenaKey::pointer(1, 1);
    let enroll_serial = || serial.add(&arena, key, down);
    let enroll_peer = || arena.add(key, peer.clone());
    let (se, pe) = if serial_first {
        (enroll_serial(), enroll_peer())
    } else {
        let pe = enroll_peer();
        (enroll_serial(), pe)
    };
    arena.close(key);
    if accept_on_down {
        pe.resolve(GestureDisposition::Accepted);
    }
    serial.route(&se, down);
    if serial_first {
        serial.route(&se, up);
        peer.release(&pe);
    } else {
        if !accept_on_down {
            peer.release(&pe);
        }
        serial.route(&se, up);
    }
    arena.sweep(key);
    let serial_won = serial_first && !accept_on_down;
    assert_eq!(
        log.take(),
        [("down", 1), (if serial_won { "up" } else { "cancel" }, 1)]
    );
    if !accept_on_down {
        assert_eq!(peer.released.get(), !serial_won);
    }
}

#[test]
fn serial_loses_to_release_if_the_release_recognizer_was_routed_first() {
    serial_competes_with_eager(false, false);
}

#[test]
fn serial_wins_if_routed_before_the_release_recognizer() {
    serial_competes_with_eager(true, false);
}

#[test]
fn serial_cancels_when_a_competitor_accepts_during_the_press() {
    serial_competes_with_eager(false, true);
}

fn serial_wins_over_double_tap(serial_first: bool) {
    let log = Log::default();
    let serial = SerialClient::new(&log);
    let recognized = Rc::new(Cell::new(0));
    let tap = DoubleTapGestureRecognizer::new().on_double_tap({
        let recognized = recognized.clone();
        move || recognized.set(recognized.get() + 1)
    });
    let arena = GestureArenaManager::new();
    let base = Instant::now();
    for pointer in 1..=3_u64 {
        let (down, up) = events(base, pointer, (pointer - 1) * 150);
        let key = GestureArenaKey::pointer(1, pointer);
        let in_fixture = serial_first || pointer < 3;
        let entry = if serial_first {
            let entry = serial.add(&arena, key, down);
            assert!(tap.add_pointer(&arena, key, down));
            entry
        } else {
            if in_fixture {
                assert!(tap.add_pointer(&arena, key, down));
            }
            serial.add(&arena, key, down)
        };
        arena.close(key);
        if !serial_first && in_fixture {
            tap.route(key, down);
        }
        serial.route(&entry, down);
        if serial_first {
            tap.route(key, down);
            serial.route(&entry, up);
            tap.route(key, up);
        } else {
            if in_fixture {
                tap.route(key, up);
            }
            serial.route(&entry, up);
        }
        arena.sweep(key);
        assert_eq!(
            log.take(),
            [("down", pointer as u32), ("up", pointer as u32)]
        );
        assert_eq!(recognized.get(), 0);
    }
}

#[test]
fn serial_wins_over_double_tap_below_it() {
    serial_wins_over_double_tap(false);
}

#[test]
fn serial_wins_over_double_tap_above_it() {
    serial_wins_over_double_tap(true);
}

#[test]
fn deferred_arena_resolution_cannot_publish_both_up_and_cancel() {
    let log = Log::default();
    let mut gesture = serial(&log).with_external_arena(|_, _| None);
    let base = Instant::now();
    let (down, up) = events(base, 1, 0);
    gesture.add_pointer(down);
    gesture.handle_event(up);
    assert_eq!(log.take(), [("down", 1)]);
    assert!(gesture.is_tracking_pointer());
    gesture.reject_gesture(1);
    assert_eq!(log.take(), [("cancel", 1)]);
    assert!(!gesture.is_tracking_pointer());
    assert_eq!(gesture.next_deadline(), None);

    let (down, up) = events(base, 2, 150);
    gesture.add_pointer(down);
    gesture.handle_event(up);
    gesture.reject_gesture(1);
    assert_eq!(log.take(), [("down", 1)]);
    gesture.accept_gesture(2);
    gesture.accept_gesture(2);
    assert_eq!(log.take(), [("up", 1)]);
    assert!(!gesture.is_tracking_pointer());
}

#[test]
fn synchronous_arena_rejection_on_release_does_not_publish_up() {
    let log = Log::default();
    let mut recognizer =
        serial(&log).with_external_arena(|_, _| Some(GestureDisposition::Rejected));
    let (down, up) = events(Instant::now(), 1, 0);
    recognizer.add_pointer(down);
    recognizer.handle_event(up);
    assert_eq!(log.take(), [("down", 1), ("cancel", 1)]);
    assert_eq!(recognizer.next_deadline(), None);
}

#[test]
fn raw_gesture_adapter_waits_for_the_detector_before_dispatching_serial_up() {
    use incular_gestures::{ArenaGestureRecognizer, GestureDecision};

    let log = Log::default();
    let mut wrapped = ArenaGestureRecognizer::new(serial(&log));
    let base = Instant::now();
    let (down, up) = events(base, 1, 0);
    assert_eq!(wrapped.observe_raw(down), GestureDecision::Pending);
    assert_eq!(
        wrapped.observe_raw(up),
        GestureDecision::Accept(GestureAction::ArenaAccepted { pointer: 1 })
    );
    assert_eq!(log.take(), [("down", 1)]);
    wrapped.cancel();
    assert_eq!(log.take(), [("cancel", 1)]);

    let (down, up) = events(base, 2, 150);
    assert_eq!(wrapped.observe_raw(down), GestureDecision::Pending);
    assert_eq!(
        wrapped.observe_raw(up),
        GestureDecision::Accept(GestureAction::ArenaAccepted { pointer: 2 })
    );
    assert_eq!(log.take(), [("down", 1)]);
    wrapped.dispatch(GestureAction::ArenaAccepted { pointer: 2 });
    assert_eq!(log.take(), [("up", 1)]);
}
