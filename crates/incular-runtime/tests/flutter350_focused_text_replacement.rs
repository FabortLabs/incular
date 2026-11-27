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
