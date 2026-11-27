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

fn app_at_fixture_size(child: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(scene(child)).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    runtime
}

fn check_custom_actions_in_focus_block_scope(
    mode: AccessibilityFocusBlockType,
    expected_child_blocked: bool,
) {
    use incular_semantics::CustomSemanticsAction;

    let actions = [
        CustomSemanticsAction::new("action1"),
        CustomSemanticsAction::new("action2"),
    ];
    let calls = Rc::new(Cell::new(0));
    let children = std::array::from_fn::<_, 2, _>(|index| {
        let call_count = calls.clone();
        let mut child = Semantics::new(SizedBox::square(10.)).container(true);
        if index == 0 {
            child = child.accessibility_focus_block_type(AccessibilityFocusBlockType::None);
        }
        child.custom_action(actions[index].clone(), move || {
            call_count.set(call_count.get() + 1)
        })
    });
    let mut runtime = app_at_fixture_size(
        Semantics::new(Column::new(children))
            .container(true)
            .accessibility_focus_block_type(mode),
    );
    let tree = runtime.tree().semantics();
    let [parent] = tree.roots() else {
        panic!("one scene semantic root");
    };
    let parent = tree.node(*parent).unwrap();
    assert!(parent.state.accessibility_focus_block_type.blocks_focus());
    assert!(parent.actions.is_empty());
    let children: [SemanticNodeId; 2] = parent.children.as_slice().try_into().unwrap();
    for (child, action) in children.into_iter().zip(actions) {
        let node = runtime.tree().semantics().node(child).unwrap();
        assert_eq!(
            node.state.accessibility_focus_block_type.blocks_focus(),
            expected_child_blocked
        );
        assert_eq!(node.actions.as_slice(), &[SemanticActionKind::Custom]);
        assert_eq!(
            node.state.custom_actions.as_ref(),
            &[action.get_identifier()]
        );
        assert!(node.children.is_empty());
        assert!(
            runtime
                .dispatch_semantic_action(child, SemanticAction::Custom(action.get_identifier()))
        );
    }
    assert_eq!(calls.get(), 2);
}

// widgets/semantics_test.dart:875
#[test]
fn subtree_focus_block_retains_both_custom_actions_and_blocks_both_children() {
    check_custom_actions_in_focus_block_scope(AccessibilityFocusBlockType::BlockSubtree, true);
}

// widgets/semantics_test.dart:940
#[test]
fn node_focus_block_retains_both_custom_actions_without_blocking_children() {
    check_custom_actions_in_focus_block_scope(AccessibilityFocusBlockType::BlockNode, false);
}

// widgets/semantics_test.dart:995
#[test]
fn explicit_merge_absorbs_a_block_node_flag_and_both_labels() {
    let child = |label| {
        Semantics::new(SizedBox::square(10.))
            .container(true)
            .label(label)
    };
    let runtime = app_at_fixture_size(MergeSemantics::new(Column::new([
        child("node1").accessibility_focus_block_type(AccessibilityFocusBlockType::BlockNode),
        child("node2"),
    ])));
    let tree = runtime.tree().semantics();
    let merged = semantic_id(&runtime, "node1\nnode2");
    assert_eq!(tree.roots(), &[merged]);
    let node = tree.node(merged).unwrap();
    assert!(node.children.is_empty());
    assert!(node.state.accessibility_focus_block_type.blocks_focus());
}

fn implicit_focus_block_scene(mode: AccessibilityFocusBlockType) -> Widget {
    let child = |label| Semantics::new(SizedBox::square(10.)).label(label);
    Semantics::new(
        Semantics::new(Column::new([
            child("semantics label 1"),
            child("semantics label 2"),
        ]))
        .label("semantics label 0")
        .accessibility_focus_block_type(mode),
    )
    .label("root")
    .into()
}

// widgets/semantics_test.dart:1043
#[test]
fn implicit_node_block_does_not_absorb_unblocked_parent_or_children() {
    let runtime = app_at_fixture_size(implicit_focus_block_scene(
        AccessibilityFocusBlockType::BlockNode,
    ));
    let tree = runtime.tree().semantics();
    let root = semantic_id(&runtime, "root");
    let blocked = semantic_id(&runtime, "semantics label 0");
    let children = [
        semantic_id(&runtime, "semantics label 1"),
        semantic_id(&runtime, "semantics label 2"),
    ];
    assert_eq!(tree.roots(), &[root]);
    assert_eq!(tree.node(root).unwrap().children.as_slice(), &[blocked]);
    assert_eq!(tree.node(blocked).unwrap().children.as_slice(), &children);
    assert!(
        tree.node(blocked)
            .unwrap()
            .state
            .accessibility_focus_block_type
            .blocks_focus()
    );
    for id in [root, children[0], children[1]] {
        assert!(
            !tree
                .node(id)
                .unwrap()
                .state
                .accessibility_focus_block_type
                .blocks_focus()
        );
    }
}

// widgets/semantics_test.dart:1098
#[test]
fn implicit_subtree_block_absorbs_compatible_inherited_blocked_labels_but_not_parent() {
    let runtime = app_at_fixture_size(implicit_focus_block_scene(
        AccessibilityFocusBlockType::BlockSubtree,
    ));
    let tree = runtime.tree().semantics();
    let root = semantic_id(&runtime, "root");
    assert_eq!(tree.roots(), &[root]);
    assert!(
        !tree
            .node(root)
            .unwrap()
            .state
            .accessibility_focus_block_type
            .blocks_focus()
    );
    let [blocked] = tree.node(root).unwrap().children.as_slice() else {
        panic!("one blocked branch");
    };
    let blocked = tree.node(*blocked).unwrap();
    assert_eq!(
        blocked.label.as_deref(),
        Some("semantics label 0\nsemantics label 1\nsemantics label 2")
    );
    assert!(blocked.children.is_empty());
    assert!(blocked.state.accessibility_focus_block_type.blocks_focus());
}

