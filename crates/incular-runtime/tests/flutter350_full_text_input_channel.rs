use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{TextInputClientId, TextInputCommand};
use incular_runtime::{FlutterTextInputEditingError, Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextRange, TextSelection};
use incular_widgets::{Column, EditableText, FocusNode, Widget};
use serde_json::{Value, json};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(180., 80.)))
        .unwrap();
}

fn attached(runtime: &mut Runtime) -> TextInputClientId {
    runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .unwrap()
}

fn field(controller: &TextEditingController, node: &FocusNode, autofocus: bool) -> EditableText {
    EditableText::new(controller.clone())
        .focus_node(node.clone())
        .autofocus(autofocus)
        .text_direction(TextDirection::Ltr)
        .size(Size::new(180., 40.))
}

fn update(sender: Value, state: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "method": "TextInputClient.updateEditingState",
        "args": [sender, state],
    }))
    .unwrap()
}

#[test]
fn invalid_utf16_ranges_are_reported_and_leave_the_mounted_editing_state_unchanged() {
    let node = FocusNode::new();
    let text = TextEditingController::with_text("test3");
    let mut runtime = Runtime::new(field(&text, &node, true).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    frame(&mut runtime);
    let client = attached(&mut runtime);
    assert!(node.has_primary_focus());

    let baseline = text.with_value(Clone::clone);
    let invalid = json!({"text":"1", "selectionBase":2, "selectionExtent":3});
    let error = runtime
        .handle_flutter_text_input_editing_call(&update(json!(client.get()), invalid.clone()))
        .unwrap_err();
    assert!(matches!(
        error,
        FlutterTextInputEditingError::OutOfRange {
            range: "selection",
            edge: "start",
            offset: 2,
            length: 1
        }
    ));
    assert!(
        error
            .to_string()
            .contains("Range start 2 is out of text of length 1")
    );
    assert!(
        error
            .to_string()
            .contains("range.start >= 0 && range.start <= text.length")
    );
    let debug_override =
        runtime.handle_flutter_text_input_editing_call(&update(json!(-1), invalid));
    if cfg!(debug_assertions) {
        assert!(matches!(
            debug_override,
            Err(FlutterTextInputEditingError::OutOfRange { .. })
        ));
    } else {
        assert_eq!(debug_override.unwrap(), Some(false));
    }

    for invalid in [
        json!({"text":"a😀z", "selectionBase":2, "selectionExtent":3}),
        json!({"text":"private-value", "selectionBase":-1, "selectionExtent":1}),
        json!({"text":"private-value", "composingBase":3, "composingExtent":2}),
        json!({"text":"private-value", "selectionAffinity":17}),
        json!({"text":"private-value", "selectionIsDirectional":"true"}),
    ] {
        let error = runtime
            .handle_flutter_text_input_editing_call(&update(json!(client.get()), invalid))
            .unwrap_err();
        assert!(!error.to_string().contains("private-value"));
        assert!(!error.to_string().contains("😀"));
    }
    frame(&mut runtime);
    assert_eq!(text.with_value(Clone::clone), baseline);
    assert!(node.has_primary_focus());
    assert!(
        !runtime
            .take_text_input_commands()
            .iter()
            .any(|cmd| matches!(cmd, TextInputCommand::Update { .. }))
    );
}

#[test]
fn reversed_selections_and_composition_convert_to_bytes_without_echoing_the_host_state() {
    let node = FocusNode::new();
    let controller = TextEditingController::with_text("test3");
    let mut runtime = Runtime::new(field(&controller, &node, true).into()).unwrap();
    frame(&mut runtime);
    let client = attached(&mut runtime);
    let state = json!({
        "text":"a😀z",
        "selectionBase":3, "selectionExtent":1,
        "selectionAffinity":"TextAffinity.upstream", "selectionIsDirectional":true,
        "composingBase":1, "composingExtent":3
    });
    assert_eq!(
        runtime
            .handle_flutter_text_input_editing_call(&update(json!(client.get()), state))
            .unwrap(),
        Some(true)
    );
    let actual = controller.with_value(Clone::clone);
    assert_eq!(actual.text, "a😀z");
    assert_eq!(actual.selection, TextSelection::new(5, 1));
    assert_eq!(actual.composing, Some(TextRange::new(1, 5)));
    assert_eq!(
        controller.caret_affinity(),
        incular_text::TextAffinity::Upstream
    );
    assert!(controller.selection_is_directional());
    frame(&mut runtime);
    assert!(
        !runtime
            .take_text_input_commands()
            .iter()
            .any(|cmd| matches!(cmd, TextInputCommand::Update { .. }))
    );

    assert_eq!(
        runtime
            .handle_flutter_text_input_editing_call(&update(
                json!(client.get()),
                json!({"text":"é"})
            ))
            .unwrap(),
        Some(true)
    );
    let actual = controller.with_value(Clone::clone);
    assert_eq!(actual.selection, TextSelection::collapsed(2));
    assert_eq!(actual.composing, None);
}

#[test]
fn stale_connections_are_rejected_before_their_state_is_decoded() {
    let left_text = TextEditingController::with_text("left");
    let right_text = TextEditingController::with_text("right");
    let left = FocusNode::new();
    let right = FocusNode::new();
    let scene = Column::new([
        Widget::from(field(&left_text, &left, true)),
        field(&right_text, &right, false).into(),
    ]);
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    let old = attached(&mut runtime);
    right.request_focus();
    frame(&mut runtime);
    let new = attached(&mut runtime);
    assert_ne!(old, new);
    assert_eq!(
        runtime
            .handle_flutter_text_input_editing_call(&update(json!(old.get()), Value::Null))
            .unwrap(),
        Some(false)
    );
    assert!(matches!(
        runtime.handle_flutter_text_input_editing_call(&update(json!(new.get()), Value::Null)),
        Err(FlutterTextInputEditingError::InvalidArguments)
    ));
    assert_eq!(
        runtime
            .handle_flutter_text_input_editing_call(
                br#"{"method":"TextInputClient.performAction","args":null}"#
            )
            .unwrap(),
        None
    );
    assert_eq!(left_text.text(), "left");
    assert_eq!(right_text.text(), "right");
    assert!(right.has_primary_focus());
}

#[test]
fn a_full_edit_only_trusts_affinity_when_the_text_or_selection_origin_changes() {
    use incular_text::TextAffinity;

    let focus = FocusNode::new();
    let controller = TextEditingController::with_text("first line");
    controller.set_selection(TextSelection::collapsed(1));
    let format_calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let _formatter = controller.register_edit_transform({
        let count = format_calls.clone();
        move |_, incoming| {
            count.set(count.get() + 1);
            incoming.clone()
        }
    });
    let mut runtime = Runtime::new(field(&controller, &focus, true).into()).unwrap();
    frame(&mut runtime);
    let client = attached(&mut runtime);
    let set = |runtime: &mut Runtime, text: &str, offset, affinity| {
        assert_eq!(
            runtime
                .handle_flutter_text_input_editing_call(&update(
                    json!(client.get()),
                    json!({
                        "text": text,
                        "selectionBase": offset,
                        "selectionExtent": offset,
                        "selectionAffinity": affinity,
                    }),
                ))
                .unwrap(),
            Some(true)
        );
    };

    set(&mut runtime, "first line", 2, "TextAffinity.upstream");
    assert_eq!(controller.selection(), TextSelection::collapsed(2));
    assert_eq!(controller.caret_affinity(), TextAffinity::Upstream);
    let revision = controller.revisions();
    set(&mut runtime, "first line", 2, "TextAffinity.downstream");
    assert_eq!(controller.caret_affinity(), TextAffinity::Upstream);
    assert_eq!(
        controller.revisions(),
        revision,
        "no redundant caret mutation"
    );
    assert_eq!(format_calls.get(), 0);
    set(&mut runtime, "First line", 2, "TextAffinity.downstream");
    assert_eq!(controller.text(), "First line");
    assert_eq!(format_calls.get(), 1);
    assert_eq!(controller.caret_affinity(), TextAffinity::Downstream);

    set(&mut runtime, "First line", 3, "TextAffinity.upstream");
    assert_eq!(controller.caret_affinity(), TextAffinity::Upstream);
    // Unrecognized string names follow TextEditingValue.fromJSON's default.
    set(&mut runtime, "First line", 4, "unknown");
    assert_eq!(controller.selection(), TextSelection::collapsed(4));
    assert_eq!(controller.caret_affinity(), TextAffinity::Downstream);
    frame(&mut runtime);
    assert!(
        !runtime
            .take_text_input_commands()
            .iter()
            .any(|command| matches!(command, TextInputCommand::Update { .. }))
    );
}

#[test]
fn physical_keys_and_native_typed_text_report_user_selection_changes() {
    use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey};
    use incular_text::SelectionChangedCause::Keyboard;
    use std::{cell::RefCell, rc::Rc};

    let focus = FocusNode::new();
    let controller = TextEditingController::with_text("abcdef");
    controller.set_selection(TextSelection::collapsed(2));
    let log = Rc::new(RefCell::new(Vec::new()));
    let editor = field(&controller, &focus, true).on_selection_changed({
        let log = log.clone();
        move |selection, cause| log.borrow_mut().push((selection, cause))
    });
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    frame(&mut runtime);
    assert!(focus.has_primary_focus());
    assert!(log.borrow().is_empty());
    let right = |runtime: &mut Runtime, modifiers| {
        let key = KeyboardKey::Named(NamedKey::ArrowRight);
        let _ = runtime.dispatch_keyboard_event(KeyboardEvent {
            modifiers,
            ..KeyboardEvent::key_down(key.clone(), Code::ArrowRight)
        });
        let _ = runtime.dispatch_keyboard_event(KeyboardEvent {
            modifiers,
            ..KeyboardEvent::key_up(key, Code::ArrowRight)
        });
    };

    right(&mut runtime, Modifiers::empty());
    assert_eq!(controller.selection(), TextSelection::collapsed(3));
    assert_eq!(*log.borrow(), [(TextSelection::collapsed(3), Keyboard)]);
    right(&mut runtime, Modifiers::SHIFT);
    assert_eq!(controller.selection(), TextSelection::new(3, 4));
    assert_eq!(
        *log.borrow(),
        [
            (TextSelection::collapsed(3), Keyboard),
            (TextSelection::new(3, 4), Keyboard),
        ],
    );

    let _ = runtime.handle_input(InputEvent::Text("X".into()));
    assert_eq!(controller.text(), "abcXef");
    assert_eq!(controller.selection(), TextSelection::collapsed(4));
    assert_eq!(
        log.borrow().last(),
        Some(&(TextSelection::collapsed(4), Keyboard))
    );
    controller.set_selection(TextSelection::collapsed(1));
    frame(&mut runtime);
    assert_eq!(log.borrow().len(), 3, "programmatic changes are silent");
}
