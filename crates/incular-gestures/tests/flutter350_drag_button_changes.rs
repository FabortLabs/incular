use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, SECONDARY_POINTER_BUTTON,
    time::Instant,
};
use incular_gestures::{
    GestureCallbacks, GestureDecision, GestureRecognizer, PointerGestureRecognizer, RawPointerEvent,
};

struct Host {
    recognizer: PointerGestureRecognizer,
    log: Rc<RefCell<Vec<&'static str>>>,
    now: Instant,
}

impl Host {
    fn new() -> Self {
        let log = Rc::new(RefCell::new(Vec::new()));
        macro_rules! record {
            ($name:literal) => {{
                let log = log.clone();
                Some(Rc::new(move |_| log.borrow_mut().push($name)))
            }};
        }
        let cancelled = log.clone();
        let recognizer = PointerGestureRecognizer::new(GestureCallbacks {
            on_tap: Some(Rc::new(|| {})), // the pan is not a sole member
            on_pan_down: record!("down"),
            on_pan_start: record!("start"),
            on_pan_update: record!("update"),
            on_pan_end: record!("end"),
            on_pan_cancel: Some(Rc::new(move || cancelled.borrow_mut().push("cancel"))),
            ..GestureCallbacks::default()
        });
        Self {
            recognizer,
            log,
            now: Instant::now(),
        }
    }

    fn emit(&mut self, id: u64, phase: PointerPhase, xy: (f32, f32), buttons: u32) {
        let event = RawPointerEvent::new(
            id,
            1,
            PointerDeviceKind::Mouse,
            buttons,
            Offset::new(xy.0, xy.1),
            phase,
            self.now,
        );
        match self.recognizer.observe_raw(event) {
            GestureDecision::Accept(action) => {
                self.recognizer.dispatch(action);
                if let Some(update) = self.recognizer.take_start_update() {
                    self.recognizer.dispatch(update);
                }
            }
            GestureDecision::Reject | GestureDecision::Cancelled => self.recognizer.reject(),
            GestureDecision::Pending => {}
        }
        self.now += Duration::from_millis(10);
    }

    fn down(&mut self, id: u64) {
        self.emit(id, PointerPhase::Down, (10., 10.), PRIMARY_POINTER_BUTTON);
    }
    fn move_(&mut self, id: u64, xy: (f32, f32)) {
        self.emit(id, PointerPhase::Move, xy, PRIMARY_POINTER_BUTTON);
    }
    fn changed(&mut self, id: u64, xy: (f32, f32)) {
        self.emit(id, PointerPhase::Move, xy, SECONDARY_POINTER_BUTTON);
    }
    fn up(&mut self, id: u64, xy: (f32, f32)) {
        self.emit(id, PointerPhase::Up, xy, 0);
    }
    fn expect(&self, log: &[&str]) {
        assert_eq!(self.log.borrow().as_slice(), log);
    }
    fn next_drag(&mut self) {
        self.log.borrow_mut().clear();
        self.down(6);
        self.expect(&["down"]);
        self.move_(6, (30., 30.));
        self.expect(&["down", "start"]);
        self.up(6, (30., 30.));
        self.expect(&["down", "start", "end"]);
    }
}

#[test]
fn changed_buttons_cancel_before_acceptance_even_if_the_move_crosses_slop() {
    let mut host = Host::new();
    host.down(5);
    host.expect(&["down"]);
    host.changed(5, (30., 30.));
    host.expect(&["down", "cancel"]);
    host.up(5, (30., 30.));
    host.expect(&["down", "cancel"]);
}

#[test]
fn a_canceled_mouse_pan_does_not_contaminate_the_next_pointer() {
    let mut host = Host::new();
    host.down(5);
    host.changed(5, (10., 10.));
    host.up(5, (10., 10.));
    host.expect(&["down", "cancel"]);
    host.next_drag();
}

#[test]
fn changing_buttons_ends_an_accepted_pan_and_ignores_further_moves() {
    let mut host = Host::new();
    host.down(5);
    host.expect(&["down"]);
    host.move_(5, (30., 30.));
    host.expect(&["down", "start"]);
    host.changed(5, (30., 30.));
    host.expect(&["down", "start", "end"]);
    host.move_(5, (50., 50.));
    host.expect(&["down", "start", "end"]);
    host.up(5, (50., 50.));
    host.expect(&["down", "start", "end"]);
}

#[test]
fn a_button_terminated_pan_does_not_contaminate_the_next_pointer() {
    let mut host = Host::new();
    host.down(5);
    host.move_(5, (30., 30.));
    host.changed(5, (30., 31.));
    host.up(5, (30., 31.));
    host.expect(&["down", "start", "end"]);
    host.next_drag();
}
