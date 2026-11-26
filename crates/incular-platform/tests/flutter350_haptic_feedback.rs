use incular_platform::{
    FlutterPlatformMessenger, HapticFeedback, HapticFeedbackError, RouteInformationUpdate,
    SystemNavigator, SystemSound, SystemSoundType,
    message_codecs::{JsonMethodCodec, JsonMethodCodecError, MethodCall},
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    future::{Future, ready},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

type Log = Rc<RefCell<Vec<(&'static str, Vec<u8>)>>>;

struct Messenger {
    calls: Log,
    reply: Result<Option<Vec<u8>>, &'static str>,
}

impl FlutterPlatformMessenger for Messenger {
    type Error = &'static str;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        self.calls.borrow_mut().push((channel, payload));
        ready(self.reply.clone())
    }
}

fn finish<T>(future: impl Future<Output = T>) -> T {
    let mut cx = Context::from_waker(Waker::noop());
    match pin!(future).poll(&mut cx) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("this test messenger has a ready reply"),
    }
}

fn service(reply: Result<Option<Vec<u8>>, &'static str>) -> (HapticFeedback<Messenger>, Log) {
    let log = Log::default();
    (
        HapticFeedback::new(Messenger {
            calls: log.clone(),
            reply,
        }),
        log,
    )
}

fn check_command(log: &Log, expected_channel: &str, method: &str, arguments: Value) {
    let recorded = log.take();
    let [(channel, payload)] = recorded.as_slice() else {
        panic!("expected exactly one platform request, got {recorded:?}");
    };
    assert_eq!(*channel, expected_channel);
    assert_eq!(
        JsonMethodCodec::decode_method_call(payload).unwrap(),
        MethodCall {
            method: method.to_owned(),
            arguments
        }
    );
}

fn check(log: &Log, arguments: Value) {
    check_command(log, "flutter/platform", "HapticFeedback.vibrate", arguments);
}

#[test]
fn regular_vibration_emits_one_platform_method_with_null_arguments() {
    let (mut haptic, log) = service(Ok(Some(b"[null]".to_vec())));
    finish(haptic.vibrate()).unwrap();
    check(&log, Value::Null);
}

#[test]
fn all_seven_vibration_variants_put_the_exact_effect_on_the_platform_channel() {
    let (mut haptic, log) = service(Ok(Some(b"[null]".to_vec())));
    finish(haptic.light_impact()).unwrap();
    check(&log, json!("HapticFeedbackType.lightImpact"));
    finish(haptic.medium_impact()).unwrap();
    check(&log, json!("HapticFeedbackType.mediumImpact"));
    finish(haptic.heavy_impact()).unwrap();
    check(&log, json!("HapticFeedbackType.heavyImpact"));
    finish(haptic.selection_click()).unwrap();
    check(&log, json!("HapticFeedbackType.selectionClick"));
    finish(haptic.success_notification()).unwrap();
    check(&log, json!("HapticFeedbackType.successNotification"));
    finish(haptic.warning_notification()).unwrap();
    check(&log, json!("HapticFeedbackType.warningNotification"));
    finish(haptic.error_notification()).unwrap();
    check(&log, json!("HapticFeedbackType.errorNotification"));
}

#[test]
fn an_unhandled_optional_channel_is_a_noop_but_real_errors_propagate() {
    let (mut unbound, log) = service(Ok(None));
    finish(unbound.vibrate()).unwrap();
    check(&log, Value::Null);

    let (mut broken, _) = service(Err("host detached"));
    assert!(matches!(
        finish(broken.vibrate()),
        Err(HapticFeedbackError::Transport("host detached"))
    ));
    let (mut denied, _) = service(Ok(Some(br#"["denied","Haptics disabled",null]"#.to_vec())));
    assert!(matches!(
        finish(denied.vibrate()),
        Err(HapticFeedbackError::Codec(JsonMethodCodecError::Platform(detail)))
            if detail.code == "denied"
    ));
}

#[test]
fn system_navigator_pop_uses_the_platform_channel_with_an_optional_animation_flag() {
    let log = Log::default();
    let mut navigator = SystemNavigator::new(Messenger {
        calls: log.clone(),
        reply: Ok(Some(b"[null]".to_vec())),
    });
    finish(navigator.pop(None)).unwrap();
    check_command(&log, "flutter/platform", "SystemNavigator.pop", Value::Null);

    finish(navigator.pop(Some(false))).unwrap();
    check_command(
        &log,
        "flutter/platform",
        "SystemNavigator.pop",
        json!(false),
    );
}

#[test]
fn history_mode_and_route_state_use_the_navigation_channel() {
    let log = Log::default();
    let mut navigator = SystemNavigator::new(Messenger {
        calls: log.clone(),
        reply: Ok(Some(b"[null]".to_vec())),
    });
    finish(navigator.select_single_entry_history()).unwrap();
    check_command(
        &log,
        "flutter/navigation",
        "selectSingleEntryHistory",
        Value::Null,
    );
    finish(navigator.select_multi_entry_history()).unwrap();
    check_command(
        &log,
        "flutter/navigation",
        "selectMultiEntryHistory",
        Value::Null,
    );

    for update in [
        RouteInformationUpdate::default(),
        RouteInformationUpdate {
            state: json!(true),
            replace: false,
        },
        RouteInformationUpdate {
            state: json!(true),
            replace: true,
        },
    ] {
        let expected = json!({ "uri": "a", "state": update.state, "replace": update.replace });
        finish(navigator.route_information_updated("a", update)).unwrap();
        check_command(
            &log,
            "flutter/navigation",
            "routeInformationUpdated",
            expected,
        );
    }
}

#[test]
fn system_sound_sends_a_single_method_call_with_the_sound_name() {
    let log = Log::default();
    let mut messenger = Messenger {
        calls: log.clone(),
        reply: Ok(Some(b"[null]".to_vec())),
    };
    for (sound, name) in [
        (SystemSoundType::Click, "SystemSoundType.click"),
        (SystemSoundType::Tick, "SystemSoundType.tick"),
        (SystemSoundType::Alert, "SystemSoundType.alert"),
    ] {
        finish(SystemSound::play_on_flutter_channel(&mut messenger, sound)).unwrap();
        check_command(&log, "flutter/platform", "SystemSound.play", json!(name));
    }
}
