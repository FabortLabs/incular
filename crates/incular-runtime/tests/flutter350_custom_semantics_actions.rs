//! Pinned Flutter 3.50.0 custom action collection and merged dispatch.

use incular_config::{Constraints, EdgeInsets};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{CustomSemanticsAction, SemanticAction, SemanticNodeId};
use incular_widgets::{Column, MergeSemantics, Padding, Semantics, SizedBox, Widget};
use std::{cell::Cell, rc::Rc};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(100.0, 100.0)))
        .unwrap();
}

fn labelled_node(runtime: &Runtime, label: &str) -> SemanticNodeId {
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.label.as_deref() == Some(label)).then_some(id))
        .expect("custom action semantics node")
}

fn actions(runtime: &Runtime, node: SemanticNodeId) -> Vec<String> {
    let mut names = runtime
        .tree()
        .semantics()
        .node(node)
        .unwrap()
        .state
        .custom_actions
        .iter()
        .map(|id| {
            CustomSemanticsAction::get_action(*id)
                .unwrap()
                .label()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn explicit_semantics_publishes_both_custom_actions() {
    let mut runtime = Runtime::new(
        Semantics::new(SizedBox::shrink())
            .container(true)
            .label("Target")
            .custom_action(CustomSemanticsAction::new("foo"), || {})
            .custom_action(CustomSemanticsAction::new("bar"), || {})
            .into(),
    )
    .unwrap();
    frame(&mut runtime);
    assert_eq!(
        actions(&runtime, labelled_node(&runtime, "Target")),
        ["bar", "foo"]
    );
}

#[test]
fn merge_semantics_publishes_both_sibling_actions() {
    let child = |label, action| {
        Widget::from(
            Semantics::new(SizedBox::shrink())
                .container(true)
                .label(label)
                .custom_action(CustomSemanticsAction::new(action), || {}),
        )
    };
    let mut runtime = Runtime::new(
        MergeSemantics::new(Column::new([
            child("One", "action1"),
            child("Two", "action2"),
        ]))
        .into(),
    )
    .unwrap();
    frame(&mut runtime);
    assert_eq!(
        actions(&runtime, labelled_node(&runtime, "One\nTwo")),
        ["action1", "action2"],
    );
}

#[test]
fn merged_siblings_and_root_dispatch_only_the_requested_callback() {
    let root_action = CustomSemanticsAction::new("root");
    let first_action = CustomSemanticsAction::new("first");
    let second_action = CustomSemanticsAction::new("second");
    let counts = [
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
    ];
    let callback = |index: usize| {
        let count = counts[index].clone();
        move || count.set(count.get() + 1)
    };
    let first = Widget::from(
        Semantics::new(SizedBox::shrink())
            .label("first child")
            .custom_action(first_action.clone(), callback(1)),
    );
    let second = Widget::from(
        Semantics::new(SizedBox::shrink())
            .label("second child")
            .custom_action(second_action.clone(), callback(2)),
    );
    let mut runtime = Runtime::new(
        MergeSemantics::new(
            Semantics::new(Column::new([first, second]))
                .custom_action(root_action.clone(), callback(0)),
        )
        .into(),
    )
    .unwrap();
    frame(&mut runtime);
    let id = labelled_node(&runtime, "first child\nsecond child");
    assert_eq!(actions(&runtime, id), ["first", "root", "second"]);
    for (action, expected) in [
        (second_action, [0, 0, 1]),
        (first_action, [0, 1, 1]),
        (root_action, [1, 1, 1]),
    ] {
        assert!(
            runtime.dispatch_semantic_action(id, SemanticAction::Custom(action.get_identifier()))
        );
        assert_eq!(counts.each_ref().map(|count| count.get()), expected);
    }
    let unrelated = CustomSemanticsAction::new("unrelated").get_identifier();
    assert!(!runtime.dispatch_semantic_action(id, SemanticAction::Custom(unrelated)));
    assert_eq!(counts.each_ref().map(|count| count.get()), [1, 1, 1]);
}

fn check_nested_action_owners(labels: &[&str], order: &[usize]) {
    let mut widget: Widget = Semantics::new(SizedBox::shrink()).label("Test").into();
    let mut bindings = Vec::new();
    for &label in labels.iter().rev() {
        let action = CustomSemanticsAction::new(label);
        let count = Rc::new(Cell::new(0));
        let observed = count.clone();
        // Keep a real retained element per owner even though Incular lowers
        // an annotation directly into its child descriptor.
        widget = Semantics::new(Padding::new(EdgeInsets::ZERO, widget))
            .custom_action(action.clone(), move || observed.set(observed.get() + 1))
            .into();
        bindings.push((action.get_identifier(), count));
    }
    let mut runtime = Runtime::new(MergeSemantics::new(widget).into()).unwrap();
    frame(&mut runtime);
    let node = labelled_node(&runtime, "Test");
    let mut expected = vec![0; labels.len()];
    for &index in order {
        assert!(runtime.dispatch_semantic_action(node, SemanticAction::Custom(bindings[index].0)));
        expected[index] += 1;
        assert_eq!(
            bindings
                .iter()
                .map(|(_, count)| count.get())
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn a_nested_custom_action_routes_to_its_outer_or_inner_owner() {
    check_nested_action_owners(&["Activate outer node", "Activate inner node"], &[0, 1]);
}

#[test]
fn a_three_deep_merge_routes_each_custom_action_to_its_owner() {
    check_nested_action_owners(&["A", "B", "C"], &[0, 2, 1]);
}

#[test]
fn an_inactive_merged_child_cannot_advertise_or_handle_its_custom_action() {
    use incular_widgets::IndexedStack;

    let a = CustomSemanticsAction::new("first action");
    let b = CustomSemanticsAction::new("second action");
    let seen = Rc::new(Cell::new([0, 0]));
    let stack = |index: usize| {
        let child = |slot, label, action| {
            let seen = seen.clone();
            Widget::from(
                Semantics::new(SizedBox::shrink())
                    .label(label)
                    .custom_action(action, move || {
                        let mut counts = seen.get();
                        counts[slot] += 1;
                        seen.set(counts);
                    }),
            )
        };
        MergeSemantics::new(
            IndexedStack::new([child(0, "First", a.clone()), child(1, "Second", b.clone())])
                .index(index),
        )
        .into()
    };
    let mut runtime = Runtime::new(stack(0)).unwrap();
    frame(&mut runtime);
    let initial = labelled_node(&runtime, "First");
    assert_eq!(actions(&runtime, initial), ["first action"]);
    let call_a = SemanticAction::Custom(a.get_identifier());
    let call_b = SemanticAction::Custom(b.get_identifier());
    assert!(!runtime.dispatch_semantic_action(initial, call_b.clone()));
    assert!(runtime.dispatch_semantic_action(initial, call_a.clone()));
    assert_eq!(seen.get(), [1, 0]);

    runtime
        .schedule_update(runtime.tree().root().unwrap(), stack(1))
        .unwrap();
    frame(&mut runtime);
    let retained = labelled_node(&runtime, "Second");
    assert_eq!(retained, initial);
    assert_eq!(actions(&runtime, retained), ["second action"]);
    assert!(!runtime.dispatch_semantic_action(retained, call_a));
    assert!(runtime.dispatch_semantic_action(retained, call_b));
    assert_eq!(seen.get(), [1, 1]);
}

#[test]
fn an_accesskit_request_reaches_the_live_retained_custom_callback() {
    use accesskit::{Action, ActionData, ActionRequest, NodeId, TreeId};
    use incular_accessibility::AccessKitProjection;

    let action = CustomSemanticsAction::new("Move down");
    let action_id = action.get_identifier();
    let calls = Rc::new(Cell::new(0));
    let observed_calls = calls.clone();
    let mut runtime = Runtime::new(
        Semantics::new(SizedBox::shrink())
            .label("Item")
            .custom_action(action, move || observed_calls.set(observed_calls.get() + 1))
            .into(),
    )
    .unwrap();
    frame(&mut runtime);
    let mut projection = AccessKitProjection::new();
    projection.sync(runtime.tree().semantics(), 1.0).unwrap();
    let target = labelled_node(&runtime, "Item");
    let request = ActionRequest {
        action: Action::CustomAction,
        target_tree: TreeId::ROOT,
        target_node: NodeId(projection.native_node_id(target).unwrap()),
        data: Some(ActionData::CustomAction(
            i32::try_from(action_id.0).unwrap(),
        )),
    };
    let mapped = projection.translate_action(&request).unwrap();
    assert!(runtime.dispatch_semantic_action(mapped.node, mapped.action.clone()));
    assert_eq!(calls.get(), 1);

    runtime
        .schedule_update(
            runtime.tree().root().unwrap(),
            Semantics::new(SizedBox::shrink()).label("Item").into(),
        )
        .unwrap();
    frame(&mut runtime);
    // The live graph rejects an event from an older native projection.
    assert_eq!(labelled_node(&runtime, "Item"), target);
    assert!(!runtime.dispatch_semantic_action(mapped.node, mapped.action));
    projection.sync(runtime.tree().semantics(), 1.0).unwrap();
    assert!(projection.translate_action(&request).is_none());
    assert_eq!(calls.get(), 1);
}

#[test]
fn tap_and_long_press_hints_are_overrides_not_additional_callbacks() {
    use incular_semantics::SemanticActionKind;

    let calls = Rc::new(Cell::new([0, 0]));
    for (kind, hint, index) in [
        (SemanticActionKind::Activate, "test", 0usize),
        (SemanticActionKind::LongPress, "foo", 1usize),
    ] {
        let observed = calls.clone();
        let callback = move || {
            let mut next = observed.get();
            next[index] += 1;
            observed.set(next);
        };
        let widget = Semantics::new(SizedBox::shrink())
            .container(true)
            .label("Target");
        let (widget, action) = if kind == SemanticActionKind::Activate {
            (
                widget
                    .on_tap(callback)
                    .on_tap_hint("earlier")
                    .on_tap_hint(hint),
                SemanticAction::Activate,
            )
        } else {
            (
                widget.on_long_press(callback).on_long_press_hint(hint),
                SemanticAction::LongPress,
            )
        };
        let mut runtime = Runtime::new(widget.into()).unwrap();
        frame(&mut runtime);
        let id = labelled_node(&runtime, "Target");
        let projected = runtime.tree().semantics().node(id).unwrap();
        assert!(projected.actions.contains(&kind));
        assert!(!projected.actions.contains(&SemanticActionKind::Custom));
        assert_eq!(projected.state.custom_actions.len(), 1);
        let override_id = projected.state.custom_actions[0];
        let description = CustomSemanticsAction::get_action(override_id).unwrap();
        assert_eq!(description.label(), None);
        assert_eq!(description.hint(), Some(hint));
        assert_eq!(description.action(), Some(kind));
        assert!(!runtime.dispatch_semantic_action(id, SemanticAction::Custom(override_id)));
        assert!(runtime.dispatch_semantic_action(id, action));
    }
    assert_eq!(calls.get(), [1, 1]);
}
