//! Flutter 3.50 absolute accessibility scroll actions.

use accesskit::{Action, ActionData, ActionRequest, NodeId, Point, TreeId};
use incular_accessibility::AccessKitProjection;
use incular_config::{Axis, Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_scroll::{ScrollController, ScrollPhysics};
use incular_semantics::{Role, SemanticAction, SemanticActionKind, SemanticNodeId};
use incular_widgets::{Directionality, ListView, SizedBox};

fn scene(
    axis: Axis,
    reverse: bool,
    count: usize,
    physics: Option<ScrollPhysics>,
) -> (Runtime, ScrollController, SemanticNodeId) {
    let controller = ScrollController::new();
    let view = ListView::new((0..count).map(|_| SizedBox::square(40.)))
        .item_extent(40.)
        .scroll_direction(axis)
        .reverse(reverse)
        .controller(controller.clone());
    let view = match physics {
        Some(physics) => view.physics(physics),
        None => view,
    };
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, view).into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let id = runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.role == Role::ScrollView).then_some(id))
        .expect("scroll view semantics");
    (runtime, controller, id)
}

#[test]
fn native_absolute_scroll_uses_the_selected_axis_and_preserves_fractional_values() {
    for (axis, reverse, expected) in [
        (Axis::Vertical, false, 456.),
        (Axis::Horizontal, false, 123.),
        (Axis::Vertical, true, 456.),
        (Axis::Horizontal, true, 123.),
    ] {
        let (mut runtime, controller, id) = scene(axis, reverse, 60, None);
        let tree = runtime.tree().semantics();
        let scroll = tree.node(id).unwrap();
        assert!(scroll.state.has_implicit_scrolling);
        assert!(scroll.actions.contains(&SemanticActionKind::ScrollToOffset));
        let first_direction = match (axis, reverse) {
            (Axis::Vertical, false) => SemanticActionKind::ScrollUp,
            (Axis::Vertical, true) => SemanticActionKind::ScrollDown,
            (Axis::Horizontal, false) => SemanticActionKind::ScrollLeft,
            (Axis::Horizontal, true) => SemanticActionKind::ScrollRight,
        };
        assert!(scroll.actions.contains(&first_direction));
        let mut bridge = AccessKitProjection::new();
        bridge.activate();
        let update = bridge.sync(tree, 1.).unwrap().into_accesskit();
        let native = NodeId(bridge.native_node_id(id).unwrap());
        let node = &update.nodes.iter().find(|(id, _)| *id == native).unwrap().1;
        assert!(node.supports_action(Action::SetScrollOffset));
        match axis {
            Axis::Horizontal => {
                assert_eq!(node.scroll_x(), Some(0.));
                assert_eq!(node.scroll_x_max(), Some(1600.));
                assert_eq!(node.scroll_y(), None);
            }
            Axis::Vertical => {
                assert_eq!(node.scroll_y(), Some(0.));
                assert_eq!(node.scroll_y_max(), Some(1800.));
                assert_eq!(node.scroll_x(), None);
            }
        }
        let request = |data| ActionRequest {
            target_node: native,
            target_tree: TreeId::ROOT,
            action: Action::SetScrollOffset,
            data,
        };
        assert!(bridge.translate_action(&request(None)).is_none());
        assert!(
            bridge
                .translate_action(&request(Some(ActionData::SetScrollOffset(Point::new(
                    f64::NAN,
                    0.
                )))))
                .is_none()
        );
        for increment in [0., 0.25] {
            let action = bridge
                .translate_action(&request(Some(ActionData::SetScrollOffset(Point::new(
                    123. + increment,
                    456. + increment,
                )))))
                .unwrap();
            assert_eq!(action.node, id);
            assert!(runtime.dispatch_semantic_action(action.node, action.action));
            assert_eq!(controller.offset(), expected + increment as f32);
        }
    }
}

#[test]
fn absolute_action_requires_both_scrollable_dimensions_and_implicit_scrolling() {
    for (count, physics, implicit) in [
        (3, None, true),
        (
            60,
            Some(ScrollPhysics::CLAMPING.with_implicit_scrolling(false)),
            false,
        ),
    ] {
        let (mut runtime, controller, id) = scene(Axis::Vertical, false, count, physics);
        let node = runtime.tree().semantics().node(id).unwrap();
        assert_eq!(node.state.has_implicit_scrolling, implicit);
        assert!(!node.actions.contains(&SemanticActionKind::ScrollToOffset));
        assert!(
            !runtime
                .dispatch_semantic_action(id, SemanticAction::ScrollToOffset { x: 123., y: 456. })
        );
        assert_eq!(controller.offset(), 0.);
    }
}

