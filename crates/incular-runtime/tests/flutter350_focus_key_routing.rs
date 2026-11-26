//! Routed Focus key outcomes and retained handler updates from Flutter 3.50.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyState, KeyboardEvent, KeyboardKey, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{Column, Focus, FocusNode, KeyEventResult, SizedBox, Widget};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(200., 100.)))
        .unwrap();
}

fn deliver(runtime: &mut Runtime, name: NamedKey, code: Code) -> bool {
    let key = KeyboardEvent::key_down(KeyboardKey::Named(name), code);
    let consumed = runtime.dispatch_keyboard_event(key.clone());
    let mut up = key;
    up.state = KeyState::Up;
    let _ = runtime.dispatch_keyboard_event(up);
    consumed
}

#[test]
fn rebuilding_focus_handlers_retains_focus_and_uses_each_new_outcome() {
    use KeyEventResult::{Handled, Ignored, SkipRemainingHandlers};

    let outer = FocusNode::new();
    let inner = FocusNode::new();
    inner.set_skip_traversal(true);
    let route = Rc::new(RefCell::new(Vec::new()));
    let scene = |outcome| -> Widget {
        let child = Focus::new(SizedBox::shrink())
            .node(inner.clone())
            .on_key_event({
                let route = route.clone();
                let inner = inner.clone();
                move |node, event| {
                    if event.state == KeyState::Down {
                        assert_eq!(*node, inner);
                        route.borrow_mut().push("inner");
                        return outcome;
                    }
                    Ignored
                }
            });
        Focus::new(child)
            .node(outer.clone())
            .on_key_event({
                let route = route.clone();
                let outer = outer.clone();
                move |node, event| {
                    if event.state == KeyState::Down {
                        assert_eq!(*node, outer);
                        route.borrow_mut().push("outer");
                        return Handled;
                    }
                    Ignored
                }
            })
            .into()
    };

    let mut runtime = Runtime::new(scene(Ignored)).unwrap();
    frame(&mut runtime);
    inner.request_focus();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let target = runtime.focused_element().unwrap();
    for (result, consumed, path) in [
        (Ignored, true, &["inner", "outer"][..]),
        (Handled, true, &["inner"][..]),
        (SkipRemainingHandlers, false, &["inner"][..]),
    ] {
        runtime.schedule_update(root, scene(result)).unwrap();
        frame(&mut runtime);
        assert_eq!(runtime.focused_element(), Some(target));
        assert!(inner.has_primary_focus());
        assert!(outer.has_focus());
        assert_eq!(
            deliver(&mut runtime, NamedKey::Enter, Code::Enter),
            consumed
        );
        assert_eq!(route.borrow().as_slice(), path);
        route.borrow_mut().clear();
    }
}

#[test]
fn skipped_tab_is_not_consumed_and_does_not_run_default_framework_traversal() {
    let first = FocusNode::new();
    let second = FocusNode::new();
    let skip = Rc::new(std::cell::Cell::new(true));
    let source = Focus::new(SizedBox::new().width(50.).height(20.))
        .node(first.clone())
        .on_key_event({
            let skip = skip.clone();
            move |_, event| {
                if event.code == Code::Tab && skip.get() {
                    KeyEventResult::SkipRemainingHandlers
                } else {
                    KeyEventResult::Ignored
                }
            }
        });
    let next = Focus::new(SizedBox::new().width(50.).height(20.)).node(second.clone());
    let mut runtime =
        Runtime::new(Column::new([Widget::from(source), next.into()]).into()).unwrap();
    frame(&mut runtime);
    first.request_focus();
    frame(&mut runtime);
    assert!(first.has_primary_focus());

    assert!(!deliver(&mut runtime, NamedKey::Tab, Code::Tab));
    assert!(first.has_primary_focus() && !second.has_focus());

    skip.set(false);
    assert!(deliver(&mut runtime, NamedKey::Tab, Code::Tab));
    assert!(second.has_primary_focus() && !first.has_focus());
}

