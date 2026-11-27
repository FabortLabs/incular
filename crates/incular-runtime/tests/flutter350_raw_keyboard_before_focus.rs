use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{KeyState, Size};
use incular_runtime::{KeyMessage, Runtime};
use incular_widgets::{
    Focus, FocusNode, FuchsiaRawKeyData, KeyEventResult, RawKeyboardListener, SizedBox, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(160., 100.)))
        .unwrap();
}

#[test]
fn a_raw_ancestor_observes_before_a_focused_child_that_handles_the_key() {
    let ancestor = FocusNode::new();
    let child = FocusNode::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let child_widget = || {
        Widget::from(
            Focus::new(SizedBox::expand())
                .node(child.clone())
                .on_key_event({
                    let events = events.clone();
                    move |_, key| {
                        events.borrow_mut().push(if key.state.is_down() {
                            "typed down"
                        } else {
                            "typed up"
                        });
                        KeyEventResult::Handled
                    }
                }),
        )
    };
    let root = RawKeyboardListener::new(child_widget())
        .focus_node(ancestor.clone())
        .on_raw_key({
            let events = events.clone();
            move |key| {
                events.borrow_mut().push(if key.is_key_down() {
                    "raw down"
                } else {
                    "raw up"
                })
            }
        });
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    child.request_focus();
    frame(&mut runtime);
    assert!(ancestor.has_focus() && child.has_primary_focus());

    let deliver = |runtime: &mut Runtime, combined| {
        for state in [KeyState::Down, KeyState::Up] {
            let raw = FuchsiaRawKeyData {
                hid_usage: 0x0007_0004,
                code_point: u32::from(b'a'),
                modifiers: 0,
            }
            .into_event(state, false);
            let typed = raw.portable().clone();
            let message = if combined {
                KeyMessage::TypedAndRaw { typed, raw }
            } else {
                KeyMessage::Typed(typed)
            };
            assert!(runtime.dispatch_key_message(message));
        }
    };
    deliver(&mut runtime, true);
    assert_eq!(
        *events.borrow(),
        ["raw down", "typed down", "raw up", "typed up"]
    );
    events.borrow_mut().clear();
    deliver(&mut runtime, false);
    assert_eq!(*events.borrow(), ["typed down", "typed up"]);

    events.borrow_mut().clear();
    runtime
        .schedule_update(runtime.tree().root().unwrap(), child_widget())
        .unwrap();
    frame(&mut runtime);
    child.request_focus();
    frame(&mut runtime);
    assert!(!ancestor.has_focus());
    deliver(&mut runtime, true);
    assert_eq!(*events.borrow(), ["typed down", "typed up"]);
}
