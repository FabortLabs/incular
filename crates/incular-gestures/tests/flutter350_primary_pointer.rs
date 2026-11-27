use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{DeviceGestureSettings, Offset, PointerPhase, Transform, time::Instant};
use incular_gestures::{
    GestureArenaClient, GestureArenaKey, GestureArenaManager, GestureDisposition,
    GestureRecognizerState, GestureSlopTolerance, OffsetPair, PointerDeviceKind, PointerRouter,
    PrimaryPointerGestureHandler, PrimaryPointerGestureOptions, PrimaryPointerGestureRecognizer,
    RawPointerEvent,
};

use GestureDisposition::{Accepted, Rejected};
use GestureRecognizerState::{Defunct, Possible, Ready};
use PointerPhase::{Cancel, Down, Move, Up};

const VIEW: u64 = 11;

struct Indefinite;
impl GestureArenaClient for Indefinite {
    fn accept_gesture(&self, _: GestureArenaKey) {}
    fn reject_gesture(&self, _: GestureArenaKey) {}
}

#[derive(Clone, Default)]
struct Calls {
    resolved: Rc<RefCell<Vec<(u64, GestureDisposition)>>>,
    phases: Rc<RefCell<Vec<(u64, PointerPhase)>>>,
    deadlines: Rc<RefCell<Vec<u64>>>,
}

struct Probe {
    calls: Calls,
    resolution: (PointerPhase, GestureDisposition),
}

impl PrimaryPointerGestureHandler for Probe {
    fn debug_description(&self) -> &'static str {
        "test primary"
    }

    fn debug_class_name(&self) -> &'static str {
        "TestPrimaryPointer"
    }

    fn handle_primary_pointer(&self, event: RawPointerEvent) -> Option<GestureDisposition> {
        self.calls
            .phases
            .borrow_mut()
            .push((event.pointer, event.phase));
        (event.phase == self.resolution.0).then_some(self.resolution.1)
    }
    fn did_resolve(&self, key: GestureArenaKey, disposition: GestureDisposition) {
        self.calls
            .resolved
            .borrow_mut()
            .push((key.pointer_id().unwrap(), disposition));
    }
    fn did_exceed_deadline(&self, event: RawPointerEvent) -> Option<GestureDisposition> {
        self.calls.deadlines.borrow_mut().push(event.pointer);
        None
    }
}

struct Fixture {
    recognition: PrimaryPointerGestureRecognizer,
    router: PointerRouter,
    arena: GestureArenaManager,
    calls: Calls,
    start: Instant,
}

impl Fixture {
    fn new(
        resolution: (PointerPhase, GestureDisposition),
        options: PrimaryPointerGestureOptions,
    ) -> Self {
        let router = PointerRouter::new();
        let arena = GestureArenaManager::new();
        let calls = Calls::default();
        let recognition = PrimaryPointerGestureRecognizer::new(
            VIEW,
            router.clone(),
            arena.clone(),
            options,
            Probe {
                calls: calls.clone(),
                resolution,
            },
        );
        Self {
            recognition,
            router,
            arena,
            calls,
            start: Instant::now(),
        }
    }

    fn event(&self, id: u64, phase: PointerPhase, point: Offset) -> RawPointerEvent {
        RawPointerEvent::new(id, 0, PointerDeviceKind::Touch, 1, point, phase, self.start)
    }

    fn down(&self, id: u64, transform: Option<Transform>) {
        let key = GestureArenaKey::pointer(VIEW, id);
        let down = self.event(id, Down, Offset::new(10., 10.));
        self.arena.add(key, Rc::new(Indefinite));
        assert!(self.recognition.add_pointer(down, transform));
        self.arena.close(key);
        self.arena.flush_deferred_resolutions();
        self.router.route(down);
    }

    fn route(&self, id: u64, phase: PointerPhase, x: f32, y: f32) {
        self.router.route(self.event(id, phase, Offset::new(x, y)));
    }
}

#[test]
fn releases_the_initial_position_after_winning_but_keeps_the_last_primary_id() {
    let f = Fixture::new((Up, Accepted), PrimaryPointerGestureOptions::default());
    assert_eq!(f.recognition.state(), Ready);
    assert_eq!(f.recognition.primary_pointer(), None);
    assert_eq!(f.recognition.initial_position(), None);

    let transform = Transform::translation(Offset::new(5., -3.));
    f.down(5, Some(transform));
    assert_eq!(f.recognition.state(), Possible);
    assert_eq!(f.recognition.primary_pointer(), Some(5));
    assert_eq!(
        f.recognition.initial_position(),
        Some(OffsetPair::new(Offset::new(15., 7.), Offset::new(10., 10.))),
    );
    assert!(f.calls.resolved.borrow().is_empty());
    f.route(5, Up, 15., 15.);
    assert_eq!(f.recognition.state(), Ready);
    assert_eq!(f.recognition.primary_pointer(), Some(5));
    assert_eq!(f.recognition.initial_position(), None);
    assert_eq!(&*f.calls.resolved.borrow(), &[(5, Accepted)]);
    f.route(5, Move, 14., 12.);
    assert_eq!(&*f.calls.phases.borrow(), &[(5, Down), (5, Up)]);
}

