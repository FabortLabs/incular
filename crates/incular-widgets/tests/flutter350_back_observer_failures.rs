//! Flutter 3.50.0 catches observer errors without stopping later observers.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_core::Offset;
use incular_platform::message_codecs::{MessageValue as V, MethodCall, StandardMethodCodec};
use incular_widgets::{
    BackGestureDispatch, BackGestureEvent, BackGestureNotification as Stage, BackGestureObserver,
    BackGestureObserverFailure, PredictiveBackEvent, add_shared_back_gesture_observer,
    dispatch_back_gesture_checked, dispatch_pop_route_checked,
};

struct ThrowAtStart;
impl BackGestureObserver for ThrowAtStart {
    fn handle_start_back_gesture(&self, _: &PredictiveBackEvent) -> bool {
        panic!("Intentional test exception from observer");
    }
}

struct Logging(Rc<RefCell<Vec<&'static str>>>);
impl BackGestureObserver for Logging {
    fn handle_start_back_gesture(&self, event: &PredictiveBackEvent) -> bool {
        assert_eq!(event.touch_offset, Some(Offset::new(5., 300.)));
        self.0.borrow_mut().push("start");
        true
    }

    fn handle_update_back_gesture_progress(&self, event: &PredictiveBackEvent) {
        assert_eq!(event.progress, 0.35);
        self.0.borrow_mut().push("update");
    }

    fn handle_commit_back_gesture(&self) {
        self.0.borrow_mut().push("commit");
    }

    fn handle_cancel_back_gesture(&self) {
        self.0.borrow_mut().push("cancel");
    }

    fn handle_pop_route(&self) -> bool {
        self.0.borrow_mut().push("pop");
        false
    }
}

fn wire(name: &str, arguments: V) -> BackGestureEvent {
    let bytes = StandardMethodCodec::encode_method_call(&MethodCall {
        method: name.into(),
        arguments,
    })
    .unwrap();
    let call = StandardMethodCodec::decode_method_call(&bytes).unwrap();
    BackGestureEvent::try_from_method_call(&call).unwrap()
}

fn start() -> BackGestureEvent {
    wire(
        "startBackGesture",
        V::Map(vec![
            (
                V::String("touchOffset".into()),
                V::List(vec![V::Float(5.), V::Float(300.)]),
            ),
            (V::String("progress".into()), V::Float(0.)),
            (V::String("swipeEdge".into()), V::Int(0)),
        ]),
    )
}

fn expect_one(report: BackGestureDispatch, handled: bool, stage: Stage) {
    assert_eq!(report.handled, handled);
    assert_eq!(
        report.failures,
        [BackGestureObserverFailure {
            notification: stage,
            message: "Intentional test exception from observer".into(),
        }]
    );
}

// widgets/binding_test.dart:1058
#[test]
fn panicking_start_observer_does_not_stop_the_following_observer() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let _throws = add_shared_back_gesture_observer(Rc::new(ThrowAtStart));
    let _later = add_shared_back_gesture_observer(Rc::new(Logging(log.clone())));
    expect_one(dispatch_back_gesture_checked(&start()), true, Stage::Start);
    assert_eq!(*log.borrow(), ["start"]);
}

struct ThrowsOnce(Cell<Option<Stage>>);

impl ThrowsOnce {
    fn at(&self, stage: Stage) {
        if self.0.get() == Some(stage) {
            self.0.set(None);
            panic!("Intentional test exception from observer");
        }
    }
}

impl BackGestureObserver for ThrowsOnce {
    fn handle_start_back_gesture(&self, _: &PredictiveBackEvent) -> bool {
        true
    }
    fn handle_update_back_gesture_progress(&self, _: &PredictiveBackEvent) {
        self.at(Stage::Update);
    }
    fn handle_commit_back_gesture(&self) {
        self.at(Stage::Commit);
    }
    fn handle_cancel_back_gesture(&self) {
        self.at(Stage::Cancel);
    }
    fn handle_pop_route(&self) -> bool {
        self.at(Stage::PopRoute);
        false
    }
}

// widgets/binding_test.dart:1175,1225,1270
#[test]
fn panicking_followers_do_not_block_progress_commit_or_cancel() {
    let cases = [
        (
            Stage::Update,
            "update",
            wire(
                "updateBackGestureProgress",
                V::Map(vec![
                    (V::String("x".into()), V::Float(100.)),
                    (V::String("y".into()), V::Float(300.)),
                    (V::String("progress".into()), V::Float(0.35)),
                    (V::String("swipeEdge".into()), V::Int(0)),
                ]),
            ),
        ),
        (Stage::Commit, "commit", wire("commitBackGesture", V::Null)),
        (Stage::Cancel, "cancel", wire("cancelBackGesture", V::Null)),
    ];
    for (stage, name, event) in cases {
        let log = Rc::new(RefCell::new(Vec::new()));
        let _throws = add_shared_back_gesture_observer(Rc::new(ThrowsOnce(Cell::new(Some(stage)))));
        let _later = add_shared_back_gesture_observer(Rc::new(Logging(log.clone())));
        let begun = dispatch_back_gesture_checked(&start());
        assert!(begun.handled && begun.failures.is_empty());
        expect_one(dispatch_back_gesture_checked(&event), true, stage);
        assert_eq!(*log.borrow(), ["start", name]);
    }
}

// widgets/binding_test.dart:866
#[test]
fn failed_system_pop_still_reaches_the_other_observers() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let _later = add_shared_back_gesture_observer(Rc::new(Logging(log.clone())));
    let _throws =
        add_shared_back_gesture_observer(Rc::new(ThrowsOnce(Cell::new(Some(Stage::PopRoute)))));
    expect_one(dispatch_pop_route_checked(), false, Stage::PopRoute);
    assert_eq!(*log.borrow(), ["pop"]);
}
