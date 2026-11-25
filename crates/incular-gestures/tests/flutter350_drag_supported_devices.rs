use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{
    DragStartBehavior, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    time::Instant,
};
use incular_gestures::{
    GestureCallbacks, GestureDecision, GestureRecognizer, PointerGestureRecognizer, RawPointerEvent,
};

#[derive(Clone, Debug, PartialEq)]
enum Report {
    Start,
    Update(f32),
    End,
}

struct Fixture {
    state: PointerGestureRecognizer,
    trace: Rc<RefCell<Vec<Report>>>,
    clock: Instant,
}

impl Fixture {
    fn new(horizontal: bool, allowed: PointerDeviceKind) -> Self {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let start = {
            let trace = trace.clone();
            Rc::new(move |_| trace.borrow_mut().push(Report::Start))
        };
        let update = {
            let trace = trace.clone();
            Rc::new(move |event: incular_gestures::DragUpdateDetails| {
                trace
                    .borrow_mut()
                    .push(Report::Update(event.primary_delta.unwrap()))
            })
        };
        let end = {
            let trace = trace.clone();
            Rc::new(move |_| trace.borrow_mut().push(Report::End))
        };
        let mut callbacks = GestureCallbacks {
            drag_start_behavior: DragStartBehavior::Down,
            ..Default::default()
        };
        if horizontal {
            callbacks.on_horizontal_drag_start = Some(start);
            callbacks.on_horizontal_drag_update = Some(update);
            callbacks.on_horizontal_drag_end = Some(end);
        } else {
            callbacks.on_vertical_drag_start = Some(start);
            callbacks.on_vertical_drag_update = Some(update);
            callbacks.on_vertical_drag_end = Some(end);
        }
        let mut state = PointerGestureRecognizer::new(callbacks);
        state.set_supported_devices([allowed]);
        Self {
            state,
            trace,
            clock: Instant::now(),
        }
    }

    fn send(&mut self, kind: PointerDeviceKind, phase: PointerPhase, position: Offset) {
        let buttons = if phase == PointerPhase::Up {
            0
        } else {
            PRIMARY_POINTER_BUTTON
        };
        let action = self.state.observe_raw(RawPointerEvent::new(
            5, 1, kind, buttons, position, phase, self.clock,
        ));
        if phase == PointerPhase::Down {
            assert!(!matches!(action, GestureDecision::Accept(_)));
            if let Some(start) = self.state.accept_on_press() {
                self.state.dispatch(start);
            }
        } else if let GestureDecision::Accept(action) = action {
            self.state.dispatch(action);
            if let Some(pending) = self.state.take_start_update() {
                self.state.dispatch(pending);
            }
        }
        self.clock += Duration::from_millis(10);
    }

    fn rejected(&mut self, kind: PointerDeviceKind, move_to: Offset) {
        assert!(self.trace.borrow().is_empty());
        self.send(kind, PointerPhase::Down, Offset::new(10., 10.));
        assert!(self.trace.borrow().is_empty());
        self.send(kind, PointerPhase::Move, move_to);
        assert!(self.trace.borrow().is_empty());
        self.send(kind, PointerPhase::Move, move_to);
        assert!(self.trace.borrow().is_empty());
        self.send(kind, PointerPhase::Up, move_to);
        assert!(self.trace.borrow().is_empty());
    }
}

#[test]
fn horizontal_touch_only_drag_rejects_a_mouse_sequence() {
    Fixture::new(true, PointerDeviceKind::Touch)
        .rejected(PointerDeviceKind::Mouse, Offset::new(20., 25.));
}

#[test]
fn vertical_touch_only_drag_rejects_a_mouse_sequence() {
    Fixture::new(false, PointerDeviceKind::Touch)
        .rejected(PointerDeviceKind::Mouse, Offset::new(25., 20.));
}

#[test]
fn ignored_touch_does_not_prevent_the_next_mouse_drag_on_the_same_recognizer() {
    let mut fixture = Fixture::new(true, PointerDeviceKind::Mouse);
    let move_to = Offset::new(20., 25.);
    fixture.rejected(PointerDeviceKind::Touch, move_to);
    fixture.send(
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        Offset::new(10., 10.),
    );
    assert_eq!(*fixture.trace.borrow(), [Report::Start]);
    fixture.send(PointerDeviceKind::Mouse, PointerPhase::Move, move_to);
    assert_eq!(
        *fixture.trace.borrow(),
        [Report::Start, Report::Update(10.)]
    );
    fixture.send(PointerDeviceKind::Mouse, PointerPhase::Up, move_to);
    assert_eq!(
        *fixture.trace.borrow(),
        [Report::Start, Report::Update(10.), Report::End]
    );
}