#[test]
fn loses_on_move_but_keeps_the_position_through_terminal_delivery() {
    let f = Fixture::new((Move, Rejected), PrimaryPointerGestureOptions::default());
    assert_eq!(f.recognition.state(), Ready);
    assert_eq!(f.recognition.primary_pointer(), None);
    assert_eq!(f.recognition.initial_position(), None);
    f.down(5, None);
    assert_eq!(f.recognition.state(), Possible);
    f.route(5, Move, 15., 15.);
    assert_eq!(f.recognition.state(), Defunct);
    assert_eq!(f.recognition.primary_pointer(), Some(5));
    assert_eq!(
        f.recognition.initial_position(),
        Some(OffsetPair::new(
            Offset::new(10., 10.),
            Offset::new(10., 10.)
        )),
    );
    assert_eq!(&*f.calls.resolved.borrow(), &[(5, Rejected)]);
    f.route(5, Up, 15., 15.);
    assert_eq!(f.recognition.state(), Ready);
    assert_eq!(f.recognition.initial_position(), None);
    assert_eq!(&*f.calls.resolved.borrow(), &[(5, Rejected)]);
}

#[test]
fn recycling_resets_acceptance_and_uses_the_pre_accept_limit_again() {
    let options = PrimaryPointerGestureOptions {
        pre_accept: GestureSlopTolerance::Pixels(15.),
        post_accept: GestureSlopTolerance::Pixels(1000.),
        ..Default::default()
    };
    let f = Fixture::new((Up, Accepted), options);
    f.down(5, None);
    f.route(5, Up, 15., 15.);
    assert_eq!(f.calls.resolved.take(), [(5, Accepted)]);

    f.down(6, None);
    f.route(6, Move, 100., 200.);
    assert_eq!(&*f.calls.resolved.borrow(), &[(6, Rejected)]);
    assert_eq!(f.recognition.state(), Ready);
    f.route(6, Up, 100., 200.);
    assert_eq!(f.calls.resolved.take(), [(6, Rejected)]);
}

#[test]
fn distinguishes_default_overridden_fixed_and_unlimited_pre_accept_slop() {
    let default = Fixture::new(
        (Up, Accepted),
        PrimaryPointerGestureOptions {
            post_accept: GestureSlopTolerance::Unlimited,
            ..Default::default()
        },
    );
    assert_eq!(default.recognition.pre_accept_slop_tolerance(), Some(18.));
    default.down(5, None);
    default.route(5, Move, 15., 15.);
    default.route(5, Up, 15., 15.);
    assert_eq!(default.calls.resolved.take(), [(5, Accepted)]);

    default
        .recognition
        .set_gesture_settings(DeviceGestureSettings {
            touch_slop: Some(5.),
        });
    assert_eq!(default.recognition.pre_accept_slop_tolerance(), Some(5.));
    default.down(5, Some(Transform::scale(0.01)));
    // Distance is global even if the local route scales it.
    default.route(5, Move, 15., 15.);
    default.route(5, Up, 15., 15.);
    assert_eq!(default.calls.resolved.take(), [(5, Rejected)]);

    for (pre_accept, x, y, expected) in [
        (GestureSlopTolerance::Pixels(5.), 15., 15., Rejected),
        (GestureSlopTolerance::Unlimited, 100., 200., Accepted),
    ] {
        let f = Fixture::new(
            (Up, Accepted),
            PrimaryPointerGestureOptions {
                pre_accept,
                post_accept: GestureSlopTolerance::Unlimited,
                ..Default::default()
            },
        );
        f.down(6, None);
        f.route(6, Move, x, y);
        f.route(6, Up, x, y);
        assert_eq!(f.calls.resolved.take(), [(6, expected)]);
    }
}

