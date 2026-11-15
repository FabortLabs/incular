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
