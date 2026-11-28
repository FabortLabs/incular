//! Pinned Flutter semantic update behavior at Incular's two native seams.

use accesskit::{Action, ActionData, ActionRequest, NodeId, Role as AccessKitRole, TreeId};
use accesskit_consumer::{Node, NodeId as ConsumerId, Tree, TreeChangeHandler};
use incular_accessibility::{
    AccessKitProjection, AccessKitUpdateKind, MobileAccessibilityProjection,
};
use incular_config::{Constraints, TextDirection, TransientPresentation};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{CustomSemanticsAction, Role, SemanticAction, SemanticNodeId};
use incular_widgets::{
    Directionality, MergeSemantics, OverlayPortal, Padding, Semantics, SizedBox, Text, Widget,
    internal::ActionSurface,
};
use std::{cell::Cell, rc::Rc};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn scene(widget: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, widget).into()
}

fn frame(runtime: &mut Runtime) {
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
}

fn semantic_id(runtime: &Runtime, label: &str) -> SemanticNodeId {
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.label.as_deref() == Some(label)).then_some(id))
        .expect("visible labelled semantic node")
}

fn patch(runtime: &mut Runtime, widget: Widget) {
    let root = runtime.tree().root().unwrap();
    runtime.tree_mut().update(root, widget).unwrap();
    frame(runtime);
}

#[test]
fn only_the_retained_merged_node_is_republished_when_an_inner_label_changes() {
    let merged = |inner| {
        scene(MergeSemantics::new(
            Semantics::new(Padding::zero(
                Semantics::new(Padding::zero(Text::new("text")))
                    .label(inner)
                    .container(true),
            ))
            .label("outer")
            .role(Role::Group),
        ))
    };
    let mut runtime = Runtime::new(merged("inner")).unwrap();
    frame(&mut runtime);
    let mut accesskit = AccessKitProjection::new();
    let mut mobile = MobileAccessibilityProjection::new();
    let merged_id = semantic_id(&runtime, "outer\ninner\ntext");
    let semantic = runtime.tree().semantics();
    let initial = accesskit.sync(semantic, 1.).unwrap().into_accesskit();
    let native_id = NodeId(accesskit.native_node_id(merged_id).unwrap());
    assert_eq!(initial.nodes.len(), 2);
    let native_tree = Tree::new(initial, true);
    let children = native_tree.state().root().children().collect::<Vec<_>>();
    assert_eq!(children.len(), 1);
    assert_eq!(
        children[0].id(),
        native_tree
            .state()
            .node_by_tree_local_id(native_id, TreeId::ROOT)
            .unwrap()
            .id(),
    );
    assert_eq!(children[0].label().as_deref(), Some("outer\ninner\ntext"));
    assert_eq!(children[0].children().count(), 0);

    let initial_mobile = mobile.sync(semantic).unwrap();
    assert_eq!(initial_mobile.nodes.len(), 1);
    let mobile_id = initial_mobile.nodes[0].native_id;
    assert_eq!(initial_mobile.roots, [mobile_id]);
    assert!(initial_mobile.nodes[0].children.is_empty());

    patch(&mut runtime, merged("inner-updated"));
    assert_eq!(
        semantic_id(&runtime, "outer\ninner-updated\ntext"),
        merged_id
    );
    let semantic = runtime.tree().semantics();
    let desktop_patch = accesskit.sync(semantic, 1.).unwrap();
    assert_eq!(desktop_patch.kind(), AccessKitUpdateKind::Incremental);
    let nodes = desktop_patch.into_accesskit().nodes;
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].0, native_id);
    assert_eq!(nodes[0].1.label(), Some("outer\ninner-updated\ntext"));
    assert!(nodes[0].1.children().is_empty());
    let mobile_patch = mobile.sync(semantic).unwrap();
    assert_eq!(mobile_patch.nodes.len(), 1);
    assert_eq!(mobile_patch.nodes[0].native_id, mobile_id);
    assert_eq!(
        mobile_patch.nodes[0].semantic.label.as_deref(),
        Some("outer\ninner-updated\ntext")
    );
    assert!(mobile_patch.nodes[0].children.is_empty());
    assert!(mobile_patch.removed.is_empty());
    assert!(accesskit.sync(semantic, 1.).is_none());
    assert!(mobile.sync(semantic).is_none());
}

