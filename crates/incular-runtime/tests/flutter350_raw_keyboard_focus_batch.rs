use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyState, Size};
use incular_runtime::{KeyMessage, Runtime};
use incular_widgets::{Focus, FocusNode, KeyEventResult, SizedBox, WebRawKeyData};

#[test]
fn each_focus_node_sees_the_typed_corrections_then_the_original_raw_event() {
    for handle_modifier in [false, true] {
        let log = Rc::new(RefCell::new(Vec::new()));
        let child = Focus::new(SizedBox::shrink())
            .node(FocusNode::new())
            .autofocus(true)
            .on_key_event({
                let log = log.clone();
                move |_, event| {
                    let shift = event.code == Code::ShiftLeft;
                    log.borrow_mut()
                        .push(if shift { "inner shift" } else { "inner A" });
                    if handle_modifier && shift {
                        KeyEventResult::Handled
                    } else {
                        KeyEventResult::Ignored
                    }
                }
            })
            .on_key({
                let log = log.clone();
                move |_, event| {
                    assert_eq!(event.physical_key(), Code::KeyA);
                    log.borrow_mut().push("inner raw");
                    KeyEventResult::Ignored
                }
            });
        let root = Focus::new(child)
            .node(FocusNode::new())
            .on_key_event({
                let log = log.clone();
                move |_, event| {
                    log.borrow_mut().push(if event.code == Code::ShiftLeft {
                        "outer shift"
                    } else {
                        "outer A"
                    });
                    KeyEventResult::Ignored
                }
            })
            .on_key({
                let log = log.clone();
                move |_, event| {
                    assert_eq!(event.physical_key(), Code::KeyA);
                    log.borrow_mut().push("outer raw");
                    KeyEventResult::Ignored
                }
            });
        let mut runtime = Runtime::new(root.into()).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(100., 100.)))
            .unwrap();
        let raw = WebRawKeyData {
            code: "KeyA".into(),
            key: "a".into(),
            meta_state: WebRawKeyData::SHIFT,
            key_code: 65,
            ..WebRawKeyData::default()
        }
        .into_event(KeyState::Down, false, false);
        assert_eq!(
            runtime.dispatch_key_message(KeyMessage::Raw(raw)),
            handle_modifier
        );
        let mut expected = vec!["inner shift", "inner A", "inner raw"];
        if !handle_modifier {
            expected.extend(["outer shift", "outer A", "outer raw"]);
        }
        assert_eq!(log.take(), expected);
    }
}
