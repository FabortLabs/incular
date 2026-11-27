//! Compare the pinned Flutter standalone arena, including its held sweep.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_core::{DragStartBehavior, Offset, PointerPhase, time::Instant};
use incular_gestures::{
    DoubleTapGestureRecognizer, GestureAction, GestureArenaClient, GestureArenaKey,
    GestureArenaManager, GestureArenaRegistration, GestureCallbacks, GestureDecision,
    GestureDisposition, GestureRecognizer, PointerDeviceKind, PointerGestureRecognizer,
    RawPointerEvent, TapAndDragGestureRecognizer,
};

struct Combined(RefCell<TapAndDragGestureRecognizer>);

impl GestureArenaClient for Combined {
    fn accept_gesture(&self, key: GestureArenaKey) {
        self.0
            .borrow_mut()
            .accept_gesture(key.pointer_id().unwrap());
    }

    fn reject_gesture(&self, key: GestureArenaKey) {
        self.0
            .borrow_mut()
            .reject_gesture(key.pointer_id().unwrap());
    }
}

struct SimpleTap {
    recognizer: RefCell<PointerGestureRecognizer>,
    release: Cell<Option<GestureAction>>,
    tracked: Cell<Option<u64>>,
    winner: Cell<bool>,
}

impl SimpleTap {
    fn new(callbacks: GestureCallbacks) -> Self {
        Self {
            recognizer: RefCell::new(PointerGestureRecognizer::new(callbacks)),
            release: Cell::new(None),
            tracked: Cell::new(None),
            winner: Cell::new(false),
        }
    }

    fn route(&self, entry: &GestureArenaRegistration, event: RawPointerEvent) {
        if event.phase == PointerPhase::Down {
            self.tracked.set(Some(event.pointer));
            self.release.set(None);
            self.winner.set(false);
        }
        let decision = self.recognizer.borrow_mut().observe_raw(event);
        match decision {
            GestureDecision::Accept(action) if self.winner.get() => {
                GestureRecognizer::dispatch(&mut *self.recognizer.borrow_mut(), action);
            }
            GestureDecision::Accept(action) => self.release.set(Some(action)),
            GestureDecision::Reject | GestureDecision::Cancelled => {
                entry.resolve(GestureDisposition::Rejected);
            }
            GestureDecision::Pending => {}
        }
    }
}

impl GestureArenaClient for SimpleTap {
    fn accept_gesture(&self, key: GestureArenaKey) {
        if key.pointer_id() != self.tracked.get() {
            return;
        }
        self.winner.set(true);
        let mut tap = self.recognizer.borrow_mut();
        if let Some(action) = self.release.take() {
            GestureRecognizer::dispatch(&mut *tap, action);
        } else {
            tap.accept_tap_on_arena_close(key.pointer_id().unwrap());
        }
    }

    fn reject_gesture(&self, key: GestureArenaKey) {
        if key.pointer_id() == self.tracked.get() {
            self.release.set(None);
            self.recognizer.borrow_mut().reject();
        }
    }
}

