use std::{
    collections::hash_map::DefaultHasher,
    future::{Future, ready},
    hash::{Hash, Hasher},
    pin::pin,
    task::{Context, Poll, Waker},
};

use incular_config::TextDirection;
use incular_platform::{
    FlutterPlatformMessenger, FlutterTextInputAlignment as Align, FlutterTextInputStyle as Style,
    FlutterTextInputStyleSendError, FlutterTextInputWeight as Weight,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use serde_json::{Value, json};

fn style() -> Style {
    Style {
        font_family: Some("Roboto".into()),
        font_size: Some(16.),
        font_weight: Some(Weight::BOLD),
        text_direction: TextDirection::Ltr,
        text_align: Align::Center,
        letter_spacing: Some(1.2),
        word_spacing: Some(2.),
        line_height: Some(24.),
    }
}

fn hash(style: &Style) -> u64 {
    let mut state = DefaultHasher::new();
    style.hash(&mut state);
    state.finish()
}

#[test]
fn styles_compare_every_field_instead_of_only_the_font_family() {
    let a = style();
    let b = style();
    assert_eq!(a, b);
    assert_eq!(hash(&a), hash(&b));
    let mut other = style();
    other.font_family = Some("Other".into());
    other.letter_spacing = None;
    other.word_spacing = None;
    other.line_height = None;
    assert_ne!(a, other);
    assert_eq!(
        (
            a.font_family,
            a.font_size,
            a.font_weight,
            a.text_direction,
            a.text_align,
            a.letter_spacing,
            a.word_spacing,
            a.line_height
        ),
        (
            b.font_family,
            b.font_size,
            b.font_weight,
            b.text_direction,
            b.text_align,
            b.letter_spacing,
            b.word_spacing,
            b.line_height
        ),
    );
}

#[test]
fn equal_styles_hash_equally_including_signed_zero() {
    assert_eq!(hash(&style()), hash(&style()));
    let (mut a, mut b) = (style(), style());
    a.letter_spacing = Some(0.);
    b.letter_spacing = Some(-0.);
    assert_eq!(a, b);
    assert_eq!(hash(&a), hash(&b));
}

#[test]
fn a_complete_style_uses_the_exact_flutter_field_names_and_indices() {
    assert_eq!(
        style().to_json().unwrap(),
        json!({
            "fontFamily":"Roboto", "fontSize":16.0, "fontWeightIndex":6, "textAlignIndex":2,
            "textDirectionIndex":1, "letterSpacing":1.2, "wordSpacing":2.0, "lineHeight":24.0,
        })
    );
}

#[test]
fn an_unstyled_input_still_serializes_alignment_and_direction() {
    assert_eq!(
        Style::new(TextDirection::Ltr, Align::Left)
            .to_json()
            .unwrap(),
        json!({
            "fontFamily":null, "fontSize":null, "fontWeightIndex":null, "textAlignIndex":0,
            "textDirectionIndex":1, "letterSpacing":null, "wordSpacing":null, "lineHeight":null,
        })
    );
    assert_eq!(
        Style::new(TextDirection::Rtl, Align::Start)
            .to_json()
            .unwrap()["textDirectionIndex"],
        0,
    );
}

struct Host {
    calls: Vec<(&'static str, MethodCall<Value>)>,
    failure: Option<&'static str>,
}

impl FlutterPlatformMessenger for Host {
    type Error = std::convert::Infallible;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        self.calls.push((
            channel,
            JsonMethodCodec::decode_method_call(&payload.unwrap()).unwrap(),
        ));
        let envelope = match self.failure {
            None => JsonMethodCodec::encode_success_envelope(&Value::Null),
            Some(message) => JsonMethodCodec::encode_error_envelope(
                &incular_platform::message_codecs::PlatformException {
                    code: "error".into(),
                    message: Some(message.into()),
                    details: Value::Null,
                    stacktrace: None,
                },
            ),
        };
        ready(Ok(Some(envelope.unwrap())))
    }
}

fn run<T>(future: impl Future<Output = T>) -> T {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("host completes synchronously"),
    }
}

#[test]
fn only_valid_styles_are_sent_to_text_input_set_style() {
    let mut host = Host {
        calls: Vec::new(),
        failure: None,
    };
    let mut style = style();
    run(style.send(&mut host)).unwrap();
    assert_eq!(
        host.calls,
        vec![(
            "flutter/textinput",
            MethodCall {
                method: "TextInput.setStyle".into(),
                arguments: style.to_json().unwrap(),
            }
        )]
    );
    style.word_spacing = Some(f64::INFINITY);
    assert!(matches!(
        run(style.send(&mut host)),
        Err(FlutterTextInputStyleSendError::InvalidNumber(_))
    ));
    assert_eq!(host.calls.len(), 1);
}

#[test]
fn rejected_host_style_is_reported_with_its_platform_message_and_context() {
    let mut host = Host {
        calls: Vec::new(),
        failure: Some("Failed to set style"),
    };
    let mut errors = Vec::new();
    let mut style = Style::new(TextDirection::Ltr, Align::Left);
    style.font_family = Some("Roboto".into());
    style.font_size = Some(12.);
    style.font_weight = Some(Weight::NORMAL);
    run(style.send_reporting(&mut host, |error| errors.push(error)));
    assert_eq!(host.calls.len(), 1);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .to_string()
            .contains("while updating text input style")
    );
    let FlutterTextInputStyleSendError::Channel(
        incular_platform::FlutterPlatformChannelError::Codec(
            incular_platform::message_codecs::JsonMethodCodecError::Platform(error),
        ),
    ) = &errors[0]
    else {
        panic!("expected the host platform exception");
    };
    assert_eq!(error.message.as_deref(), Some("Failed to set style"));
}
