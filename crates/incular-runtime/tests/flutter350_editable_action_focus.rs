use incular_config::Constraints;
use incular_core::Size;
use incular_platform::{TextInputAction, TextInputCommand};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextRange};
use incular_widgets::{Column, EditableText, Focus, FocusNode, SizedBox, TextDirection, Widget};
use std::{cell::RefCell, rc::Rc};

fn editable(node: &FocusNode, text: &TextEditingController) -> EditableText {
    EditableText::new(text.clone())
        .focus_node(node.clone())
        .text_direction(TextDirection::Ltr)
        .autofocus(true)
        .size(Size::new(180., 40.))
}

fn mount(widget: Widget) -> Runtime {
    let mut runtime = Runtime::new(widget).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    runtime
        .run_frame(Constraints::tight(Size::new(180., 100.)))
        .unwrap();
    runtime
}

#[test]
fn completion_override_keeps_done_from_unfocusing_the_editor() {
    let node = FocusNode::new();
    let calls = Rc::new(RefCell::new(0));
    let called = calls.clone();
    let mut runtime = mount(
        editable(&node, &TextEditingController::new())
            .on_editing_complete(move || *called.borrow_mut() += 1)
            .into(),
    );
    assert!(node.has_primary_focus());
    assert!(runtime.handle_text_input_action(TextInputAction::Done));
    assert!(node.has_primary_focus());
    assert_eq!(*calls.borrow(), 1);
}

fn verify_callback_order(action: TextInputAction) {
    let node = FocusNode::new();
    let following = FocusNode::new();
    let hits = Rc::new(RefCell::new(Vec::<String>::new()));
    let [completion, submitted] = [hits.clone(), hits.clone()];
    let text = TextEditingController::with_text("typed");
    let editor = editable(&node, &text)
        .on_editing_complete(move || completion.borrow_mut().push("complete".into()))
        .on_submit(move |text| submitted.borrow_mut().push(text));
    let mut runtime = mount(
        Column::new([
            Widget::from(editor),
            Widget::from(Focus::new(SizedBox::square(25.)).node(following.clone())),
        ])
        .into(),
    );
    assert!(runtime.handle_text_input_action(action));
    assert_eq!(*hits.borrow(), ["complete", "typed"]);
    assert!(node.has_primary_focus());
    assert!(!following.has_focus());
}

#[test]
fn done_calls_editing_complete_before_the_submitted_text() {
    verify_callback_order(TextInputAction::Done);
}

#[test]
fn next_calls_editing_complete_before_the_submitted_text() {
    verify_callback_order(TextInputAction::Next);
}

#[test]
fn plain_done_retires_the_connection_without_reopening_it() {
    let node = FocusNode::new();
    let mut runtime = mount(
        editable(&node, &TextEditingController::new())
            .on_submit(|_| {})
            .into(),
    );
    runtime.take_text_input_commands();
    assert!(node.has_primary_focus());

    assert!(runtime.handle_text_input_action(TextInputAction::Done));
    let commands = runtime.take_text_input_commands();
    assert!(!node.has_focus());
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, TextInputCommand::Clear { .. }))
    );
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, TextInputCommand::SetClient { .. }))
    );
}

