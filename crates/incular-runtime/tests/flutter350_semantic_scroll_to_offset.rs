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