#[derive(Default)]
struct Changes {
    removed: Vec<ConsumerId>,
}

impl TreeChangeHandler for Changes {
    fn node_added(&mut self, _: &Node) {}
    fn node_updated(&mut self, _: &Node, _: &Node) {}
    fn focus_moved(&mut self, _: Option<&Node>, _: Option<&Node>) {}
    fn node_removed(&mut self, node: &Node) {
        self.removed.push(node.id());
    }
}

fn contains(node: Node<'_>, sought: ConsumerId) -> bool {
    node.id() == sought || node.children().any(|child| contains(child, sought))
}

#[test]
fn detached_overlay_nodes_leave_native_traversal_and_cannot_receive_stale_actions() {
    let calls = Rc::new(Cell::new(0));
    let portal = |show| {
        scene(
            OverlayPortal::new(ActionSurface::new("anchor").size(Size::new(40., 20.)))
                .presentation(TransientPresentation::Overlay)
                .overlay_child(
                    ActionSurface::new("menu item")
                        .size(Size::new(80., 40.))
                        .on_press({
                            let calls = calls.clone();
                            move || calls.set(calls.get() + 1)
                        }),
                )
                .show(show),
        )
    };
    let mut runtime = Runtime::new(portal(true)).unwrap();
    frame(&mut runtime);
    let anchor = semantic_id(&runtime, "anchor");
    let menu = semantic_id(&runtime, "menu item");
    let mut accesskit = AccessKitProjection::new();
    let mut mobile = MobileAccessibilityProjection::new();
    let semantic = runtime.tree().semantics();
    let initial = accesskit.sync(semantic, 1.).unwrap().into_accesskit();
    let anchor_native = NodeId(accesskit.native_node_id(anchor).unwrap());
    let menu_native = NodeId(accesskit.native_node_id(menu).unwrap());
    let mut consumer = Tree::new(initial, true);
    let consumer_id = |id| {
        consumer
            .state()
            .node_by_tree_local_id(id, TreeId::ROOT)
            .unwrap()
            .id()
    };
    let anchor_consumer = consumer_id(anchor_native);
    let menu_consumer = consumer_id(menu_native);
    assert!(contains(consumer.state().root(), anchor_consumer));
    assert!(contains(consumer.state().root(), menu_consumer));
    mobile.sync(semantic).unwrap();
    let menu_mobile = mobile.native_node_id(menu).unwrap();
    assert!(mobile.native_node_id(anchor).is_some());
    let request = ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: menu_native,
        data: None,
    };
    let queued_before_detach = accesskit.translate_action(&request).unwrap();
    assert!(runtime.dispatch_semantic_action(
        queued_before_detach.node,
        queued_before_detach.action.clone()
    ));
    assert_eq!(calls.get(), 1);

    patch(&mut runtime, portal(false));
    let semantic = runtime.tree().semantics();
    assert!(semantic.node(menu).is_none());
    let update = accesskit.sync(semantic, 1.).unwrap().into_accesskit();
    assert!(
        update
            .nodes
            .iter()
            .all(|(id, node)| *id != menu_native && !node.children().contains(&menu_native))
    );
    assert!(
        update
            .nodes
            .iter()
            .any(|(_, node)| node.children() == [anchor_native])
    );
    let mut changes = Changes::default();
    consumer.update_and_process_changes(update, &mut changes);
    assert!(changes.removed.contains(&menu_consumer));
    assert!(contains(consumer.state().root(), anchor_consumer));
    assert!(!contains(consumer.state().root(), menu_consumer));
    assert!(accesskit.translate_action(&request).is_none());
    assert!(
        !runtime.dispatch_semantic_action(queued_before_detach.node, queued_before_detach.action)
    );
    assert_eq!(calls.get(), 1);
    let update = mobile.sync(runtime.tree().semantics()).unwrap();
    assert!(update.removed.contains(&menu_mobile));
    assert!(
        update
            .nodes
            .iter()
            .all(|node| !node.children.contains(&menu_mobile))
    );
}

