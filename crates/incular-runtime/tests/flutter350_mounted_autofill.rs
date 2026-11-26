use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_platform::{
    autofill::{AutofillEditingValue, AutofillMessageError, AutofillSelectionAffinity},
    message_codecs::{JsonMethodCodec, MethodCall},
};
use incular_runtime::{Runtime, Signal};
use incular_text::{TextAffinity, TextEditingController, TextSelection};
use incular_widgets::{
    AutofillGroup, Builder, Column, EditableText, FocusNode, TextDirection, Widget,
};
use serde_json::{Value, json};

fn limits() -> Constraints {
    Constraints::loose(Size::new(320.0, 400.0))
}

fn field(
    controller: &TextEditingController,
    focus: &FocusNode,
    hint: Option<&str>,
) -> EditableText {
    EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .focus_node(focus.clone())
        .autofill_hints(hint.map(|hint| [hint]))
}

fn set_client(runtime: &Runtime) -> Value {
    let opts = json!({"viewId": 5, "enableDeltaModel": true});
    let bytes = runtime
        .encode_flutter_scoped_autofill_client(opts.as_object().unwrap().clone())
        .unwrap()
        .expect("focused client has a group");
    let call = JsonMethodCodec::decode_method_call(&bytes).unwrap();
    assert_eq!(call.method, "TextInput.setClient");
    assert!(call.arguments[0].as_u64().unwrap() > 0);
    call.arguments[1].clone()
}

