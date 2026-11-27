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
        payload: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        self.calls
            .borrow_mut()
            .push((channel, payload.expect("method call bytes")));
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

#[test]
fn deferred_overlay_style_sends_one_exact_nullable_payload_and_only_the_last_style() {
    use incular_platform::SystemUiOverlayStyle as Style;
    let (mut chrome, log) = fixture(true);
    assert!(chrome.queue_system_ui_overlay_style(Style::DARK));
    assert!(wait(chrome.flush_system_ui_overlay_style()).unwrap());
    log.borrow_mut().clear();

    assert!(chrome.queue_system_ui_overlay_style(Style::LIGHT));
    assert!(log.borrow().is_empty());
    assert!(wait(chrome.flush_system_ui_overlay_style()).unwrap());
    check(
        &log,
        "SystemChrome.setSystemUIOverlayStyle",
        json!({
            "systemNavigationBarColor": 4278190080_u32,
            "systemNavigationBarDividerColor": null,
            "systemStatusBarContrastEnforced": null,
            "statusBarColor": null,
            "statusBarBrightness": "Brightness.dark",
            "statusBarIconBrightness": "Brightness.light",
            "systemNavigationBarIconBrightness": "Brightness.light",
            "systemNavigationBarContrastEnforced": null,
        }),
    );
    assert!(!chrome.queue_system_ui_overlay_style(Style::LIGHT));
    assert!(!wait(chrome.flush_system_ui_overlay_style()).unwrap());
    assert!(log.borrow().is_empty());

    let contrast = Style {
        system_status_bar_contrast_enforced: Some(false),
        system_navigation_bar_contrast_enforced: Some(true),
        ..Style::default()
    };
    assert!(chrome.queue_system_ui_overlay_style(contrast));
    assert!(wait(chrome.flush_system_ui_overlay_style()).unwrap());
    check(
        &log,
        "SystemChrome.setSystemUIOverlayStyle",
        json!({
            "systemNavigationBarColor": null,
            "systemNavigationBarDividerColor": null,
            "systemStatusBarContrastEnforced": false,
            "statusBarColor": null,
            "statusBarBrightness": null,
            "statusBarIconBrightness": null,
            "systemNavigationBarIconBrightness": null,
            "systemNavigationBarContrastEnforced": true,
        }),
    );

    assert!(chrome.queue_system_ui_overlay_style(Style::DARK));
    assert!(!chrome.queue_system_ui_overlay_style(contrast));
    assert!(!wait(chrome.flush_system_ui_overlay_style()).unwrap());
    assert!(log.borrow().is_empty());
}

#[test]
fn a_platform_overlay_failure_is_reported_once_with_exception_and_context() {
    use incular_platform::{
        FlutterPlatformChannelError, SystemUiOverlayStyle as Style,
        message_codecs::{JsonMethodCodecError, PlatformException},
    };
    const MESSAGE: &str = "Failed to set system UI overlay style";
    struct FailingMessenger {
        calls: usize,
        reply_ready: Rc<std::cell::Cell<bool>>,
    }

    impl FlutterPlatformMessenger for FailingMessenger {
        type Error = std::convert::Infallible;

        fn send_platform_message(
            &mut self,
            channel: &'static str,
            payload: Option<Vec<u8>>,
        ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
            assert_eq!(channel, "flutter/platform");
            let method = JsonMethodCodec::decode_method_call(&payload.unwrap()).unwrap();
            assert_eq!(method.method, "SystemChrome.setSystemUIOverlayStyle");
            self.calls += 1;
            assert_eq!(self.calls, 1);
            let reply = JsonMethodCodec::encode_error_envelope(&PlatformException {
                code: "error".into(),
                message: Some(MESSAGE.into()),
                details: Value::Null,
                stacktrace: None,
            })
            .unwrap();
            let reply_ready = self.reply_ready.clone();
            let mut reply = Some(reply);
            std::future::poll_fn(move |_| {
                if reply_ready.get() {
                    Poll::Ready(Ok(reply.take()))
                } else {
                    Poll::Pending
                }
            })
        }
    }

    let errors = Rc::new(RefCell::new(Vec::new()));
    let reply_ready = Rc::new(std::cell::Cell::new(false));
    let mut chrome = SystemChrome::new(FailingMessenger {
        calls: 0,
        reply_ready: reply_ready.clone(),
    });
    chrome.set_overlay_style_error_callback(Some(Box::new({
        let errors = errors.clone();
        move |report| {
            let FlutterPlatformChannelError::Codec(JsonMethodCodecError::Platform(error)) =
                &report.source
            else {
                panic!("expected typed PlatformException");
            };
            errors
                .borrow_mut()
                .push((report.context(), error.message.clone()));
        }
    })));
    assert!(chrome.queue_system_ui_overlay_style(Style::LIGHT));
    assert!(errors.borrow().is_empty());
    let report = {
        let mut delivery = pin!(chrome.flush_system_ui_overlay_style());
        let mut context = Context::from_waker(Waker::noop());
        assert!(delivery.as_mut().poll(&mut context).is_pending());
        assert!(errors.borrow().is_empty());
        reply_ready.set(true);
        let Poll::Ready(Err(report)) = delivery.as_mut().poll(&mut context) else {
            panic!("expected the rejected platform method reply");
        };
        report
    };
    assert!(report.to_string().contains(MESSAGE));
    assert_eq!(
        *errors.borrow(),
        [(
            "while setting the system UI overlay style",
            Some(MESSAGE.into())
        )]
    );
    assert!(!chrome.queue_system_ui_overlay_style(Style::LIGHT));
    assert!(!wait(chrome.flush_system_ui_overlay_style()).unwrap());
    assert_eq!(errors.borrow().len(), 1);
}
