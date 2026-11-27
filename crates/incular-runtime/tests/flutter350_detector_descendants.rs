use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, PointerPhase, Size,
    time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Action, ActionResult, ActionScope, Actions, Center, Column, Directionality, Focus, FocusNode,
    FocusableActionDetector, Intent, LogicalShortcutKey, PointerDeviceKind, Shortcuts, SizedBox,
    Widget, internal::RawPointerEvent,
};

const CHILD: u64 = 941;
fn target() -> Widget {
    Widget::from(SizedBox::new().width(100.).height(100.)).with_key(CHILD)
}
fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}
fn focus_child(node: &FocusNode) -> Focus {
    Focus::new(SizedBox::new().height(30.)).node(node.clone())
}
fn rebuild(runtime: &mut Runtime, widget: Widget) {
    runtime
        .schedule_update(runtime.tree().root().unwrap(), widget)
        .unwrap();
    frame(runtime);
}

#[test]
fn disabled_inner_action_shadows_the_enabled_ancestor_without_dispatching() {
    let called = Rc::new(Cell::new(false));
    let mapping = |enabled| {
        let call = called.clone();
        let action = Action::new("test", move || call.set(true));
        action.set_enabled(enabled);
        let mut actions = Actions::new();
        actions.register_action(action);
        actions
    };
    let dispatched = Rc::new(Cell::new(false));
    let dispatcher = {
        let dispatched = dispatched.clone();
        Rc::new(move |call: incular_widgets::ActionDispatchInvocation<'_>| {
            dispatched.set(true);
            call.invoke()
        })
    };
    let inner = ActionScope::new(Rc::new(mapping(false)), target()).dispatcher(dispatcher.clone());
    let outer = ActionScope::new(Rc::new(mapping(true)), inner).dispatcher(dispatcher);
    let mut runtime = Runtime::new(outer.into()).unwrap();
    frame(&mut runtime);
    let id = runtime.tree().element_with_key(&CHILD.into()).unwrap();
    assert_eq!(
        runtime.invoke_action_at(id, &Intent::<incular_widgets::Command>::from("test")),
        Some(ActionResult::Ignored)
    );
    assert!(!called.get());
    assert!(!dispatched.get());
}

#[test]
fn detector_shortcut_and_focus_do_not_require_callbacks() {
    let called = Rc::new(Cell::new(false));
    let log = called.clone();
    let node = FocusNode::new();
    let mut actions = Actions::new();
    actions.register("run", move || log.set(true));
    let actions = Rc::new(actions);
    let mut keys = Shortcuts::new();
    keys.bind_logical(
        LogicalShortcutKey::new(KeyboardKey::Named(NamedKey::Enter), Modifiers::empty()),
        "run",
    );
    let keys = Rc::new(keys);
    let build = |enabled| {
        Center::new(
            FocusableActionDetector::new(target())
                .focus_node(node.clone())
                .actions(actions.clone())
                .shortcuts(keys.clone())
                .enabled(enabled),
        )
        .into()
    };
    let mut runtime = Runtime::new(build(true)).unwrap();
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    let move_to = |runtime: &mut Runtime, at| {
        runtime.handle_raw_pointer_event(RawPointerEvent::new(
            1,
            1,
            PointerDeviceKind::Mouse,
            0,
            at,
            PointerPhase::Move,
            Instant::now(),
        ));
        frame(runtime);
    };
    let key = |runtime: &mut Runtime| {
        runtime.dispatch_keyboard_event(KeyboardEvent::key_down(
            KeyboardKey::Named(NamedKey::Enter),
            Code::Enter,
        ));
        runtime.dispatch_keyboard_event(KeyboardEvent::key_up(
            KeyboardKey::Named(NamedKey::Enter),
            Code::Enter,
        ));
        frame(runtime);
    };
    move_to(&mut runtime, Offset::new(400., 300.));
    key(&mut runtime);
    assert!(called.replace(false));
    for enabled in [false, true, false] {
        rebuild(&mut runtime, build(enabled));
        key(&mut runtime);
        assert!(!called.get());
    }
    move_to(&mut runtime, Offset::ZERO);
    rebuild(&mut runtime, build(true));
    assert!(!called.get());
}

#[test]
fn detector_can_withdraw_descendant_focus_permission_from_a_mounted_child() {
    let node = FocusNode::new();
    let build = |allowed| {
        FocusableActionDetector::new(focus_child(&node))
            .descendants_are_focusable(allowed)
            .into()
    };
    let mut runtime = Runtime::new(build(true)).unwrap();
    frame(&mut runtime);
    assert!(!node.has_focus());
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    rebuild(&mut runtime, build(false));
    assert!(!node.has_focus());
    node.request_focus();
    frame(&mut runtime);
    assert!(!node.has_focus());
}

#[test]
fn detector_can_exclude_descendants_from_traversal_but_not_explicit_focus() {
    use incular_widgets::{FocusScope, FocusScopeNode, TraversalEdgeBehavior};

    let first = FocusNode::new();
    let second = FocusNode::new();
    let outer = FocusNode::new();
    outer.set_skip_traversal(true);
    let parent = FocusScopeNode::new();
    let route = FocusScopeNode::new();
    route.set_traversal_edge_behavior(TraversalEdgeBehavior::ParentScope);
    let build = |traversable, routed| {
        let children = Column::new([focus_child(&first), focus_child(&second)]);
        let detector = FocusableActionDetector::new(children)
            .focus_node(outer.clone())
            .descendants_are_traversable(traversable);
        let tree: Widget = if routed {
            FocusScope::new(FocusScope::new(detector).node(route.clone()))
                .node(parent.clone())
                .into()
        } else {
            detector.into()
        };
        Directionality::new(TextDirection::Ltr, tree).into()
    };
    {
        let mut runtime = Runtime::new(build(false, false)).unwrap();
        frame(&mut runtime);
        first.request_focus();
        frame(&mut runtime);
        assert!(!runtime.next_focus_from(&first));
        assert!(first.has_primary_focus());
    }
    let mut runtime = Runtime::new(build(true, true)).unwrap();
    frame(&mut runtime);
    first.request_focus();
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    assert!(runtime.next_focus_from(&first));
    assert!(second.has_primary_focus());
    rebuild(&mut runtime, build(false, true));
    first.request_focus();
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    assert!(runtime.next_focus_from(&first));
    frame(&mut runtime);
    assert!(!first.has_focus());
    assert!(!second.has_focus());
    assert!(route.scope_focus_node().has_primary_focus());
}

#[test]
fn skip_traversal_setting_is_forwarded_to_the_mounted_focus_node() {
    let detector = FocusNode::new();
    let build = |skip| {
        let field = FocusableActionDetector::new(target()).focus_node(detector.clone());
        let field = if skip {
            field.skip_traversal(true)
        } else {
            field
        };
        assert_eq!(field.skips_traversal(), skip);
        field.into()
    };
    let mut runtime = Runtime::new(build(false)).unwrap();
    frame(&mut runtime);
    assert!(!detector.skip_traversal());
    rebuild(&mut runtime, build(true));
    assert!(detector.skip_traversal());
}