#[test]
fn replacing_only_a_custom_action_republishes_the_node_and_invalidates_the_old_action() {
    let first = CustomSemanticsAction::new("Action 1");
    let second = CustomSemanticsAction::new("Action 2");
    let first_id = first.get_identifier();
    let second_id = second.get_identifier();
    let callbacks = [Rc::new(Cell::new(0)), Rc::new(Cell::new(0))];
    let subject = |action, slot: usize| {
        let calls = callbacks[slot].clone();
        scene(
            Semantics::new(SizedBox::square(10.))
                .container(true)
                .label("target")
                .custom_action(action, move || calls.set(calls.get() + 1)),
        )
    };
    let mut runtime = Runtime::new(subject(first, 0)).unwrap();
    frame(&mut runtime);
    let node = semantic_id(&runtime, "target");
    let mut accesskit = AccessKitProjection::new();
    let mut mobile = MobileAccessibilityProjection::new();
    let initial = runtime.tree().semantics();
    let tree_update = accesskit.sync(initial, 1.).unwrap().into_accesskit();
    let native = NodeId(accesskit.native_node_id(node).unwrap());
    let first_patch = tree_update
        .nodes
        .iter()
        .find(|(id, _)| *id == native)
        .unwrap();
    assert_eq!(
        first_patch.1.custom_actions()[0].description.as_ref(),
        "Action 1"
    );
    assert!(accesskit.sync(initial, 1.).is_none());
    assert!(mobile.sync(initial).is_some());
    assert!(mobile.sync(initial).is_none());
    let mobile_id = mobile.native_node_id(node).unwrap();
    let request = |raw| ActionRequest {
        action: Action::CustomAction,
        target_tree: TreeId::ROOT,
        target_node: native,
        data: Some(ActionData::CustomAction(raw)),
    };
    let old_request = request(i32::try_from(first_id.0).unwrap());
    let queued = accesskit.translate_action(&old_request).unwrap();
    assert_eq!(queued.action, SemanticAction::Custom(first_id));

    patch(&mut runtime, subject(second, 1));
    assert_eq!(semantic_id(&runtime, "target"), node);
    let semantic = runtime.tree().semantics();
    let changed = accesskit.sync(semantic, 1.).unwrap().into_accesskit();
    assert_eq!(changed.nodes.len(), 1);
    assert_eq!(changed.nodes[0].0, native);
    assert_eq!(changed.nodes[0].1.custom_actions().len(), 1);
    assert_eq!(
        changed.nodes[0].1.custom_actions()[0].description.as_ref(),
        "Action 2"
    );
    let changed = mobile.sync(semantic).unwrap();
    assert_eq!(changed.nodes.len(), 1);
    assert_eq!(changed.nodes[0].native_id, mobile_id);
    assert_eq!(
        changed.nodes[0].semantic.state.custom_actions.as_ref(),
        &[second_id]
    );
    assert!(
        mobile
            .translate_action(mobile_id, SemanticAction::Custom(first_id))
            .is_none()
    );
    assert!(accesskit.translate_action(&old_request).is_none());
    assert!(!runtime.dispatch_semantic_action(queued.node, queued.action));

    let new_request = request(i32::try_from(second_id.0).unwrap());
    let mapped = accesskit.translate_action(&new_request).unwrap();
    assert!(runtime.dispatch_semantic_action(mapped.node, mapped.action));
    assert_eq!(callbacks.each_ref().map(|calls| calls.get()), [0, 1]);
}

#[test]
fn changing_only_the_role_marks_a_node_changed_and_publishes_the_new_role() {
    let subject = |role| {
        scene(
            Semantics::new(SizedBox::square(10.))
                .label("target")
                .role(role),
        )
    };
    let mut runtime = Runtime::new(subject(Role::GenericContainer)).unwrap();
    frame(&mut runtime);
    let mut accesskit = AccessKitProjection::new();
    let id = semantic_id(&runtime, "target");
    let tree = runtime.tree().semantics();
    let initial_revision = tree.revision();
    let initial = accesskit.sync(tree, 1.).unwrap().into_accesskit();
    let native = NodeId(accesskit.native_node_id(id).unwrap());
    assert_eq!(tree.node(id).unwrap().role, Role::GenericContainer);
    let original = initial.nodes.iter().find(|(id, _)| *id == native).unwrap();
    assert_eq!(original.1.role(), AccessKitRole::GenericContainer);
    assert!(accesskit.sync(tree, 1.).is_none());

    patch(&mut runtime, subject(Role::Tab));
    let tree = runtime.tree().semantics();
    assert_eq!(tree.node(id).unwrap().role, Role::Tab);
    assert_eq!(
        tree.changed_since(initial_revision)
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        [id]
    );
    let update = accesskit.sync(tree, 1.).unwrap().into_accesskit();
    assert_eq!(update.nodes.len(), 1);
    assert_eq!(update.nodes[0].0, native);
    assert_eq!(update.nodes[0].1.role(), AccessKitRole::Tab);
    assert!(accesskit.sync(tree, 1.).is_none());
}