fn tag(config: &Value, hint: &str) -> String {
    config["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["autofill"]["hints"][0] == hint)
        .unwrap()["autofill"]["uniqueIdentifier"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn message(values: impl Iterator<Item = (String, Value)>) -> Vec<u8> {
    let values: serde_json::Map<_, _> = values.collect();
    JsonMethodCodec::encode_method_call(&MethodCall {
        method: "TextInputClient.updateEditingStateWithTag".into(),
        // Tagged updates aren't filtered on the native input-client number.
        arguments: json!([0, values]),
    })
    .unwrap()
}

#[test]
fn attachment_and_tagged_values_follow_mounted_scope_without_moving_focus() {
    let first = TextEditingController::with_text("first");
    let second = TextEditingController::with_text("second");
    let nested = TextEditingController::with_text("nested");
    let readonly = TextEditingController::with_text("read only");
    let first_focus = FocusNode::new();
    let second_focus = FocusNode::new();
    let nested_focus = FocusNode::new();
    let readonly_focus = FocusNode::new();
    let enabled = Signal::new(true);
    let edits = Rc::new(Cell::new(0));
    let mut runtime = Runtime::new(
        Builder::new({
            let first = first.clone();
            let second = second.clone();
            let nested = nested.clone();
            let readonly = readonly.clone();
            let first_focus = first_focus.clone();
            let second_focus = second_focus.clone();
            let nested_focus = nested_focus.clone();
            let readonly_focus = readonly_focus.clone();
            let enabled = enabled.clone();
            let edits = edits.clone();
            move |_| {
                let secondary = field(&second, &second_focus, enabled.get().then_some("username"))
                    .on_changed({
                        let edits = edits.clone();
                        move |_| edits.set(edits.get() + 1)
                    });
                let inner: Widget =
                    AutofillGroup::new(field(&nested, &nested_focus, Some("nested"))).into();
                AutofillGroup::new(Column::new([
                    field(&first, &first_focus, Some("email")).into(),
                    secondary.into(),
                    inner,
                    field(&readonly, &readonly_focus, Some("readonly"))
                        .read_only(true)
                        .into(),
                ]))
                .into()
            }
        })
        .into(),
    )
    .unwrap();

    runtime.run_frame(limits()).unwrap();
    assert!(
        runtime
            .encode_flutter_scoped_autofill_client(Default::default())
            .unwrap()
            .is_none()
    );
    nested_focus.request_focus();
    runtime.run_frame(limits()).unwrap();
    let nested_tag = tag(&set_client(&runtime), "nested");
    first_focus.request_focus();
    runtime.run_frame(limits()).unwrap();
    let cfg = set_client(&runtime);
    assert_eq!(cfg["enableDeltaModel"], true);
    assert_eq!(cfg["viewId"], 5);
    assert_eq!(cfg["fields"].as_array().unwrap().len(), 3);
    assert_eq!(
        cfg["fields"][1]["autofill"]["editingValue"]["text"],
        "second"
    );
    assert_eq!(
        cfg["fields"][1]["autofill"]["editingValue"]["selectionBase"],
        -1
    );
    let first_tag = tag(&cfg, "email");
    let second_tag = tag(&cfg, "username");
    let readonly_tag = tag(&cfg, "readonly");

    let edit = AutofillEditingValue {
        text: "A😃z".into(),
        selection: (4, 1),
        composing: (1, 3),
        selection_affinity: AutofillSelectionAffinity::Upstream,
        selection_is_directional: true,
    };
    let update = message(
        [
            (second_tag.clone(), edit.to_json().unwrap()),
            (nested_tag.clone(), json!({"text":"out of scope"})),
            (readonly_tag, json!({"text":"write denied"})),
        ]
        .into_iter(),
    );
    assert_eq!(runtime.handle_flutter_autofill_update(&update).unwrap(), 1);
    assert_eq!(second.text(), "A😃z");
    assert_eq!(second.selection(), TextSelection::new(6, 1));
    assert_eq!(second.composing(), None);
    assert_eq!(second.caret_affinity(), TextAffinity::Upstream);
    assert!(second.selection_is_directional());
    let roundtrip = set_client(&runtime);
    assert_eq!(
        roundtrip["fields"][1]["autofill"]["editingValue"]["selectionIsDirectional"],
        true
    );
    assert_eq!(nested.text(), "nested");
    assert_eq!(readonly.text(), "read only");
    assert_eq!(edits.get(), 1);
    assert!(first_focus.has_focus());
    assert!(!second_focus.has_focus());

    let malformed = message(
        [
            (first_tag, json!({"text":"must stay unchanged"})),
            (
                second_tag.clone(),
                json!({"text":"A😃z","selectionBase":2,"selectionExtent":2}),
            ),
        ]
        .into_iter(),
    );
    assert!(matches!(
        runtime.handle_flutter_autofill_update(&malformed),
        Err(AutofillMessageError::InvalidField("UTF-16 edit boundary"))
    ));
    assert_eq!(first.text(), "first");
    assert_eq!(second.text(), "A😃z");
    assert_eq!(edits.get(), 1);

    let reset = message([(second_tag.clone(), json!({"text":"new second"}))].into_iter());
    assert_eq!(runtime.handle_flutter_autofill_update(&reset).unwrap(), 1);
    assert_eq!(second.selection_if_set(), None);
    assert_eq!(edits.get(), 2);
    enabled.set(false);
    runtime.run_frame(limits()).unwrap();
    assert_eq!(runtime.handle_flutter_autofill_update(&reset).unwrap(), 0);
    assert_eq!(second.text(), "new second");
    assert_eq!(set_client(&runtime)["fields"].as_array().unwrap().len(), 2);
}

#[test]
fn a_tag_from_another_runtime_cannot_edit_the_same_local_client_id() {
    let start = |text: &str| {
        let controller = TextEditingController::with_text(text);
        let focus = FocusNode::new();
        let mut runtime =
            Runtime::new(AutofillGroup::new(field(&controller, &focus, Some("email"))).into())
                .unwrap();
        runtime.run_frame(limits()).unwrap();
        focus.request_focus();
        runtime.run_frame(limits()).unwrap();
        (runtime, controller)
    };
    let (one, first) = start("first");
    let (mut two, second) = start("second");
    let key_one = tag(&set_client(&one), "email");
    let key_two = tag(&set_client(&two), "email");
    assert_eq!(
        key_one.split_once(':').unwrap().1,
        key_two.split_once(':').unwrap().1
    );
    assert_ne!(key_one, key_two);
    let update = message([(key_one, json!({"text":"wrong runtime"}))].into_iter());
    assert_eq!(two.handle_flutter_autofill_update(&update).unwrap(), 0);
    assert_eq!(first.text(), "first");
    assert_eq!(second.text(), "second");
}