#[test]
fn the_first_combined_tap_waits_for_the_competing_double_tap_to_release_its_sweep() {
    let trace = Rc::new(RefCell::new(Vec::<String>::new()));
    let log = |label: &'static str| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(label.to_owned())
    };
    let numbered = |label: &'static str| {
        let trace = trace.clone();
        move |count| trace.borrow_mut().push(format!("{label}#{count}"))
    };

    let requested = Rc::new(RefCell::new(Vec::new()));
    let requests = requested.clone();
    let down = numbered("down");
    let up = numbered("up");
    let start = numbered("panstart");
    let update = numbered("panupdate");
    let end = numbered("panend");
    let combined = Rc::new(Combined(RefCell::new(
        TapAndDragGestureRecognizer::pan()
            .drag_start_behavior(DragStartBehavior::Down)
            .eager_victory_on_drag(true)
            .max_consecutive_tap(3)
            .on_tap_down(move |d| down(d.consecutive_tap_count))
            .on_tap_up(move |d| up(d.consecutive_tap_count))
            .on_drag_start(move |d| start(d.consecutive_tap_count))
            .on_drag_update(move |d| update(d.consecutive_tap_count))
            .on_drag_end(move |d| end(d.consecutive_tap_count))
            .on_cancel(log("cancel"))
            .with_external_arena(move |pointer, disposition| {
                requests.borrow_mut().push((pointer, disposition));
                None
            }),
    )));
    let tap_down = log("tapdown");
    let tap_up = log("tapup");
    let single = Rc::new(SimpleTap::new(GestureCallbacks {
        on_tap_down: Some(Rc::new(move |_| tap_down())),
        on_tap_up: Some(Rc::new(move |_| tap_up())),
        on_tap_cancel: Some(Rc::new(log("tapscancel"))),
        ..GestureCallbacks::default()
    }));
    let double_down = log("doubletapdown");
    let double = DoubleTapGestureRecognizer::new()
        .on_double_tap_down(move |_| double_down())
        .on_double_tap(log("doubletapup"))
        .on_double_tap_cancel(log("doubletapcancel"));

    let arena = GestureArenaManager::new();
    let key = GestureArenaKey::pointer(0, 1);
    let time = Instant::now();
    let event = |phase, position| {
        RawPointerEvent::new(
            1,
            0,
            PointerDeviceKind::Touch,
            u32::from(phase == PointerPhase::Down),
            position,
            phase,
            time,
        )
    };
    let down = event(PointerPhase::Down, Offset::new(10., 10.));
    let up = event(PointerPhase::Up, Offset::new(11., 9.));
    let combined_entry = arena.add(key, combined.clone());
    let flush_combined = || {
        for (pointer, disposition) in requested.take() {
            assert_eq!(pointer, 1);
            combined_entry.resolve(disposition);
        }
    };
    combined.0.borrow_mut().add_pointer(down);
    flush_combined();
    let single_entry = arena.add(key, single.clone());
    assert!(double.add_pointer(&arena, key, down));
    arena.close(key);
    arena.flush_deferred_resolutions();

    for event in [down, up] {
        combined.0.borrow_mut().handle_event(event);
        flush_combined();
        single.route(&single_entry, event);
        double.route(key, event);
    }
    arena.sweep(key);
    assert!(
        trace.borrow().is_empty(),
        "the release must not settle a held arena"
    );
    for millis in [299, 300] {
        let now = time + Duration::from_millis(millis);
        combined.0.borrow_mut().poll(now);
        flush_combined();
        let _ = single.recognizer.borrow_mut().poll(now);
        double.advance_to(now);
        arena.flush_deferred_resolutions();
        if millis == 299 {
            assert!(trace.borrow().is_empty());
        }
    }
    assert_eq!(&*trace.borrow(), &["down#1", "up#1"]);
    assert!(double.deadline().is_none());
    assert!(single.release.get().is_none());
}

#[test]
fn a_held_first_arena_does_not_prevent_the_lone_tap_on_a_second_pointer() {
    let count = Rc::new(Cell::new(0));
    let tapped = count.clone();
    let single = Rc::new(SimpleTap::new(GestureCallbacks {
        on_tap: Some(Rc::new(move || tapped.set(tapped.get() + 1))),
        ..GestureCallbacks::default()
    }));
    let double = DoubleTapGestureRecognizer::new()
        .on_double_tap(|| panic!("no second pointer was registered with double tap"));
    let arena = GestureArenaManager::new();
    let first_key = GestureArenaKey::pointer(0, 1);
    let second_key = GestureArenaKey::pointer(0, 2);
    let start = Instant::now();
    let sample = |pointer, millis, phase, x, y| {
        RawPointerEvent::new(
            pointer,
            0,
            PointerDeviceKind::Touch,
            u32::from(matches!(phase, PointerPhase::Down | PointerPhase::Move)),
            Offset::new(x, y),
            phase,
            start + Duration::from_millis(millis),
        )
    };
    let first_down = sample(1, 0, PointerPhase::Down, 10., 10.);
    assert!(double.add_pointer(&arena, first_key, first_down));
    let first_tap = arena.add(first_key, single.clone());
    arena.close(first_key);
    arena.flush_deferred_resolutions();
    for event in [first_down, sample(1, 0, PointerPhase::Up, 11., 9.)] {
        double.route(first_key, event);
        single.route(&first_tap, event);
    }
    arena.sweep(first_key);
    assert_eq!(count.get(), 0);

    let second_tap = arena.add(second_key, single.clone());
    arena.close(second_key);
    single.route(&second_tap, sample(2, 100, PointerPhase::Down, 12., 12.));
    arena.flush_deferred_resolutions();
    single.route(&second_tap, sample(2, 100, PointerPhase::Move, 12., 12.));
    single.route(&second_tap, sample(2, 100, PointerPhase::Up, 13., 11.));
    assert_eq!(count.get(), 1);
    arena.sweep(second_key);

    double.advance_to(start + Duration::from_millis(400));
    arena.flush_deferred_resolutions();
    assert_eq!(count.get(), 1);
    assert!(double.deadline().is_none());
}