#[test]
fn a_merged_native_label_tracks_an_opposing_descendants_retained_direction() {
    let subject = |outer, inner| {
        scene(MergeSemantics::new(
            Semantics::new(Padding::zero(Directionality::new(
                inner,
                Semantics::new(SizedBox::square(10.))
                    .label("مرحبا")
                    .container(true),
            )))
            .label(outer)
            .role(Role::Group),
        ))
    };
    let mut runtime = Runtime::new(subject("Welcome", TextDirection::Rtl)).unwrap();
    frame(&mut runtime);
    let node = runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.role == Role::Group).then_some(id))
        .unwrap();
    let mut accesskit = AccessKitProjection::new();
    let mut mobile = MobileAccessibilityProjection::new();
    let sem = runtime.tree().semantics();
    assert_eq!(
        sem.node(node).unwrap().label.as_deref(),
        Some("Welcome\n\u{202b}مرحبا\u{202c}")
    );
    let output = accesskit.sync(sem, 1.).unwrap().into_accesskit();
    let native_id = NodeId(accesskit.native_node_id(node).unwrap());
    assert!(
        output
            .nodes
            .iter()
            .any(|(id, n)| *id == native_id && n.label() == Some("Welcome\n\u{202b}مرحبا\u{202c}"))
    );
    assert_eq!(
        mobile.sync(sem).unwrap().nodes[0].semantic.label.as_deref(),
        Some("Welcome\n\u{202b}مرحبا\u{202c}")
    );
    let mobile_id = mobile.native_node_id(node).unwrap();

    for (outer, dir, expected) in [
        ("Welcome", TextDirection::Ltr, "Welcome\nمرحبا"),
        ("", TextDirection::Rtl, "\u{202b}مرحبا\u{202c}"),
    ] {
        patch(&mut runtime, subject(outer, dir));
        let sem = runtime.tree().semantics();
        assert_eq!(sem.node(node).unwrap().label.as_deref(), Some(expected));
        let update = accesskit.sync(sem, 1.).unwrap().into_accesskit();
        assert_eq!(update.nodes.len(), 1);
        assert_eq!(update.nodes[0].0, native_id);
        assert_eq!(update.nodes[0].1.label(), Some(expected));
        let update = mobile.sync(sem).unwrap();
        assert_eq!(update.nodes.len(), 1);
        assert_eq!(update.nodes[0].native_id, mobile_id);
        assert_eq!(update.nodes[0].semantic.label.as_deref(), Some(expected));
    }
}

#[test]
fn explicit_reading_direction_overrides_ambient_and_reaches_both_projections() {
    let subject = |direction| {
        scene(
            Semantics::new(SizedBox::square(10.))
                .label("مرحبا")
                .role(Role::Group)
                .text_direction(direction),
        )
    };
    let mut runtime = Runtime::new(subject(TextDirection::Rtl)).unwrap();
    frame(&mut runtime);
    let node = semantic_id(&runtime, "مرحبا");
    let mut accesskit = AccessKitProjection::new();
    let mut mobile = MobileAccessibilityProjection::new();
    let before = runtime.tree().semantics();
    let native_before = accesskit.sync(before, 1.).unwrap().into_accesskit();
    let native_id = NodeId(accesskit.native_node_id(node).unwrap());
    assert_eq!(
        native_before
            .nodes
            .iter()
            .find(|(id, _)| *id == native_id)
            .unwrap()
            .1
            .text_direction(),
        Some(accesskit::TextDirection::RightToLeft)
    );
    let mobile_before = mobile.sync(before).unwrap();
    assert_eq!(
        mobile_before.nodes[0].semantic.state.text_direction,
        Some(TextDirection::Rtl)
    );

    patch(&mut runtime, subject(TextDirection::Ltr));
    let after = runtime.tree().semantics();
    let native_after = accesskit.sync(after, 1.).unwrap().into_accesskit();
    assert_eq!(native_after.nodes.len(), 1);
    assert_eq!(native_after.nodes[0].0, native_id);
    assert_eq!(
        native_after.nodes[0].1.text_direction(),
        Some(accesskit::TextDirection::LeftToRight)
    );
    assert_eq!(
        mobile.sync(after).unwrap().nodes[0]
            .semantic
            .state
            .text_direction,
        Some(TextDirection::Ltr)
    );
}

