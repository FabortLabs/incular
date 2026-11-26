use incular_platform::{
    FlutterPlatformMessenger, HapticFeedback, HapticFeedbackError,
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

fn check(log: &Log, arguments: Value) {
    let recorded = log.take();
    let [(channel, payload)] = recorded.as_slice() else {
        panic!("expected exactly one platform request, got {recorded:?}");
    };
    assert_eq!(*channel, "flutter/platform");
    assert_eq!(
        JsonMethodCodec::decode_method_call(payload).unwrap(),
        MethodCall {
            method: "HapticFeedback.vibrate".to_owned(),
            arguments
        }
    );
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
