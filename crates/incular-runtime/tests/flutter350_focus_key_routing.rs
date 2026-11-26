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
        assert!(widget.raw_key_handler().is_none());
        assert!(node.on_key().is_none());
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
fn external_focus_node_controls_both_callbacks_independently_of_rebuilds() {
    use incular_widgets::{FocusKeyEventHandler, FocusRawKeyEventHandler};
    let node = FocusNode::new();
    let original: FocusKeyEventHandler = Rc::new(|_, _| KeyEventResult::Ignored);
    let original_raw: FocusRawKeyEventHandler = Rc::new(|_, _| KeyEventResult::Ignored);
    let called = Rc::new(RefCell::new(Vec::new()));
    let changed: FocusKeyEventHandler = Rc::new({
        let called = called.clone();
        move |_, event| {
            if event.state == KeyState::Down {
                called.borrow_mut().push("typed");
            }
            KeyEventResult::Handled
        }
    });
    let changed_raw: FocusRawKeyEventHandler = Rc::new({
        let called = called.clone();
        move |_, event| {
            if event.is_key_down() {
                assert_eq!(event.physical_key(), Code::Enter);
                called.borrow_mut().push("raw");
            }
            KeyEventResult::Handled
        }
    });
    node.set_on_key_event(Some(original.clone()));
    node.set_on_key(Some(original_raw.clone()));
    node.set_descendants_are_focusable(false);
    node.set_descendants_are_traversable(false);
    let scene = || Focus::with_external_focus_node(SizedBox::shrink(), node.clone());
    let verify = |typed: &FocusKeyEventHandler, raw: &FocusRawKeyEventHandler, descendants| {
        let focus = scene();
        assert!(Rc::ptr_eq(
            focus.key_event_handler().as_ref().unwrap(),
            typed
        ));
        assert!(Rc::ptr_eq(node.on_key_event().as_ref().unwrap(), typed));
        assert!(Rc::ptr_eq(focus.raw_key_handler().as_ref().unwrap(), raw));
        assert!(Rc::ptr_eq(node.on_key().as_ref().unwrap(), raw));
        assert_eq!(focus.allows_descendant_focus(), descendants);
        assert_eq!(focus.allows_descendant_traversal(), descendants);
        assert_eq!(node.descendants_are_focusable(), descendants);
        assert_eq!(node.descendants_are_traversable(), descendants);
        assert!(focus.allows_focus_requests() && node.can_request_focus());
        assert!(!focus.skips_traversal() && !node.skip_traversal());
    };
    let mut runtime = Runtime::new(scene().into()).unwrap();
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    let target = runtime.focused_element().unwrap();
    verify(&original, &original_raw, false);
    assert!(!deliver(&mut runtime, NamedKey::Enter, Code::Enter));
    assert!(called.borrow().is_empty());

    node.set_on_key_event(Some(changed.clone()));
    node.set_on_key(Some(changed_raw.clone()));
    node.set_descendants_are_focusable(true);
    node.set_descendants_are_traversable(true);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene().into()).unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.focused_element(), Some(target));
    verify(&changed, &changed_raw, true);
    assert!(deliver(&mut runtime, NamedKey::Enter, Code::Enter));
    assert_eq!(&*called.borrow(), &["typed", "raw"]);
}

#[test]
fn mounted_focus_replaces_the_raw_callback_without_replacing_the_node() {
    use incular_widgets::FocusRawKeyEventHandler;
    let node = FocusNode::new();
    let handled = Rc::new(std::cell::Cell::new(false));
    let ignored: FocusRawKeyEventHandler = Rc::new(|_, _| KeyEventResult::Ignored);
    let active: FocusRawKeyEventHandler = Rc::new({
        let handled = handled.clone();
        move |_, event| {
            assert_eq!(event.logical_key(), &KeyboardKey::Named(NamedKey::Enter));
            assert_eq!(event.physical_key(), Code::Enter);
            if event.is_key_down() {
                handled.set(true);
            }
            KeyEventResult::Handled
        }
    });
    let scene = |handler| {
        Focus::new(SizedBox::shrink())
            .node(node.clone())
            .can_request_focus(true)
            .skip_traversal(true)
            .with_raw_key_handler(Some(handler))
    };
    let verify = |widget: &Focus, handler: &FocusRawKeyEventHandler| {
        assert!(Rc::ptr_eq(
            widget.raw_key_handler().as_ref().unwrap(),
            handler
        ));
        assert!(Rc::ptr_eq(node.on_key().as_ref().unwrap(), handler));
        assert!(widget.key_event_handler().is_none() && node.on_key_event().is_none());
        assert_eq!(
            widget.allows_descendant_focus(),
            node.descendants_are_focusable()
        );
        assert!(widget.skips_traversal() && node.skip_traversal());
        assert!(widget.allows_focus_requests() && node.can_request_focus());
    };
    let start = scene(ignored.clone());
    let mut runtime = Runtime::new(start.clone().into()).unwrap();
    frame(&mut runtime);
    verify(&start, &ignored);
    node.request_focus();
    frame(&mut runtime);
    let focus = runtime.focused_element().unwrap();
    assert!(!deliver(&mut runtime, NamedKey::Enter, Code::Enter));
    assert!(!handled.get());

    let updated = scene(active.clone());
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, updated.clone().into())
        .unwrap();
    frame(&mut runtime);
    verify(&updated, &active);
    assert_eq!(runtime.focused_element(), Some(focus));
    assert!(deliver(&mut runtime, NamedKey::Enter, Code::Enter));
    assert!(handled.get());
}

#[test]
fn typed_and_raw_combine_before_bubbling_and_only_native_keys_have_a_raw_view() {
    use KeyEventResult::{Handled, Ignored, SkipRemainingHandlers};
    let node = FocusNode::new();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let raw_result = Rc::new(std::cell::Cell::new(Handled));
    let leaf = Focus::new(SizedBox::shrink())
        .node(node.clone())
        .on_key_event({
            let calls = calls.clone();
            move |_, event| {
                if event.state == KeyState::Down {
                    calls.borrow_mut().push("typed");
                }
                SkipRemainingHandlers
            }
        })
        .on_key({
            let calls = calls.clone();
            let raw_result = raw_result.clone();
            move |_, event| {
                if event.is_key_down() {
                    calls.borrow_mut().push("raw");
                }
                raw_result.get()
            }
        });
    let root = Focus::new(leaf).on_key_event({
        let calls = calls.clone();
        move |_, event| {
            if event.state == KeyState::Down {
                calls.borrow_mut().push("parent");
            }
            Ignored
        }
    });
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    assert!(deliver(&mut runtime, NamedKey::Enter, Code::Enter));
    assert_eq!(&*calls.borrow(), &["typed", "raw"]);
    calls.borrow_mut().clear();
    raw_result.set(Ignored);
    assert!(!deliver(&mut runtime, NamedKey::Enter, Code::Enter));
    assert_eq!(&*calls.borrow(), &["typed", "raw"]);
    calls.borrow_mut().clear();

    let focused = runtime.focused_element().unwrap();
    assert!(!runtime.tree().dispatch_semantic_increment(focused, true));
    assert_eq!(&*calls.borrow(), &["typed"]);
}
