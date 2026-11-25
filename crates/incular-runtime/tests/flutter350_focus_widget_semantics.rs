//! Focus widget and retained semantics cases pinned to Flutter 3.50.

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_semantics::{SemanticAction, SemanticActionKind, SemanticNodeId};
use incular_widgets::{Center, Directionality, Focus, FocusNode, Text, TextDirection, Widget};

fn scene(node: &FocusNode, direction: TextDirection, child: impl Into<Widget>) -> Widget {
    Directionality::new(direction, Focus::new(child).node(node.clone())).into()
}

fn semantic_text(runtime: &Runtime, text: &str) -> SemanticNodeId {
    let mut nodes = runtime
        .tree()
        .semantics()
        .iter()
        .filter_map(|(id, node)| (node.label.as_deref() == Some(text)).then_some(id));
    let node = nodes.next().expect("text must appear in semantics");
    assert!(
        nodes.next().is_none(),
        "the focus label must not duplicate text"
    );
    node
}

#[test]
fn semantic_focus_action_requests_keyboard_focus_from_the_rtl_labeled_node() {
    let focus = FocusNode::new();
    let view = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(&focus, TextDirection::Rtl, Text::new("Test"))).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    runtime.run_frame(view).unwrap();
    assert!(!focus.has_focus());

    let id = semantic_text(&runtime, "Test");
    let node = runtime.tree().semantics().node(id).unwrap();
    assert_eq!(node.state.text_direction, Some(TextDirection::Rtl));
    assert_eq!(runtime.tree().semantics().roots(), [id]);
    let before = (
        node.state.focusable,
        node.state.focused,
        node.actions.clone(),
    );

    let handled = runtime.dispatch_semantic_action(id, SemanticAction::Focus);
    runtime.run_frame(view).unwrap();
    assert_eq!(semantic_text(&runtime, "Test"), id);
    assert_eq!(
        (
            before,
            handled,
            focus.has_focus(),
            runtime.tree().semantics().node(id).unwrap().state.focused
        ),
        (
            (true, false, vec![SemanticActionKind::Focus]),
            true,
            true,
            true
        ),
    );
}

#[test]
fn focus_semantics_tracks_keyboard_focus_and_permissions_on_the_retained_node() {
    let focus = FocusNode::new();
    let view = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(&focus, TextDirection::Ltr, Text::new("a"))).unwrap();
    runtime.run_frame(view).unwrap();
    let id = semantic_text(&runtime, "a");
    let state = &runtime.tree().semantics().node(id).unwrap().state;
    let initial = (state.focusable, state.focused);
    assert!(!focus.has_focus());

    focus.request_focus();
    runtime.run_frame(view).unwrap();
    assert!(focus.has_focus());
    let focused = runtime.tree().semantics().node(id).unwrap().state.focused;

    focus.set_can_request_focus(false);
    runtime.run_frame(view).unwrap();
    let node = runtime.tree().semantics().node(id).unwrap();
    assert!(!focus.has_focus() && !focus.can_request_focus());
    assert!(!node.state.focusable && !node.state.focused);
    assert!(!node.actions.contains(&SemanticActionKind::Focus));
    assert_eq!((initial, focused), ((true, false), true));
}

#[test]
fn requesting_focus_under_a_zero_area_view_does_not_panic() {
    let focus = FocusNode::new();
    let view = Constraints::tight(Size::ZERO);
    let child = Focus::new(Text::new("X")).node(focus.clone());
    let root: Widget = Directionality::new(TextDirection::Ltr, Center::new(child)).into();
    let mut runtime = Runtime::new(root).unwrap();
    runtime.run_frame(view).unwrap();
    let tree = runtime.tree();
    let mut widget = tree.root().unwrap();
    loop {
        if tree.focus_node_at(widget).as_ref() == Some(&focus) {
            break;
        }
        let [child] = tree.children(widget).expect("bound Focus descendant") else {
            panic!("Fixture is a chain ending in a mounted Focus node");
        };
        widget = *child;
    }
    assert_eq!(tree.element_bounds(widget).unwrap().size, Size::ZERO);
    focus.request_focus();
    runtime.run_frame(view).unwrap();
}
