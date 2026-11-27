use std::{
    cell::{Cell, RefCell},
    future::{Future, Ready, poll_fn, ready},
    panic::resume_unwind,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_platform::message_codecs::{JsonMethodCodec, MethodCall};
use incular_platform::{
    AppExitResponse, AppLifecycleBinding, FlutterAppExit, FlutterInboundMessenger,
    FlutterJsonMethodResult,
};
use serde_json::{Value, json};

fn exit_call() -> MethodCall<Value> {
    MethodCall {
        method: "System.requestAppExit".into(),
        arguments: Value::Null,
    }
}

fn finish<T>(future: impl Future<Output = T>) -> T {
    let mut future = Box::pin(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("expected a ready test future"),
    }
}

#[test]
fn inbound_exit_reaches_a_registered_listener_and_returns_its_decision() {
    let binding = Rc::new(AppLifecycleBinding::default());
    let called = Rc::new(Cell::new(0));
    let _listen = binding.exit_observers().add_observer({
        let called = called.clone();
        move || {
            called.set(called.get() + 1);
            ready(AppExitResponse::Cancel)
        }
    });

    let bus = FlutterInboundMessenger::new(|task| task());
    FlutterAppExit::CHANNEL.set_method_call_handler(&bus, move |call, reply| {
        let result = finish(binding.handle_app_exit_request(&call, |_| panic!("unexpected error")));
        reply(result.unwrap_or(FlutterJsonMethodResult::NotImplemented)).unwrap();
    });
    let response = Rc::new(RefCell::new(None));
    let captured = response.clone();
    bus.push(
        "flutter/platform",
        Some(JsonMethodCodec::encode_method_call(&exit_call()).unwrap()),
        move |answer| {
            *captured.borrow_mut() = answer;
        },
    );
    assert_eq!(called.get(), 1);
    let encoded = response.borrow();
    assert_eq!(
        JsonMethodCodec::decode_envelope(encoded.as_ref().unwrap()).unwrap(),
        json!({"response":"cancel"})
    );
}

#[test]
fn disposed_and_new_observers_are_skipped_when_a_request_is_already_pending() {
    let binding = AppLifecycleBinding::default();
    let listeners = binding.exit_observers();
    let waiting = Rc::new(Cell::new(false));
    let entered = Rc::new(Cell::new(0));
    let first = listeners.add_observer({
        let waiting = waiting.clone();
        let entered = entered.clone();
        move || {
            entered.set(entered.get() + 1);
            let waiting = waiting.clone();
            poll_fn(move |_| {
                if waiting.get() {
                    Poll::Ready(AppExitResponse::Exit)
                } else {
                    Poll::Pending
                }
            })
        }
    });
    let disposed = listeners.add_observer(|| async { panic!("removed before its first call") });
    let call = exit_call();
    let mut pending =
        Box::pin(binding.handle_app_exit_request(&call, |_| panic!("unexpected error")));
    let mut context = Context::from_waker(Waker::noop());
    assert!(pending.as_mut().poll(&mut context).is_pending());
    assert_eq!(entered.get(), 1);
    assert!(disposed.dispose());
    assert!(!disposed.dispose());
    let late_calls = Rc::new(Cell::new(0));
    let _late = listeners.add_observer({
        let late_calls = late_calls.clone();
        move || {
            late_calls.set(late_calls.get() + 1);
            ready(AppExitResponse::Exit)
        }
    });
    waiting.set(true);
    let Poll::Ready(Some(FlutterJsonMethodResult::Success(value))) =
        pending.as_mut().poll(&mut context)
    else {
        panic!("the released request must respond");
    };
    assert_eq!(value, json!({"response":"exit"}));
    assert_eq!(late_calls.get(), 0);
    drop(pending);
    drop(first);
    let response = finish(binding.handle_app_exit_request(&call, |_| panic!("unexpected error")));
    assert!(
        matches!(response, Some(FlutterJsonMethodResult::Success(value)) if value == json!({"response":"exit"}))
    );
    assert_eq!(late_calls.get(), 1);
}

#[test]
fn cancellation_and_errors_do_not_prevent_later_observers_from_running() {
    let binding = AppLifecycleBinding::default();
    let listeners = binding.exit_observers();
    let order = Rc::new(RefCell::new(Vec::new()));
    let _cancel = listeners.add_observer({
        let order = order.clone();
        move || {
            order.borrow_mut().push("cancel");
            ready(AppExitResponse::Cancel)
        }
    });
    let startup = listeners.add_observer(|| -> Ready<AppExitResponse> {
        resume_unwind(Box::new("initialization panic"))
    });
    let tick = listeners.add_observer(|| async { resume_unwind(Box::new("future panic")) });
    let _last = listeners.add_observer({
        let order = order.clone();
        move || {
            order.borrow_mut().push("last");
            ready(AppExitResponse::Exit)
        }
    });
    let mut failures = Vec::new();
    let response = finish(binding.handle_app_exit_request(&exit_call(), |failure| {
        let text = *failure.payload.downcast::<&'static str>().unwrap();
        failures.push((failure.observer, text));
        order.borrow_mut().push("report");
    }));
    assert!(
        matches!(response, Some(FlutterJsonMethodResult::Success(value)) if value == json!({"response":"cancel"}))
    );
    assert_eq!(*order.borrow(), ["cancel", "report", "report", "last"]);
    assert_eq!(
        failures,
        [
            (startup.id(), "initialization panic"),
            (tick.id(), "future panic")
        ]
    );
}

#[test]
fn empty_binding_permits_exit_and_unrelated_methods_do_not_start_a_query() {
    let binding = AppLifecycleBinding::default();
    let response =
        finish(binding.handle_app_exit_request(&exit_call(), |_| panic!("no observers")));
    assert!(
        matches!(response, Some(FlutterJsonMethodResult::Success(value)) if value == json!({"response":"exit"}))
    );

    let _observer = binding
        .exit_observers()
        .add_observer(|| async { panic!("wrong method") });
    let call = MethodCall {
        method: "SystemChrome.systemUIChange".into(),
        arguments: Value::Null,
    };
    assert!(
        finish(binding.handle_app_exit_request(&call, |_| panic!("unrelated method"))).is_none()
    );
}
