//! Semantic SetText is a user edit. Flutter's RenderEditable `_handleSetText`
//! calls `userUpdateTextEditingValue`, so input formatters and `onChanged` run
//! for an accessibility replacement, while a programmatic controller update
//! (`controller.text = ...`) is stored as given.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind};
use incular_widgets::{
    EditableText, LengthLimitingTextInputFormatter, Stack, Widget, internal::TextEditingController,
};

fn mount_field(controller: TextEditingController, changes: Rc<RefCell<Vec<String>>>) -> Runtime {
    let field = EditableText::new(controller)
        .text_direction(incular_widgets::TextDirection::Ltr)
        .input_formatter(LengthLimitingTextInputFormatter::new(3))
        .on_changed(move |text| changes.borrow_mut().push(text.to_owned()))
        .size(Size::new(180., 32.));
    let mut runtime = Runtime::new(Stack::new([Widget::from(field)]).into()).expect("mount");
    runtime
        .run_frame(Constraints::tight(Size::new(200., 100.)))
        .expect("frame");
    // SetText is advertised only on the focused, editable field.
    let _ = runtime.handle_input(InputEvent::Pointer {
        phase: PointerPhase::Down,
        position: Offset::new(5., 5.),
    });
    runtime
        .run_frame(Constraints::tight(Size::new(200., 100.)))
        .expect("frame after focus");
    runtime
}

#[test]
fn semantic_set_text_runs_formatters_and_on_changed_as_a_user_edit() {
    let controller = TextEditingController::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mount_field(controller.clone(), changes.clone());
    let tree = runtime.tree();
    let root = tree.root().expect("root");
    let field = tree.children(root).expect("children").to_vec()[0];
    let node = tree.semantic_node_for_element(field).expect("field node");
    assert!(
        runtime
            .tree()
            .semantics()
            .node(node)
            .expect("node")
            .actions
            .contains(&SemanticActionKind::SetText)
    );

    assert!(runtime.dispatch_semantic_action(node, SemanticAction::SetText("abcdef".into())));
    assert_eq!(controller.text(), "abc");
    assert_eq!(controller.selection().extent, 3);
    assert_eq!(&*changes.borrow(), &["abc"]);

    // A programmatic update is stored as given: no formatter, no onChanged.
    controller.set_text("zzzzz");
    assert_eq!(controller.text(), "zzzzz");
    assert_eq!(&*changes.borrow(), &["abc"]);
}

#[test]
fn semantics_and_toolbar_edits_report_their_distinct_selection_causes() {
    use incular_text::{SelectionChangedCause, TextSelection};

    let controller = TextEditingController::with_text("abcdef");
    controller.set_selection(TextSelection::collapsed(0));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let editor = EditableText::new(controller.clone())
        .text_direction(incular_widgets::TextDirection::Ltr)
        .autofocus(true)
        .on_selection_changed({
            let calls = calls.clone();
            move |selection, cause| calls.borrow_mut().push((selection, cause))
        });
    let mut runtime = Runtime::new(editor.into()).unwrap();
    let constraints = Constraints::loose(Size::new(200., 100.));
    runtime.run_frame(constraints).unwrap();
    let field = runtime.focused_element().unwrap();
    let node = runtime.tree().semantic_node_for_element(field).unwrap();
    assert!(calls.borrow().is_empty());

    assert!(
        runtime.dispatch_semantic_action(node, SemanticAction::SetSelection { base: 1, extent: 3 })
    );
    runtime.run_frame(constraints).unwrap();
    assert!(runtime.dispatch_semantic_action(
        node,
        SemanticAction::MoveCursorForwardByCharacter {
            extend_selection: true
        },
    ));
    runtime.run_frame(constraints).unwrap();
    assert!(runtime.dispatch_semantic_action(node, SemanticAction::SetText("ok".into())));
    assert_eq!(controller.text(), "ok");
    assert!(runtime.select_all_text(field, SelectionChangedCause::Toolbar));
    assert_eq!(runtime.focused_element(), Some(field));
    assert_eq!(
        calls.borrow().as_slice(),
        &[
            (TextSelection::new(1, 3), SelectionChangedCause::Keyboard),
            (TextSelection::new(1, 4), SelectionChangedCause::Keyboard),
            (TextSelection::collapsed(2), SelectionChangedCause::Keyboard),
            (TextSelection::new(0, 2), SelectionChangedCause::Toolbar),
        ]
    );
}
