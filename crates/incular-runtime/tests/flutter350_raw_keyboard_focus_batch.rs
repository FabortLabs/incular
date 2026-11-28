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

#[test]
fn a_raw_modifier_down_missing_from_its_snapshot_is_delivered_then_released() {
    use incular_core::HardwareKeyOrigin;

    for own_mask in [false, true] {
        let focus_log = Rc::new(RefCell::new(Vec::new()));
        let raw_log = Rc::new(RefCell::new(Vec::new()));
        let focused = Focus::new(SizedBox::shrink())
            .autofocus(true)
            .on_key_event({
                let log = focus_log.clone();
                move |_, event| {
                    log.borrow_mut().push((event.code, event.state));
                    KeyEventResult::Ignored
                }
            })
            .on_key({
                let focus_log = focus_log.clone();
                let raw_log = raw_log.clone();
                move |_, raw| {
                    let code = raw.physical_key();
                    assert!(raw.is_key_down(), "the original raw down is unchanged");
                    let last = if code == Code::ShiftLeft && !own_mask {
                        KeyState::Up
                    } else {
                        KeyState::Down
                    };
                    assert_eq!(focus_log.borrow().last(), Some(&(code, last)));
                    raw_log.borrow_mut().push(code);
                    KeyEventResult::Ignored
                }
            });
        let mut runtime = Runtime::new(focused.into()).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(100., 100.)))
            .unwrap();
        let hardware_log = Rc::new(RefCell::new(Vec::new()));
        let _observer = runtime.add_hardware_key_handler_with_origin({
            let log = hardware_log.clone();
            move |event, origin| {
                log.borrow_mut().push((event.code, event.state, origin));
                false
            }
        });
        let a = WebRawKeyData {
            code: "KeyA".into(),
            key: "a".into(),
            key_code: 65,
            ..Default::default()
        }
        .into_event(KeyState::Down, false, false);
        assert!(!runtime.dispatch_key_message(KeyMessage::Raw(a)));
        let shift = WebRawKeyData {
            code: "ShiftLeft".into(),
            key: "Shift".into(),
            key_code: 16,
            meta_state: if own_mask { WebRawKeyData::SHIFT } else { 0 },
            ..Default::default()
        }
        .into_event(KeyState::Down, false, false);
        assert!(!runtime.dispatch_key_message(KeyMessage::Raw(shift)));

        let mut expected = vec![
            (Code::KeyA, KeyState::Down, HardwareKeyOrigin::Supplied),
            (Code::ShiftLeft, KeyState::Down, HardwareKeyOrigin::Supplied),
        ];
        if !own_mask {
            expected.push((
                Code::ShiftLeft,
                KeyState::Up,
                HardwareKeyOrigin::LegacyModifierReconciliation,
            ));
        }
        assert_eq!(hardware_log.take(), expected);
        assert_eq!(
            focus_log.take(),
            expected
                .into_iter()
                .map(|(code, state, _)| (code, state))
                .collect::<Vec<_>>()
        );
        assert_eq!(raw_log.take(), [Code::KeyA, Code::ShiftLeft]);
        let mut pressed = std::collections::HashSet::from([Code::KeyA]);
        if own_mask {
            pressed.insert(Code::ShiftLeft);
        }
        assert_eq!(runtime.hardware_keyboard().physical_keys_pressed(), pressed);
    }
}
