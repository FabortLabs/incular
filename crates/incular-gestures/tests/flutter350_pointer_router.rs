use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Instant,
};

use incular_core::{Offset, PointerDeviceKind, PointerPhase, Transform};
use incular_gestures::{PointerRouter, RawPointerEvent};

fn pointer(id: u64, phase: PointerPhase, position: Offset) -> RawPointerEvent {
    RawPointerEvent::new(
        id,
        77,
        PointerDeviceKind::Stylus,
        1,
        position,
        phase,
        Instant::now(),
    )
}

fn down(id: u64) -> RawPointerEvent {
    pointer(id, PointerPhase::Down, Offset::ZERO)
}

#[test]
fn routes_only_the_matching_pointer_until_removed() {
    let router = PointerRouter::new();
    let seen = Rc::new(Cell::new(0));
    let token = router.add_route(3, None, {
        let seen = seen.clone();
        move |routed| seen.set(routed.event.pointer)
    });
    router.route(down(2));
    assert_eq!(seen.get(), 0);
    router.route(down(3));
    assert_eq!(seen.get(), 3);

    assert!(!PointerRouter::new().remove_route(3, token));
    assert!(!router.remove_route(2, token));
    assert!(router.remove_route(3, token));
    assert!(!router.remove_route(3, token));
    seen.set(0);
    router.route(pointer(3, PointerPhase::Up, Offset::ZERO));
    assert_eq!(seen.get(), 0);
}

#[test]
fn reentrant_removal_cancels_a_per_pointer_route_immediately() {
    let router = PointerRouter::new();
    let token = Rc::new(Cell::new(None));
    let sink = Rc::new(Cell::new(0));
    router.add_route(2, None, {
        let router = router.downgrade();
        let token = token.clone();
        move |_| {
            assert!(
                router
                    .upgrade()
                    .unwrap()
                    .remove_route(2, token.get().unwrap())
            )
        }
    });
    token.set(Some(router.add_route(2, None, {
        let sink = sink.clone();
        move |_| sink.set(sink.get() + 1)
    })));
    router.route(down(2));
    assert_eq!(sink.get(), 0);
}

#[test]
fn global_additions_wait_for_the_next_event() {
    let router = PointerRouter::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    router.add_global_route(None, {
        let router = router.downgrade();
        let log = log.clone();
        let added = Cell::new(false);
        move |_| {
            log.borrow_mut().push("first");
            if !added.replace(true) {
                router.upgrade().unwrap().add_global_route(None, {
                    let log = log.clone();
                    move |_| log.borrow_mut().push("second")
                });
            }
        }
    });
    router.route(down(2));
    assert_eq!(*log.borrow(), ["first"]);
    router.route(down(7));
    assert_eq!(*log.borrow(), ["first", "first", "second"]);
    assert_eq!(router.global_route_count(), 2);
}

#[test]
fn reentrant_global_removal_cancels_the_current_delivery() {
    let router = PointerRouter::new();
    let token = Rc::new(Cell::new(None));
    let called = Rc::new(Cell::new(false));
    router.add_global_route(None, {
        let router = router.downgrade();
        let token = token.clone();
        move |_| {
            assert!(
                router
                    .upgrade()
                    .unwrap()
                    .remove_global_route(token.get().unwrap())
            )
        }
    });
    token.set(Some(router.add_global_route(None, {
        let called = called.clone();
        move |_| called.set(true)
    })));
    router.route(down(2));
    assert!(!called.get());
    assert_eq!(router.global_route_count(), 1);
}

#[test]
fn per_pointer_callbacks_cannot_add_a_global_callback_to_the_same_event() {
    let router = PointerRouter::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    router.add_route(2, None, {
        let router = router.downgrade();
        let log = log.clone();
        move |_| {
            log.borrow_mut().push("per pointer");
            router.upgrade().unwrap().add_global_route(None, {
                let log = log.clone();
                move |_| log.borrow_mut().push("global")
            });
        }
    });
    router.route(down(2));
    assert_eq!(*log.borrow(), ["per pointer"]);
    router.route(down(3));
    assert_eq!(*log.borrow(), ["per pointer", "global"]);
}

