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
