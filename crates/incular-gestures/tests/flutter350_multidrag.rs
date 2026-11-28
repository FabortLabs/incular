//! Pinned Flutter 3.50 multidrag contracts and a simultaneous-pointer control.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{Offset, PointerDeviceKind, PointerPhase, time::Instant};
use incular_gestures::{
    Drag, DragEndDetails, DragUpdateDetails, MultiDragGestureRecognizer, RawPointerEvent,
};

#[derive(Debug, PartialEq)]
enum Trace {
    Start(Offset),
    Update(Offset, Offset, Offset),
    End(Offset),
    Cancel(Offset),
}

type Log = Rc<RefCell<Vec<Trace>>>;

struct Client(Offset, Log);

impl Drag for Client {
    fn update(&mut self, details: DragUpdateDetails) {
        self.1.borrow_mut().push(Trace::Update(
            self.0,
            details.global_position,
            details.delta,
        ));
    }

    fn end(&mut self, _: DragEndDetails) {
        self.1.borrow_mut().push(Trace::End(self.0));
    }

    fn cancel(&mut self) {
        self.1.borrow_mut().push(Trace::Cancel(self.0));
    }
}

fn at(t: Instant, ms: u64) -> Instant {
    t + Duration::from_millis(ms)
}

struct Input {
    base: Instant,
    pointer: u64,
    kind: PointerDeviceKind,
    buttons: u32,
}

impl Input {
    fn event(&self, ms: u64, phase: PointerPhase, x: f32, y: f32) -> RawPointerEvent {
        RawPointerEvent::new(
            self.pointer,
            self.pointer,
            self.kind,
            if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
                0
            } else {
                self.buttons
            },
            Offset::new(x, y),
            phase,
            at(self.base, ms),
        )
    }
}

fn touch(base: Instant, pointer: u64) -> Input {
    Input {
        base,
        pointer,
        kind: PointerDeviceKind::Touch,
        buttons: 1,
    }
}

fn record(recognizer: MultiDragGestureRecognizer) -> (MultiDragGestureRecognizer, Log) {
    let log = Log::default();
    let capture = log.clone();
    let recognizer = recognizer.on_start(move |start| {
        capture.borrow_mut().push(Trace::Start(start));
        Some(Box::new(Client(start, capture.clone())))
    });
    (recognizer, log)
}

#[test]
fn moving_before_delay_rejects() {
    use PointerPhase::{Down, Move};
    let (mut drag, log) = record(MultiDragGestureRecognizer::delayed(None));
    let now = Instant::now();
    let pointer = touch(now, 5);
    drag.handle_raw(pointer.event(0, Down, 10., 10.));
    assert!(log.borrow().is_empty());
    drag.poll(now);
    assert!(log.borrow().is_empty());
    drag.handle_raw(pointer.event(1, Move, 20., 60.));
    assert!(log.borrow().is_empty());
    drag.poll(at(now, 1000));
    assert!(log.borrow().is_empty());
    drag.handle_raw(pointer.event(1001, Move, 30., 120.));
    assert!(log.borrow().is_empty());
}

#[test]
fn staying_inside_slop_starts_at_delay() {
    use PointerPhase::{Down, Move};
    let (mut drag, log) = record(MultiDragGestureRecognizer::delayed(None));
    let now = Instant::now();
    let pointer = touch(now, 5);
    let down = Offset::new(10., 10.);
    drag.handle_raw(pointer.event(0, Down, 10., 10.));
    assert!(log.borrow().is_empty());
    drag.poll(now);
    drag.handle_raw(pointer.event(1, Move, 20., 20.));
    drag.poll(at(now, 499));
    assert!(log.borrow().is_empty());
    drag.poll(at(now, 500));
    assert_eq!(
        *log.borrow(),
        [
            Trace::Start(down),
            Trace::Update(down, down, Offset::new(10., 10.))
        ]
    );
    drag.handle_raw(pointer.event(1001, Move, 30., 70.));
    assert_eq!(
        log.borrow().last(),
        Some(&Trace::Update(
            down,
            Offset::new(30., 70.),
            Offset::new(10., 50.)
        ))
    );
}

#[test]
fn device_rejection_does_not_allocate_a_pending_drag() {
    use PointerPhase::{Down, Move};
    let (mut drag, log) = record(
        MultiDragGestureRecognizer::delayed(None).supported_devices([PointerDeviceKind::Touch]),
    );
    let now = Instant::now();
    let pointer = Input {
        kind: PointerDeviceKind::Mouse,
        ..touch(now, 5)
    };
    drag.handle_raw(pointer.event(0, Down, 10., 10.));
    assert!(log.borrow().is_empty());
    assert_eq!(drag.next_deadline(), None);
    drag.poll(now);
    drag.handle_raw(pointer.event(1, Move, 20., 20.));
    assert!(log.borrow().is_empty());
    drag.poll(at(now, 1000));
    assert!(log.borrow().is_empty());
    drag.handle_raw(pointer.event(1001, Move, 30., 70.));
    assert!(log.borrow().is_empty());
}

