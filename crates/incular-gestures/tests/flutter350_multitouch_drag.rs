use std::{cell::RefCell, rc::Rc, time::Duration};

use PointerPhase::{Down, Move, Up};
use incular_core::{
    DragStartBehavior, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    time::Instant,
};
use incular_gestures::{
    DragAxis, DragCallbacks, GestureArenaClient, GestureArenaKey, GestureArenaManager,
    GestureDisposition, MonodragGestureRecognizer, MultitouchDragStrategy, RawPointerEvent,
};

fn input(at: Instant, pointer: u64, phase: PointerPhase, position: Offset) -> RawPointerEvent {
    RawPointerEvent::new(
        pointer,
        1,
        PointerDeviceKind::Touch,
        if phase == Up {
            0
        } else {
            PRIMARY_POINTER_BUTTON
        },
        position,
        phase,
        at,
    )
}

fn averaged(axis: DragAxis, points: impl Fn(f32) -> Offset, expected: &[f32]) {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let mut drag = MonodragGestureRecognizer::new(
        axis,
        DragCallbacks {
            on_update: Some(Rc::new({
                let observed = observed.clone();
                move |update| observed.borrow_mut().push(update.delta)
            })),
            ..Default::default()
        },
    )
    .with_multitouch_drag_strategy(MultitouchDragStrategy::AverageBoundaryPointers);
    let at = Instant::now();
    drag.set_multitouch_frame_time(at);
    for (pointer, position) in [(5, 100.), (6, 110.), (7, -100.), (8, -110.)] {
        assert!(drag.observe_raw(input(at, pointer, Down, Offset::ZERO)));
        drag.accept();
        assert!(drag.observe_raw(input(at, pointer, Move, points(position))));
    }
    for (pointer, position) in [(5, 120.), (7, -120.)] {
        assert!(drag.observe_raw(input(at, pointer, Move, points(position))));
    }

    let next = at + Duration::from_millis(100);
    drag.set_multitouch_frame_time(next);
    for (pointer, position) in [(6, 120.), (8, -120.), (5, 130.), (7, -130.)] {
        assert!(drag.observe_raw(input(next, pointer, Move, points(position))));
    }
    for pointer in [5, 6, 7, 8] {
        assert!(drag.observe_raw(input(next, pointer, Up, Offset::ZERO)));
    }
    assert!(!drag.is_tracking());
    let observed = observed.borrow();
    assert_eq!(observed.len(), expected.len());
    for (actual, wanted) in observed.iter().zip(expected.iter().copied().map(points)) {
        assert!(
            (actual.x - wanted.x).abs() < 0.001 && (actual.y - wanted.y).abs() < 0.001,
            "{actual:?} differs from {wanted:?}"
        );
    }
}

const AXIS_EXPECTED: [f32; 10] = [100., 10., -100., -10., 10., -10., 10., -10., 0., 0.];

#[test]
fn horizontal_boundary_deltas_reset_only_on_the_next_system_frame() {
    averaged(DragAxis::Horizontal, |x| Offset::new(x, 0.), &AXIS_EXPECTED);
}

#[test]
fn vertical_boundary_deltas_reset_only_on_the_next_system_frame() {
    averaged(DragAxis::Vertical, |y| Offset::new(0., y), &AXIS_EXPECTED);
}

#[test]
fn pans_average_across_accepted_contacts_and_keep_per_frame_accumulators() {
    averaged(
        DragAxis::Pan,
        |p| Offset::new(p, p),
        &[
            100.,
            5.,
            110. / 3. - 105.,
            -110. / 3.,
            5.,
            -5.,
            2.5,
            -2.5,
            2.5,
            -2.5,
        ],
    );
}

type Trace = Rc<RefCell<Vec<&'static str>>>;

struct DragClient(RefCell<MonodragGestureRecognizer>);

impl GestureArenaClient for DragClient {
    fn accept_gesture(&self, key: GestureArenaKey) {
        self.0.borrow_mut().accept_stream(key);
    }
    fn reject_gesture(&self, key: GestureArenaKey) {
        self.0.borrow_mut().reject_stream(key);
    }
}

fn client(axis: DragAxis, policy: Option<MultitouchDragStrategy>, trace: &Trace) -> Rc<DragClient> {
    let names = if axis == DragAxis::Horizontal {
        ["1-down", "1-start", "1-update", "1-end", "1-cancel"]
    } else {
        ["2-down", "2-start", "2-update", "2-end", "2-cancel"]
    };
    let callback = |i: usize| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(names[i])
    };
    let (down, start, update, end, cancel) = (
        callback(0),
        callback(1),
        callback(2),
        callback(3),
        callback(4),
    );
    let mut drag = MonodragGestureRecognizer::new(
        axis,
        DragCallbacks {
            on_start: Some(Rc::new(move |_| start())),
            on_update: Some(Rc::new(move |_| update())),
            on_end: Some(Rc::new(move |_| end())),
        },
    )
    .with_drag_start_behavior(DragStartBehavior::Down)
    .with_pending_callbacks(Some(Rc::new(move |_| down())), Some(Rc::new(cancel)));
    if let Some(policy) = policy {
        drag.set_multitouch_drag_strategy(policy);
    }
    Rc::new(DragClient(RefCell::new(drag)))
}

