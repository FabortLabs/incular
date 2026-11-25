use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{DragStartBehavior, Offset, PointerPhase, time::Instant};
use incular_gestures::{
    DragEndDetails, DragStartDetails, DragUpdateDetails, GestureArena, GestureArenaKey,
    GestureCallbacks, GestureDecision, GestureDisposition, GestureRecognizer, PointerEvent,
    PointerGestureRecognizer,
};

#[derive(Default)]
struct Trace {
    starts: Vec<DragStartDetails>,
    updates: Vec<DragUpdateDetails>,
    ends: Vec<DragEndDetails>,
}

fn horizontal(
    behavior: Option<DragStartBehavior>,
) -> (PointerGestureRecognizer, Rc<RefCell<Trace>>) {
    let trace = Rc::new(RefCell::new(Trace::default()));
    macro_rules! record {
        ($field:ident) => {{
            let trace = trace.clone();
            Some(Rc::new(move |event| trace.borrow_mut().$field.push(event)))
        }};
    }
    let mut callbacks = GestureCallbacks {
        on_horizontal_drag_start: record!(starts),
        on_horizontal_drag_update: record!(updates),
        on_horizontal_drag_end: record!(ends),
        ..GestureCallbacks::default()
    };
    if let Some(behavior) = behavior {
        callbacks.drag_start_behavior = behavior;
    }
    (PointerGestureRecognizer::new(callbacks), trace)
}

fn event(phase: PointerPhase, position: Offset, time: Instant) -> PointerEvent {
    PointerEvent {
        pointer: 5,
        position,
        phase,
        time,
    }
}

fn drive_drag(r: &mut PointerGestureRecognizer, event: PointerEvent) {
    let GestureDecision::Accept(action) = r.observe(event) else {
        panic!("accepted drag must receive this sample");
    };
    r.dispatch(action);
    if let Some(update) = r.take_start_update() {
        r.dispatch(update);
    }
}

fn competing(behavior: Option<DragStartBehavior>) -> Rc<RefCell<Trace>> {
    let key = GestureArenaKey::pointer(0, 5);
    let mut arena = GestureArena::new();
    let h = arena.add(key, false);
    let v = arena.add(key, false);
    let (mut horizontal, trace) = horizontal(behavior);
    let mut vertical = PointerGestureRecognizer::new(GestureCallbacks {
        on_vertical_drag_start: Some(Rc::new(|_| panic!("vertical does not win the sample"))),
        ..GestureCallbacks::default()
    });
    let clock = Instant::now();
    let down = event(PointerPhase::Down, Offset::new(10., 10.), clock);
    assert!(matches!(horizontal.observe(down), GestureDecision::Pending));
    assert!(matches!(vertical.observe(down), GestureDecision::Pending));
    let motion = event(
        PointerPhase::Move,
        Offset::new(30., 0.),
        clock + Duration::from_millis(10),
    );
    let GestureDecision::Accept(claim) = horizontal.observe(motion) else {
        panic!("horizontal crosses its own hit slop");
    };
    assert!(matches!(vertical.observe(motion), GestureDecision::Pending));
    let entries = arena.accept(key, h);
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.member == h)
            .unwrap()
            .disposition,
        GestureDisposition::Accepted
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.member == v)
            .unwrap()
            .disposition,
        GestureDisposition::Rejected
    );
    vertical.reject();
    horizontal.dispatch(claim);
    if let Some(update) = horizontal.take_start_update() {
        horizontal.dispatch(update);
    }
    trace
}

#[test]
fn default_competing_start_uses_the_event_that_won() {
    let trace = competing(None);
    assert_eq!(
        trace.borrow().starts[0].global_position,
        Offset::new(30., 0.)
    );
}

#[test]
fn explicit_start_uses_the_event_that_won_without_an_update() {
    let trace = competing(Some(DragStartBehavior::Start));
    let results = trace.borrow();
    assert_eq!(results.starts[0].global_position, Offset::new(30., 0.));
    assert!(results.updates.is_empty());
}

