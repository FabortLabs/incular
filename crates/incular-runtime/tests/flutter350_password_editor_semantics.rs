use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_semantics::{
    SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticsInputType,
};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{EditableText, FocusNode, TextDirection, Widget};

const PASSWORD: &str = "super-secret-password!!1";
const BOUNDS: Size = Size {
    width: 300.,
    height: 40.,
};

fn field(controller: &TextEditingController, focus: &FocusNode, hidden: bool) -> Widget {
    EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .text_direction(TextDirection::Ltr)
        .size(BOUNDS)
        .obscure_text(hidden)
        .into()
}

fn frame(runtime: &mut Runtime) {
    runtime.run_frame(Constraints::tight(BOUNDS)).unwrap();
}

fn node(runtime: &Runtime) -> &SemanticNode {
    let tree = runtime.tree().semantics();
    let [id] = tree.roots() else {
        panic!("expected one editable semantic root")
    };
    let node = tree.node(*id).unwrap();
    assert_eq!(node.role, SemanticRole::TextField);
    assert_eq!(node.state.input_type, SemanticsInputType::Text);
    assert_eq!(node.state.text_direction, Some(TextDirection::Ltr));
    assert!(node.state.focusable);
    node
}

#[test]
fn an_unfocused_password_masks_its_value_without_advertising_edit_actions() {
    let controller = TextEditingController::with_text(PASSWORD);
    let mut runtime = Runtime::new(field(&controller, &FocusNode::new(), true)).unwrap();
    frame(&mut runtime);
    let node = node(&runtime);
    assert!(node.state.obscured);
    assert!(!node.state.focused);
    assert_eq!(node.value.as_deref(), Some("•".repeat(24).as_str()));
    assert_eq!(node.state.selection, None);
    assert!(
        node.actions
            .iter()
            .all(|action| *action == SemanticActionKind::Focus)
    );
}

#[test]
fn a_focused_password_toggle_reports_the_retained_caret_and_available_edits() {
    let controller = TextEditingController::with_text(PASSWORD);
    let focus = FocusNode::new();
    let mut runtime = Runtime::new(field(&controller, &focus, false)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    frame(&mut runtime);
    let before = node(&runtime);
    assert_eq!(before.value.as_deref(), Some(PASSWORD));
    assert!(!before.state.focused && !before.state.obscured);
    let id = before.id;
    let root = runtime.tree().root().unwrap();
    let _ = focus.request_focus();
    runtime
        .schedule_update(root, field(&controller, &focus, true))
        .unwrap();
    frame(&mut runtime);
    let after = node(&runtime);
    assert_eq!(after.id, id);
    assert!(after.state.focused && after.state.obscured);
    assert_eq!(after.value.as_deref(), Some("•".repeat(24).as_str()));
    let position = after.state.selection.unwrap();
    assert_eq!((position.base, position.extent), (24, 24));
    for kind in [
        SemanticActionKind::SetText,
        SemanticActionKind::SetSelection,
        SemanticActionKind::MoveCursorBackwardByCharacter,
        SemanticActionKind::MoveCursorBackwardByWord,
    ] {
        assert!(
            after.actions.contains(&kind),
            "missing supported action {kind:?}"
        );
    }
    assert!(
        !after
            .actions
            .contains(&SemanticActionKind::MoveCursorForwardByCharacter)
    );
}

fn move_action(forward: bool, word: bool, extend_selection: bool) -> SemanticAction {
    match (forward, word) {
        (false, false) => SemanticAction::MoveCursorBackwardByCharacter { extend_selection },
        (true, false) => SemanticAction::MoveCursorForwardByCharacter { extend_selection },
        (false, true) => SemanticAction::MoveCursorBackwardByWord { extend_selection },
        (true, true) => SemanticAction::MoveCursorForwardByWord { extend_selection },
    }
}

fn walk(plain: &str, word: bool, extend: bool, backwards: &[usize], forwards: &[usize]) {
    let controller = TextEditingController::with_text(plain);
    controller.set_selection(TextSelection::collapsed(plain.len()));
    let mut runtime = Runtime::new(field(&controller, &FocusNode::new(), false)).unwrap();
    frame(&mut runtime);
    let id = node(&runtime).id;
    assert!(!node(&runtime).state.focused);
    let assert_boundary_actions = |runtime: &Runtime, offset| {
        let actions = &node(runtime).actions;
        for word in [false, true] {
            assert_eq!(
                actions.contains(&move_action(false, word, false).kind()),
                offset > 0
            );
            assert_eq!(
                actions.contains(&move_action(true, word, false).kind()),
                offset < plain.len()
            );
        }
    };
    assert_boundary_actions(&runtime, plain.len());
    for &extent in backwards {
        assert!(runtime.dispatch_semantic_action(id, move_action(false, word, extend)));
        let start = if extend { plain.len() } else { extent };
        assert_eq!(controller.selection(), TextSelection::new(start, extent));
        frame(&mut runtime);
        assert_boundary_actions(&runtime, extent);
    }
    assert!(!runtime.dispatch_semantic_action(id, move_action(false, word, extend)));
    for (index, &extent) in forwards.iter().enumerate() {
        let extend_step = extend && index > 0;
        assert!(runtime.dispatch_semantic_action(id, move_action(true, word, extend_step)));
        let start = if extend_step { forwards[0] } else { extent };
        assert_eq!(controller.selection(), TextSelection::new(start, extent));
        frame(&mut runtime);
        assert_boundary_actions(&runtime, extent);
    }
    assert_eq!(controller.text(), plain);
}

#[test]
fn character_accessibility_motion_and_extension_match_the_pinned_offsets() {
    walk("test", false, false, &[3, 2, 1, 0], &[1]);
    walk("test", false, true, &[3, 2, 1, 0], &[1, 2]);
}

#[test]
fn word_accessibility_motion_and_extension_skip_separator_ranges() {
    walk("test for words", true, false, &[9, 5, 0], &[5, 9]);
    walk("test for words", true, true, &[9, 5, 0], &[5, 9]);
}

#[test]
fn a_focused_editor_advertises_only_the_cursor_directions_that_can_move() {
    let controller = TextEditingController::with_text("test");
    let focus = FocusNode::new();
    let mut runtime = Runtime::new(field(&controller, &focus, false)).unwrap();
    frame(&mut runtime);
    let _ = focus.request_focus();
    frame(&mut runtime);
    for offset in [4, 2, 0] {
        controller.set_selection(TextSelection::collapsed(offset));
        frame(&mut runtime);
        let field = node(&runtime);
        assert!(field.state.focused);
        let selected = field.state.selection.unwrap();
        assert_eq!((selected.base, selected.extent), (offset, offset));
        assert!(field.actions.contains(&SemanticActionKind::SetText));
        assert!(field.actions.contains(&SemanticActionKind::SetSelection));
        for word in [false, true] {
            assert_eq!(
                field
                    .actions
                    .contains(&move_action(false, word, false).kind()),
                offset != 0
            );
            assert_eq!(
                field
                    .actions
                    .contains(&move_action(true, word, false).kind()),
                offset != 4
            );
        }
    }
}