#[test]
fn requesting_the_same_semantics_recomputes_without_republishing() {
    use incular_core::{Offset, Rect};
    use incular_rendering::DisplayList;
    use incular_widgets::{CustomPaint, CustomPainter, CustomPainterSemantics, SemanticsBuilder};

    struct LabelPainter {
        label: &'static str,
        describes: Rc<Cell<usize>>,
    }

    impl CustomPainter for LabelPainter {
        fn paint(&self, _: Size) -> DisplayList {
            DisplayList::default()
        }

        fn should_repaint(&self, _: &Self) -> bool {
            false
        }

        fn should_rebuild_semantics(&self, _: &Self) -> bool {
            true
        }

        fn semantics_builder(&self) -> Option<SemanticsBuilder> {
            let label = self.label;
            let describes = self.describes.clone();
            Some(Rc::new(move |size| {
                describes.set(describes.get() + 1);
                vec![
                    CustomPainterSemantics::new(
                        Rect::from_origin_size(Offset::ZERO, size),
                        Semantics::builder().label(label).build(),
                    )
                    .key(1_u64),
                ]
            }))
        }
    }

    let describes = Rc::new(Cell::new(0));
    let build = |label| {
        scene(SizedBox::square(20.).child(CustomPaint::from_painter(
            Size::new(20., 20.),
            LabelPainter {
                label,
                describes: describes.clone(),
            },
        )))
    };
    let mut runtime = Runtime::new(build("hello")).unwrap();
    let mut desktop = AccessKitProjection::new();
    let mut mobile = MobileAccessibilityProjection::new();
    frame(&mut runtime);
    assert!(describes.replace(0) > 0);
    let id = semantic_id(&runtime, "hello");
    let semantics = runtime.tree().semantics();
    let revision = semantics.revision();
    assert!(desktop.sync(semantics, 1.).is_some());
    assert!(mobile.sync(semantics).is_some());

    patch(&mut runtime, build("hello"));
    assert!(describes.replace(0) > 0);
    let semantics = runtime.tree().semantics();
    assert_eq!(semantics.revision(), revision);
    assert!(desktop.sync(semantics, 1.).is_none());
    assert!(mobile.sync(semantics).is_none());

    patch(&mut runtime, build("bye"));
    assert!(describes.get() > 0);
    assert_eq!(semantic_id(&runtime, "bye"), id);
    let semantics = runtime.tree().semantics();
    let desktop_patch = desktop.sync(semantics, 1.).unwrap().into_accesskit();
    assert_eq!(desktop_patch.nodes.len(), 1);
    assert_eq!(desktop_patch.nodes[0].1.label(), Some("bye"));
    let mobile_patch = mobile.sync(semantics).unwrap();
    assert_eq!(mobile_patch.nodes.len(), 1);
    assert_eq!(mobile_patch.nodes[0].semantic.label.as_deref(), Some("bye"));
}

#[test]
fn unresolved_named_child_does_not_reach_either_native_accessibility_order() {
    use incular_core::Offset;
    use incular_widgets::Column;

    let mut runtime = Runtime::new(scene(Column::new([
        Widget::from(
            Semantics::new(SizedBox::new().width(10.).height(10.))
                .traversal_child_identifier("111"),
        ),
        SizedBox::new().width(10.).height(10.).into(),
    ])))
    .unwrap();
    frame(&mut runtime);
    let tree = runtime.tree();
    assert!(tree.semantics().roots().is_empty());
    assert_eq!(tree.semantics().iter().count(), 0);
    assert!(
        tree.semantic_action_target_at(Offset::new(1., 1.), &SemanticAction::Activate)
            .is_none()
    );
    let mut accesskit = AccessKitProjection::new();
    let native = accesskit
        .sync(tree.semantics(), 1.)
        .unwrap()
        .into_accesskit();
    let native_tree = Tree::new(native, true);
    assert_eq!(native_tree.state().root().children().count(), 0);
    let mut mobile = MobileAccessibilityProjection::new();
    let update = mobile.sync(tree.semantics()).unwrap();
    assert!(update.roots.is_empty() && update.nodes.is_empty());
}

