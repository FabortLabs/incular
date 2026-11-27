use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    future::{Future, ready},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_platform::{
    FlutterChannelMicrotask, FlutterInboundMessenger, FlutterJsonMethodChannel as Channel,
    FlutterJsonMethodResult as Reply, FlutterPlatformChannelError as Error,
    FlutterPlatformMessenger,
    message_codecs::{JsonMethodCodec, JsonMethodCodecError, MethodCall, PlatformException},
};
use serde_json::{Value, json};

const REQUIRED: Channel = Channel::new("ch7");
const OPTIONAL: Channel = Channel::optional("ch8");

fn finish<T>(future: impl Future<Output = T>) -> T {
    let mut cx = Context::from_waker(Waker::noop());
    match pin!(future).poll(&mut cx) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("this transport has a ready reply"),
    }
}

#[derive(Default)]
struct Wire {
    calls: Vec<(&'static str, MethodCall<Value>)>,
    response: Option<Vec<u8>>,
}

impl Wire {
    fn success(value: Value) -> Self {
        Self {
            response: Some(JsonMethodCodec::encode_success_envelope(&value).unwrap()),
            ..Self::default()
        }
    }

    fn check_calls(&self, channels: &[&str]) {
        let names: Vec<_> = self
            .calls
            .iter()
            .map(|(channel, call)| {
                assert_eq!(call.method, "sayHello");
                assert_eq!(call.arguments, "hello");
                *channel
            })
            .collect();
        assert_eq!(names, channels);
    }
}

impl FlutterPlatformMessenger for Wire {
    type Error = &'static str;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        self.calls.push((
            channel,
            JsonMethodCodec::decode_method_call(&payload.expect("method call bytes")).unwrap(),
        ));
        ready(Ok(self.response.clone()))
    }
}

#[test]
fn invoke_method_returns_success_envelope_value() {
    let mut wire = Wire::success(json!("hello world"));
    assert_eq!(
        finish(REQUIRED.invoke(&mut wire, "sayHello", json!("hello"))).unwrap(),
        "hello world"
    );
    wire.check_calls(&["ch7"]);
}

#[test]
fn invoke_propagates_exact_platform_code_message_and_details() {
    let details = json!({"a": 42, "b": 3.14});
    let expected = PlatformException {
        code: "bad".into(),
        message: Some("Something happened".into()),
        details,
        stacktrace: None,
    };
    let mut wire = Wire {
        response: Some(JsonMethodCodec::encode_error_envelope(&expected).unwrap()),
        ..Wire::default()
    };
    assert!(matches!(
        finish(REQUIRED.invoke(&mut wire, "sayHello", json!("hello"))),
        Err(Error::Codec(JsonMethodCodecError::Platform(error))) if *error == expected
    ));
    wire.check_calls(&["ch7"]);
}

#[test]
fn strict_and_optional_channels_distinguish_missing_from_null_success() {
    let mut wire = Wire::default();
    assert!(matches!(
        finish(REQUIRED.invoke(&mut wire, "sayHello", json!("hello"))),
        Err(Error::MissingPlugin(error)) if error.message.as_deref() == Some("No implementation found for method sayHello on channel ch7")
    ));
    assert_eq!(
        finish(OPTIONAL.invoke(&mut wire, "sayHello", json!("hello"))).unwrap(),
        Value::Null
    );
    wire.response = Some(JsonMethodCodec::encode_success_envelope(&Value::Null).unwrap());
    assert_eq!(
        finish(REQUIRED.invoke(&mut wire, "sayHello", json!("hello"))).unwrap(),
        Value::Null
    );
    wire.check_calls(&["ch7", "ch8", "ch7"]);
}

#[test]
fn list_and_map_results_preserve_null_instead_of_converting_to_empty() {
    let mut wire = Wire::success(Value::Null);
    assert_eq!(
        finish(REQUIRED.invoke_list::<String, _>(&mut wire, "sayHello", json!("hello"))).unwrap(),
        None
    );
    assert_eq!(
        finish(REQUIRED.invoke_map::<String, _>(&mut wire, "sayHello", json!("hello"))).unwrap(),
        None
    );
    wire.check_calls(&["ch7", "ch7"]);
}

#[test]
fn typed_list_and_map_decode_nonempty_values_and_reject_bad_elements() {
    let mut list = Wire::success(json!(["hello", "world"]));
    assert_eq!(
        finish(REQUIRED.invoke_list::<String, _>(&mut list, "sayHello", json!("hello"))).unwrap(),
        Some(vec![String::from("hello"), String::from("world")])
    );
    let mut map = Wire::success(json!({"hello": "world"}));
    assert_eq!(
        finish(REQUIRED.invoke_map::<String, _>(&mut map, "sayHello", json!("hello"))).unwrap(),
        Some(BTreeMap::from([(
            String::from("hello"),
            String::from("world")
        )]))
    );
    let mut wrong = Wire::success(json!(["hello", 2]));
    assert!(matches!(
        finish(REQUIRED.invoke_list::<String, _>(&mut wrong, "sayHello", json!("hello"))),
        Err(Error::Codec(JsonMethodCodecError::Json(_)))
    ));
}

type Queue = Rc<RefCell<VecDeque<FlutterChannelMicrotask>>>;

fn pump(queue: &Queue) {
    loop {
        let next = queue.borrow_mut().pop_front();
        match next {
            Some(task) => task(),
            None => return,
        }
    }
}

