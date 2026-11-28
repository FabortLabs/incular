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

#[test]
fn live_interactive_selection_updates_the_existing_keyboard_client_without_showing_it_again() {
    let focus = FocusNode::new();
    let controller = TextEditingController::with_text("test");
    controller.set_selection(TextSelection::collapsed(2));
    let build = |enabled| {
        field(&controller, &focus, true)
            .max_lines(None)
            .enable_interactive_selection(enabled)
    };
    let mut runtime = Runtime::new(build(true).into()).unwrap();
    frame(&mut runtime);
    let attached = |commands: Vec<TextInputCommand>| {
        commands
            .into_iter()
            .find_map(|command| match command {
                TextInputCommand::SetClient {
                    configuration,
                    state,
                    ..
                } => Some((configuration, state)),
                _ => None,
            })
            .unwrap()
    };
    let (initial, value) = attached(runtime.take_text_input_commands());
    assert!(initial.traits.enable_interactive_selection);
    assert!(initial.multiline);
    let field = runtime.focused_element().unwrap();

    runtime.schedule_update(field, build(false).into()).unwrap();
    frame(&mut runtime);
    let commands = runtime.take_text_input_commands();
    assert!(!commands.iter().any(|command| matches!(
        command,
        TextInputCommand::Show { .. }
            | TextInputCommand::Hide { .. }
            | TextInputCommand::Clear { .. }
    )));
    let (updated, latest_value) = attached(commands);
    assert_eq!(updated.client, initial.client);
    assert!(!updated.traits.enable_interactive_selection);
    assert_eq!(latest_value, value);
    assert_eq!(runtime.focused_element(), Some(field));
    assert!(focus.has_primary_focus());

    frame(&mut runtime);
    assert!(runtime.take_text_input_commands().is_empty());
}

#[test]
fn a_platform_text_edit_notifies_selection_before_on_changed() {
    use incular_core::InputEvent;
    use incular_text::SelectionChangedCause;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    let focus = FocusNode::new();
    let controller = TextEditingController::with_text("hi");
    controller.set_selection(TextSelection::collapsed(2));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let restore = Rc::new(Cell::new(false));
    let editor = field(&controller, &focus, true)
        .on_selection_changed({
            let calls = calls.clone();
            let restore = restore.clone();
            let controller = controller.clone();
            move |selection, cause| {
                assert_eq!(cause, SelectionChangedCause::Keyboard);
                calls
                    .borrow_mut()
                    .push(format!("selection:{}", selection.extent));
                if restore.get() {
                    controller.set_value(
                        incular_text::TextEditingValue::new("hello")
                            .with_selection(TextSelection::collapsed(5)),
                    );
                }
            }
        })
        .on_changed({
            let calls = calls.clone();
            move |text| calls.borrow_mut().push(format!("text:{text}"))
        });
    let mut runtime = Runtime::new(editor.into()).unwrap();
    frame(&mut runtime);
    let client = attached(&mut runtime);
    assert!(calls.borrow().is_empty());

    let message = update(
        json!(client.get()),
        json!({"text":"hello","selectionBase":5,"selectionExtent":5}),
    );
    assert_eq!(
        runtime
            .handle_flutter_text_input_editing_call(&message)
            .unwrap(),
        Some(true)
    );
    assert_eq!(calls.borrow().as_slice(), &["selection:5", "text:hello"]);

    calls.borrow_mut().clear();
    let _ = runtime.handle_input(InputEvent::Text("!".into()));
    assert_eq!(calls.borrow().as_slice(), &["selection:6", "text:hello!"]);

    controller.set_value(
        incular_text::TextEditingValue::new("hello").with_selection(TextSelection::collapsed(5)),
    );
    restore.set(true);
    calls.borrow_mut().clear();
    let message = update(
        json!(client.get()),
        json!({"text":"hello!","selectionBase":6,"selectionExtent":6}),
    );
    assert_eq!(
        runtime
            .handle_flutter_text_input_editing_call(&message)
            .unwrap(),
        Some(true)
    );
    assert_eq!(controller.text(), "hello");
    assert_eq!(calls.borrow().as_slice(), &["selection:6"]);
    frame(&mut runtime);
    assert!(
        runtime
            .take_text_input_commands()
            .iter()
            .any(|command| matches!(
                command, TextInputCommand::Update { state, .. } if state.text == "hello"
            ))
    );
}

