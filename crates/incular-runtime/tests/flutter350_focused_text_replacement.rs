use incular_config::{Constraints, TextDirection};
use incular_core::{Code, KeyboardEvent, KeyboardKey, NamedKey, Size};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{Column, Directionality, EditableText, FocusNode, SizedBox, Widget};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(200.0, 80.0)))
        .unwrap();
}

fn editor(controller: &TextEditingController, node: &FocusNode) -> Widget {
    EditableText::new(controller.clone())
        .focus_node(node.clone())
        .into()
}

#[test]
fn programmatic_replacement_sets_a_valid_caret_while_a_field_is_focused() {
    let controller = TextEditingController::with_text("Text");
    let node = FocusNode::new();
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, editor(&controller, &node)).into())
            .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    frame(&mut runtime);
    assert!(!node.has_focus());
    assert_eq!(controller.selection_if_set(), None);
    let tab = KeyboardKey::Named(NamedKey::Tab);
    assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(tab.clone(), Code::Tab)));
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(tab, Code::Tab));
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    assert_eq!(
        controller.selection_if_set(),
        Some(TextSelection::collapsed(4))
    );
    controller.set_text("Updated");
    // Flutter fixes the invalid selection synchronously while still focused.
    assert!(node.has_primary_focus());
    assert_eq!(
        controller.selection_if_set(),
        Some(TextSelection::collapsed(7))
    );
}

#[test]
fn focused_selection_policy_ends_when_a_field_unmounts() {
    let controller = TextEditingController::with_text("Text");
    let node = FocusNode::new();
    let build = |visible| {
        Directionality::new(
            TextDirection::Ltr,
            Column::new([if visible {
                editor(&controller, &node)
            } else {
                SizedBox::shrink().into()
            }]),
        )
    };
    let mut runtime = Runtime::new(build(true).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    controller.set_text("Updated");
    assert_eq!(
        controller.selection_if_set(),
        Some(TextSelection::new(0, 7))
    );
    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(false).into())
        .unwrap();
    frame(&mut runtime);
    assert!(!node.has_focus());
    controller.set_text("Off stage");
    assert_eq!(controller.selection_if_set(), None);
}

#[test]
fn replacing_a_focused_widgets_controller_moves_the_selection_policy() {
    let before = TextEditingController::with_text("First");
    let after = TextEditingController::with_text("Second");
    let node = FocusNode::new();
    let build = |controller: &TextEditingController| {
        Directionality::new(TextDirection::Ltr, editor(controller, &node))
    };
    let mut runtime = Runtime::new(build(&before).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(&after).into())
        .unwrap();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    before.set_text("Detached");
    assert_eq!(before.selection_if_set(), None);
    after.set_text("Attached");
    assert_eq!(after.selection_if_set(), Some(TextSelection::new(0, 8)));
}

#[test]
fn reattaching_the_same_focus_node_keeps_the_editors_selection() {
    use incular_widgets::Padding;

    let controller = TextEditingController::with_text("ABCDE");
    let node = FocusNode::new();
    let build = |wrapped| {
        let child = editor(&controller, &node);
        let child = if wrapped {
            Widget::from(Padding::all(5., child))
        } else {
            child
        };
        Directionality::new(TextDirection::Ltr, child)
    };
    let mut runtime = Runtime::new(build(false).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    let old = runtime.focused_element().unwrap();
    let expected = TextSelection::new(1, 3);
    controller.set_selection(expected);
    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(true).into())
        .unwrap();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    let new = runtime.focused_element().unwrap();
    assert_ne!(new, old);
    assert!(!runtime.tree().element_exists(old));
    assert_eq!(controller.text(), "ABCDE");
    assert_eq!(controller.selection(), expected);
}

#[test]
fn reparenting_a_focused_read_only_editor_can_attach_the_new_writable_input() {
    use incular_core::{InputEvent, Offset};
    use incular_platform::TextInputCommand;
    use incular_widgets::{FractionalTranslation, GlobalKey};

    let before = TextEditingController::with_text("blah blah");
    let after = TextEditingController::with_text("blah blah");
    let node = FocusNode::new();
    let key = GlobalKey::new();
    let build = |controller: &TextEditingController, read_only| {
        let field = Widget::from(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .focus_node(node.clone())
                .read_only(read_only),
        )
        .with_key(key);
        let child = if read_only {
            field
        } else {
            FractionalTranslation::new(Offset::new(0.1, 0.1), field).into()
        };
        Widget::from(Directionality::new(TextDirection::Ltr, child))
    };
    let mut runtime = Runtime::new(build(&before, true)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    let _ = runtime.take_text_input_commands();
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(&after, false)).unwrap();
    frame(&mut runtime);

    assert!(node.has_primary_focus());
    let field = runtime.focused_element().unwrap();
    assert_eq!(runtime.tree().text_controller(field), Some(after.clone()));
    let commands = runtime.take_text_input_commands();
    assert!(
        commands.iter().any(|command| matches!(
            command,
            TextInputCommand::SetClient { configuration, state, .. }
                if !configuration.read_only && state.text == "blah blah"
        )),
        "{commands:?}"
    );

    after.set_selection(TextSelection::collapsed(9));
    let _ = runtime.handle_input(InputEvent::Text("!".into()));
    frame(&mut runtime);
    assert_eq!(before.text(), "blah blah");
    assert_eq!(after.text(), "blah blah!");
    assert!(node.has_primary_focus());
}

#[test]
fn replacement_controller_text_reaches_the_attached_input_once_without_truncation() {
    use incular_platform::TextInputCommand;
    use incular_widgets::LengthLimitingTextInputFormatter;

    let first = TextEditingController::new();
    let node = FocusNode::new();
    let build = |controller: &TextEditingController| {
        Widget::from(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .focus_node(node.clone())
                .max_lines(Some(2))
                .input_formatter(LengthLimitingTextInputFormatter::new(6)),
        )
    };
    let mut runtime = Runtime::new(build(&first)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    let field = runtime.focused_element().unwrap();
    let render = runtime.tree().render_id(field);
    let attached = runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|cmd| match cmd {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .unwrap();

    for expected in ["new text", "new new text"] {
        let replacement = TextEditingController::with_text(expected);
        runtime.schedule_update(field, build(&replacement)).unwrap();
        frame(&mut runtime);
        assert_eq!(runtime.focused_element(), Some(field));
        assert_eq!(runtime.tree().render_id(field), render);
        assert_eq!(replacement.selection_if_set(), None);
        assert_eq!(replacement.text(), expected);
        let written: Vec<_> = runtime
            .take_text_input_commands()
            .into_iter()
            .filter_map(|cmd| match cmd {
                TextInputCommand::Update { client, state } => Some((client, state.text)),
                TextInputCommand::SetClient { .. } => {
                    panic!("swapping a controller must not attach again")
                }
                _ => None,
            })
            .collect();
        assert_eq!(written, [(attached, expected.to_owned())]);
        frame(&mut runtime);
        assert!(runtime.take_text_input_commands().is_empty());
    }
}