#[test]
fn omitted_and_reset_button_filters_allow_only_the_primary_button() {
    use PointerPhase::Down;
    let default = MultiDragGestureRecognizer::immediate();
    let mut reset =
        MultiDragGestureRecognizer::immediate().allowed_buttons_filter(|buttons| buttons == 4);
    reset.set_allowed_buttons_filter(None);
    let now = Instant::now();
    for recognizer in [default, reset] {
        let (mut drag, log) = record(recognizer);
        for (id, mask) in [4, 3, 0].into_iter().enumerate() {
            let pointer = Input {
                buttons: mask,
                kind: PointerDeviceKind::Mouse,
                ..touch(now, id as u64)
            };
            drag.handle_raw(pointer.event(0, Down, 10., 10.));
            assert!(log.borrow().is_empty());
        }
        let primary = touch(now, 5);
        drag.handle_raw(primary.event(0, Down, 10., 10.));
        assert_eq!(
            log.borrow().first(),
            Some(&Trace::Start(Offset::new(10., 10.)))
        );
    }
}

#[test]
fn independent_delayed_drags_fire_by_deadline_then_registration_order() {
    use PointerPhase::{Cancel, Down, Move, Up};
    use Trace::{End, Start, Update};
    let (mut drag, log) = record(MultiDragGestureRecognizer::delayed(None));
    let now = Instant::now();
    let first = touch(now, 90);
    let second = touch(now, 10);
    let third = touch(now, 2);
    let a = Offset::new(10., 10.);
    let b = Offset::new(100., 100.);
    let c = Offset::new(200., 200.);
    drag.handle_raw(first.event(0, Down, 10., 10.));
    drag.handle_raw(second.event(20, Down, 100., 100.));
    drag.handle_raw(third.event(20, Down, 200., 200.));
    drag.handle_raw(first.event(120, Move, 13., 14.));
    drag.handle_raw(second.event(120, Move, 100., 107.));
    assert!(log.borrow().is_empty());
    assert_eq!(drag.next_deadline(), Some(at(now, 500)));

    drag.poll(at(now, 600));
    assert_eq!(
        *log.borrow(),
        [
            Start(a),
            Update(a, a, Offset::new(3., 4.)),
            Start(b),
            Update(b, b, Offset::new(0., 7.)),
            Start(c),
            Update(c, c, Offset::ZERO),
        ]
    );
    assert_eq!(drag.next_deadline(), None);

    drag.handle_raw(second.event(600, Move, 98., 111.));
    drag.handle_raw(first.event(600, Move, 18., 14.));
    drag.handle_raw(first.event(600, Cancel, 18., 14.));
    drag.handle_raw(second.event(600, Up, 98., 111.));
    drag.handle_raw(third.event(600, Up, 200., 200.));
    assert_eq!(
        log.borrow()[6..],
        [
            Update(b, Offset::new(98., 111.), Offset::new(-2., 4.)),
            Update(a, Offset::new(18., 14.), Offset::new(5., 0.)),
            Trace::Cancel(a),
            End(b),
            End(c),
        ]
    );
}

#[cfg(debug_assertions)]
#[test]
fn direct_pointer_state_and_recognizer_contacts_publish_matched_lifetimes() {
    use PointerPhase::{Cancel, Down};
    use incular_core::{
        DeviceGestureSettings,
        memory::allocations::{MemoryAllocations, ObjectEventKind},
    };
    use incular_gestures::MultiDragPointerState;

    let observed = Rc::new(RefCell::new(Vec::new()));
    let captured = observed.clone();
    let _subscription = MemoryAllocations::current().add_listener(move |event| {
        if event.class_name == "MultiDragPointerState" && event.library == "gestures" {
            captured.borrow_mut().push(event);
        }
    });
    let state = MultiDragPointerState::new(
        Offset::ZERO,
        PointerDeviceKind::Touch,
        DeviceGestureSettings::default(),
    );
    assert_eq!(state.initial_position(), Offset::ZERO);
    assert_eq!(state.pending_delta(), Some(Offset::ZERO));
    assert_eq!(state.kind(), PointerDeviceKind::Touch);
    assert_eq!(observed.borrow()[0].kind, ObjectEventKind::Created);
    drop(state);

    let t = Instant::now();
    let included = touch(t, 1);
    let rejected = Input {
        buttons: 4,
        ..touch(t, 2)
    };
    let mut recognizer = MultiDragGestureRecognizer::delayed(None);
    recognizer.handle_raw(rejected.event(0, Down, 0., 0.));
    assert_eq!(observed.borrow().len(), 2);
    recognizer.handle_raw(included.event(0, Down, 0., 0.));
    recognizer.handle_raw(included.event(1, Cancel, 0., 0.));
    // An already cancelled contact is not disposed a second time.
    recognizer.handle_raw(included.event(2, Cancel, 0., 0.));
    assert_eq!(recognizer.next_deadline(), None);
    drop(recognizer);
    let events = observed.borrow();
    assert_eq!(events.len(), 4);
    for events in events.chunks_exact(2) {
        assert_eq!(events[0].id, events[1].id);
        assert_eq!(events[0].kind, ObjectEventKind::Created);
        assert_eq!(events[1].kind, ObjectEventKind::Disposed);
    }
    assert_ne!(events[0].id, events[2].id);
}