#[test]
fn mounted_focus_installs_the_exact_typed_handler_on_its_node() {
    use incular_widgets::FocusKeyEventHandler;
    let node = FocusNode::new();
    let handled = Rc::new(std::cell::Cell::new(false));
    let ignored: FocusKeyEventHandler = Rc::new(|_, _| KeyEventResult::Ignored);
    let replacement: FocusKeyEventHandler = Rc::new({
        let handled = handled.clone();
        move |_, event| {
            if event.code == Code::Enter && event.state == KeyState::Down {
                handled.set(true);
            }
            KeyEventResult::Handled
        }
    });
    let scene = |handler: Option<FocusKeyEventHandler>| {
        Focus::new(SizedBox::shrink())
            .node(node.clone())
            .skip_traversal(true)
            .can_request_focus(true)
            .with_key_event_handler(handler)
    };
    let verify = |widget: &Focus, expected: &FocusKeyEventHandler| {
        assert!(Rc::ptr_eq(
            widget.key_event_handler().as_ref().unwrap(),
            expected
        ));
        assert!(Rc::ptr_eq(node.on_key_event().as_ref().unwrap(), expected));
        assert_eq!(
            widget.allows_descendant_focus(),
            node.descendants_are_focusable()
        );
        assert_eq!(widget.skips_traversal(), node.skip_traversal());
        assert_eq!(widget.allows_focus_requests(), node.can_request_focus());
    };
    let first = scene(Some(ignored.clone()));
    assert!(node.on_key_event().is_none());
    let mut runtime = Runtime::new(first.clone().into()).unwrap();
    frame(&mut runtime);
    verify(&first, &ignored);
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    assert!(!deliver(&mut runtime, NamedKey::Enter, Code::Enter));
    assert!(!handled.get());

    let id = runtime.focused_element().unwrap();
    let root = runtime.tree().root().unwrap();
    let changed = scene(Some(replacement.clone()));
    runtime
        .schedule_update(root, changed.clone().into())
        .unwrap();
    frame(&mut runtime);
    verify(&changed, &replacement);
    assert!(node.has_primary_focus());
    assert_eq!(runtime.focused_element(), Some(id));
    assert!(deliver(&mut runtime, NamedKey::Enter, Code::Enter));
    assert!(handled.get());
    runtime.schedule_update(root, scene(None).into()).unwrap();
    frame(&mut runtime);
    assert!(node.on_key_event().is_none());
    assert_eq!(runtime.focused_element(), Some(id));
    assert!(!deliver(&mut runtime, NamedKey::Enter, Code::Enter));
}

#[test]
fn abandoned_focus_descriptor_cannot_change_live_focus_permissions() {
    use incular_widgets::FocusKeyEventHandler;
    let node = FocusNode::new();
    let called = Rc::new(std::cell::Cell::new(0));
    let handler: FocusKeyEventHandler = Rc::new({
        let called = called.clone();
        move |_, event| {
            if event.code == Code::Enter && event.state == KeyState::Down {
                called.set(called.get() + 1);
            }
            KeyEventResult::Handled
        }
    });
    let scene = |allowed| {
        Widget::from(
            Focus::new(SizedBox::shrink())
                .node(node.clone())
                .can_request_focus(allowed)
                .skip_traversal(allowed)
                .descendants_are_focusable(allowed)
                .with_key_event_handler(Some(handler.clone())),
        )
    };
    let mut runtime = Runtime::new(scene(true)).unwrap();
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    let target = runtime.focused_element().unwrap();
    let abandoned = scene(false);
    assert!(node.can_request_focus() && node.has_primary_focus());
    assert!(node.skip_traversal() && node.descendants_are_focusable());
    drop(abandoned);
    frame(&mut runtime);
    assert_eq!(runtime.focused_element(), Some(target));
    assert!(node.can_request_focus() && node.has_primary_focus());
    assert!(deliver(&mut runtime, NamedKey::Enter, Code::Enter));
    assert_eq!(called.get(), 1);

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(false)).unwrap();
    frame(&mut runtime);
    assert!(!node.can_request_focus() && !node.has_primary_focus());
    assert!(!node.skip_traversal() && !node.descendants_are_focusable());
    assert_ne!(runtime.focused_element(), Some(target));
    assert!(!deliver(&mut runtime, NamedKey::Enter, Code::Enter));
    assert_eq!(called.get(), 1);
}

#[test]
fn external_focus_node_controls_the_callback_independently_of_rebuilds() {
    use incular_widgets::FocusKeyEventHandler;
    let node = FocusNode::new();
    let original: FocusKeyEventHandler = Rc::new(|_, _| KeyEventResult::Ignored);
    let changed: FocusKeyEventHandler = Rc::new(|_, _| KeyEventResult::Handled);
    node.set_on_key_event(Some(original.clone()));
    let scene = || Focus::with_external_focus_node(SizedBox::shrink(), node.clone());
    let mut runtime = Runtime::new(scene().into()).unwrap();
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    let target = runtime.focused_element().unwrap();
    assert!(Rc::ptr_eq(
        scene().key_event_handler().as_ref().unwrap(),
        &original
    ));
    assert!(!deliver(&mut runtime, NamedKey::Enter, Code::Enter));

    node.set_on_key_event(Some(changed.clone()));
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene().into()).unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.focused_element(), Some(target));
    assert!(Rc::ptr_eq(
        scene().key_event_handler().as_ref().unwrap(),
        &changed
    ));
    assert!(Rc::ptr_eq(node.on_key_event().as_ref().unwrap(), &changed));
    assert!(deliver(&mut runtime, NamedKey::Enter, Code::Enter));
}