#[test]
fn down_uses_the_press_and_projects_the_pending_update() {
    let trace = competing(Some(DragStartBehavior::Down));
    let results = trace.borrow();
    assert_eq!(results.starts[0].global_position, Offset::new(10., 10.));
    assert_eq!(results.updates[0].global_position, Offset::new(30., 10.));
    assert_eq!(results.updates[0].delta, Offset::new(20., 0.));
}

fn owned_down(base: Instant) -> (PointerGestureRecognizer, Rc<RefCell<Trace>>) {
    let (mut r, trace) = horizontal(Some(DragStartBehavior::Down));
    let key = GestureArenaKey::pointer(0, 5);
    let mut arena = GestureArena::new();
    let member = arena.add(key, false);
    assert!(trace.borrow().starts.is_empty());
    let down = event(
        PointerPhase::Down,
        Offset::new(10., 10.),
        base + Duration::from_millis(100),
    );
    assert!(matches!(r.observe(down), GestureDecision::Pending));
    assert!(r.can_accept_on_press());
    assert_eq!(
        arena.accept(key, member)[0].disposition,
        GestureDisposition::Accepted
    );
    let accepted = r.accept_on_press().expect("sole drag wins on Down");
    r.dispatch(accepted);
    (r, trace)
}

#[test]
fn owned_drag_delivers_zero_delta_moves_and_ends_only_on_release() {
    let clock = Instant::now();
    let (mut r, trace) = owned_down(clock);
    assert_eq!(trace.borrow().starts.len(), 1);
    assert!(trace.borrow().updates.is_empty() && trace.borrow().ends.is_empty());
    let pos = Offset::new(20., 25.);
    for (index, (time, delta)) in [(200, 10.), (300, 0.)].into_iter().enumerate() {
        drive_drag(
            &mut r,
            event(PointerPhase::Move, pos, clock + Duration::from_millis(time)),
        );
        let trace = trace.borrow();
        assert_eq!(trace.starts.len(), 1);
        assert!(trace.ends.is_empty());
        assert_eq!(trace.updates.len(), index + 1);
        assert_eq!(trace.updates[index].primary_delta, Some(delta));
        assert_eq!(trace.updates[index].delta, Offset::new(delta, 0.));
    }
    drive_drag(
        &mut r,
        event(PointerPhase::Up, pos, clock + Duration::from_millis(400)),
    );
    let trace = trace.borrow();
    assert_eq!(trace.starts.len(), 1);
    assert_eq!(trace.updates.len(), 2);
    assert_eq!(trace.ends.len(), 1);
}

#[test]
fn drag_details_keep_original_down_and_even_stationary_move_timestamps() {
    let clock = Instant::now();
    let (mut r, trace) = owned_down(clock);
    assert_eq!(
        trace.borrow().starts[0].source_time,
        Some(clock + Duration::from_millis(100))
    );
    let pos = Offset::new(20., 25.);
    for (index, time) in [200, 300].into_iter().enumerate() {
        let stamp = clock + Duration::from_millis(time);
        drive_drag(&mut r, event(PointerPhase::Move, pos, stamp));
        assert_eq!(trace.borrow().updates[index].source_time, Some(stamp));
    }
}

#[test]
fn world_positions_follow_every_event_while_horizontal_deltas_drop_the_y_axis() {
    let clock = Instant::now();
    let (mut r, trace) = owned_down(clock);
    assert_eq!(
        trace.borrow().starts[0].global_position,
        Offset::new(10., 10.)
    );
    assert!(trace.borrow().updates.is_empty());
    for (index, (position, dx)) in [(Offset::new(20., 25.), 10.), (Offset::new(0., 45.), -20.)]
        .into_iter()
        .enumerate()
    {
        drive_drag(
            &mut r,
            event(
                PointerPhase::Move,
                position,
                clock + Duration::from_millis(200 + index as u64 * 100),
            ),
        );
        let trace = trace.borrow();
        assert_eq!(trace.updates[index].global_position, position);
        assert_eq!(trace.updates[index].delta, Offset::new(dx, 0.));
    }
    drive_drag(
        &mut r,
        event(
            PointerPhase::Up,
            Offset::new(0., 45.),
            clock + Duration::from_millis(400),
        ),
    );
}
