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
