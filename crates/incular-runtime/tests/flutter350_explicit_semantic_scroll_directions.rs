use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind};
use incular_widgets::{Semantics, SizedBox};

#[test]
fn explicit_semantics_keeps_four_scroll_callbacks_separate() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = |direction| {
        let seen = seen.clone();
        move || seen.borrow_mut().push(direction)
    };
    let mut runtime = Runtime::new(
        Semantics::new(SizedBox::square(100.))
            .container(true)
            .label("directional")
            .on_scroll_left(record("left"))
            .on_scroll_right(record("right"))
            .on_scroll_up(record("up"))
            .on_scroll_down(record("down"))
            .into(),
    )
    .unwrap();
    runtime
        .run_frame(Constraints::loose(Size::new(200., 100.)))
        .unwrap();
    let (id, node) = runtime
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("directional"))
        .unwrap();
    assert_eq!(
        node.actions,
        [
            SemanticActionKind::ScrollLeft,
            SemanticActionKind::ScrollRight,
            SemanticActionKind::ScrollUp,
            SemanticActionKind::ScrollDown,
        ]
    );
    for action in [
        SemanticAction::ScrollLeft,
        SemanticAction::ScrollRight,
        SemanticAction::ScrollUp,
        SemanticAction::ScrollDown,
    ] {
        assert!(runtime.dispatch_semantic_action(id, action));
    }
    assert_eq!(*seen.borrow(), ["left", "right", "up", "down"]);
}

#[test]
fn a_withdrawn_arrow_does_not_fall_through_to_an_unrelated_logical_callback() {
    use accesskit::{Action, ActionRequest, NodeId, TreeId};
    use incular_accessibility::AccessKitProjection;

    let seen = Rc::new(RefCell::new(Vec::new()));
    let build = |left| {
        let record = |direction| {
            let seen = seen.clone();
            move || seen.borrow_mut().push(direction)
        };
        let mut semantics = Semantics::new(SizedBox::square(100.))
            .label("retained")
            .on_scroll_backward(record("backward"))
            .on_scroll_down(record("down"));
        if left {
            semantics = semantics.on_scroll_left(record("left"));
        }
        semantics.into()
    };
    let constraints = Constraints::at_most(200., 100.);
    let mut runtime = Runtime::new(build(true)).unwrap();
    runtime.run_frame(constraints).unwrap();
    let id = runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.label.as_deref() == Some("retained")).then_some(id))
        .unwrap();
    let mut projection = AccessKitProjection::new();
    projection.sync(runtime.tree().semantics(), 1.).unwrap();
    let native = NodeId(projection.native_node_id(id).unwrap());
    let request = |action| ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node: native,
        data: None,
    };
    let old = projection
        .translate_action(&request(Action::ScrollLeft))
        .unwrap();
    assert_eq!(old.action, SemanticAction::ScrollLeft);
    assert!(runtime.dispatch_semantic_action(id, old.action.clone()));

    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(false))
        .unwrap();
    runtime.run_frame(constraints).unwrap();
    assert!(runtime.tree().semantics().node(id).is_some());
    assert!(!runtime.dispatch_semantic_action(id, old.action));
    projection.sync(runtime.tree().semantics(), 1.).unwrap();
    assert!(
        projection
            .translate_action(&request(Action::ScrollLeft))
            .is_none()
    );
    for (native, expected) in [
        (Action::ScrollUp, SemanticAction::ScrollBackward),
        (Action::ScrollDown, SemanticAction::ScrollDown),
    ] {
        let mapped = projection.translate_action(&request(native)).unwrap();
        assert_eq!(mapped.action, expected);
        assert!(runtime.dispatch_semantic_action(mapped.node, mapped.action));
    }
    assert_eq!(*seen.borrow(), ["left", "backward", "down"]);
}

#[test]
fn a_builder_only_arrow_creates_and_dispatches_its_own_semantic_node() {
    let tapped = Rc::new(std::cell::Cell::new(false));
    let callbacks = {
        let tapped = tapped.clone();
        Semantics::builder()
            .on_scroll_right(Some(Rc::new(move || tapped.set(true))))
            .child(SizedBox::square(100.))
            .build()
    };
    let mut runtime = Runtime::new(callbacks.into()).unwrap();
    runtime.run_frame(Constraints::at_most(200., 100.)).unwrap();
    let id = runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.actions == [SemanticActionKind::ScrollRight]).then_some(id))
        .unwrap();
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::ScrollRight));
    assert!(tapped.get());
}
