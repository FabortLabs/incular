//! Observable Flutter action masks, not keyboard focus or OS-specific speech.
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{CustomSemanticsAction, SemanticAction, SemanticActionKind, SemanticRole};
use incular_widgets::{
    AbsorbPointer, Center, Focus, FocusNode, IgnorePointer, Semantics, SizedBox, Visibility, Widget,
};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug)]
enum Wrapper {
    Absorb,
    Ignore,
    Visibility,
}

fn check_blocker(kind: Wrapper) {
    const LABEL: &str = "Action node";
    let focus = FocusNode::new();
    let calls = Rc::new(Cell::new(0));
    let custom = CustomSemanticsAction::new("custom action");
    let leaf = Widget::from(
        Semantics::new(
            Focus::new(SizedBox::new().width(60.).height(40.))
                .node(focus.clone())
                .include_semantics(false),
        )
        .container(true)
        .label(LABEL)
        .button(true)
        .enabled(true)
        .on_tap(|| {})
        .on_long_press(|| {})
        .on_increase({
            let calls = Rc::clone(&calls);
            move || calls.set(calls.get() + 1)
        })
        .on_decrease(|| {})
        .on_scroll_left(|| {})
        .custom_action(custom.clone(), || {}),
    );
    let build = |blocked| {
        let child = match kind {
            Wrapper::Absorb => Widget::from(AbsorbPointer::new(leaf.clone()).absorbing(blocked)),
            Wrapper::Ignore => Widget::from(IgnorePointer::new(leaf.clone()).ignoring(blocked)),
            Wrapper::Visibility => Widget::from(
                Visibility::maintain(leaf.clone())
                    .maintain_interactivity(false)
                    .visible(!blocked),
            ),
        };
        Widget::from(Center::new(child))
    };

    let mut runtime = Runtime::new(build(false)).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(constraints).unwrap();
    focus.request_focus();
    runtime.run_frame(constraints).unwrap();
    let root = runtime.tree().root().unwrap();
    let mut retained = None;
    let mut actual = Vec::new();
    for blocked in [false, true, false] {
        runtime.schedule_update(root, build(blocked)).unwrap();
        runtime.run_frame(constraints).unwrap();
        assert!(focus.has_primary_focus());
        let (node, mask, custom_advertised) = {
            let (id, node) = runtime
                .tree()
                .semantics()
                .iter()
                .find(|(_, node)| node.label.as_deref() == Some(LABEL))
                .expect("still published");
            assert_eq!(id, *retained.get_or_insert(id));
            assert_eq!(node.role, SemanticRole::Button);
            assert!(node.state.enabled && node.state.focusable);
            let mask = [
                SemanticActionKind::Focus,
                SemanticActionKind::Activate,
                SemanticActionKind::LongPress,
                SemanticActionKind::Increment,
                SemanticActionKind::Decrement,
                SemanticActionKind::ScrollLeft,
                SemanticActionKind::Custom,
            ]
            .map(|action| node.actions.contains(&action));
            (
                id,
                mask,
                node.state.custom_actions.contains(&custom.get_identifier()),
            )
        };
        let dispatched = runtime.dispatch_semantic_action(node, SemanticAction::Increment);
        actual.push((mask, custom_advertised, dispatched));
    }
    assert_eq!(
        actual,
        [
            ([true; 7], true, true),
            ([false; 7], false, false),
            ([true; 7], true, true)
        ],
        "{kind:?}"
    );
    assert_eq!(calls.get(), 2, "a blocked native action must not execute");
}

#[test]
fn absorbing_blocks_all_published_user_actions() {
    check_blocker(Wrapper::Absorb);
}
#[test]
fn ignoring_blocks_all_published_user_actions() {
    check_blocker(Wrapper::Ignore);
}
#[test]
fn maintaining_semantics_but_not_interactivity_blocks_user_actions() {
    check_blocker(Wrapper::Visibility);
}

#[test]
fn merged_node_keeps_labels_but_cannot_borrow_a_blocked_custom_action() {
    use incular_widgets::{Column, MergeSemantics};
    let action = CustomSemanticsAction::new("conditional");
    let id = action.get_identifier();
    let hits = Rc::new(Cell::new(0));
    let target = Widget::from(
        Semantics::new(SizedBox::new().width(60.).height(40.))
            .label("Blocked target")
            .custom_action(action, {
                let hits = Rc::clone(&hits);
                move || hits.set(hits.get() + 1)
            }),
    );
    let build = |blocked: bool| {
        Widget::from(Center::new(MergeSemantics::new(Column::new([
            Widget::from(IgnorePointer::new(target.clone()).ignoring(blocked)),
            Semantics::new(SizedBox::new().height(10.))
                .label("Plain sibling")
                .into(),
        ]))))
    };
    let constraints = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(build(false)).unwrap();
    runtime.run_frame(constraints).unwrap();
    let root = runtime.tree().root().unwrap();
    let mut retained = None;
    for blocked in [false, true, false] {
        runtime.schedule_update(root, build(blocked)).unwrap();
        runtime.run_frame(constraints).unwrap();
        let (node_id, node) = runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| {
                node.label.as_deref().is_some_and(|label| {
                    label.contains("Blocked target") && label.contains("Plain sibling")
                })
            })
            .expect("retained merged node");
        assert_eq!(node_id, *retained.get_or_insert(node_id));
        assert_eq!(node.actions.contains(&SemanticActionKind::Custom), !blocked);
        assert_eq!(node.state.custom_actions.contains(&id), !blocked);
        assert_eq!(
            runtime.dispatch_semantic_action(node_id, SemanticAction::Custom(id)),
            !blocked
        );
    }
    assert_eq!(hits.get(), 2);
}
