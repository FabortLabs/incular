use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind, SemanticNode, SemanticNodeId};
use incular_widgets::{Column, Directionality, Semantics, SizedBox, Widget};

type Calls = Rc<RefCell<Vec<&'static str>>>;

fn mount(child: Semantics) -> Runtime {
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, child).into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    runtime
}

fn annotation(label: &'static str, blocked: bool, long: bool, calls: &Calls) -> Semantics {
    let calls = calls.clone();
    let node = Semantics::new(SizedBox::new().width(10.).height(10.))
        .label(label)
        .block_user_actions(blocked);
    if long {
        node.on_long_press(move || calls.borrow_mut().push(label))
    } else {
        node.on_tap(move || calls.borrow_mut().push(label))
    }
}

fn node<'a>(runtime: &'a Runtime, label: &str) -> &'a SemanticNode {
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(_, node)| (node.label.as_deref() == Some(label)).then_some(node))
        .unwrap_or_else(|| {
            panic!(
                "missing {label:?}; published: {:?}",
                runtime
                    .tree()
                    .semantics()
                    .iter()
                    .map(|(_, node)| (node.label.as_deref(), &node.actions))
                    .collect::<Vec<_>>()
            )
        })
}

fn parent_of(runtime: &Runtime, children: [SemanticNodeId; 2]) -> &SemanticNode {
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(_, node)| (node.children == children).then_some(node))
        .expect("both explicit labels remain direct siblings")
}

#[test]
fn a_blocking_parent_preserves_explicit_labels_but_removes_both_actions() {
    let calls = Calls::default();
    let mut runtime = mount(
        Semantics::new(Column::new([
            Widget::from(annotation("label1", false, false, &calls)),
            Widget::from(annotation("label2", false, false, &calls)),
        ]))
        .block_user_actions(true)
        .explicit_child_nodes(true),
    );
    let children = ["label1", "label2"].map(|label| {
        let node = node(&runtime, label);
        assert!(node.actions.is_empty());
        node.id
    });
    parent_of(&runtime, children);
    for id in children {
        assert!(!runtime.dispatch_semantic_action(id, SemanticAction::Activate));
    }
    assert!(calls.borrow().is_empty());
}

#[test]
fn merged_children_keep_labels_and_the_unblocked_long_press_only() {
    let calls = Calls::default();
    let mut runtime = mount(
        Semantics::new(Column::new([
            Widget::from(annotation("label1", true, false, &calls)),
            Widget::from(annotation("label2", false, true, &calls)),
        ]))
        .container(true),
    );

    let merged = node(&runtime, "label1\nlabel2");
    assert_eq!(merged.actions, [SemanticActionKind::LongPress]);
    assert!(merged.children.is_empty());
    let id = merged.id;
    assert!(!runtime.dispatch_semantic_action(id, SemanticAction::Activate));
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::LongPress));
    assert_eq!(*calls.borrow(), ["label2"]);
}

#[test]
fn declared_conflicting_taps_keep_distinct_nodes_even_if_one_is_blocked() {
    let calls = Calls::default();
    let mut runtime = mount(
        Semantics::new(Column::new([
            Widget::from(annotation("label1", true, false, &calls)),
            Widget::from(annotation("label2", false, false, &calls)),
        ]))
        .container(true),
    );

    let [first, second] = ["label1", "label2"].map(|label| node(&runtime, label).id);
    parent_of(&runtime, [first, second]);
    assert!(node(&runtime, "label1").actions.is_empty());
    assert_eq!(
        node(&runtime, "label2").actions,
        [SemanticActionKind::Activate]
    );
    assert!(!runtime.dispatch_semantic_action(first, SemanticAction::Activate));
    assert!(runtime.dispatch_semantic_action(second, SemanticAction::Activate));
    assert_eq!(*calls.borrow(), ["label2"]);
}

