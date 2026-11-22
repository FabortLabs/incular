//! Flutter's back gesture dispatch in `WidgetsBinding` (support-002): only
//! observers that accept a gesture's start hear the rest of it.

use std::{cell::RefCell, rc::Rc};

use incular_core::Offset;
use incular_widgets::{
    BackGestureEvent, BackGestureObserver, PredictiveBackEvent, SwipeEdge,
    add_shared_back_gesture_observer, dispatch_back_gesture,
};

struct Recorder {
    name: &'static str,
    accepts: bool,
    log: Rc<RefCell<Vec<String>>>,
}

impl BackGestureObserver for Recorder {
    fn handle_start_back_gesture(&self, event: &PredictiveBackEvent) -> bool {
        self.log
            .borrow_mut()
            .push(format!("{} start {}", self.name, event.progress));
        self.accepts
    }

    fn handle_update_back_gesture_progress(&self, event: &PredictiveBackEvent) {
        self.log
            .borrow_mut()
            .push(format!("{} update {}", self.name, event.progress));
    }

    fn handle_commit_back_gesture(&self) {
        self.log.borrow_mut().push(format!("{} commit", self.name));
    }

    fn handle_cancel_back_gesture(&self) {
        self.log.borrow_mut().push(format!("{} cancel", self.name));
    }
}

fn swipe(progress: f32) -> PredictiveBackEvent {
    PredictiveBackEvent::new(Some(Offset::new(5., 5.)), progress, SwipeEdge::Left)
}

fn recorder(name: &'static str, accepts: bool, log: &Rc<RefCell<Vec<String>>>) -> Rc<Recorder> {
    Rc::new(Recorder {
        name,
        accepts,
        log: log.clone(),
    })
}

#[test]
fn only_observers_that_accept_the_start_follow_the_gesture() {
    let log = Rc::default();
    let _a = add_shared_back_gesture_observer(recorder("a", false, &log));
    let _b = add_shared_back_gesture_observer(recorder("b", true, &log));
    assert!(dispatch_back_gesture(&BackGestureEvent::Start(swipe(0.))));
    assert!(dispatch_back_gesture(&BackGestureEvent::Update(swipe(0.5))));
    assert!(dispatch_back_gesture(&BackGestureEvent::Commit));
    assert_eq!(
        *log.borrow(),
        ["a start 0", "b start 0", "b update 0.5", "b commit"]
    );
}

#[test]
fn unfollowed_gestures_report_false() {
    let log = Rc::default();
    let _a = add_shared_back_gesture_observer(recorder("a", false, &log));
    assert!(!dispatch_back_gesture(&BackGestureEvent::Start(swipe(0.))));
    assert!(!dispatch_back_gesture(&BackGestureEvent::Commit));
    assert!(!dispatch_back_gesture(&BackGestureEvent::Cancel));
}

#[test]
fn removing_a_following_observer_cancels_its_gesture() {
    let log = Rc::default();
    let registration = add_shared_back_gesture_observer(recorder("a", true, &log));
    dispatch_back_gesture(&BackGestureEvent::Start(swipe(0.)));
    drop(registration);
    assert_eq!(*log.borrow(), ["a start 0", "a cancel"]);
    assert!(!dispatch_back_gesture(&BackGestureEvent::Update(swipe(
        0.3
    ))));
}

#[test]
fn a_root_back_button_dispatcher_answers_system_back_while_it_has_callbacks() {
    use incular_widgets::{RootBackButtonDispatcher, dispatch_pop_route};

    let root = RootBackButtonDispatcher::new();
    let hits = Rc::new(std::cell::Cell::new(0));
    let counter = hits.clone();
    let subscription = root.add_callback(move || {
        counter.set(counter.get() + 1);
        true
    });
    assert!(dispatch_pop_route());
    assert_eq!(hits.get(), 1);

    drop(subscription);
    assert!(!dispatch_pop_route());
    assert_eq!(hits.get(), 1);
}