#[test]
fn directional_semantic_actions_follow_physical_axes_and_update_at_both_edges() {
    let size = Constraints::tight(Size::new(800., 600.));
    use SemanticAction::{ScrollDown, ScrollLeft, ScrollRight, ScrollUp};
    for (axis, reverse, forward, backward, native_forward, native_backward, step) in [
        (
            Axis::Vertical,
            false,
            ScrollUp,
            ScrollDown,
            Action::ScrollDown,
            Action::ScrollUp,
            480.,
        ),
        (
            Axis::Vertical,
            true,
            ScrollDown,
            ScrollUp,
            Action::ScrollUp,
            Action::ScrollDown,
            480.,
        ),
        (
            Axis::Horizontal,
            false,
            ScrollLeft,
            ScrollRight,
            Action::ScrollRight,
            Action::ScrollLeft,
            640.,
        ),
        (
            Axis::Horizontal,
            true,
            ScrollRight,
            ScrollLeft,
            Action::ScrollLeft,
            Action::ScrollRight,
            640.,
        ),
    ] {
        let (mut runtime, scroll, id) = scene(axis, reverse, 60, None);
        let sem = |rt: &Runtime, positive, negative| {
            let actions = &rt.tree().semantics().node(id).unwrap().actions;
            assert_eq!(actions.contains(&forward.kind()), positive);
            assert_eq!(actions.contains(&backward.kind()), negative);
        };
        sem(&runtime, true, false);
        let mut bridge = AccessKitProjection::new();
        bridge.activate();
        let update = bridge
            .sync(runtime.tree().semantics(), 1.)
            .unwrap()
            .into_accesskit();
        let native_id = NodeId(bridge.native_node_id(id).unwrap());
        let native = &update
            .nodes
            .iter()
            .find(|(id, _)| *id == native_id)
            .unwrap()
            .1;
        assert!(native.supports_action(native_forward));
        assert!(!native.supports_action(native_backward));
        if axis == Axis::Horizontal {
            assert!(!native.supports_action(Action::ScrollUp));
            assert!(!native.supports_action(Action::ScrollDown));
        }
        let req = |action| ActionRequest {
            action,
            target_node: native_id,
            target_tree: TreeId::ROOT,
            data: None,
        };
        assert!(bridge.translate_action(&req(native_backward)).is_none());
        let first = bridge.translate_action(&req(native_forward)).unwrap();
        assert_eq!(first.action, forward);
        assert!(runtime.dispatch_semantic_action(id, first.action));
        runtime.run_frame(size).unwrap();
        assert_eq!(scroll.offset(), step);
        sem(&runtime, true, true);
        bridge.sync(runtime.tree().semantics(), 1.).unwrap();
        let back = bridge.translate_action(&req(native_backward)).unwrap();
        assert_eq!(back.action, backward);
        assert!(runtime.dispatch_semantic_action(id, back.action));
        runtime.run_frame(size).unwrap();
        assert_eq!(scroll.offset(), 0.);
        sem(&runtime, true, false);
        assert!(scroll.jump_to(scroll.max_offset()));
        runtime.run_frame(size).unwrap();
        sem(&runtime, false, true);
        assert!(!runtime.dispatch_semantic_action(id, forward));
    }
}

