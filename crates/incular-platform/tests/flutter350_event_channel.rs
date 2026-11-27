use std::{
    cell::RefCell,
    collections::VecDeque,
    future::{Future, ready},
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_platform::{
    FlutterEventOperation, FlutterEventTask, FlutterInboundMessenger, FlutterJsonEventChannel,
    FlutterJsonStreamEvent as Event, FlutterPlatformChannelError, FlutterPlatformMessenger,
    message_codecs::{JsonMethodCodec, MethodCall, PlatformException},
};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct Loop(Rc<RefCell<VecDeque<FlutterEventTask>>>);

impl Loop {
    fn enqueue(&self, task: FlutterEventTask) {
        self.0.borrow_mut().push_back(task);
    }

    fn flush(&self) {
        loop {
            let task = self.0.borrow_mut().pop_front();
            let Some(mut task) = task else { break };
            assert!(matches!(
                task.as_mut().poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(())
            ));
        }
    }
}

#[derive(Clone, Copy)]
enum Script {
    DataThenClose,
    ErrorOnly,
    Quiet,
    Missing,
}

type Calls = Rc<RefCell<Vec<MethodCall<Value>>>>;
type Errors = Rc<RefCell<Vec<FlutterEventOperation>>>;

#[derive(Clone)]
struct Host {
    inbound: FlutterInboundMessenger,
    calls: Calls,
    script: Script,
}

impl FlutterPlatformMessenger for Host {
    type Error = std::convert::Infallible;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        let method = JsonMethodCodec::decode_method_call(&payload.unwrap()).unwrap();
        if method.method == "listen" {
            let send = |payload| {
                self.inbound
                    .push(channel, payload, |reply| assert_eq!(reply, None))
            };
            match self.script {
                Script::DataThenClose => {
                    for suffix in [1, 2] {
                        let data = json!(format!("{}{suffix}", method.arguments.as_str().unwrap()));
                        send(Some(
                            JsonMethodCodec::encode_success_envelope(&data).unwrap(),
                        ));
                    }
                    send(None);
                }
                Script::ErrorOnly => {
                    let error = PlatformException {
                        code: "404".into(),
                        message: Some("Not Found.".into()),
                        details: method.arguments.clone(),
                        stacktrace: None,
                    };
                    send(Some(
                        JsonMethodCodec::encode_error_envelope(&error).unwrap(),
                    ));
                }
                Script::Quiet | Script::Missing => {}
            }
        }
        self.calls.borrow_mut().push(method);
        let reply = (!matches!(self.script, Script::Missing))
            .then(|| JsonMethodCodec::encode_success_envelope(&Value::Null).unwrap());
        ready(Ok(reply))
    }
}

fn setup(
    script: Script,
) -> (
    Loop,
    FlutterInboundMessenger,
    FlutterJsonEventChannel<Host>,
    Calls,
    Errors,
) {
    let tasks = Loop::default();
    let scheduler = tasks.clone();
    let inbound = FlutterInboundMessenger::new(move |microtask| {
        scheduler.enqueue(Box::pin(async move { microtask() }));
    });
    let calls = Calls::default();
    let errors = Errors::default();
    let scheduler = tasks.clone();
    let reported = errors.clone();
    let channel = FlutterJsonEventChannel::new(
        "ch",
        inbound.clone(),
        Host {
            inbound: inbound.clone(),
            calls: calls.clone(),
            script,
        },
        move |task| scheduler.enqueue(task),
        move |operation, error| {
            assert!(
                matches!(error, FlutterPlatformChannelError::MissingPlugin(_))
                    || operation == FlutterEventOperation::Decode
            );
            reported.borrow_mut().push(operation);
        },
    );
    (tasks, inbound, channel, calls, errors)
}

fn capture(
    stream: &incular_platform::FlutterJsonBroadcast<Host>,
) -> (
    Rc<RefCell<Vec<Event>>>,
    incular_platform::FlutterEventSubscription,
) {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let sender = observed.clone();
    let guard = stream.subscribe(move |event| sender.borrow_mut().push(event));
    (observed, guard)
}

