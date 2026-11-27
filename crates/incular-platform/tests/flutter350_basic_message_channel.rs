use std::{
    cell::RefCell,
    collections::VecDeque,
    future::{Future, ready},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_platform::{
    FlutterBasicMessageChannel, FlutterBasicMessageError, FlutterBinaryPayload,
    FlutterChannelMicrotask, FlutterInboundMessenger, FlutterPlatformMessenger,
    message_codecs::{
        BinaryCodec, JsonMessageCodec, MessageValue, StandardMessageCodec, StringCodec,
    },
};
use serde_json::json;

const STRING: FlutterBasicMessageChannel<StringCodec> = FlutterBasicMessageChannel::new("ch");

fn finish<T>(future: impl Future<Output = T>) -> T {
    let mut cx = Context::from_waker(Waker::noop());
    match pin!(future).poll(&mut cx) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("this transport has a ready reply"),
    }
}

struct Messenger<F>(F);

impl<F: FnMut(&str, FlutterBinaryPayload) -> FlutterBinaryPayload> FlutterPlatformMessenger
    for Messenger<F>
{
    type Error = &'static str;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: FlutterBinaryPayload,
    ) -> impl Future<Output = Result<FlutterBinaryPayload, Self::Error>> {
        ready(Ok((self.0)(channel, payload)))
    }
}

#[test]
fn send_string_and_decode_peer_reply_without_a_method_envelope() {
    let mut transport = Messenger(|name: &str, payload: FlutterBinaryPayload| {
        assert_eq!(name, "ch");
        assert_eq!(payload.as_deref(), Some(b"hello".as_slice()));
        let raw = payload.unwrap();
        let message = std::str::from_utf8(&raw).unwrap();
        Some(format!("{message} world").into_bytes())
    });
    assert_eq!(
        finish(STRING.send(&mut transport, Some(String::from("hello")))).unwrap(),
        Some(String::from("hello world"))
    );
}

#[test]
fn receive_string_and_reply_as_unwrapped_utf8_in_a_later_turn() {
    let queue = Rc::new(RefCell::new(VecDeque::<FlutterChannelMicrotask>::new()));
    let inbound = FlutterInboundMessenger::new({
        let queue = queue.clone();
        move |task| queue.borrow_mut().push_back(task)
    });
    STRING.set_message_handler(&inbound, {
        let queue = queue.clone();
        move |message, reply| {
            assert_eq!(message.as_deref(), Some("hello"));
            queue.borrow_mut().push_back(Box::new(move || {
                reply(Some(format!("{} world", message.unwrap()))).unwrap();
            }));
        }
    });

    let received = Rc::new(RefCell::new(None));
    inbound.push("ch", Some(b"hello".to_vec()), {
        let received = received.clone();
        move |payload| assert!(received.replace(Some(payload)).is_none())
    });
    assert!(received.borrow().is_none());
    let first = queue.borrow_mut().pop_front().unwrap();
    first();
    assert!(received.borrow().is_none());
    let second = queue.borrow_mut().pop_front().unwrap();
    second();
    assert_eq!(received.take(), Some(Some(b"hello world".to_vec())));
    let stop = queue.borrow_mut().pop_front().unwrap();
    stop();
    assert!(queue.borrow().is_empty());
}

#[test]
fn null_empty_invalid_utf8_and_nested_codec_values_remain_distinct() {
    let mut string = Messenger(|_: &str, payload: FlutterBinaryPayload| payload);
    assert_eq!(finish(STRING.send(&mut string, None)).unwrap(), None);
    assert_eq!(
        finish(STRING.send(&mut string, Some(String::new()))).unwrap(),
        Some(String::new())
    );
    let mut invalid = Messenger(|_: &str, _: FlutterBinaryPayload| Some(vec![0xff]));
    assert!(matches!(
        finish(STRING.send(&mut invalid, Some("test".into()))),
        Err(FlutterBasicMessageError::Codec(_))
    ));

    let mut echo = Messenger(|_: &str, payload: FlutterBinaryPayload| payload);
    let binary = FlutterBasicMessageChannel::<BinaryCodec>::new("binary");
    assert_eq!(finish(binary.send(&mut echo, None)).unwrap(), None);
    assert_eq!(
        finish(binary.send(&mut echo, Some(vec![]))).unwrap(),
        Some(vec![])
    );
    let json_channel = FlutterBasicMessageChannel::<JsonMessageCodec>::new("json");
    let json_value = json!({"nested": [false, null, "🦀"]});
    assert_eq!(
        finish(json_channel.send(&mut echo, json_value.clone())).unwrap(),
        json_value
    );
    let standard = FlutterBasicMessageChannel::<StandardMessageCodec>::new("standard");
    let values = MessageValue::List(vec![MessageValue::Null, MessageValue::Int(42)]);
    assert_eq!(
        finish(standard.send(&mut echo, values.clone())).unwrap(),
        values
    );
}
