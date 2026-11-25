//! Public Runtime counterparts to the pinned semantics binding/owner cases.

use incular_config::Constraints;
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset, Rect, Size};
use incular_platform::{PhysicalSize, PlatformEvent, WindowMetrics};
use incular_runtime::Runtime;
use incular_semantics::{
    AccessibilityFocusBlockType, SemanticAction, SemanticActionKind, SemanticNodeId,
};
use incular_widgets::{
    Align, Alignment, Column, Directionality, FocusNode, KeyboardListener, MergeSemantics,
    Semantics, SizedBox, Text, TextDirection, Transform, Widget,
};
use std::{cell::Cell, rc::Rc};

fn scene(child: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, child).into()
}

fn semantic_id(runtime: &Runtime, label: &str) -> SemanticNodeId {
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.label.as_deref() == Some(label)).then_some(id))
        .expect("mounted semantic node")
}

#[test]
fn transformed_semantic_bounds_are_in_logical_view_coordinates() {
    let target = Semantics::new(SizedBox::new().width(100.).height(50.)).label("target");
    let translated = Transform::translation(Offset::new(40., 20.), target);
    let mut runtime = Runtime::new(scene(Align::new(Alignment::TOP_LEFT, translated))).unwrap();
    let metrics = WindowMetrics::new(PhysicalSize::new(800, 600), 2.);
    runtime.handle_platform_event(PlatformEvent::Metrics(metrics));
    assert_eq!(runtime.environment().viewport, Size::new(400., 300.));
    runtime
        .run_frame(Constraints::tight(runtime.environment().viewport))
        .unwrap();

    let target = semantic_id(&runtime, "target");
    assert_eq!(
        runtime.tree().semantics().node(target).unwrap().bounds,
        Rect::from_origin_size(Offset::new(40., 20.), Size::new(100., 50.)),
    );
}

#[test]
fn show_on_screen_ignores_a_stale_semantic_node() {
    let mut runtime = Runtime::new(scene(
        Semantics::new(Text::new("Hello World")).explicit_child_nodes(true),
    ))
    .unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(constraints).unwrap();
    let stale = semantic_id(&runtime, "Hello World");

    let root = runtime.tree().root().unwrap();
    runtime
        .tree_mut()
        .update(root, scene(SizedBox::shrink()))
        .unwrap();
    runtime.run_frame(constraints).unwrap();
    assert!(runtime.tree().semantics().node(stale).is_none());
    assert!(!runtime.dispatch_semantic_action(stale, SemanticAction::ShowOnScreen));
}