fn verify_refocus(include_show: bool) {
    let node = FocusNode::new();
    let target = node.clone();
    let calls = Rc::new(RefCell::new(0));
    let submitted = calls.clone();
    let mut runtime = mount(
        editable(&node, &TextEditingController::with_text("typed"))
            .on_submit(move |text| {
                assert_eq!(text, "typed");
                *submitted.borrow_mut() += 1;
                target.request_focus();
            })
            .into(),
    );
    let initial = runtime.take_text_input_commands();
    let first = initial
        .iter()
        .find_map(|c| match c {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .unwrap();
    assert!(node.has_primary_focus());

    assert!(runtime.handle_text_input_action(TextInputAction::Done));
    let commands = runtime.take_text_input_commands();
    let clear = commands
        .iter()
        .position(|c| *c == TextInputCommand::Clear { client: first })
        .expect("old client clears");
    let (attached, second) = commands
        .iter()
        .enumerate()
        .find_map(|(i, c)| match c {
            TextInputCommand::SetClient { configuration, .. } if i > clear => {
                Some((i, configuration.client))
            }
            _ => None,
        })
        .expect("submitting and refocusing replaces the client before the host drains this event");
    assert_ne!(first, second);
    assert!(node.has_primary_focus());
    if include_show {
        assert!(commands[attached + 1..].contains(&TextInputCommand::Show { client: second }));
        assert!(!commands.contains(&TextInputCommand::Hide { client: first }));
    }

    assert!(runtime.handle_text_input_action(TextInputAction::Unspecified));
    assert!(node.has_primary_focus());
    assert_eq!(*calls.borrow(), 2);
    assert!(
        !runtime
            .take_text_input_commands()
            .iter()
            .any(|command| matches!(
                command,
                TextInputCommand::Clear { .. }
                    | TextInputCommand::SetClient { .. }
                    | TextInputCommand::Hide { .. }
            ))
    );
}

#[test]
fn a_submit_callback_can_cancel_unfocus_and_reset_the_active_client() {
    verify_refocus(false);
}

#[test]
fn refocusing_during_submission_keeps_the_onscreen_keyboard_open() {
    verify_refocus(true);
}

#[test]
fn noncompletion_action_commits_the_composing_range_without_blur() {
    let node = FocusNode::new();
    let text = TextEditingController::with_text("typed");
    let mut runtime = mount(editable(&node, &text).into());
    text.set_composing(Some(TextRange::new(0, 2)));
    assert!(text.value().composing.is_some());
    assert!(runtime.handle_text_input_action(TextInputAction::None));
    assert!(node.has_primary_focus());
    assert!(text.value().composing.is_none());
    assert_eq!(text.text(), "typed");
}

#[test]
fn a_single_line_newline_completes_before_the_submitted_text() {
    verify_callback_order(TextInputAction::Newline);
}

#[test]
fn a_multiline_newline_skips_completion_and_submission() {
    let node = FocusNode::new();
    let controller = TextEditingController::with_text("typed");
    let mut runtime = mount(
        editable(&node, &controller)
            .multiline(true)
            .on_editing_complete(|| panic!("multiline action must not complete"))
            .on_submit(|_| panic!("multiline action must not submit"))
            .into(),
    );
    assert!(runtime.handle_text_input_action(TextInputAction::Newline));
    assert!(node.has_primary_focus());
    assert_eq!(controller.text(), "typed");
}

#[test]
fn changing_selection_through_the_keyboard_does_not_request_show() {
    use incular_platform::{PlatformEvent, TextInputState};

    let node = FocusNode::new();
    let controller = TextEditingController::with_text("123");
    controller.set_selection(incular_text::TextSelection::collapsed(0));
    let mut runtime = mount(editable(&node, &controller).into());
    runtime.take_text_input_commands();
    assert!(node.has_primary_focus());

    runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "123".into(),
        selection_start: 1,
        selection_end: 1,
        composing: None,
    }));
    runtime
        .run_frame(Constraints::tight(Size::new(180., 100.)))
        .unwrap();
    assert_eq!(
        controller.selection(),
        incular_text::TextSelection::collapsed(1)
    );
    assert!(node.has_primary_focus());
    assert!(!runtime.take_text_input_commands().iter().any(|c| {
        matches!(
            c,
            TextInputCommand::Show { .. } | TextInputCommand::SetClient { .. }
        )
    }));
}

#[test]
fn each_keyboard_action_retains_releases_or_traverses_focus_like_flutter() {
    use TextInputAction::*;
    for (action, expected) in [
        (None, "editor"),
        (Unspecified, "editor"),
        (Done, "none"),
        (Go, "none"),
        (Search, "none"),
        (Send, "none"),
        (ContinueAction, "editor"),
        (Join, "editor"),
        (Route, "editor"),
        (EmergencyCall, "editor"),
        (Newline, "none"),
        (Next, "following"),
        (Previous, "preceding"),
    ] {
        let [preceding, editor, following] = std::array::from_fn(|_| FocusNode::new());
        let mut runtime = mount(
            Column::new([
                Widget::from(Focus::new(SizedBox::square(25.)).node(preceding.clone())),
                Widget::from(editable(&editor, &TextEditingController::new())),
                Widget::from(Focus::new(SizedBox::square(25.)).node(following.clone())),
            ])
            .into(),
        );
        assert!(editor.has_primary_focus(), "{action:?}");
        assert!(runtime.handle_text_input_action(action), "{action:?}");
        assert_eq!(
            editor.has_primary_focus(),
            expected == "editor",
            "{action:?}"
        );
        assert_eq!(
            preceding.has_primary_focus(),
            expected == "preceding",
            "{action:?}"
        );
        assert_eq!(
            following.has_primary_focus(),
            expected == "following",
            "{action:?}"
        );
    }
}