#[test]
fn deadline_runs_once_and_is_cancelled_by_arena_resolution_or_the_last_contact() {
    let f = Fixture::new(
        (Up, Accepted),
        PrimaryPointerGestureOptions {
            deadline: Some(Duration::from_millis(25)),
            ..Default::default()
        },
    );
    f.down(5, None);
    f.recognition
        .advance_time(f.start + Duration::from_millis(24));
    assert!(f.calls.deadlines.borrow().is_empty());
    f.recognition
        .advance_time(f.start + Duration::from_millis(25));
    f.recognition
        .advance_time(f.start + Duration::from_millis(50));
    assert_eq!(f.calls.deadlines.take(), [5]);
    f.route(5, Up, 10., 10.);

    f.down(6, None);
    f.recognition.resolve(Accepted);
    f.recognition
        .advance_time(f.start + Duration::from_millis(25));
    assert!(f.calls.deadlines.borrow().is_empty());
    f.route(6, Up, 10., 10.);

    f.down(7, None);
    f.route(7, Cancel, 10., 10.);
    f.recognition
        .advance_time(f.start + Duration::from_millis(25));
    assert!(f.calls.deadlines.borrow().is_empty());
}

#[test]
fn disposition_can_arrive_after_the_route_is_gone_and_does_not_mutate_a_new_primary() {
    let f = Fixture::new((Move, Rejected), PrimaryPointerGestureOptions::default());
    f.down(5, None);
    f.route(5, Up, 10., 10.);
    assert_eq!(f.recognition.state(), Ready);
    f.down(6, None);
    f.arena.sweep(GestureArenaKey::pointer(VIEW, 5));
    assert_eq!(f.recognition.state(), Possible);
    assert_eq!(f.recognition.primary_pointer(), Some(6));
    assert_eq!(f.calls.resolved.take(), [(5, Rejected)]);
    f.recognition.dispose();
    f.route(6, Move, 15., 15.);
    assert_eq!(f.calls.phases.take(), [(5, Down), (5, Up), (6, Down)]);
    assert!(f.calls.resolved.borrow().is_empty());
}

#[test]
fn exceeding_post_accept_slop_stops_delivery_without_reversing_the_win() {
    let f = Fixture::new(
        (Move, Accepted),
        PrimaryPointerGestureOptions {
            post_accept: GestureSlopTolerance::Pixels(5.),
            ..Default::default()
        },
    );
    f.down(5, None);
    f.route(5, Move, 15., 10.);
    assert_eq!(f.recognition.state(), Possible);
    assert_eq!(f.calls.resolved.take(), [(5, Accepted)]);
    f.route(5, Move, 16., 10.);
    assert_eq!(f.recognition.state(), Ready);
    assert_eq!(f.recognition.initial_position(), None);
    f.route(5, Up, 16., 10.);
    assert!(f.calls.resolved.borrow().is_empty());
    assert_eq!(f.calls.phases.take(), [(5, Down), (5, Move)]);
}

#[test]
fn only_the_primary_is_dispatched_but_all_contact_routes_keep_the_sequence_open() {
    let f = Fixture::new((Move, Rejected), PrimaryPointerGestureOptions::default());
    f.down(5, None);
    f.down(6, None);
    f.route(6, Move, 10., 10.);
    f.route(5, Cancel, 10., 10.);
    assert_eq!(f.recognition.state(), Possible);
    assert_eq!(f.recognition.primary_pointer(), Some(5));
    assert!(f.recognition.initial_position().is_some());
    f.route(6, Up, 10., 10.);
    assert_eq!(f.recognition.state(), Ready);
    assert_eq!(f.recognition.initial_position(), None);
    assert_eq!(f.calls.phases.take(), [(5, Down), (5, Cancel)]);
}

#[cfg(debug_assertions)]
#[test]
fn custom_recognizer_debug_identity_and_memory_events_survive_explicit_disposal() {
    use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};

    let events = Rc::new(RefCell::new(Vec::new()));
    let _subscription = MemoryAllocations::current().add_listener({
        let events = events.clone();
        move |event| {
            if event.library == "gestures" && event.class_name == "TestPrimaryPointer" {
                events.borrow_mut().push(event);
            }
        }
    });
    let recognizer = Fixture::new((Up, Accepted), PrimaryPointerGestureOptions::default())
        .recognition
        .with_debug_owner(0);
    let debug = format!("{recognizer:#?}");
    assert!(debug.starts_with("TestPrimaryPointer {"));
    assert!(debug.contains("description: \"test primary\""));
    assert!(debug.contains("state: Ready"));
    assert!(debug.contains("debug_owner: \"0\""));
    assert_eq!(events.borrow().len(), 1);
    recognizer.dispose();
    recognizer.dispose();
    drop(recognizer);
    let recorded = events.borrow();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0].kind, ObjectEventKind::Created);
    assert_eq!(recorded[1].kind, ObjectEventKind::Disposed);
    assert_eq!(recorded[0].id, recorded[1].id);
}