#[test]
fn named_traversal_uses_logical_ancestry_without_moving_the_hit_target() {
    use incular_core::Offset;
    use incular_widgets::{Positioned, Stack};

    let build = |host_present, child_named| {
        let child = Semantics::new(SizedBox::shrink())
            .container(true)
            .label("outside")
            .on_tap(|| {});
        let child = if child_named {
            child.traversal_child_identifier("owner")
        } else {
            child
        };
        let mut items = vec![Widget::from(
            Positioned::new(Widget::from(child).with_key("child"))
                .left(140.)
                .top(110.)
                .width(30.)
                .height(30.),
        )];
        if host_present {
            let owner = Semantics::new(SizedBox::shrink())
                .container(true)
                .label("owner")
                .traversal_parent_identifier("owner");
            items.push(
                Positioned::new(Widget::from(owner).with_key("host"))
                    .left(10.)
                    .top(10.)
                    .width(25.)
                    .height(25.)
                    .into(),
            );
        }
        scene(
            Semantics::new(Stack::new(items))
                .label("root")
                .container(true)
                .explicit_child_nodes(true),
        )
    };
    let mut runtime = Runtime::new(build(true, true)).unwrap();
    frame(&mut runtime);
    let host = semantic_id(&runtime, "owner");
    let child = semantic_id(&runtime, "outside");
    let root = semantic_id(&runtime, "root");
    let tree = runtime.tree();
    assert_eq!(tree.semantics().node(root).unwrap().children, [host]);
    assert_eq!(tree.semantics().node(host).unwrap().children, [child]);
    assert_eq!(
        tree.semantic_inverse_hit_test_children(root).unwrap().len(),
        2
    );
    assert!(
        tree.semantic_inverse_hit_test_children(host)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        tree.semantic_action_target_at(Offset::new(145., 115.), &SemanticAction::Activate),
        Some(child)
    );

    // Removing only the parent removes the child's accessibility exposure
    // immediately, without removing its retained rendered widget.
    patch(&mut runtime, build(false, true));
    let tree = runtime.tree();
    assert!(tree.element_with_key(&"child".into()).is_some());
    assert!(tree.semantics().node(child).is_none());
    assert!(
        tree.semantic_inverse_hit_test_children(root)
            .unwrap()
            .is_empty()
    );

    // Removing the last named annotation restores ordinary ancestry.
    patch(&mut runtime, build(false, false));
    let child = semantic_id(&runtime, "outside");
    let tree = runtime.tree();
    assert_eq!(tree.semantics().node(root).unwrap().children, [child]);
    assert_eq!(
        tree.semantic_inverse_hit_test_children(root).unwrap(),
        [child]
    );
}

#[test]
fn duplicate_targets_and_cycles_cannot_create_detached_native_roots() {
    use incular_widgets::Column;
    let branch = |label, name, destination| {
        let node = Semantics::new(SizedBox::new().width(20.).height(20.))
            .label(label)
            .container(true)
            .traversal_parent_identifier(name);
        let node = if let Some(target) = destination {
            node.traversal_child_identifier(target)
        } else {
            node
        };
        Widget::from(node)
    };
    let mut runtime = Runtime::new(scene(Column::new([
        branch("name-a1", "ambiguous", None),
        branch("name-a2", "ambiguous", None),
        branch("cycle-a", "one", Some("two")),
        branch("cycle-b", "two", Some("one")),
        branch("cannot-attach", "unique", Some("ambiguous")),
    ])))
    .unwrap();
    frame(&mut runtime);
    let semantic = runtime.tree().semantics();
    assert_eq!(semantic.roots().len(), 2);
    assert_eq!(semantic.iter().count(), 2);
    assert!(semantic.iter().all(|(_, node)| node.children.is_empty()));
}
