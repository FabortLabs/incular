use incular_config::Constraints;
use incular_core::Size;
use incular_platform::{TextInputCommand, message_codecs::JsonMethodCodec};
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{EditableText, FocusNode, SmartDashesType, SmartQuotesType, TextDirection};
use serde_json::{Value, json};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::loose(Size::new(300., 70.)))
        .unwrap();
}

fn verify(
    obscure: bool,
    policy: Option<(SmartDashesType, SmartQuotesType)>,
    prediction: Option<bool>,
    expected_dashes: SmartDashesType,
    expected_quotes: SmartQuotesType,
) -> Value {
    let focus = FocusNode::new();
    let mut field = EditableText::new(TextEditingController::new())
        .text_direction(TextDirection::Ltr)
        .focus_node(focus.clone())
        .obscure_text(obscure)
        .enable_inline_prediction(prediction);
    if let Some((dashes, quotes)) = policy {
        field = field.smart_dashes_type(dashes).smart_quotes_type(quotes);
    }
    let mut runtime = Runtime::new(field.into()).unwrap();
    frame(&mut runtime);
    assert!(
        runtime
            .encode_flutter_text_input_client(Default::default())
            .unwrap()
            .is_none()
    );
    focus.request_focus();
    frame(&mut runtime);
    let traits = runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|cmd| match cmd {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.traits),
            _ => None,
        })
        .unwrap();
    assert_eq!(traits.smart_dashes_type, expected_dashes);
    assert_eq!(traits.smart_quotes_type, expected_quotes);
    assert_eq!(traits.enable_inline_prediction, prediction);

    let wire = runtime
        .encode_flutter_text_input_client(Default::default())
        .unwrap()
        .unwrap();
    let call = JsonMethodCodec::decode_method_call(&wire).unwrap();
    assert_eq!(call.method, "TextInput.setClient");
    let configuration = call.arguments[1].clone();
    assert!(
        configuration.get("fields").is_none(),
        "plain client has no scoped fields"
    );
    configuration
}

#[test]
fn smart_replacements_default_to_the_obscuring_policy_but_can_be_overridden_independently() {
    use SmartDashesType::{Disabled as DashOff, Enabled as DashOn};
    use SmartQuotesType::{Disabled as QuoteOff, Enabled as QuoteOn};

    for (obscure, policy, dash, quote, wire_dash, wire_quote) in [
        (false, None, DashOn, QuoteOn, "1", "1"),
        (true, None, DashOff, QuoteOff, "0", "0"),
        (
            false,
            Some((DashOff, QuoteOff)),
            DashOff,
            QuoteOff,
            "0",
            "0",
        ),
        (true, Some((DashOn, QuoteOff)), DashOn, QuoteOff, "1", "0"),
        (false, Some((DashOn, QuoteOff)), DashOn, QuoteOff, "1", "0"),
    ] {
        let result = verify(obscure, policy, None, dash, quote);
        assert_eq!(result["smartDashesType"], wire_dash);
        assert_eq!(result["smartQuotesType"], wire_quote);
    }
}

#[test]
fn inline_prediction_distinguishes_on_off_and_unspecified_in_the_method_message() {
    for (configured, expected) in [
        (Some(true), json!(true)),
        (Some(false), json!(false)),
        (None, Value::Null),
    ] {
        let result = verify(
            false,
            None,
            configured,
            SmartDashesType::Enabled,
            SmartQuotesType::Enabled,
        );
        assert_eq!(result.get("enableInlinePrediction"), Some(&expected));
    }
}
