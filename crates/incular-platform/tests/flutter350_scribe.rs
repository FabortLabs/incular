use incular_platform::{
    FlutterChannelMicrotask, FlutterInboundMessenger, FlutterPlatformMessenger, Scribe,
    ScribeError,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::VecDeque,
    future::{Future, ready},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

type Log = Rc<RefCell<Vec<(&'static str, MethodCall<Value>)>>>;

struct Host {
    calls: Log,
    reply: Value,
}

impl FlutterPlatformMessenger for Host {
    type Error = std::convert::Infallible;
    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        self.calls.borrow_mut().push((
            channel,
            JsonMethodCodec::decode_method_call(&payload.expect("method call bytes")).unwrap(),
        ));
        ready(Ok(Some(
            JsonMethodCodec::encode_success_envelope(&self.reply).unwrap(),
        )))
    }
}

fn wait<T>(future: impl Future<Output = T>) -> T {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("ready fixture"),
    }
}

fn service(reply: Value) -> (Scribe<Host>, Log) {
    let calls = Log::default();
    (
        Scribe::new(Host {
            calls: calls.clone(),
            reply,
        }),
        calls,
    )
}

fn check(log: &Log, method: &str) {
    assert_eq!(
        log.take(),
        [(
            "flutter/scribe",
            MethodCall {
                method: method.to_owned(),
                arguments: Value::Null,
            }
        )]
    );
}

#[test]
fn each_stylus_capability_keeps_false_true_and_null_distinct() {
    for (reply, expected) in [
        (json!(false), Some(false)),
        (json!(true), Some(true)),
        (Value::Null, None),
    ] {
        let (mut scribe, log) = service(reply);
        for (method, result) in [
            (
                "Scribe.isStylusHandwritingAvailable",
                wait(scribe.is_stylus_handwriting_available()),
            ),
            (
                "Scribe.isFeatureAvailable",
                wait(scribe.is_feature_available()),
            ),
        ] {
            if let Some(expected) = expected {
                assert_eq!(result.unwrap(), expected);
            } else {
                assert!(matches!(result, Err(ScribeError::MissingCapability)));
            }
            let recorded = log.borrow_mut().remove(0);
            assert_eq!(
                recorded,
                (
                    "flutter/scribe",
                    MethodCall {
                        method: method.to_owned(),
                        arguments: Value::Null,
                    }
                )
            );
        }
        assert!(log.borrow().is_empty());
    }
}

#[test]
fn starting_handwriting_makes_one_call_and_accepts_a_null_void_reply() {
    let (mut scribe, log) = service(Value::Null);
    wait(scribe.start_stylus_handwriting()).unwrap();
    check(&log, "Scribe.startStylusHandwriting");
}

#[test]
fn unsupported_platform_scribe_message_replies_null_and_does_not_replay() {
    let tasks = Rc::new(RefCell::new(VecDeque::<FlutterChannelMicrotask>::new()));
    let inbound = FlutterInboundMessenger::new({
        let tasks = tasks.clone();
        move |task| tasks.borrow_mut().push_back(task)
    });
    let channel = Scribe::<Host>::CHANNEL;
    let replies = Rc::new(RefCell::new(Vec::new()));
    // The Flutter original uses JSONMessageCodec with only a method key,
    // rather than a request encoded by JSONMethodCodec.
    let payload = serde_json::to_vec(&json!({"method": "Scribe.unsupportedMessage"})).unwrap();
    inbound.dispatch_unbuffered(channel, Some(payload), {
        let replies = replies.clone();
        move |reply| replies.borrow_mut().push(reply)
    });
    assert!(replies.borrow().is_empty());

    inbound.set_message_handler(channel, |_, _| panic!("unsupported call was buffered"));
    loop {
        let task = tasks.borrow_mut().pop_front();
        let Some(task) = task else { break };
        task();
    }
    assert_eq!(*replies.borrow(), [None]);
}