#[test]
fn blocked_label_merge_refreshes_after_edits_and_preserves_explicit_children() {
    let mode = AccessibilityFocusBlockType::BlockSubtree;
    let build = |label: &'static str, explicit| {
        let first = Semantics::new(SizedBox::square(10.))
            .label(label)
            .container(explicit);
        let boundary = Semantics::new(SizedBox::square(10.))
            .label("boundary")
            .container(true);
        let last = Semantics::new(SizedBox::square(10.)).label("last");
        Semantics::new(Column::new([first, boundary, last]))
            .label("blocked")
            .accessibility_focus_block_type(mode)
    };
    let mut runtime = app_at_fixture_size(build("first", false));
    let id = semantic_id(&runtime, "blocked\nfirst\nlast");
    let boundary = semantic_id(&runtime, "boundary");
    let root = runtime.tree().root().unwrap();
    for (label, explicit, merged) in [
        ("first", false, "blocked\nfirst\nlast"),
        ("updated", false, "blocked\nupdated\nlast"),
        ("updated", true, "blocked\nlast"),
        ("again", false, "blocked\nagain\nlast"),
    ] {
        runtime
            .tree_mut()
            .update(root, scene(build(label, explicit)))
            .unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        assert_eq!(semantic_id(&runtime, merged), id);
        assert_eq!(semantic_id(&runtime, "boundary"), boundary);
        let tree = runtime.tree().semantics();
        let children = &tree.node(id).unwrap().children;
        if explicit {
            assert_eq!(children, &[semantic_id(&runtime, label), boundary]);
        } else {
            assert_eq!(children, &[boundary]);
        }
        assert!(
            tree.node(boundary)
                .unwrap()
                .state
                .accessibility_focus_block_type
                .blocks_focus()
        );
    }
}

#[test]
fn semantic_geometry_rejects_unknown_windows_and_nodes_independently() {
    use incular_core::ArenaId;
    use incular_platform::WindowId;
    use incular_runtime::{Application, SemanticViewLookupError as Error};

    let mut runtime = Runtime::new(scene(Align::new(
        Alignment::TOP_LEFT,
        Semantics::new(SizedBox::new().width(100.).height(50.)).label("target"),
    )))
    .unwrap();
    let _semantics = runtime.ensure_semantics();
    let constraints = Constraints::tight(Size::new(400., 300.));
    runtime.run_frame(constraints).unwrap();
    let target = semantic_id(&runtime, "target");
    assert_eq!(
        runtime
            .tree()
            .semantics()
            .node(target)
            .unwrap()
            .label
            .as_deref(),
        Some("target")
    );

    let app = Application::from_runtime(runtime, |_| {});
    let view = app.primary_window();
    let unknown_view = WindowId::from_parts(999, view.generation());
    let absent = SemanticNodeId(ArenaId::from_parts(u32::MAX, 0));
    assert_eq!(
        app.semantic_rect_in_view(view, target),
        Ok(Rect::from_origin_size(Offset::ZERO, Size::new(100., 50.)))
    );
    assert_eq!(
        app.semantic_rect_in_view(unknown_view, target),
        Err(Error::UnknownView(unknown_view))
    );
    assert_eq!(
        app.semantic_rect_in_view(view, absent),
        Err(Error::UnknownNode { view, node: absent })
    );
    let alias_with_wrong_generation = WindowId::from_parts(view.index(), view.generation() + 1);
    assert_eq!(
        app.semantic_rect_in_view(alias_with_wrong_generation, target),
        Err(Error::UnknownView(alias_with_wrong_generation))
    );
}

#[test]
fn native_tree_binding_receives_enabled_state_from_before_registration() {
    let mut runtime =
        Runtime::new(scene(Semantics::new(SizedBox::new()).label("initial"))).unwrap();
    runtime.set_platform_semantics_enabled(false);
    runtime.set_platform_semantics_enabled(true);
    let enabled = Rc::new(Cell::new(false));
    let bridge = enabled.clone();
    let _platform = runtime.bind_semantics_tree_enabled(move |value| bridge.set(value));
    assert!(enabled.get());
    assert!(runtime.semantics_enabled());
}

#[test]
fn semantics_handle_synchronously_toggles_the_bound_native_tree() {
    use std::cell::RefCell;

    let mut runtime = Runtime::new(scene(Semantics::new(SizedBox::new()).label("native"))).unwrap();
    runtime.set_platform_semantics_enabled(false);
    let state = Rc::new(Cell::new(true));
    let transitions = Rc::new(RefCell::new(Vec::new()));
    let bridge = state.clone();
    let observed = transitions.clone();
    let _platform = runtime.bind_semantics_tree_enabled(move |enabled| {
        bridge.set(enabled);
        observed.borrow_mut().push(enabled);
    });
    assert!(!state.get());
    let handle = runtime.ensure_semantics();
    assert!(state.get());
    let second_handle = runtime.ensure_semantics();
    drop(handle);
    assert!(state.get());
    drop(second_handle);
    assert!(!state.get());
    assert_eq!(*transitions.borrow(), [false, true, false]);

    runtime.process_runtime_work();
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    assert!(runtime.tree().semantics().root().is_none());
}
