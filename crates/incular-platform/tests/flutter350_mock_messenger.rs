use std::{
    cell::RefCell,
    future::{Future, ready},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_platform::{
    FlutterBinaryPayload, FlutterJsonMethodChannel, FlutterJsonMethodResult, FlutterMockHandler,
    FlutterMockMessenger, FlutterPlatformMessenger,
};
use serde_json::{Value, json};

fn finish<T>(future: impl Future<Output = T>) -> T {
    let mut cx = Context::from_waker(Waker::noop());
    match pin!(future).poll(&mut cx) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("this mock replies immediately"),
    }
}

#[test]
fn binary_mocks_can_be_checked_by_handler_identity_and_cleared() {
    const CHANNEL: &str = "test_channel";
    let mut messenger = FlutterMockMessenger::default();
    let mock = FlutterMockHandler::binary(|_| ready(None));
    let identical_behavior = FlutterMockHandler::binary(|_| ready(None));
    assert!(messenger.check_mock_message_handler(CHANNEL, None));
    assert!(!messenger.check_mock_message_handler(CHANNEL, Some(&mock)));
    messenger.set_mock_message_handler(CHANNEL, Some(mock.clone()));
    assert!(messenger.check_mock_message_handler(CHANNEL, Some(&mock)));
    assert!(!messenger.check_mock_message_handler(CHANNEL, Some(&identical_behavior)));
    assert!(!messenger.check_mock_message_handler(CHANNEL, None));
    messenger.set_mock_message_handler(CHANNEL, None);
    assert!(messenger.check_mock_message_handler(CHANNEL, None));
    assert!(!messenger.check_mock_message_handler(CHANNEL, Some(&mock)));
}

#[test]
fn a_binary_mock_observes_bytes_and_removing_it_restores_the_fallback() {
    let mut messenger = FlutterMockMessenger::default();
    let captured = Rc::new(RefCell::new(Vec::new()));
    let mock = FlutterMockHandler::binary({
        let captured = captured.clone();
        move |payload| {
            captured.borrow_mut().push(payload);
            ready(None)
        }
    });
    messenger.set_mock_message_handler("test1", Some(mock));
    let bytes = Some(vec![0xab, 0xcd]);
    assert_eq!(
        finish(messenger.send_platform_message("test1", bytes.clone())).unwrap(),
        None
    );
    assert_eq!(captured.borrow().as_slice(), &[bytes.clone()]);
    captured.borrow_mut().clear();
    messenger.set_mock_message_handler("test1", None);
    assert_eq!(
        finish(messenger.send_platform_message("test1", bytes)).unwrap(),
        None
    );
    assert!(captured.borrow().is_empty());
}

#[test]
fn a_json_method_handler_has_the_same_identity_when_it_decodes_real_calls() {
    let channel = FlutterJsonMethodChannel::new("test_handler");
    let mut messenger = FlutterMockMessenger::default();
    let mock = FlutterMockHandler::json_method(|call| {
        assert_eq!(call.method, "read");
        assert_eq!(call.arguments, json!({"input": 7}));
        ready(FlutterJsonMethodResult::Success(Value::Null))
    });
    assert!(messenger.check_mock_message_handler(channel.name(), None));
    assert!(!messenger.check_mock_message_handler(channel.name(), Some(&mock)));
    messenger.set_mock_message_handler(channel.name(), Some(mock.clone()));
    assert!(messenger.check_mock_message_handler(channel.name(), Some(&mock)));
    assert_eq!(
        finish(channel.invoke(&mut messenger, "read", json!({"input": 7}))).unwrap(),
        Value::Null
    );
}

#[test]
fn an_unmatched_message_reaches_the_delegate_without_collapsing_null_and_empty() {
    struct Echo;
    impl FlutterPlatformMessenger for Echo {
        type Error = std::convert::Infallible;
        async fn send_platform_message(
            &mut self,
            _: &'static str,
            payload: FlutterBinaryPayload,
        ) -> Result<FlutterBinaryPayload, Self::Error> {
            Ok(payload)
        }
    }
    let mut messenger = FlutterMockMessenger::new(Echo);
    messenger.set_mock_message_handler(
        "intercept",
        Some(FlutterMockHandler::binary(|_| ready(None))),
    );
    for payload in [None, Some(vec![]), Some(vec![7])] {
        assert_eq!(
            finish(messenger.send_platform_message("other", payload.clone())).unwrap(),
            payload
        );
    }
    assert_eq!(
        finish(messenger.send_platform_message("intercept", Some(vec![7]))).unwrap(),
        None
    );
}

#[test]
fn a_registered_mock_can_leave_the_channel_awaiting_its_async_reply() {
    use std::{cell::Cell, future::poll_fn};

    let release = Rc::new(Cell::new(false));
    let mock = FlutterMockHandler::binary({
        let release = release.clone();
        move |mut bytes| {
            let release = release.clone();
            poll_fn(move |_| {
                if release.get() {
                    Poll::Ready(bytes.take())
                } else {
                    Poll::Pending
                }
            })
        }
    });
    let mut messenger = FlutterMockMessenger::default();
    messenger.set_mock_message_handler("async", Some(mock));
    let future = messenger.send_platform_message("async", Some(vec![0, 7]));
    let mut pending = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(pending.as_mut().poll(&mut cx), Poll::Pending);
    release.set(true);
    assert_eq!(
        pending.as_mut().poll(&mut cx),
        Poll::Ready(Ok(Some(vec![0, 7])))
    );
}