#[test]
fn an_editing_complete_error_is_reported_and_submission_still_runs() {
    use incular_runtime::TextEditingCallback;
    let node = FocusNode::new();
    let labels = Rc::new(RefCell::new(Vec::new()));
    let sent = labels.clone();
    let mut runtime = mount(
        editable(
            &node,
            &TextEditingController::with_text("flutter is the best!"),
        )
        .on_editing_complete(|| panic!("editing fixture rejected completion"))
        .on_submit(move |value| sent.borrow_mut().push(value))
        .into(),
    );
    let failures = Rc::new(RefCell::new(Vec::new()));
    let observed = failures.clone();
    runtime.observe_text_editing_error(move |error| observed.borrow_mut().push(error.clone()));
    assert!(runtime.handle_text_input_action(TextInputAction::Done));
    assert_eq!(failures.borrow().len(), 1);
    assert_eq!(
        failures.borrow()[0].callback,
        TextEditingCallback::EditingComplete
    );
    assert_eq!(
        failures.borrow()[0].message,
        "editing fixture rejected completion"
    );
    assert_eq!(*labels.borrow(), ["flutter is the best!"]);
    assert!(node.has_primary_focus());
}

#[test]
fn an_on_submitted_error_is_reported_without_preventing_default_blur() {
    use incular_runtime::TextEditingCallback;
    let node = FocusNode::new();
    let mut runtime = mount(
        editable(
            &node,
            &TextEditingController::with_text("flutter is the best!"),
        )
        .on_submit(|_| panic!("submission fixture failed"))
        .into(),
    );
    runtime.take_text_input_commands();
    let failures = Rc::new(RefCell::new(Vec::new()));
    let observed = failures.clone();
    runtime.observe_text_editing_error(move |error| observed.borrow_mut().push(error.clone()));
    assert!(runtime.handle_text_input_action(TextInputAction::Done));
    assert_eq!(failures.borrow().len(), 1);
    assert_eq!(
        failures.borrow()[0].callback,
        TextEditingCallback::Submitted
    );
    assert_eq!(failures.borrow()[0].message, "submission fixture failed");
    assert!(!node.has_focus());
    let commands = runtime.take_text_input_commands();
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, TextInputCommand::Clear { .. }))
    );
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, TextInputCommand::SetClient { .. }))
    );
}

fn mounted_keyboard_wire(
    build: impl FnOnce(EditableText) -> EditableText,
) -> serde_json::Map<String, serde_json::Value> {
    use incular_platform::FlutterTextInputConfiguration;

    let node = FocusNode::new();
    let text = TextEditingController::new();
    let mut runtime = mount(build(editable(&node, &text)).into());
    assert!(node.has_primary_focus());
    let commands = runtime.take_text_input_commands();
    let config = commands
        .iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration),
            _ => None,
        })
        .unwrap();
    assert!(commands.iter().any(|command| matches!(
        command,
        TextInputCommand::Show { client } if *client == config.client
    )));
    text.set_text("test");
    runtime
        .run_frame(Constraints::tight(Size::new(180., 100.)))
        .unwrap();
    assert!(runtime.take_text_input_commands().iter().any(|command| matches!(
        command,
        TextInputCommand::Update { client, state } if *client == config.client && state.text == "test"
    )));
    FlutterTextInputConfiguration::from_native_editor(config, None).to_json()
}

#[test]
fn each_explicit_keyboard_action_reaches_the_attached_client_unchanged() {
    use incular_widgets::TextInputActionHint::*;

    for (hint, suffix) in [
        (Unspecified, "unspecified"),
        (None, "none"),
        (Done, "done"),
        (Send, "send"),
        (Go, "go"),
        (Search, "search"),
        (Next, "next"),
        (Previous, "previous"),
        (ContinueAction, "continueAction"),
        (Join, "join"),
        (Route, "route"),
        (EmergencyCall, "emergencyCall"),
    ] {
        let config = mounted_keyboard_wire(|field| field.input_action(hint));
        assert_eq!(
            config["inputAction"],
            format!("TextInputAction.{suffix}"),
            "{hint:?}"
        );
    }
}

