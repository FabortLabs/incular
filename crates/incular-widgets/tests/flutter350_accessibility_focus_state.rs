//! Flutter 3.50.0 widgets/semantics_test.dart:1149.
use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_semantics::{AccessibilityFocusBlockType, CustomSemanticsAction, SemanticActionKind};
use incular_widgets::{Semantics, SizedBox, Widget, internal::WidgetTree};

#[test]
fn blocked_subtree_clears_focus_presence_but_preserves_the_named_action() {
    let action = CustomSemanticsAction::new("action1");
    let id = action.get_identifier();
    let invocations = Rc::new(Cell::new(0));
    let scene = |block, focused| {
        let invocations = invocations.clone();
        Widget::from(
            Semantics::new(SizedBox::new().width(10.).height(10.))
                .container(true)
                .accessibility_focus_block_type(block)
                .focused(focused)
                .custom_action(action.clone(), move || {
                    invocations.set(invocations.get() + 1)
                }),
        )
    };
    let mut tree = WidgetTree::new();
    let mut root = None;
    for (block, focused, expected) in [
        (AccessibilityFocusBlockType::BlockSubtree, true, None),
        (AccessibilityFocusBlockType::None, true, Some(true)),
        (AccessibilityFocusBlockType::None, false, Some(false)),
        (AccessibilityFocusBlockType::BlockSubtree, false, None),
    ] {
        let widget = scene(block, focused);
        if let Some(root) = root {
            tree.update(root, widget).unwrap();
        } else {
            root = Some(tree.mount(widget).unwrap());
        }
        tree.layout(Constraints::loose(Size::new(100., 100.)))
            .unwrap();
        let _ = tree.paint();
        tree.update_semantics();
        let (node_id, node) = tree
            .semantics()
            .iter()
            .find(|(_, node)| node.state.custom_actions.contains(&id))
            .unwrap();
        assert_eq!(node.state.accessibility_focus_block_type, block);
        assert_eq!(node.state.input_focus_state(), expected);
        assert_eq!(&*node.state.custom_actions, &[id]);
        if block.blocks_focus() {
            assert!(!node.actions.contains(&SemanticActionKind::Focus));
        }
        let element = tree.element_for_semantic_node(node_id).unwrap();
        tree.custom_semantic_action_callback(element, id).unwrap()();
    }
    assert_eq!(invocations.get(), 4);
}

#[test]
fn the_default_focus_block_does_not_annotate_or_insert_a_semantic_node() {
    let annotation =
        |block| Semantics::new(SizedBox::square(10.)).accessibility_focus_block_type(block);
    assert!(Semantics::new(SizedBox::square(10.)).explicit().is_none());
    assert!(
        annotation(AccessibilityFocusBlockType::None)
            .explicit()
            .is_none()
    );
    let real_block = annotation(AccessibilityFocusBlockType::BlockSubtree)
        .explicit()
        .expect("a changed focus-block value is an explicit annotation");
    assert_eq!(
        real_block.state.accessibility_focus_block_type,
        AccessibilityFocusBlockType::BlockSubtree
    );

    let mut tree = WidgetTree::new();
    let root = tree
        .mount(annotation(AccessibilityFocusBlockType::None).into())
        .unwrap();
    let box_size = Constraints::tight(Size::new(100., 100.));
    for block in [
        AccessibilityFocusBlockType::None,
        AccessibilityFocusBlockType::BlockSubtree,
        AccessibilityFocusBlockType::None,
    ] {
        tree.update(root, annotation(block).into()).unwrap();
        tree.layout(box_size).unwrap();
        tree.update_semantics();
        assert_eq!(
            tree.semantics().is_empty(),
            block == AccessibilityFocusBlockType::None
        );
        if block != AccessibilityFocusBlockType::None {
            let [id] = tree.semantics().roots() else {
                panic!("the annotation creates one semantic boundary");
            };
            let state = &tree.semantics().node(*id).unwrap().state;
            assert_eq!(state.accessibility_focus_block_type, block);
            assert!(!state.focusable);
        }
    }
}