#[test]
fn changing_one_declaration_splits_then_remerges_a_live_container() {
    let calls = Calls::default();
    let build = |conflicting: bool| {
        Semantics::new(Column::new([
            Widget::from(annotation("label1", true, false, &calls)),
            Widget::from(annotation("label2", false, !conflicting, &calls)),
        ]))
        .container(true)
    };
    let mut runtime = mount(build(false));
    let stable = node(&runtime, "label1\nlabel2").id;
    let root = runtime.tree().root().unwrap();
    let limits = Constraints::tight(Size::new(800., 600.));

    let update = |runtime: &mut Runtime, conflicting| {
        runtime
            .schedule_update(
                root,
                Directionality::new(TextDirection::Ltr, build(conflicting)).into(),
            )
            .unwrap();
        runtime.run_frame(limits).unwrap();
    };

    update(&mut runtime, true);
    let ids = ["label1", "label2"].map(|label| node(&runtime, label).id);
    assert_eq!(parent_of(&runtime, ids).id, stable);
    assert!(!runtime.dispatch_semantic_action(stable, SemanticAction::LongPress));
    assert!(runtime.dispatch_semantic_action(ids[1], SemanticAction::Activate));

    update(&mut runtime, false);
    assert_eq!(node(&runtime, "label1\nlabel2").id, stable);
    assert!(!runtime.dispatch_semantic_action(ids[1], SemanticAction::Activate));
    assert!(runtime.dispatch_semantic_action(stable, SemanticAction::LongPress));
    assert_eq!(*calls.borrow(), ["label2", "label2"]);
}

#[test]
fn diagnostics_mark_masked_actions_without_advertising_or_dispatching_them() {
    use incular_config::Alignment;
    use incular_core::{Offset, Rect};
    use incular_widgets::{Align, Transform};

    let calls = Calls::default();
    let scene = |blocked: bool| {
        let record = |name: &'static str| {
            let calls = calls.clone();
            move || calls.borrow_mut().push(name)
        };
        let annotation = Semantics::new(SizedBox::from_size(Size::new(20., 30.)))
            .label("Translated control")
            .block_user_actions(blocked)
            .on_scroll_up(record("scroll"))
            .on_long_press(record("long"))
            .on_show_on_screen(record("show"))
            .on_did_gain_accessibility_focus(record("gain"));
        Directionality::new(
            TextDirection::Ltr,
            Align::new(
                Alignment::TOP_LEFT,
                Transform::translation(
                    Offset::new(50., 10.),
                    Transform::translation(Offset::new(10., 10.), annotation),
                ),
            ),
        )
        .into()
    };
    let limits = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(true)).unwrap();
    runtime.run_frame(limits).unwrap();
    let blocked = node(&runtime, "Translated control");
    assert_eq!(
        blocked.bounds,
        Rect::from_origin_size(Offset::new(60., 20.), Size::new(20., 30.))
    );
    assert_eq!(
        blocked.actions,
        [SemanticActionKind::DidGainAccessibilityFocus]
    );
    assert_eq!(
        blocked
            .state
            .blocked_user_actions
            .iter()
            .collect::<Vec<_>>(),
        [
            SemanticActionKind::LongPress,
            SemanticActionKind::ScrollUp,
            SemanticActionKind::ShowOnScreen,
        ]
    );
    let id = blocked.id;
    let dump = runtime.tree().semantics_debug_dump();
    assert!(
        dump.contains("actions=[DidGainAccessibilityFocus]"),
        "{dump}"
    );
    assert!(
        dump.contains("blockedActions: longPress🚫️, scrollUp🚫️, showOnScreen🚫️"),
        "{dump}"
    );

    for action in [
        SemanticAction::LongPress,
        SemanticAction::ScrollUp,
        SemanticAction::ShowOnScreen,
    ] {
        assert!(!runtime.dispatch_semantic_action(id, action));
    }
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::DidGainAccessibilityFocus));
    assert_eq!(*calls.borrow(), ["gain"]);

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(false)).unwrap();
    runtime.run_frame(limits).unwrap();
    let unblocked = node(&runtime, "Translated control");
    assert_eq!(unblocked.id, id);
    assert!(unblocked.state.blocked_user_actions.is_empty());
    assert!(
        !runtime
            .tree()
            .semantics_debug_dump()
            .contains("blockedActions:")
    );
    for action in [
        SemanticAction::LongPress,
        SemanticAction::ScrollUp,
        SemanticAction::ShowOnScreen,
    ] {
        assert!(runtime.dispatch_semantic_action(id, action));
    }
    assert_eq!(*calls.borrow(), ["gain", "long", "scroll", "show"]);
}
