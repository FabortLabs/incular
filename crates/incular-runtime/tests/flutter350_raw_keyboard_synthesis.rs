use std::{cell::RefCell, rc::Rc};

use incular_core::{Code, HardwareKeyOrigin, KeyState};
use incular_runtime::{KeyMessage, Runtime};
use incular_widgets::{SizedBox, WebRawKeyData};

fn packet(state: KeyState, shift: bool) -> incular_widgets::RawKeyEvent {
    WebRawKeyData {
        code: "KeyA".into(),
        key: "a".into(),
        meta_state: if shift { WebRawKeyData::SHIFT } else { 0 },
        key_code: 65,
        ..WebRawKeyData::default()
    }
    .into_event(state, false, false)
}

#[test]
fn raw_only_infers_missing_modifier_but_paired_input_trusts_typed_packets() {
    for paired in [false, true] {
        let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let raw_log = events.clone();
        let _raw = runtime.add_raw_keyboard_listener(move |event| {
            raw_log
                .borrow_mut()
                .push(("raw", event.physical_key(), event.portable().state));
        });
        let hardware_log = events.clone();
        let _hardware = runtime.add_hardware_key_handler_with_origin(move |event, origin| {
            let label = match origin {
                HardwareKeyOrigin::Supplied => "hardware",
                HardwareKeyOrigin::LegacyModifierReconciliation => "synthesized",
                HardwareKeyOrigin::EngineSynthesized => "engine synthesized",
            };
            hardware_log
                .borrow_mut()
                .push((label, event.code, event.state));
            false
        });
        for (state, shift) in [(KeyState::Down, true), (KeyState::Up, false)] {
            let raw = packet(state, shift);
            let message = if paired {
                KeyMessage::TypedAndRaw {
                    typed: raw.portable().clone(),
                    raw,
                }
            } else {
                KeyMessage::Raw(raw)
            };
            assert!(!runtime.dispatch_key_message(message));
            let mut expected = vec![("raw", Code::KeyA, state)];
            if !paired {
                expected.push(("synthesized", Code::ShiftLeft, state));
            }
            expected.push(("hardware", Code::KeyA, state));
            assert_eq!(events.take(), expected, "paired={paired}");
            assert_eq!(
                runtime
                    .hardware_keyboard()
                    .is_physical_key_pressed(Code::ShiftLeft),
                shift && !paired
            );
        }
    }
}