#[test]
fn pointer_routes_run_before_global_routes_in_insertion_order() {
    let router = PointerRouter::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let push = |label| {
        let log = log.clone();
        move |_| log.borrow_mut().push(label)
    };
    router.add_global_route(None, push("global 1"));
    router.add_route(2, None, push("per-pointer 1"));
    router.add_global_route(None, push("global 2"));
    router.add_route(2, None, push("per-pointer 2"));
    router.route(down(2));
    assert_eq!(
        *log.borrow(),
        ["per-pointer 1", "per-pointer 2", "global 1", "global 2"]
    );
}

#[test]
fn callback_failure_reports_router_route_and_event_then_keeps_routing() {
    let router = PointerRouter::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let token = Rc::new(Cell::new(None));
    let event = down(2);
    let push = |label| {
        let log = log.clone();
        move |_| log.borrow_mut().push(label)
    };
    router.add_route(2, None, push("per-pointer 1"));
    token.set(Some(router.add_route(2, Some(Transform::scale(0.5)), {
        let log = log.clone();
        move |_| {
            log.borrow_mut().push("per-pointer 2");
            panic!("Having a bad day!");
        }
    })));
    router.add_route(2, None, push("per-pointer 3"));
    let failures = Rc::new(RefCell::new(Vec::new()));
    let reports = failures.clone();
    let callback_log = log.clone();
    router.set_error_handler(Some(Rc::new(move |error| {
        callback_log.borrow_mut().push("error report");
        reports.borrow_mut().push(error);
    })));
    router.route(event);
    assert_eq!(
        *log.borrow(),
        [
            "per-pointer 1",
            "per-pointer 2",
            "error report",
            "per-pointer 3"
        ]
    );
    let failures = failures.borrow();
    let error = &failures[0];
    assert_eq!(failures.len(), 1);
    assert_eq!(error.route, token.get().unwrap());
    assert_eq!(error.event.event, event);
    assert_eq!(error.event.transform, Some(Transform::scale(0.5)));
    assert_eq!(error.message(), Some("Having a bad day!"));
    assert!(error.router.upgrade().unwrap().remove_route(2, error.route));
    assert!(!router.remove_route(2, error.route));
    router.set_error_handler(None);
}

#[test]
fn transformed_routes_keep_global_device_data_and_compute_local_positions_and_deltas() {
    let router = PointerRouter::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let global = Rc::new(RefCell::new(Vec::new()));
    let transform = Transform::scale(0.5).then(Transform::translation(Offset::new(-10., -30.)));
    router.add_route(1, Some(transform), {
        let events = events.clone();
        move |event| events.borrow_mut().push(event)
    });
    router.add_global_route(Some(transform), {
        let global = global.clone();
        move |event| global.borrow_mut().push(event)
    });
    let first = pointer(1, PointerPhase::Down, Offset::new(16., 36.));
    let mut moved = pointer(1, PointerPhase::Move, Offset::new(20., 40.));
    moved.delta = moved.position - first.position;
    for event in [first, moved] {
        router.route(event);
    }
    let events = events.borrow();
    assert_eq!(*events, *global.borrow());
    for (i, source) in [first, moved].iter().enumerate() {
        let view = events[i];
        assert_eq!(view.transform, Some(transform));
        assert_eq!(view.event.position, source.position);
        assert_eq!(view.event.delta, source.delta);
        assert_eq!(view.event.phase, source.phase);
        assert_eq!(view.event.device, source.device);
        assert_eq!(view.event.time, source.time);
        assert_eq!(view.event.kind, PointerDeviceKind::Stylus);
    }
    assert_eq!(events[0].event.local_position, Offset::new(3., 3.));
    assert_eq!(events[0].event.local_delta, Offset::ZERO);
    assert_eq!(events[1].event.local_position, Offset::new(5., 5.));
    assert_eq!(events[1].event.local_delta, Offset::new(2., 2.));
}

#[test]
fn a_nested_dispatch_observes_new_routes_without_extending_the_outer_snapshot() {
    let router = PointerRouter::new();
    let visited = Rc::new(RefCell::new(Vec::new()));
    let weak = router.downgrade();
    router.add_route(2, None, {
        let visited = visited.clone();
        let weak = weak.clone();
        move |_| {
            let router = weak.upgrade().unwrap();
            router.add_global_route(None, {
                let visited = visited.clone();
                move |event| visited.borrow_mut().push(event.event.pointer)
            });
            router.route(down(3));
        }
    });
    router.route(down(2));
    assert_eq!(*visited.borrow(), [3]);
    drop(router);
    assert!(weak.upgrade().is_none());
}