fn competition(policy: Option<MultitouchDragStrategy>) {
    use MultitouchDragStrategy::{LatestPointer, SumAllPointers};
    let trace = Trace::default();
    let clients = [
        client(DragAxis::Horizontal, policy, &trace),
        client(DragAxis::Vertical, policy, &trace),
    ];
    let at = Instant::now();
    let arena = GestureArenaManager::new();
    let mut active = std::collections::HashMap::<_, [_; 2]>::new();
    let mut send = |pointer, phase, x, y, step| {
        let key = GestureArenaKey::pointer_device(1, 1, pointer);
        let sample = input(at, pointer, phase, Offset::new(x, y));
        if phase == Down {
            let _ = active.insert(
                pointer,
                clients.clone().map(|client| arena.add(key, client)),
            );
        }
        for (client, reg) in clients.iter().zip(&active[&pointer]) {
            let wants = {
                let mut drag = client.0.borrow_mut();
                drag.observe_raw(sample.clone()) && drag.wants_acceptance()
            };
            if wants {
                reg.resolve(GestureDisposition::Accepted);
            }
        }
        if phase == Down {
            arena.close(key);
        }
        if phase == Up {
            arena.sweep(key);
            active.remove(&pointer);
        }
        arena.flush_deferred_resolutions();
        if let Some(label) = step {
            trace.borrow_mut().push(label);
        }
    };

    send(5, Down, 10., 10., Some("a"));
    send(5, Move, 100., 0., Some("b"));
    send(5, Move, 50., 50., Some("c"));
    send(6, Down, 20., 20., Some("d"));
    send(5, Move, 0., 100., Some("e"));
    send(5, Move, 70., 70., Some("f"));
    send(6, Move, 0., 100., Some("g"));
    send(6, Move, 70., 70., Some("h"));

    let mut expected = vec![
        "1-down", "2-down", "a", "2-cancel", "1-start", "1-update", "b", "1-update", "c", "2-down",
        "2-cancel", "d",
    ];
    if policy == Some(SumAllPointers) {
        expected.extend([
            "1-update", "e", "1-update", "f", "1-update", "g", "1-update", "h",
        ]);
        send(5, Up, 70., 70., None);
        send(6, Up, 70., 70., None);
    } else {
        expected.extend(["e", "f", "1-update", "g", "1-update", "h"]);
        if policy == Some(LatestPointer) {
            send(7, Down, 20., 20., Some("i"));
            send(7, Up, 20., 20., Some("j"));
            expected.extend(["2-down", "2-cancel", "i", "j"]);
        } else {
            send(6, Up, 70., 70., Some("i"));
            expected.push("i");
        }
        send(5, Move, 0., 100., Some("k"));
        send(5, Move, 70., 70., Some("l"));
        expected.extend(["1-update", "k", "1-update", "l"]);
        send(5, Up, 70., 70., None);
        if policy == Some(LatestPointer) {
            send(6, Up, 70., 70., None);
        }
    }
    expected.push("1-end");
    assert_eq!(*trace.borrow(), expected);
}

#[test]
fn summing_pointers_keeps_earlier_accepted_contacts_live() {
    competition(Some(MultitouchDragStrategy::SumAllPointers));
}

#[test]
fn default_moves_only_the_latest_and_resumes_the_older_after_release() {
    competition(None);
}

#[test]
fn explicit_latest_resumes_the_first_accepted_not_the_second_latest() {
    competition(Some(MultitouchDragStrategy::LatestPointer));
}

#[test]
fn fallback_uses_award_order_when_contenders_were_registered_in_a_different_order() {
    let positions = Rc::new(RefCell::new(Vec::new()));
    let mut drag = MonodragGestureRecognizer::new(
        DragAxis::Horizontal,
        DragCallbacks {
            on_update: Some(Rc::new({
                let positions = positions.clone();
                move |update| positions.borrow_mut().push(update.global_position)
            })),
            ..Default::default()
        },
    );
    let at = Instant::now();
    for pointer in [1, 2, 3] {
        assert!(drag.observe_raw(input(at, pointer, Down, Offset::ZERO)));
    }
    for pointer in [2, 1, 3] {
        drag.accept_stream(GestureArenaKey::pointer_device(1, 1, pointer));
    }
    assert!(drag.observe_raw(input(at, 3, Up, Offset::ZERO)));
    assert!(drag.observe_raw(input(at, 1, Move, Offset::new(10., 10.))));
    assert!(drag.observe_raw(input(at, 2, Move, Offset::new(20., 20.))));
    assert_eq!(positions.borrow().as_slice(), [Offset::new(20., 20.)]);
}

#[test]
fn the_move_that_wins_at_the_threshold_counts_in_its_original_frame() {
    let deltas = Rc::new(RefCell::new(Vec::new()));
    let mut drag = MonodragGestureRecognizer::new(
        DragAxis::Horizontal,
        DragCallbacks {
            on_update: Some(Rc::new({
                let deltas = deltas.clone();
                move |update| deltas.borrow_mut().push(update.delta.x)
            })),
            ..Default::default()
        },
    )
    .with_multitouch_drag_strategy(MultitouchDragStrategy::AverageBoundaryPointers);
    let at = Instant::now();
    drag.set_multitouch_frame_time(at);
    assert!(drag.observe_raw(input(at, 5, Down, Offset::ZERO)));
    assert!(drag.observe_raw(input(at, 5, Move, Offset::new(20., 0.))));
    assert!(drag.wants_acceptance());
    drag.accept();
    assert!(
        deltas.borrow().is_empty(),
        "Start does not emit the threshold move"
    );

    assert!(drag.observe_raw(input(at, 6, Down, Offset::ZERO)));
    drag.accept();
    assert!(drag.observe_raw(input(at, 6, Move, Offset::new(25., 0.))));
    assert_eq!(deltas.borrow().as_slice(), [5.]);
}