#[test]
fn automatic_action_follows_keyboard_type_not_the_editors_visual_line_count() {
    use incular_platform::TextInputType;
    use incular_widgets::TextInputActionHint;

    for (kind, multiline, action) in [
        (None, false, "TextInputAction.done"),
        (None, true, "TextInputAction.newline"),
        (
            Some(TextInputType::Multiline),
            false,
            "TextInputAction.newline",
        ),
        (Some(TextInputType::Text), true, "TextInputAction.done"),
        (Some(TextInputType::Password), false, "TextInputAction.done"),
    ] {
        let config = mounted_keyboard_wire(|field| {
            let field = field.multiline(multiline);
            if let Some(kind) = kind {
                field.input_type(kind)
            } else {
                field
            }
        });
        assert_eq!(
            config["inputType"]["name"],
            kind.unwrap_or(if multiline {
                TextInputType::Multiline
            } else {
                TextInputType::Text
            })
            .flutter_name()
        );
        assert_eq!(config["inputAction"], action);
    }
    let explicitly_unspecified = mounted_keyboard_wire(|field| {
        field
            .multiline(true)
            .input_action(TextInputActionHint::Unspecified)
    });
    assert_eq!(
        explicitly_unspecified["inputAction"],
        "TextInputAction.unspecified"
    );
}

#[test]
fn disabling_the_two_ime_keyboard_traits_changes_only_the_requested_wire_field() {
    let no_suggestions = mounted_keyboard_wire(|field| field.enable_suggestions(false));
    assert_eq!(no_suggestions["enableSuggestions"], false);
    assert_eq!(no_suggestions["enableIMEPersonalizedLearning"], true);
    let no_learning = mounted_keyboard_wire(|field| field.enable_ime_personalized_learning(false));
    assert_eq!(no_learning["enableSuggestions"], true);
    assert_eq!(no_learning["enableIMEPersonalizedLearning"], false);
}

#[test]
fn max_lines_and_explicit_type_choose_the_keyboard_after_a_real_editable_tap() {
    use incular_core::{InputEvent, Offset, PointerPhase};
    use incular_platform::{FlutterTextInputConfiguration, TextInputType};
    use std::time::{Duration, Instant};

    let viewport = Constraints::tight(Size::new(180., 100.));
    for (line_override, explicit_type, kind, action) in [
        (
            Some(None),
            None,
            TextInputType::Multiline,
            "TextInputAction.newline",
        ),
        (
            Some(None),
            Some(TextInputType::Text),
            TextInputType::Text,
            "TextInputAction.done",
        ),
        (
            Some(Some(3)),
            Some(TextInputType::Phone),
            TextInputType::Phone,
            "TextInputAction.done",
        ),
        (
            Some(Some(3)),
            None,
            TextInputType::Multiline,
            "TextInputAction.newline",
        ),
        (None, None, TextInputType::Text, "TextInputAction.done"),
    ] {
        let text = TextEditingController::new();
        let focus = FocusNode::new();
        let mut field = editable(&focus, &text).autofocus(false);
        if let Some(lines) = line_override {
            field = field.max_lines(lines);
        }
        if let Some(input_type) = explicit_type {
            field = field.input_type(input_type);
        }
        let mut runtime = Runtime::new(field.into()).unwrap();
        runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
        let now = Instant::now();
        runtime.run_frame_at(viewport, now).unwrap();
        assert!(!focus.has_focus());
        assert!(runtime.take_text_input_commands().is_empty());
        for (index, phase) in [PointerPhase::Down, PointerPhase::Up]
            .into_iter()
            .enumerate()
        {
            let _ = runtime.handle_input_at(
                InputEvent::Pointer {
                    phase,
                    position: Offset::new(12., 12.),
                },
                now + Duration::from_millis(index as u64 + 16),
            );
        }
        runtime
            .run_frame_at(viewport, now + Duration::from_millis(32))
            .unwrap();
        assert!(focus.has_primary_focus());
        let commands = runtime.take_text_input_commands();
        let config = commands
            .iter()
            .find_map(|command| match command {
                TextInputCommand::SetClient { configuration, .. } => Some(configuration),
                _ => None,
            })
            .expect("a real pointer tap attaches the keyboard");
        assert!(commands.iter().any(|command| matches!(
            command,
            TextInputCommand::Show { client } if *client == config.client
        )));
        let wire = FlutterTextInputConfiguration::from_native_editor(config, None).to_json();
        assert_eq!(wire["inputType"]["name"], kind.flutter_name());
        assert_eq!(wire["inputAction"], action);

        text.set_text("test");
        runtime
            .run_frame_at(viewport, now + Duration::from_millis(48))
            .unwrap();
        assert!(runtime.take_text_input_commands().iter().any(|command| matches!(
            command,
            TextInputCommand::Update { client, state } if *client == config.client && state.text == "test"
        )));
    }
}
