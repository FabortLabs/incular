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

#[test]
fn disabling_focus_semantics_does_not_create_a_node_or_remove_keyboard_focus() {
    use incular_widgets::SizedBox;

    let focus = FocusNode::new();
    let view = Constraints::tight(Size::new(800., 600.));
    let root: Widget = Focus::new(SizedBox::shrink())
        .node(focus.clone())
        .include_semantics(false)
        .into();
    let mut runtime = Runtime::new(root).unwrap();
    runtime.run_frame(view).unwrap();
    assert!(runtime.tree().semantics().is_empty());

    focus.request_focus();
    runtime.run_frame(view).unwrap();
    assert!(focus.has_primary_focus());
    assert!(runtime.tree().semantics().is_empty());
}

#[test]
fn ios_exposes_focus_state_without_the_native_focus_action() {
    let focus = FocusNode::new();
    let view = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(&focus, TextDirection::Rtl, Text::new("Test"))).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    runtime.run_frame(view).unwrap();
    let id = semantic_text(&runtime, "Test");
    let sem = |runtime: &Runtime| {
        let node = runtime.tree().semantics().node(id).unwrap();
        (
            node.state.focusable,
            node.state.focused,
            node.actions.clone(),
        )
    };
    assert_eq!(sem(&runtime), (true, false, vec![]));
    assert!(!runtime.dispatch_semantic_action(id, SemanticAction::Focus));
    assert!(!focus.has_focus());

    focus.request_focus();
    runtime.run_frame(view).unwrap();
    assert_eq!(sem(&runtime), (true, true, vec![]));
    focus.set_can_request_focus(false);
    runtime.run_frame(view).unwrap();
    assert_eq!(sem(&runtime), (false, false, vec![]));

    // Permissions also invalidate the semantics while this node is unfocused.
    focus.set_can_request_focus(true);
    runtime.run_frame(view).unwrap();
    assert_eq!(sem(&runtime), (true, false, vec![]));
}

#[test]
fn focus_merges_plain_labels_but_stops_at_an_independent_focus_node() {
    use incular_widgets::Column;

    let outer = FocusNode::new();
    let inner = FocusNode::new();
    let view = Constraints::tight(Size::new(800., 600.));
    let labels = |second: &str| {
        scene(
            &outer,
            TextDirection::Rtl,
            Column::new([Widget::from(Text::new("one")), Text::new(second).into()]),
        )
    };
    let mut runtime = Runtime::new(labels("two")).unwrap();
    runtime.run_frame(view).unwrap();
    let outer_id = semantic_text(&runtime, "one\ntwo");
    assert_eq!(runtime.tree().semantics().roots(), [outer_id]);
    assert_eq!(runtime.tree().semantics().len(), 1);

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, labels("replacement"))
        .unwrap();
    runtime.run_frame(view).unwrap();
    assert_eq!(semantic_text(&runtime, "one\nreplacement"), outer_id);

    runtime
        .schedule_update(
            root,
            scene(
                &outer,
                TextDirection::Rtl,
                Focus::new(Text::new("nested")).node(inner.clone()),
            ),
        )
        .unwrap();
    runtime.run_frame(view).unwrap();
    let inner_id = semantic_text(&runtime, "nested");
    assert_ne!(inner_id, outer_id);
    let tree = runtime.tree().semantics();
    assert_eq!(tree.roots(), [outer_id]);
    let outer_semantics = tree.node(outer_id).unwrap();
    assert_eq!(outer_semantics.children, [inner_id]);
    assert_eq!(outer_semantics.label.as_deref().unwrap_or(""), "");
    assert_eq!(outer_semantics.actions, [SemanticActionKind::Focus]);
    assert_eq!(
        tree.node(inner_id).unwrap().actions,
        [SemanticActionKind::Focus]
    );

    assert!(runtime.dispatch_semantic_action(outer_id, SemanticAction::Focus));
    runtime.run_frame(view).unwrap();
    assert!(outer.has_primary_focus());
    assert!(!inner.has_focus());
    assert!(runtime.dispatch_semantic_action(inner_id, SemanticAction::Focus));
    runtime.run_frame(view).unwrap();
    assert!(inner.has_primary_focus() && outer.has_focus());
    assert!(!outer.has_primary_focus());
    assert!(
        !runtime
            .tree()
            .semantics()
            .node(outer_id)
            .unwrap()
            .state
            .focused
    );
    assert!(
        runtime
            .tree()
            .semantics()
            .node(inner_id)
            .unwrap()
            .state
            .focused
    );
}

#[test]
fn a_separate_semantic_control_keeps_its_own_label_and_activation() {
    use incular_widgets::{Semantics, SizedBox};
    use std::{cell::Cell, rc::Rc};

    let focus = FocusNode::new();
    let activated = Rc::new(Cell::new(false));
    let control = Semantics::new(SizedBox::shrink())
        .label("separate control")
        .container(true)
        .on_tap({
            let activated = activated.clone();
            move || activated.set(true)
        });
    let mut runtime = Runtime::new(scene(&focus, TextDirection::Rtl, control)).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();

    let child_id = semantic_text(&runtime, "separate control");
    let tree = runtime.tree().semantics();
    let [parent_id] = tree.roots() else {
        panic!("focus root")
    };
    let parent = tree.node(*parent_id).unwrap();
    assert_eq!(parent.children, [child_id]);
    assert_eq!(parent.label, None);
    assert_eq!(parent.actions, [SemanticActionKind::Focus]);
    assert_eq!(
        tree.node(child_id).unwrap().actions,
        [SemanticActionKind::Activate]
    );
    assert!(runtime.dispatch_semantic_action(child_id, SemanticAction::Activate));
    assert!(activated.get());
    assert!(!focus.has_focus());
}

#[test]
fn excluding_focus_denies_outer_and_inner_explicit_requests_without_callbacks() {
    use incular_widgets::{ExcludeFocus, SizedBox};
    use std::{cell::Cell, rc::Rc};

    let parent = FocusNode::new();
    let child = FocusNode::new();
    let callback_state = Rc::new(Cell::new(None));
    let widget = ExcludeFocus::new(
        Focus::new(Focus::new(SizedBox::shrink()).node(child.clone()))
            .node(parent.clone())
            .on_focus_change({
                let callback_state = callback_state.clone();
                move |focused| callback_state.set(Some(focused))
            }),
    );
    let mut runtime = Runtime::new(widget.into()).unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(view).unwrap();
    for candidate in [&parent, &child] {
        candidate.request_focus();
        runtime.run_frame(view).unwrap();
        assert!(!parent.has_focus());
        assert!(!child.has_focus());
        assert!(runtime.focused_element().is_none());
        assert_eq!(callback_state.get(), None);
    }
}

#[test]
fn exclude_focus_does_not_add_an_accessibility_node_of_its_own() {
    use incular_widgets::{ExcludeFocus, SizedBox};

    let view = Constraints::tight(Size::new(800., 600.));
    let mut baseline = Runtime::new(SizedBox::shrink().into()).unwrap();
    baseline.run_frame(view).unwrap();
    let mut covered = Runtime::new(ExcludeFocus::new(SizedBox::shrink()).into()).unwrap();
    covered.run_frame(view).unwrap();
    assert_eq!(
        baseline.tree().semantics().roots(),
        covered.tree().semantics().roots()
    );
    assert_eq!(
        baseline.tree().semantics().iter().count(),
        covered.tree().semantics().iter().count()
    );
}
