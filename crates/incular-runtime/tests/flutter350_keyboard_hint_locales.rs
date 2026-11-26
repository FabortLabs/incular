use incular_config::{Constraints, Locale};
use incular_core::Size;
use incular_platform::{TextInputCommand, message_codecs::JsonMethodCodec};
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{AutofillGroup, EditableText, FocusNode, TextDirection, Widget};
use serde_json::{Value, json};

fn editor(
    controller: &TextEditingController,
    focus: &FocusNode,
    hints: Option<Vec<Locale>>,
) -> EditableText {
    EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .focus_node(focus.clone())
        .hint_locales(hints)
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::loose(Size::new(320., 60.)))
        .unwrap();
}

fn attached_hints(commands: &[TextInputCommand]) -> Option<Vec<Locale>> {
    commands
        .iter()
        .find_map(|cmd| match cmd {
            TextInputCommand::SetClient { configuration, .. } => {
                Some(configuration.traits.hint_locales.clone())
            }
            _ => None,
        })
        .expect("the focused editor must attach")
}

#[test]
fn mounted_ime_receives_ordered_languages_and_clears_an_old_preference() {
    let controller = TextEditingController::new();
    let focus = FocusNode::new();
    let hints: Vec<Locale> = ["en", "fr"].map(|locale| locale.parse().unwrap()).into();
    let mut runtime =
        Runtime::new(editor(&controller, &focus, Some(hints.clone())).into()).unwrap();
    frame(&mut runtime);
    assert!(runtime.take_text_input_commands().is_empty());

    focus.request_focus();
    frame(&mut runtime);
    assert_eq!(
        attached_hints(&runtime.take_text_input_commands()),
        Some(hints)
    );
    let field = runtime.focused_element().unwrap();

    for value in [Some(Vec::new()), None] {
        runtime
            .schedule_update(field, editor(&controller, &focus, value.clone()).into())
            .unwrap();
        frame(&mut runtime);
        let commands = runtime.take_text_input_commands();
        assert_eq!(attached_hints(&commands), value);
        assert!(
            !commands
                .iter()
                .any(|cmd| matches!(cmd, TextInputCommand::Show { .. }))
        );
        assert_eq!(runtime.focused_element(), Some(field));
    }
}

#[test]
fn flutter_wire_distinguishes_language_tags_empty_and_null() {
    let listed = ["en", "zh-Hant-TW", "fr"];
    let hints: Vec<Locale> = listed.map(|locale| locale.parse().unwrap()).into();
    for (hints, expected) in [
        (Some(hints), json!(listed)),
        (Some(vec![]), json!([])),
        (None, Value::Null),
    ] {
        let focus = FocusNode::new();
        let field =
            editor(&TextEditingController::new(), &focus, hints).autofill_hints(Some(["username"]));
        let mut runtime = Runtime::new(Widget::from(AutofillGroup::new(field))).unwrap();
        frame(&mut runtime);
        focus.request_focus();
        frame(&mut runtime);
        let packet = runtime
            .encode_flutter_scoped_autofill_client(Default::default())
            .unwrap()
            .unwrap();
        let call = JsonMethodCodec::decode_method_call(&packet).unwrap();
        assert_eq!(call.method, "TextInput.setClient");
        assert_eq!(call.arguments[1].get("hintLocales"), Some(&expected));
        assert_eq!(
            call.arguments[1]["fields"][0].get("hintLocales"),
            Some(&expected)
        );
    }
}
