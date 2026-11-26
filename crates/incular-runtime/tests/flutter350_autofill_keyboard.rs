use incular_config::Constraints;
use incular_core::Size;
use incular_platform::{
    TextInputCommand, TextInputType,
    autofill::{AutofillKeyboardPlatform, infer_autofill_keyboard},
};
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{EditableText, FocusNode, TextDirection};

fn attach(
    hints: Option<Vec<&str>>,
    explicit: Option<TextInputType>,
    multiline: bool,
) -> TextInputType {
    let controller = TextEditingController::new();
    let node = FocusNode::new();
    let mut editor = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .focus_node(node.clone())
        .autofill_hints(hints)
        .multiline(multiline);
    if let Some(explicit) = explicit {
        editor = editor.input_type(explicit);
    }
    let bounds = Constraints::loose(Size::new(300.0, 150.0));
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.run_frame(bounds).unwrap();
    node.request_focus();
    runtime.run_frame(bounds).unwrap();
    let commands = runtime.take_text_input_commands();
    let input_type = commands
        .iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.input_type),
            _ => None,
        })
        .expect("the field was attached");
    controller.set_text("test");
    runtime.run_frame(bounds).unwrap();
    assert!(runtime.take_text_input_commands().iter().any(|command| {
        matches!(command, TextInputCommand::Update { state, .. } if state.text == "test")
    }));
    input_type
}

#[test]
fn native_keyboard_uses_only_the_first_hint_and_explicit_type_wins() {
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    assert_eq!(
        attach(Some(vec!["streetAddressLine1"]), None, false).flutter_name(),
        "TextInputType.address"
    );
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    assert_eq!(
        attach(Some(vec!["streetAddressLine1"]), None, false).flutter_name(),
        "TextInputType.name"
    );
    assert_eq!(
        attach(
            Some(vec!["streetAddressLine1"]),
            Some(TextInputType::Text),
            false
        )
        .flutter_name(),
        "TextInputType.text"
    );
    assert_eq!(
        attach(Some(vec!["emailOTPCode"]), None, false).flutter_name(),
        "TextInputType.text"
    );
    assert_eq!(
        attach(Some(vec!["unrecognizedHint", "email"]), None, false),
        TextInputType::Text
    );
    assert_eq!(
        attach(Some(vec!["email", "unrecognizedHint"]), None, false),
        TextInputType::Email
    );
    assert_eq!(attach(Some(vec![]), None, true), TextInputType::Multiline);
    assert_eq!(
        attach(None, Some(TextInputType::Text), true),
        TextInputType::Text
    );
}

#[test]
fn apple_priority_and_other_keyboard_families_follow_the_pinned_tables() {
    use AutofillKeyboardPlatform::{Apple, Other};
    use TextInputType::{DateTime, Email, Multiline, Name, Phone, StreetAddress, Text};
    for (platform, hint, multiline, expected) in [
        (Apple, Some("streetAddressLine1"), false, Name),
        (Apple, Some("streetAddressLine1"), true, Name),
        (Other, Some("streetAddressLine1"), true, Multiline),
        (Other, Some("streetAddressLine1"), false, StreetAddress),
        (Apple, Some("emailOTPCode"), false, Text),
        (Other, Some("emailOTPCode"), false, Text),
        (Apple, Some("oneTimeCode"), false, TextInputType::NUMBER),
        (Other, Some("oneTimeCode"), false, Text),
        (
            Other,
            Some("transactionAmount"),
            false,
            TextInputType::number_with_options(false, true),
        ),
        (Apple, Some("email"), true, Email),
        (Other, Some("email"), true, Multiline),
        (Other, Some("creditCardExpirationMonth"), false, DateTime),
        (Apple, Some("telephoneNumber"), false, Name),
        (Other, Some("telephoneNumber"), false, Phone),
        (Apple, None, false, Text),
        (Apple, None, true, Multiline),
        (Apple, Some("unrecognizedHint"), true, Multiline),
    ] {
        assert_eq!(
            infer_autofill_keyboard(platform, hint, multiline),
            expected,
            "{platform:?} {hint:?}"
        );
    }
}