#[derive(Clone, Copy)]
enum EditOrigin {
    Host,
    UserReplace,
    Controller,
}

fn callback_edits_publish_one_final_value(origin: EditOrigin) {
    use incular_platform::{FlutterTextInputConfiguration, FlutterTextInputControls};
    use incular_text::{SelectionChangedCause, TextEditingValue};
    use incular_widgets::LengthLimitingTextInputFormatter;
    use std::{cell::Cell, rc::Rc};

    let controller = TextEditingController::with_text("initial");
    let focus = FocusNode::new();
    let listener = controller.clone();
    let _listener = controller.observe(move |value| {
        if !value.text.ends_with("listener") {
            listener.set_value(TextEditingValue::new(format!("{} listener", value.text)));
        }
    });
    let on_changed_calls = Rc::new(Cell::new(0));
    let changed = on_changed_calls.clone();
    let editable = field(&controller, &focus, true)
        .max_lines(Some(2))
        .input_formatter(LengthLimitingTextInputFormatter::new(6))
        .on_changed({
            let controller = controller.clone();
            move |_| {
                changed.set(changed.get() + 1);
                controller.set_value(TextEditingValue::new(format!(
                    "{} onChanged",
                    controller.text()
                )));
            }
        });
    let mut runtime = Runtime::new(editable.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    frame(&mut runtime);
    let client = attached(&mut runtime);
    let target = runtime.focused_element().unwrap();
    let mut controls = FlutterTextInputControls::new(false);
    assert!(controls.attach_existing(client, FlutterTextInputConfiguration::default(), |_| {}));
    assert!(
        runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    let _ = controls.take_platform_calls();

    let incoming = "remoteremoteremote";
    let wire_value = |text: &str| {
        json!({
            "text": text,
            "selectionBase": text.len(), "selectionExtent": text.len(),
            "selectionAffinity": "TextAffinity.downstream",
            "selectionIsDirectional": false,
            "composingBase": -1, "composingExtent": -1,
        })
    };
    let send_host = |runtime: &mut Runtime, controls: &mut FlutterTextInputControls, text| {
        let data = update(json!(client.get()), wire_value(text));
        assert_eq!(controls.handle_platform_edit(&data).unwrap(), Some(true));
        assert_eq!(
            runtime
                .handle_flutter_text_input_editing_call(&data)
                .unwrap(),
            Some(true)
        );
    };
    match origin {
        EditOrigin::Host => send_host(&mut runtime, &mut controls, incoming),
        EditOrigin::UserReplace => {
            let range = TextRange::new(0, controller.text().len());
            assert!(runtime.replace_text(target, range, incoming, SelectionChangedCause::Keyboard));
        }
        EditOrigin::Controller => controller.set_text(incoming),
    }
    let expected = match origin {
        EditOrigin::Controller => "remoteremoteremote listener",
        _ => "remote listener onChanged listener",
    };
    assert_eq!(controller.text(), expected);
    assert_eq!(
        on_changed_calls.get(),
        usize::from(!matches!(origin, EditOrigin::Controller))
    );
    assert_eq!(
        controller.selection(),
        TextSelection::collapsed(expected.len())
    );
    frame(&mut runtime);
    assert!(
        runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    let calls = controls.take_platform_calls();
    let states: Vec<_> = calls
        .iter()
        .filter(|call| call.method == "TextInput.setEditingState")
        .map(|call| &call.arguments)
        .collect();
    assert_eq!(states, [&wire_value(expected)]);

    if matches!(origin, EditOrigin::Host) {
        send_host(&mut runtime, &mut controls, expected);
        frame(&mut runtime);
        assert_eq!(controller.text(), expected);
        assert_eq!(on_changed_calls.get(), 1);
    }
    assert!(
        !runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    assert!(controls.take_platform_calls().is_empty());
}

#[test]
fn host_input_sends_only_the_final_formatted_and_listened_value_without_echo() {
    callback_edits_publish_one_final_value(EditOrigin::Host);
}

#[test]
fn user_replace_sends_only_the_final_formatted_and_listened_value() {
    callback_edits_publish_one_final_value(EditOrigin::UserReplace);
}

#[test]
fn programmatic_text_only_invokes_the_controller_listener_before_publication() {
    callback_edits_publish_one_final_value(EditOrigin::Controller);
}

#[test]
fn a_controllers_reentrant_numeric_listener_can_repair_a_host_edit_and_its_caret() {
    use incular_text::TextEditingValue;

    let controller = TextEditingController::new();
    let listener = controller.clone();
    let _listener = controller.observe(move |value| {
        if value.text.parse::<f64>().unwrap_or(0.) > 5.5555 {
            listener.set_value(TextEditingValue::new("5.5555"));
        }
    });
    let focus = FocusNode::new();
    let mut runtime = Runtime::new(field(&controller, &focus, true).into()).unwrap();
    frame(&mut runtime);
    let client = attached(&mut runtime);
    for (text, offset) in [("1", 1), ("12", 2)] {
        let data = update(
            json!(client.get()),
            json!({"text":text, "selectionBase": offset, "selectionExtent": offset}),
        );
        assert_eq!(
            runtime
                .handle_flutter_text_input_editing_call(&data)
                .unwrap(),
            Some(true)
        );
        frame(&mut runtime);
    }
    assert_eq!(controller.text(), "5.5555");
    assert_eq!(controller.selection(), TextSelection::collapsed(6));
}

#[test]
fn clear_sends_one_empty_wire_value_after_a_host_edit_that_was_not_echoed() {
    use incular_core::{InputEvent, Offset, PointerPhase};
    use incular_platform::{FlutterTextInputConfiguration, FlutterTextInputControls};

    let controller = TextEditingController::new();
    let focus = FocusNode::new();
    let mut runtime = Runtime::new(field(&controller, &focus, false).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    frame(&mut runtime);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(10., 12.),
        });
    }
    frame(&mut runtime);
    assert!(focus.has_primary_focus());
    let client = attached(&mut runtime);
    let mut controls = FlutterTextInputControls::new(false);
    assert!(controls.attach_existing(client, FlutterTextInputConfiguration::default(), |_| {}));
    assert!(
        runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    let _ = controls.take_platform_calls();

    let host = update(
        json!(client.get()),
        json!({
            "text":"a", "selectionBase":0, "selectionExtent":0,
            "selectionAffinity":"TextAffinity.downstream", "selectionIsDirectional":false,
            "composingBase":-1, "composingExtent":-1,
        }),
    );
    assert_eq!(controls.handle_platform_edit(&host).unwrap(), Some(true));
    assert_eq!(
        runtime
            .handle_flutter_text_input_editing_call(&host)
            .unwrap(),
        Some(true)
    );
    frame(&mut runtime);
    assert_eq!(controller.text(), "a");
    assert!(
        !runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    assert!(controls.take_platform_calls().is_empty());

    controller.clear();
    assert!(
        runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    let calls = controls.take_platform_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].method, "TextInput.setEditingState");
    assert_eq!(
        calls[0].arguments,
        json!({
            "text":"", "selectionBase":0, "selectionExtent":0,
            "selectionAffinity":"TextAffinity.downstream", "selectionIsDirectional":false,
            "composingBase":-1, "composingExtent":-1,
        })
    );
    frame(&mut runtime);
    assert!(
        !runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    assert!(controls.take_platform_calls().is_empty());
}

#[test]
fn setting_controller_text_to_empty_removes_the_retained_glyphs() {
    use incular_rendering::{DisplayList, PaintCommand};

    let controller = TextEditingController::new();
    let focus = FocusNode::new();
    let mut runtime = Runtime::new(field(&controller, &focus, false).into()).unwrap();
    let constraints = Constraints::tight(Size::new(180., 80.));
    runtime.run_frame(constraints).unwrap();
    controller.set_text("...");
    let glyphs = |paint: &DisplayList| {
        paint
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::GlyphRun { run, .. } => Some(run.glyphs.len()),
                _ => None,
            })
            .sum::<usize>()
    };
    let (visible, _) = runtime.run_frame(constraints).unwrap();
    assert_eq!(glyphs(&visible), 3);
    assert!(!focus.has_focus());

    controller.set_text("");
    let (cleared, _) = runtime.run_frame(constraints).unwrap();
    assert_eq!(glyphs(&cleared), 0);
    assert_eq!(controller.selection_if_set(), None);
    assert!(!focus.has_focus());
}

#[test]
fn a_rejected_remote_edit_is_corrected_on_the_wire_each_time_it_arrives() {
    use incular_platform::{FlutterTextInputConfiguration, FlutterTextInputControls};
    use incular_text::TextEditingValue;
    use incular_widgets::CallbackTextInputFormatter;

    let controller = TextEditingController::new();
    let focus = FocusNode::new();
    let formatter =
        CallbackTextInputFormatter::new(|_: &TextEditingValue, _: &TextEditingValue| {
            TextEditingValue::new("Flutter is the best!")
        });
    let mut runtime = Runtime::new(
        field(&controller, &focus, true)
            .input_formatter(formatter)
            .into(),
    )
    .unwrap();
    frame(&mut runtime);
    let client = attached(&mut runtime);
    let mut controls = FlutterTextInputControls::new(false);
    assert!(controls.attach_existing(client, FlutterTextInputConfiguration::default(), |_| {}));
    assert!(
        runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    let _ = controls.take_platform_calls();

    let rejected = "I will be modified by the formatter.";
    let packet = update(
        json!(client.get()),
        json!({
            "text": rejected, "selectionBase": rejected.len(), "selectionExtent": rejected.len(),
        }),
    );
    for _ in 0..2 {
        assert_eq!(controls.handle_platform_edit(&packet).unwrap(), Some(true));
        assert_eq!(
            runtime
                .handle_flutter_text_input_editing_call(&packet)
                .unwrap(),
            Some(true)
        );
        frame(&mut runtime);
        assert!(
            runtime
                .publish_flutter_text_input_snapshot(&mut controls)
                .unwrap()
        );
        let calls = controls.take_platform_calls();
        let states: Vec<_> = calls
            .iter()
            .filter(|call| call.method == "TextInput.setEditingState")
            .collect();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].arguments["text"], "Flutter is the best!");
        assert_eq!(states[0].arguments["selectionBase"], 20);
        assert_eq!(states[0].arguments["selectionExtent"], 20);
        assert_eq!(controller.text(), "Flutter is the best!");
        assert!(
            !runtime
                .publish_flutter_text_input_snapshot(&mut controls)
                .unwrap()
        );
    }
}

#[test]
fn repeating_a_host_value_after_blur_does_not_reopen_the_keyboard_or_refocus() {
    use incular_platform::{
        FlutterTextInputConfiguration, FlutterTextInputControls, FlutterTextInputWireCall,
    };

    let controller = TextEditingController::new();
    let focus = FocusNode::new();
    let mut runtime = Runtime::new(field(&controller, &focus, true).into()).unwrap();
    frame(&mut runtime);
    let client = attached(&mut runtime);
    assert!(focus.has_primary_focus());
    let mut controls = FlutterTextInputControls::new(false);
    assert!(controls.attach_existing(client, FlutterTextInputConfiguration::default(), |_| {}));
    assert!(
        runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    let _ = controls.take_platform_calls();
    let same = update(
        json!(client.get()),
        json!({
            "text": "a", "selectionBase": 0, "selectionExtent": 0,
        }),
    );
    assert_eq!(controls.handle_platform_edit(&same).unwrap(), Some(true));
    assert_eq!(
        runtime
            .handle_flutter_text_input_editing_call(&same)
            .unwrap(),
        Some(true)
    );
    frame(&mut runtime);
    assert!(
        !runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    assert!(controls.take_platform_calls().is_empty());
    let _ = runtime.take_text_input_commands();

    focus.unfocus();
    frame(&mut runtime);
    assert!(!focus.has_focus());
    let native = runtime.take_text_input_commands();
    assert!(native.iter().any(|command| matches!(
        command, TextInputCommand::Clear { client: id } if *id == client
    )));
    assert!(controls.close(client));
    assert!(controls.flush_pending_hide());
    assert_eq!(
        controls.take_platform_calls(),
        [
            FlutterTextInputWireCall {
                method: "TextInput.clearClient",
                arguments: Value::Null
            },
            FlutterTextInputWireCall {
                method: "TextInput.hide",
                arguments: Value::Null
            },
        ]
    );

    assert_eq!(controls.handle_platform_edit(&same).unwrap(), Some(false));
    assert_eq!(
        runtime
            .handle_flutter_text_input_editing_call(&same)
            .unwrap(),
        Some(false)
    );
    frame(&mut runtime);
    assert!(!focus.has_focus());
    assert!(runtime.take_text_input_commands().is_empty());
    assert!(
        !runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    assert!(controls.take_platform_calls().is_empty());
    assert_eq!(controller.text(), "a");
}

#[test]
fn persian_keyboard_updates_move_the_rtl_caret_left_even_after_trailing_space() {
    let text = TextEditingController::new();
    let node = FocusNode::new();
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(
        EditableText::new(text.clone())
            .focus_node(node.clone())
            .text_direction(TextDirection::Rtl)
            .autofocus(true)
            .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    runtime.run_frame(bounds).unwrap();
    let client = attached(&mut runtime);
    let editor = runtime.focused_element().unwrap();
    let mut positions = Vec::new();
    for content in ["گ", "گی", "گیگ", "گیگ "] {
        let utf16 = content.encode_utf16().count();
        let editing = json!({
            "text": content,
            "selectionBase": utf16,
            "selectionExtent": utf16,
            "selectionAffinity": "TextAffinity.downstream",
            "composingBase": -1,
            "composingExtent": -1,
        });
        assert_eq!(
            runtime
                .handle_flutter_text_input_editing_call(&update(json!(client.get()), editing))
                .unwrap(),
            Some(true)
        );
        runtime.run_frame(bounds).unwrap();
        assert!(node.has_primary_focus());
        assert_eq!(text.selection(), TextSelection::collapsed(content.len()));
        let caret = runtime
            .tree()
            .editable_local_caret_rect(editor, text.selection().extent, text.caret_affinity())
            .unwrap();
        positions.push(caret.origin.x);
    }
    assert_eq!(text.text(), "گیگ ");
    assert!(
        positions.windows(2).all(|edge| edge[1] < edge[0]),
        "{positions:?}"
    );
}

#[test]
fn a_partial_grapheme_selection_renders_and_a_split_surrogate_is_rejected_safely() {
    use incular_rendering::PaintCommand;

    const FAMILY: &str = "👨‍👩‍👦";
    let text = TextEditingController::with_text("12345");
    let node = FocusNode::new();
    let mut runtime = Runtime::new(
        field(&text, &node, true)
            .min_lines(Some(2))
            .max_lines(Some(2))
            .text_align(incular_text::TextAlign::Right)
            .into(),
    )
    .unwrap();
    frame(&mut runtime);
    let client = attached(&mut runtime);
    let packet = |extent| {
        update(
            json!(client.get()),
            json!({"text": FAMILY, "selectionBase": 0, "selectionExtent": extent}),
        )
    };

    // The exact Flutter fixture uses a UTF-16 index inside the leading
    // surrogate pair; the Rust transport refuses that unrepresentable edge.
    assert!(matches!(
        runtime.handle_flutter_text_input_editing_call(&packet(1)),
        Err(FlutterTextInputEditingError::SplitScalar {
            range: "selection",
            offset: 1
        })
    ));
    assert_eq!(text.text(), "12345");

    // Ending after the first scalar is still strictly inside the family
    // grapheme, but is representable. Exercise the real shaping/paint path.
    assert_eq!(
        runtime
            .handle_flutter_text_input_editing_call(&packet(2))
            .unwrap(),
        Some(true)
    );
    let (display, _) = runtime
        .run_frame(Constraints::tight(Size::new(180., 80.)))
        .unwrap();
    assert_eq!(text.text(), FAMILY);
    assert_eq!(text.selection(), TextSelection::new(0, '👨'.len_utf8()));
    assert!(node.has_primary_focus());
    assert!(display.commands().iter().any(|command| {
        matches!(command, PaintCommand::GlyphRun { run, .. } if !run.glyphs.is_empty())
    }));
}