#[test]
fn a_platform_stream_delivers_data_in_order_and_cancels_on_end_of_stream() {
    let (tasks, _, channel, calls, errors) = setup(Script::DataThenClose);
    let stream = channel.receive_broadcast_stream(json!("hello"));
    let (observed, guard) = capture(&stream);
    assert!(observed.borrow().is_empty());
    assert!(calls.borrow().is_empty());
    tasks.flush();
    assert_eq!(
        &*observed.borrow(),
        &[
            Event::Data(json!("hello1")),
            Event::Data(json!("hello2")),
            Event::Done,
        ]
    );
    assert_eq!(
        &*calls.borrow(),
        &[
            MethodCall {
                method: "listen".into(),
                arguments: json!("hello")
            },
            MethodCall {
                method: "cancel".into(),
                arguments: json!("hello")
            },
        ]
    );
    let (late, _late_guard) = capture(&stream);
    assert!(late.borrow().is_empty());
    drop(guard);
    tasks.flush();
    assert_eq!(&*late.borrow(), &[Event::Done]);
    assert_eq!(calls.borrow().len(), 2);
    assert!(errors.borrow().is_empty());
}

#[test]
fn a_platform_error_is_an_event_and_does_not_close_the_stream() {
    let (tasks, inbound, channel, calls, errors) = setup(Script::ErrorOnly);
    let stream = channel.receive_broadcast_stream(json!("hello"));
    let (observed, guard) = capture(&stream);
    tasks.flush();
    assert_eq!(
        &*observed.borrow(),
        &[Event::Error(PlatformException {
            code: "404".into(),
            message: Some("Not Found.".into()),
            details: json!("hello"),
            stacktrace: None,
        })]
    );
    assert_eq!(calls.borrow().len(), 1);
    inbound.push(
        "ch",
        Some(JsonMethodCodec::encode_success_envelope(&Value::Null).unwrap()),
        |_| {},
    );
    tasks.flush();
    assert_eq!(observed.borrow().last(), Some(&Event::Data(Value::Null)));
    assert!(errors.borrow().is_empty());
    drop(guard);
    tasks.flush();
    assert_eq!(calls.borrow().last().unwrap().method, "cancel");
}

#[test]
fn first_and_last_subscriber_control_activation_and_errors_are_separate() {
    let (tasks, _, channel, calls, errors) = setup(Script::Quiet);
    let stream = channel.receive_broadcast_stream(Value::Null);
    let (_, first) = capture(&stream);
    let (_, second) = capture(&stream);
    tasks.flush();
    assert_eq!(calls.borrow()[0].method, "listen");
    assert_eq!(calls.borrow().len(), 1);
    drop(first);
    tasks.flush();
    assert_eq!(calls.borrow().len(), 1);
    drop(second);
    tasks.flush();
    assert_eq!(calls.borrow()[1].method, "cancel");
    assert_eq!(calls.borrow().len(), 2);
    assert!(errors.borrow().is_empty());

    let (tasks, _, channel, _, errors) = setup(Script::Missing);
    let stream = channel.receive_broadcast_stream(Value::Null);
    let (observed, guard) = capture(&stream);
    tasks.flush();
    drop(guard);
    tasks.flush();
    assert!(observed.borrow().is_empty());
    assert_eq!(
        &*errors.borrow(),
        &[FlutterEventOperation::Listen, FlutterEventOperation::Cancel]
    );
}

#[test]
fn pending_events_only_reach_the_subscribers_that_remain_active() {
    let (tasks, inbound, channel, _, errors) = setup(Script::Quiet);
    let stream = channel.receive_broadcast_stream(Value::Null);
    let (first, first_guard) = capture(&stream);
    let (second, second_guard) = capture(&stream);
    tasks.flush();

    let envelope = JsonMethodCodec::encode_success_envelope(&json!("pending")).unwrap();
    inbound.push("ch", Some(envelope), |_| {});
    drop(first_guard);
    let (late, late_guard) = capture(&stream);
    tasks.flush();
    assert!(first.borrow().is_empty());
    assert_eq!(&*second.borrow(), &[Event::Data(json!("pending"))]);
    assert!(late.borrow().is_empty());

    inbound.push("ch", Some(b"invalid JSON".to_vec()), |_| {});
    assert_eq!(&*errors.borrow(), &[FlutterEventOperation::Decode]);
    inbound.push("ch", None, |_| {});
    tasks.flush();
    assert_eq!(second.borrow().last(), Some(&Event::Done));
    assert_eq!(&*late.borrow(), &[Event::Done]);
    drop((second_guard, late_guard));
    tasks.flush();
}