#[test]
fn accessibility_focus_block_updates_children_without_changing_real_keyboard_focus() {
    use AccessibilityFocusBlockType::{BlockNode, BlockSubtree, None};

    let focus = FocusNode::new();
    let key_downs = Rc::new(Cell::new(0));
    let activated = Rc::new(Cell::new(0));
    let build = |mode| {
        let control = KeyboardListener::new(SizedBox::new().width(40.).height(40.))
            .node(focus.clone())
            .on_key_event({
                let key_downs = Rc::clone(&key_downs);
                move |_| {
                    key_downs.set(key_downs.get() + 1);
                    true
                }
            });
        let child = Semantics::new(control)
            .label("keyboard child")
            .container(true)
            .focused(true)
            .accessibility_focus_block_type(None)
            .on_tap({
                let activated = Rc::clone(&activated);
                move || activated.set(activated.get() + 1)
            });
        let ordinary = Semantics::new(SizedBox::new().width(40.).height(40.))
            .label("ordinary child")
            .container(true);
        scene(
            Semantics::new(Column::new([Widget::from(child), Widget::from(ordinary)]))
                .label("parent scope")
                .container(true)
                .explicit_child_nodes(true)
                .accessibility_focus_block_type(mode),
        )
    };
    let bounds = Constraints::tight(Size::new(600., 400.));
    let mut runtime = Runtime::new(build(None)).unwrap();
    runtime.run_frame(bounds).unwrap();
    focus.request_focus();
    runtime.run_frame(bounds).unwrap();
    let actual_focus = runtime.focused_element().expect("real keyboard focus");
    let child = semantic_id(&runtime, "keyboard child");
    let ordinary = semantic_id(&runtime, "ordinary child");
    let parent = semantic_id(&runtime, "parent scope");
    for id in [parent, child, ordinary] {
        assert_eq!(
            runtime
                .tree()
                .semantics()
                .node(id)
                .unwrap()
                .state
                .accessibility_focus_block_type,
            None
        );
    }

    for (index, mode) in [BlockNode, BlockSubtree, None].into_iter().enumerate() {
        let root = runtime.tree().root().unwrap();
        runtime.tree_mut().update(root, build(mode)).unwrap();
        runtime.run_frame(bounds).unwrap();
        assert_eq!(semantic_id(&runtime, "keyboard child"), child);
        assert_eq!(semantic_id(&runtime, "ordinary child"), ordinary);
        assert_eq!(semantic_id(&runtime, "parent scope"), parent);
        let tree = runtime.tree().semantics();
        let child_node = tree.node(child).unwrap();
        let inherited = mode == BlockSubtree;
        assert_eq!(
            tree.node(parent)
                .unwrap()
                .state
                .accessibility_focus_block_type,
            mode
        );
        assert_eq!(
            tree.node(ordinary)
                .unwrap()
                .state
                .accessibility_focus_block_type
                .blocks_focus(),
            inherited
        );
        assert_eq!(
            child_node
                .state
                .accessibility_focus_block_type
                .blocks_focus(),
            inherited
        );
        assert_eq!(child_node.state.focused, !inherited);
        assert_eq!(child_node.state.focusable, !inherited);
        assert_eq!(
            child_node.actions.contains(&SemanticActionKind::Focus),
            !inherited
        );
        assert!(child_node.actions.contains(&SemanticActionKind::Activate));
        assert_eq!(
            runtime.dispatch_semantic_action(child, SemanticAction::Focus),
            !inherited
        );
        assert!(runtime.dispatch_semantic_action(child, SemanticAction::Activate));
        assert_eq!(activated.get(), index + 1);
        assert!(focus.has_primary_focus());
        assert_eq!(runtime.focused_element(), Some(actual_focus));
        let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_down(
            KeyboardKey::Named(NamedKey::Unidentified),
            Code::KeyA,
        )));
        assert_eq!(key_downs.get(), index + 1);
    }
}

#[test]
fn a_blocked_semantic_node_clears_explicit_input_focus_metadata_but_keeps_other_actions() {
    let block = AccessibilityFocusBlockType::BlockSubtree;
    let button = Semantics::new(SizedBox::new().width(40.).height(40.))
        .label("explicitly focused")
        .container(true)
        .accessibility_focus_block_type(block)
        .focused(true)
        .on_tap(|| {});
    let mut runtime = Runtime::new(scene(button)).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(600., 400.)))
        .unwrap();
    let id = semantic_id(&runtime, "explicitly focused");
    let node = runtime.tree().semantics().node(id).unwrap();
    assert_eq!(node.state.accessibility_focus_block_type, block);
    assert!(!node.state.focused);
    assert!(node.actions.contains(&SemanticActionKind::Activate));
}

#[test]
fn merged_semantics_keeps_the_strongest_accessibility_focus_block() {
    use AccessibilityFocusBlockType::{BlockNode, BlockSubtree};
    let child = |label, block| {
        Widget::from(
            Semantics::new(SizedBox::new().width(10.).height(10.))
                .label(label)
                .accessibility_focus_block_type(block),
        )
    };
    let mut runtime = Runtime::new(scene(MergeSemantics::new(Column::new([
        child("first", BlockNode),
        child("second", BlockSubtree),
    ]))))
    .unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(600., 400.)))
        .unwrap();
    let node = semantic_id(&runtime, "first\nsecond");
    assert_eq!(
        runtime
            .tree()
            .semantics()
            .node(node)
            .unwrap()
            .state
            .accessibility_focus_block_type,
        BlockSubtree
    );
}