#[test]
fn ten_list_tiles_withdraw_the_original_direction_after_an_out_of_range_jump() {
    use SemanticActionKind::{ScrollDown, ScrollLeft, ScrollRight, ScrollToOffset, ScrollUp};
    use incular_core::time::Instant;
    use std::time::Duration;
    let size = Constraints::tight(Size::new(800., 600.));
    for (axis, reverse, forward, backward) in [
        (Axis::Vertical, false, ScrollUp, ScrollDown),
        (Axis::Vertical, true, ScrollDown, ScrollUp),
        (Axis::Horizontal, false, ScrollLeft, ScrollRight),
        (Axis::Horizontal, true, ScrollRight, ScrollLeft),
    ] {
        // In the pinned Flutter horizontal case the six-character "Tile 0"
        // uses the Ahem test font at 14px and is unbounded along the axis.
        let extent = if axis == Axis::Vertical { 150. } else { 84. };
        let controller = ScrollController::new();
        let list = ListView::builder(10, |i| incular_widgets::Text::new(format!("Tile {i}")))
            .item_extent(extent)
            .controller(controller.clone())
            .scroll_direction(axis)
            .reverse(reverse);
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, list).into()).unwrap();
        let mut time = Instant::now();
        runtime.run_frame_at(size, time).unwrap();
        let actions = |runtime: &Runtime| {
            runtime
                .tree()
                .semantics()
                .iter()
                .find_map(|(_, node)| (node.role == Role::ScrollView).then_some(node))
                .expect("current scrolling pane")
                .actions
                .iter()
                .copied()
                .filter(|a| a.is_directional_scroll() || *a == ScrollToOffset)
                .collect::<Vec<_>>()
        };
        assert_eq!(actions(&runtime), [ScrollToOffset, forward]);
        assert!(controller.jump_to(10. * 150.));
        for _ in 0..180 {
            time += Duration::from_millis(16);
            runtime.run_frame_at(size, time).unwrap();
            if !runtime.frame_requested() {
                break;
            }
        }
        assert_eq!(
            controller.offset(),
            10. * extent - if axis == Axis::Vertical { 600. } else { 800. }
        );
        assert_eq!(actions(&runtime), [ScrollToOffset, backward]);
    }
}

#[test]
fn fractional_scroll_position_uses_numeric_metadata_and_a_scroll_event_not_value_text() {
    use accesskit::NodeId;
    use incular_accessibility::{MobileAccessibilityEventKind, MobileAccessibilityProjection};
    use incular_widgets::{
        CustomScrollView, Semantics, SingleChildScrollView, Sliver, SliverToBoxAdapter,
    };

    for sliver in [false, true] {
        let controller = ScrollController::new();
        let child = Semantics::new(SizedBox::new().height(1600.)).label("body");
        let view = if sliver {
            let slivers: [Box<dyn Sliver>; 1] = [Box::new(SliverToBoxAdapter::new(child))];
            CustomScrollView::new(slivers)
                .controller(controller.clone())
                .into()
        } else {
            SingleChildScrollView::new(child)
                .controller(controller.clone())
                .into()
        };
        let mut runtime = Runtime::new(view).unwrap();
        let bounds = Constraints::tight(Size::new(800., 600.));
        let mut mobile = MobileAccessibilityProjection::new();
        let mut accesskit = AccessKitProjection::new();
        let mut id = None;

        for offset in [0., 220.25, 220.5, 0.] {
            let _ = controller.jump_to(offset);
            runtime.run_frame(bounds).unwrap();
            let tree = runtime.tree().semantics();
            let scroll = tree
                .iter()
                .map(|(_, node)| node)
                .find(|node| node.state.scroll_metrics.is_some())
                .unwrap();
            assert_eq!(scroll.value, None);
            let metrics = scroll.state.scroll_metrics.unwrap();
            assert_eq!(metrics.offset, f64::from(offset));
            assert_eq!(metrics.maximum, 1000.);
            let snapshot = mobile.sync(tree).unwrap();
            let projected = snapshot
                .nodes
                .iter()
                .find(|n| n.semantic.id == scroll.id)
                .unwrap();
            let native = accesskit.sync(tree, 1.).unwrap().into_accesskit();
            let native_id = NodeId(accesskit.native_node_id(scroll.id).unwrap());
            let native_node = &native
                .nodes
                .iter()
                .find(|(id, _)| *id == native_id)
                .unwrap()
                .1;
            assert_eq!(native_node.value(), None);
            assert_eq!(native_node.scroll_y(), Some(f64::from(offset)));
            assert!(
                snapshot
                    .events
                    .iter()
                    .all(|event| event.kind != MobileAccessibilityEventKind::ValueChanged)
            );
            let events: Vec<_> = snapshot
                .events
                .iter()
                .filter(|event| event.kind == MobileAccessibilityEventKind::Scrolled)
                .map(|event| event.native_id)
                .collect();
            if let Some((old_id, old_native)) = id {
                assert_eq!((scroll.id, projected.native_id), (old_id, old_native));
                assert_eq!(events, [old_native]);
            } else {
                assert!(events.is_empty());
            }
            id = Some((scroll.id, projected.native_id));
        }
    }
}