fn inbound_reply(response: Reply) -> Option<Vec<u8>> {
    let tasks = Queue::default();
    let inbound = FlutterInboundMessenger::new({
        let tasks = tasks.clone();
        move |task| tasks.borrow_mut().push_back(task)
    });
    let response = Rc::new(Cell::new(Some(response)));
    REQUIRED.set_method_call_handler(&inbound, {
        let tasks = tasks.clone();
        move |call, reply| {
            assert_eq!(call.method, "sayHello");
            assert_eq!(call.arguments, "hello");
            let response = response.take().expect("one handler delivery");
            tasks
                .borrow_mut()
                .push_back(Box::new(move || reply(response).unwrap()));
        }
    });
    let envelope = Rc::new(RefCell::new(None));
    inbound.push(
        REQUIRED.name(),
        Some(
            JsonMethodCodec::encode_method_call(&MethodCall {
                method: "sayHello".into(),
                arguments: json!("hello"),
            })
            .unwrap(),
        ),
        {
            let envelope = envelope.clone();
            move |bytes| {
                let old = envelope.replace(Some(bytes));
                assert!(old.is_none());
            }
        },
    );
    assert!(envelope.borrow().is_none());
    pump(&tasks);
    envelope.take().expect("one platform reply")
}

#[test]
fn inbound_method_success_and_unimplemented_use_distinct_envelopes() {
    assert_eq!(inbound_reply(Reply::NotImplemented), None);
    let success = inbound_reply(Reply::Success(json!("hello, world"))).unwrap();
    assert_eq!(
        JsonMethodCodec::decode_envelope(&success).unwrap(),
        "hello, world"
    );
}

#[test]
fn inbound_specific_and_other_errors_encode_the_expected_platform_exception() {
    let platform = PlatformException {
        code: "bad".into(),
        message: Some("sayHello failed".into()),
        details: Value::Null,
        stacktrace: None,
    };
    let encoded = inbound_reply(Reply::PlatformError(platform.clone())).unwrap();
    assert!(matches!(
        JsonMethodCodec::decode_envelope(&encoded),
        Err(JsonMethodCodecError::Platform(error)) if *error == platform
    ));
    #[derive(Debug, thiserror::Error)]
    #[error("Invalid argument(s): {0}")]
    struct InvalidArgument(&'static str);

    let encoded = inbound_reply(Reply::Error(Box::new(InvalidArgument("bad")))).unwrap();
    assert!(matches!(
        JsonMethodCodec::decode_envelope(&encoded),
        Err(JsonMethodCodecError::Platform(error))
        if error.code == "error"
            && error.message.as_deref() == Some("Invalid argument(s): bad") && error.details.is_null()
    ));
}

#[test]
fn missing_plugin_replies_null_once_when_cleared_before_or_after_dispatch() {
    for clear_before in [true, false] {
        check_missing_plugin(clear_before);
    }
}

fn check_missing_plugin(clear_before: bool) {
    let tasks: Rc<RefCell<VecDeque<FlutterChannelMicrotask>>> = Rc::default();
    let inbound = FlutterInboundMessenger::new({
        let tasks = tasks.clone();
        move |task| tasks.borrow_mut().push_back(task)
    });
    if clear_before {
        REQUIRED.set_method_call_handler(&inbound, |_, _| panic!("removed plugin"));
        REQUIRED.clear_method_call_handler(&inbound);
    }

    let envelope = Rc::new(RefCell::new(Vec::new()));
    inbound.dispatch_unbuffered(
        REQUIRED.name(),
        Some(
            JsonMethodCodec::encode_method_call(&MethodCall {
                method: "sayHello".into(),
                arguments: json!("hello"),
            })
            .unwrap(),
        ),
        {
            let envelope = envelope.clone();
            move |reply| envelope.borrow_mut().push(reply)
        },
    );
    assert!(envelope.borrow().is_empty());
    pump(&tasks);
    assert_eq!(*envelope.borrow(), [None]);
    if !clear_before {
        REQUIRED.clear_method_call_handler(&inbound);
        pump(&tasks);
        assert_eq!(*envelope.borrow(), [None]);
    }
}

#[test]
fn dispatch_without_a_plugin_does_not_replay_a_request_after_registration() {
    let tasks: Rc<RefCell<VecDeque<FlutterChannelMicrotask>>> = Rc::default();
    let inbound = FlutterInboundMessenger::new({
        let tasks = tasks.clone();
        move |task| tasks.borrow_mut().push_back(task)
    });
    let responses = Rc::new(RefCell::new(Vec::new()));
    let call = JsonMethodCodec::encode_method_call(&MethodCall {
        method: "sayHello".into(),
        arguments: json!("hello"),
    })
    .unwrap();
    let send = || {
        let responses = responses.clone();
        inbound.dispatch_unbuffered(REQUIRED.name(), Some(call.clone()), move |reply| {
            responses.borrow_mut().push(reply)
        });
    };
    send();
    REQUIRED.set_method_call_handler(&inbound, |call, reply| {
        assert_eq!(call.method, "sayHello");
        assert_eq!(call.arguments, json!("hello"));
        reply(Reply::Success(json!("arrived later"))).unwrap();
    });
    pump(&tasks);
    assert_eq!(*responses.borrow(), [None]);
    send();
    assert_eq!(responses.borrow().len(), 1);
    pump(&tasks);
    let responses = responses.borrow();
    assert_eq!(responses.len(), 2);
    assert_eq!(
        JsonMethodCodec::decode_envelope(responses[1].as_deref().unwrap()).unwrap(),
        json!("arrived later"),
    );
    REQUIRED.clear_method_call_handler(&inbound);
}
