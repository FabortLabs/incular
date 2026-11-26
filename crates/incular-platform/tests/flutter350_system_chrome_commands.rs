use incular_platform::{
    ApplicationSwitcherDescription, DeviceOrientation, FlutterPlatformMessenger, SystemChrome,
    SystemUiChangeMessageError, SystemUiMode, SystemUiOverlay,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    future::{Future, ready},
    panic::{AssertUnwindSafe, catch_unwind},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

type CommandLog = Rc<RefCell<Vec<(&'static str, Vec<u8>)>>>;
struct Messenger {
    calls: CommandLog,
    replies: bool,
}

impl FlutterPlatformMessenger for Messenger {
    type Error = std::convert::Infallible;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        self.calls.borrow_mut().push((channel, payload));
        ready(Ok(self.replies.then(|| b"[null]".to_vec())))
    }
}

fn wait<T>(future: impl Future<Output = T>) -> T {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("test transport replies immediately"),
    }
}

fn fixture(replies: bool) -> (SystemChrome<Messenger>, CommandLog) {
    let log = CommandLog::default();
    let chrome = SystemChrome::new(Messenger {
        calls: log.clone(),
        replies,
    });
    (chrome, log)
}

fn check(log: &CommandLog, method: &str, arguments: Value) {
    let calls = log.take();
    let [(channel, payload)] = calls.as_slice() else {
        panic!("expected one platform call, got {calls:?}");
    };
    assert_eq!(*channel, "flutter/platform");
    assert_eq!(
        JsonMethodCodec::decode_method_call(payload).unwrap(),
        MethodCall {
            method: method.to_owned(),
            arguments
        }
    );
}

#[test]
fn preferred_orientations_preserve_the_ordered_list() {
    let (mut chrome, log) = fixture(true);
    wait(chrome.set_preferred_orientations(&[DeviceOrientation::PortraitUp])).unwrap();
    check(
        &log,
        "SystemChrome.setPreferredOrientations",
        json!(["DeviceOrientation.portraitUp"]),
    );
    wait(chrome.set_preferred_orientations(&[])).unwrap();
    check(&log, "SystemChrome.setPreferredOrientations", json!([]));
}

#[test]
fn task_switcher_metadata_is_sent_with_the_same_unsigned_argb_even_without_a_plugin() {
    for replies in [true, false] {
        let (mut chrome, log) = fixture(replies);
        wait(
            chrome.set_application_switcher_description(ApplicationSwitcherDescription {
                label: Some("Example label".into()),
                primary_color: Some(0xff00_ff00),
            }),
        )
        .unwrap();
        check(
            &log,
            "SystemChrome.setApplicationSwitcherDescription",
            json!({"label": "Example label", "primaryColor": 4278255360_u32}),
        );
    }
}

#[test]
fn preset_mode_and_manual_bar_selection_use_different_commands() {
    let (mut chrome, log) = fixture(true);
    wait(chrome.set_enabled_system_ui_mode(SystemUiMode::LeanBack, None)).unwrap();
    check(
        &log,
        "SystemChrome.setEnabledSystemUIMode",
        json!("SystemUiMode.leanBack"),
    );

    wait(chrome.set_enabled_system_ui_mode(SystemUiMode::Manual, Some(&[SystemUiOverlay::Top])))
        .unwrap();
    check(
        &log,
        "SystemChrome.setEnabledSystemUIOverlays",
        json!(["SystemUiOverlay.top"]),
    );

    wait(chrome.set_enabled_system_ui_mode(SystemUiMode::Manual, Some(&[]))).unwrap();
    check(&log, "SystemChrome.setEnabledSystemUIOverlays", json!([]));
    wait(chrome.restore_system_ui_overlays()).unwrap();
    check(&log, "SystemChrome.restoreSystemUIOverlays", Value::Null);
}

#[test]
fn manual_mode_without_an_overlay_list_is_rejected_before_sending() {
    let (mut chrome, log) = fixture(true);
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = wait(chrome.set_enabled_system_ui_mode(SystemUiMode::Manual, None));
    }))
    .expect_err("missing manual overlay list");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(message.contains("mode == SystemUiMode.manual && overlays != null"));
    assert!(log.borrow().is_empty());
}

#[test]
fn visibility_subscription_only_sends_a_request_when_installing_a_callback() {
    let (mut chrome, log) = fixture(true);
    wait(chrome.set_system_ui_change_callback(None)).unwrap();
    assert!(log.borrow().is_empty());

    let events = Rc::new(RefCell::new(Vec::new()));
    wait(chrome.set_system_ui_change_callback(Some(Box::new({
        let events = events.clone();
        move |visible| {
            let events = events.clone();
            Box::pin(async move { events.borrow_mut().push(visible) })
        }
    }))))
    .unwrap();
    check(&log, "SystemChrome.setSystemUIChangeListener", Value::Null);

    let visible = br#"{"method":"SystemChrome.systemUIChange","args":[true]}"#;
    let hidden = br#"{"method":"SystemChrome.systemUIChange","args":[false]}"#;
    assert_eq!(
        wait(chrome.handle_platform_message(visible))
            .unwrap()
            .unwrap(),
        b"[null]"
    );
    wait(chrome.handle_platform_message(hidden)).unwrap();
    assert_eq!(*events.borrow(), [true, false]);
    assert!(matches!(
        wait(
            chrome
                .handle_platform_message(br#"{"method":"SystemChrome.systemUIChange","args":[0]}"#)
        ),
        Err(SystemUiChangeMessageError::InvalidArguments)
    ));

    wait(chrome.set_system_ui_change_callback(None)).unwrap();
    wait(chrome.handle_platform_message(visible)).unwrap();
    assert_eq!(*events.borrow(), [true, false]);
    assert!(log.borrow().is_empty());
}
