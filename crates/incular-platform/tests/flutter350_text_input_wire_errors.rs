use std::{
    convert::Infallible,
    future::{Future, ready},
    pin::pin,
    task::{Context, Poll, Waker},
};

use incular_config::TextDirection;
use incular_core::{Offset, Rect, Size};
use incular_platform::{
    FlutterPlatformChannelError, FlutterPlatformMessenger, FlutterSelectionRect,
    FlutterTextInputConfiguration, FlutterTextInputControlUpdate as Update,
    FlutterTextInputControls, FlutterTextInputWireCall,
    autofill::AutofillEditingValue,
    message_codecs::{JsonMethodCodec, JsonMethodCodecError, PlatformException},
};
use serde_json::Value;

struct Host {
    fail: &'static str,
    message: &'static str,
    called: Vec<String>,
}

impl FlutterPlatformMessenger for Host {
    type Error = Infallible;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        assert_eq!(channel, "flutter/textinput");
        let call = JsonMethodCodec::decode_method_call(&payload.unwrap()).unwrap();
        let response = if self.fail == call.method {
            JsonMethodCodec::encode_error_envelope(&PlatformException {
                code: "error".into(),
                message: Some(self.message.into()),
                details: Value::Null,
                stacktrace: None,
            })
        } else {
            JsonMethodCodec::encode_success_envelope(&Value::Null)
        };
        self.called.push(call.method);
        ready(Ok(Some(response.unwrap())))
    }
}

fn run(future: impl Future<Output = ()>) {
    assert!(matches!(
        pin!(future).poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(())
    ));
}

fn attachment_requests() -> Vec<FlutterTextInputWireCall> {
    let mut controls = FlutterTextInputControls::new(false);
    let client = controls.attach(FlutterTextInputConfiguration::default(), |_| {});
    let rect = Rect::from_origin_size(Offset::ZERO, Size::ZERO);
    let transform = std::array::from_fn(|index| if index % 5 == 0 { 1. } else { 0. });
    for update in [
        Update::Configuration(FlutterTextInputConfiguration::default()),
        Update::EditingState(AutofillEditingValue::new("test")),
        Update::Show,
        Update::EditableSizeAndTransform {
            size: Size::ZERO,
            transform,
        },
        Update::ComposingRect(rect),
        Update::CaretRect(rect),
        Update::SelectionRects(vec![FlutterSelectionRect {
            position: 0,
            bounds: rect,
            direction: TextDirection::Ltr,
        }]),
        Update::RequestAutofill,
    ] {
        assert!(controls.update(client, update).unwrap());
    }
    assert!(controls.close(client));
    assert!(controls.flush_pending_hide());
    controls.finish_autofill_context(true);
    controls.take_platform_calls()
}

fn assert_unified_reporter(method: &'static str, message: &'static str, context: &'static str) {
    let mut host = Host {
        fail: method,
        message,
        called: Vec::new(),
    };
    let mut reports = Vec::new();
    for call in attachment_requests() {
        run(call.send_reporting(&mut host, |error| reports.push(error)));
    }
    assert_eq!(
        host.called
            .iter()
            .filter(|call| call.as_str() == method)
            .count(),
        1
    );
    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.context, context);
    assert!(report.to_string().contains(context));
    let FlutterPlatformChannelError::Codec(JsonMethodCodecError::Platform(exception)) =
        &report.error
    else {
        panic!("did not preserve the platform exception");
    };
    assert_eq!(exception.message.as_deref(), Some(message));
}

macro_rules! case {
    ($name:ident, $method:literal, $message:literal, $context:literal) => {
        #[test]
        fn $name() {
            assert_unified_reporter($method, $message, $context);
        }
    };
}

case!(
    attach_reports_set_client_error,
    "TextInput.setClient",
    "Failed to set client",
    "while attaching the text input client"
);
case!(
    detach_reports_clear_client_error,
    "TextInput.clearClient",
    "Failed to clear client",
    "while detaching the text input client"
);
case!(
    configuration_reports_update_error,
    "TextInput.updateConfig",
    "Failed to update config",
    "while updating text input configuration"
);
case!(
    editing_state_reports_platform_error,
    "TextInput.setEditingState",
    "Failed to set editing state",
    "while setting text input editing state"
);
case!(
    show_reports_platform_error,
    "TextInput.show",
    "Failed to show",
    "while showing the text input client"
);
case!(
    hide_reports_platform_error,
    "TextInput.hide",
    "Failed to hide",
    "while hiding the text input client"
);
case!(
    size_and_transform_report_platform_error,
    "TextInput.setEditableSizeAndTransform",
    "Failed to set size and transform",
    "while setting text input size and transform"
);
case!(
    marked_text_rect_reports_platform_error,
    "TextInput.setMarkedTextRect",
    "Failed to set composing rect",
    "while setting text input composing rect"
);
case!(
    caret_rect_reports_platform_error,
    "TextInput.setCaretRect",
    "Failed to set caret rect",
    "while setting text input caret rect"
);
case!(
    selection_rects_report_platform_error,
    "TextInput.setSelectionRects",
    "Failed to set selection rects",
    "while setting text input selection rects"
);
case!(
    autofill_request_reports_platform_error,
    "TextInput.requestAutofill",
    "Failed to request autofill",
    "while requesting autofill"
);
case!(
    autofill_finish_reports_platform_error,
    "TextInput.finishAutofillContext",
    "Failed to finish autofill context",
    "while finishing autofill context"
);
