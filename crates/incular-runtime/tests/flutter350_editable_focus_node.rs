//! EditableText owns its attached FocusNode, not a separate event target.
use incular_config::Constraints;
use incular_core::Size;
use incular_platform::{PlatformEvent, TextInputState};
use incular_runtime::Runtime;
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{EditableText, FocusNode, TextDirection, Widget};

fn editor(controller: &TextEditingController, focus: &FocusNode, autofocus: bool) -> Widget {
    EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .focus_node(focus.clone())
        .autofocus(autofocus)
        .size(Size::new(180., 40.))
        .into()
}
fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(180., 40.)))
        .unwrap();
}
fn native_edit(runtime: &mut Runtime, text: &str) {
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: text.into(),
        selection_start: text.len(),
        selection_end: text.len(),
        composing: None,
    }));
    frame(runtime);
}

#[test]
fn an_editor_binds_the_replacement_node_without_losing_its_editing_value() {
    let controller = TextEditingController::with_text("ab");
    let first = FocusNode::new();
    let next = FocusNode::new();
    let mut runtime = Runtime::new(editor(&controller, &first, false)).unwrap();
    frame(&mut runtime);
    assert!(!first.has_primary_focus());
    first.request_focus();
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    native_edit(&mut runtime, "ab!");
    assert_eq!(controller.text(), "ab!");
    let root = runtime.tree().root().unwrap();
    assert_eq!(runtime.focused_element(), Some(root));

    next.request_focus();
    runtime
        .schedule_update(root, editor(&controller, &next, false))
        .unwrap();
    frame(&mut runtime);
    assert!(!first.has_primary_focus());
    assert!(next.has_primary_focus());
    assert_eq!(runtime.tree().focused_keyboard_element(), Some(root));
    first.request_focus();
    frame(&mut runtime);
    assert!(!first.has_primary_focus());
    assert!(next.has_primary_focus());
    native_edit(&mut runtime, "ab!?");
    assert_eq!(controller.text(), "ab!?");
    assert_eq!(controller.selection(), TextSelection::collapsed(4));
}

#[test]
fn autofocus_only_grants_an_eligible_editable_node() {
    for eligible in [false, true] {
        let node = FocusNode::new();
        node.set_can_request_focus(eligible);
        let controller = TextEditingController::new();
        let mut runtime = Runtime::new(editor(&controller, &node, true)).unwrap();
        frame(&mut runtime);
        assert_eq!(node.has_primary_focus(), eligible);
        assert_eq!(runtime.focused_element().is_some(), eligible);
    }
}
