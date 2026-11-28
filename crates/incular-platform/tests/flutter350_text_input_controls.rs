use std::{cell::RefCell, rc::Rc};

use incular_config::TextDirection;
use incular_core::{Offset, Rect, Size};
use incular_platform::{
    FlutterSelectionRect, FlutterTextInputAlignment, FlutterTextInputClientEvent as ClientEvent,
    FlutterTextInputConfiguration as Configuration, FlutterTextInputControl as Control,
    FlutterTextInputControlUpdate as Update, FlutterTextInputControls as Controls,
    FlutterTextInputStyle, FlutterTextInputWireCall, TextInputClientId, TextInputType,
    autofill::AutofillEditingValue,
};
use serde_json::{Value, json};

type Log = Rc<RefCell<Vec<Update>>>;

fn controls(web: bool) -> (Controls, Log, Control) {
    let mut controls = Controls::new(web);
    let calls: Log = Rc::default();
    let observed = calls.clone();
    let custom = Control::custom(move |update| observed.borrow_mut().push(update.clone()));
    controls.set_input_control(custom.clone());
    (controls, calls, custom)
}

fn attach(controls: &mut Controls) -> TextInputClientId {
    controls.attach(Configuration::default(), |_| {})
}

fn method(update: &Update) -> &'static str {
    match update {
        Update::Attach { .. } => "attach",
        Update::Detach(_) => "detach",
        Update::Configuration(_) => "updateConfig",
        Update::EditingState(_) => "setEditingState",
        Update::Show => "show",
        Update::Hide => "hide",
        Update::CaretRect(_) => "setCaretRect",
        Update::ComposingRect(_) => "setComposingRect",
        Update::EditableSizeAndTransform { .. } => "setEditableSizeAndTransform",
        Update::SelectionRects(_) => "setSelectionRects",
        Update::Style(_) => "updateStyle",
        Update::RequestAutofill => "requestAutofill",
        Update::FinishAutofillContext { .. } => "finishAutofillContext",
    }
}

fn wire(method: &'static str, arguments: Value) -> FlutterTextInputWireCall {
    FlutterTextInputWireCall { method, arguments }
}

fn original(input_type: TextInputType) -> Configuration {
    Configuration {
        input_type,
        ..Configuration::default()
    }
}

#[test]
fn the_current_custom_control_is_attached_and_detached_with_the_client() {
    let (mut controls, observed, _) = controls(false);
    let client = attach(&mut controls);
    assert_eq!(
        observed.borrow().iter().map(method).collect::<Vec<_>>(),
        ["attach"]
    );
    assert!(controls.close(client));
    assert_eq!(
        observed.borrow().iter().map(method).collect::<Vec<_>>(),
        ["attach", "detach"]
    );
    assert!(!controls.is_attached(client));
    assert!(!controls.close(client));
}

#[test]
fn a_custom_control_observes_configuration_state_and_detach_in_order() {
    let (mut controls, observed, _) = controls(false);
    let client = attach(&mut controls);
    observed.borrow_mut().clear();
    assert!(
        controls
            .update(client, Update::Configuration(Configuration::default()))
            .unwrap()
    );
    assert!(
        controls
            .update(
                client,
                Update::EditingState(AutofillEditingValue::default())
            )
            .unwrap()
    );
    assert!(controls.close(client));
    assert_eq!(
        observed.borrow().iter().map(method).collect::<Vec<_>>(),
        ["updateConfig", "setEditingState", "detach"]
    );
}

#[test]
fn platform_edits_reach_the_client_and_custom_control_without_platform_echo() {
    let (mut controls, observed, _) = controls(false);
    let received = Rc::new(RefCell::new(Vec::new()));
    let active = received.clone();
    let client = controls.attach(Configuration::default(), move |event| {
        if let ClientEvent::EditingValue(value) = event {
            active.borrow_mut().push(value);
        }
    });
    let _initial = controls.take_platform_calls();
    let value = AutofillEditingValue::default();
    let message = json!({"method":"TextInputClient.updateEditingState", "args": [client.get(), value.to_json().unwrap()]});
    assert_eq!(
        controls
            .handle_platform_edit(message.to_string().as_bytes())
            .unwrap(),
        Some(true)
    );
    assert_eq!(&*received.borrow(), &[value]);
    assert_eq!(
        observed.borrow().iter().map(method).collect::<Vec<_>>(),
        ["attach", "setEditingState"]
    );
    assert!(controls.take_platform_calls().is_empty());
}

#[test]
fn both_controls_receive_the_exact_geometry_and_visibility_request_sequence() {
    let (mut controls, observed, _) = controls(false);
    let client = attach(&mut controls);
    let rectangle = Rect::from_origin_size(Offset::new(2., 3.), Size::new(4., 5.));
    let zero = Rect::from_origin_size(Offset::ZERO, Size::ZERO);
    let identity: [f64; 16] = std::array::from_fn(|index| if index % 5 == 0 { 1. } else { 0. });
    let style = FlutterTextInputStyle::new(TextDirection::Ltr, FlutterTextInputAlignment::Left);
    for request in [
        Update::Show,
        Update::Configuration(original(TextInputType::NUMBER)),
        Update::Configuration(original(TextInputType::Multiline)),
        Update::ComposingRect(zero),
        Update::CaretRect(zero),
        Update::EditableSizeAndTransform {
            size: Size::ZERO,
            transform: identity,
        },
        Update::SelectionRects(vec![FlutterSelectionRect {
            position: 1,
            bounds: rectangle,
            direction: TextDirection::Rtl,
        }]),
        Update::Style(style.clone()),
    ] {
        assert!(controls.update(client, request).unwrap());
    }
    assert!(controls.close(client));
    assert_eq!(observed.borrow().last().map(method), Some("detach"));
    assert!(controls.flush_pending_hide());
    assert!(!controls.flush_pending_hide());
    let log = observed.borrow();
    assert_eq!(
        log.iter().map(method).collect::<Vec<_>>(),
        [
            "attach",
            "show",
            "updateConfig",
            "updateConfig",
            "setComposingRect",
            "setCaretRect",
            "setEditableSizeAndTransform",
            "setSelectionRects",
            "updateStyle",
            "detach",
            "hide",
        ]
    );
    let input_types = log
        .iter()
        .filter_map(|update| match update {
            Update::Attach { configuration, .. } | Update::Configuration(configuration) => {
                Some(configuration.input_type)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        input_types,
        [
            TextInputType::Text,
            TextInputType::NUMBER,
            TextInputType::Multiline
        ]
    );

    let none = Value::Object(original(TextInputType::None).to_json());
    let empty_rect = json!({"width": 0.0,"height": 0.0,"x": 0.0,"y": 0.0});
    assert_eq!(
        controls.take_platform_calls(),
        [
            wire("TextInput.setClient", json!([1, none])),
            wire("TextInput.show", Value::Null),
            wire("TextInput.updateConfig", none.clone()),
            wire("TextInput.updateConfig", none),
            wire("TextInput.setMarkedTextRect", empty_rect.clone()),
            wire("TextInput.setCaretRect", empty_rect),
            wire(
                "TextInput.setEditableSizeAndTransform",
                json!({"width": 0.0, "height": 0.0, "transform": identity})
            ),
            wire(
                "TextInput.setSelectionRects",
                json!([[2.0, 3.0, 4.0, 5.0, 1, 0]])
            ),
            wire("TextInput.setStyle", style.to_json().unwrap()),
            wire("TextInput.clearClient", Value::Null),
            wire("TextInput.hide", Value::Null),
        ]
    );
}

#[test]
fn changing_the_current_control_notifies_the_attached_client_only_once() {
    let (mut controls, custom_calls, custom) = controls(false);
    let received = Rc::new(RefCell::new(Vec::new()));
    let observed = received.clone();
    let client = controls.attach(Configuration::default(), move |event| {
        if let ClientEvent::ControlChanged { old, new } = event {
            observed.borrow_mut().push((old, new));
        }
    });
    controls.set_input_control(custom.clone());
    assert!(received.borrow().is_empty());
    controls.set_input_control(Control::Hidden);
    assert_eq!(received.borrow().len(), 1);
    assert!(received.borrow()[0].0 == custom);
    assert!(received.borrow()[0].1 == Control::Hidden);
    assert!(controls.update(client, Update::Show).unwrap());
    assert_eq!(received.borrow().len(), 1);
    assert_eq!(
        custom_calls.borrow().iter().map(method).collect::<Vec<_>>(),
        ["attach"]
    );
}

#[test]
fn custom_keyboard_browser_configuration_preserves_multiline_and_defers_hide_safely() {
    let (mut controls, _, _) = controls(true);
    let first = controls.attach(original(TextInputType::Multiline), |_| {});
    let initial = controls.take_platform_calls();
    assert_eq!(
        initial[0].arguments[1]["inputType"]["name"],
        "TextInputType.none"
    );
    assert_eq!(initial[0].arguments[1]["inputType"]["isMultiline"], true);
    assert!(controls.close(first));

    let next = controls.attach(original(TextInputType::Text), |_| {});
    assert_ne!(next, first);
    assert!(!controls.flush_pending_hide());
    let calls = controls.take_platform_calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].method, "TextInput.clearClient");
    assert_eq!(calls[1].arguments[1]["inputType"]["isMultiline"], false);
    assert!(!controls.update(first, Update::Show).unwrap());
}

#[test]
fn repeated_geometry_does_not_send_or_notify_but_a_new_client_gets_its_own_snapshot() {
    let (mut controls, observed, _) = controls(false);
    let client = attach(&mut controls);
    let rect = Rect::from_origin_size(Offset::ZERO, Size::ZERO);
    assert!(controls.update(client, Update::CaretRect(rect)).unwrap());
    assert!(!controls.update(client, Update::CaretRect(rect)).unwrap());
    assert_eq!(controls.take_platform_calls().len(), 2);
    assert_eq!(
        observed.borrow().iter().map(method).collect::<Vec<_>>(),
        ["attach", "setCaretRect"]
    );
    let second = attach(&mut controls);
    assert!(controls.update(second, Update::CaretRect(rect)).unwrap());
    assert_eq!(controls.take_platform_calls().len(), 2);
}

#[test]
fn an_externally_bound_client_publishes_only_changes_and_does_not_echo_host_edits() {
    let mut controls = Controls::new(false);
    let client = TextInputClientId::new(7);
    assert!(controls.attach_existing(client, Configuration::default(), |_| {}));
    let initial = controls.take_platform_calls();
    assert_eq!(initial[0].arguments[0], 7);
    assert!(!controls.attach_existing(client, Configuration::default(), |_| {}));
    assert!(controls.is_attached(client));

    let value = AutofillEditingValue::default();
    assert!(
        controls
            .publish_editing_state(client, value.clone())
            .unwrap()
    );
    assert!(
        !controls
            .publish_editing_state(client, value.clone())
            .unwrap()
    );
    assert_eq!(
        controls.take_platform_calls(),
        [wire("TextInput.setEditingState", value.to_json().unwrap())]
    );

    let remote = AutofillEditingValue {
        text: "platform".into(),
        ..value
    };
    let message = json!({
        "method": "TextInputClient.updateEditingState",
        "args": [client.get(), remote.to_json().unwrap()],
    });
    assert_eq!(
        controls
            .handle_platform_edit(message.to_string().as_bytes())
            .unwrap(),
        Some(true)
    );
    assert!(!controls.publish_editing_state(client, remote).unwrap());
    assert!(controls.take_platform_calls().is_empty());
    assert!(controls.close(client));
    assert_eq!(attach(&mut controls), TextInputClientId::new(8));
}

#[test]
fn style_snapshot_replays_for_a_new_control_and_never_for_a_stale_client() {
    let mut controls = Controls::new(false);
    let old = controls.attach(Configuration::default(), |_| {});
    let style = FlutterTextInputStyle::new(TextDirection::Ltr, FlutterTextInputAlignment::Left);
    assert!(controls.publish_style(old, style.clone()).unwrap());
    assert!(!controls.publish_style(old, style.clone()).unwrap());

    let observed = Rc::new(RefCell::new(0));
    let seen = Rc::clone(&observed);
    controls.set_input_control(Control::custom(move |update| {
        if matches!(update, Update::Style(_)) {
            *seen.borrow_mut() += 1;
        }
    }));
    assert!(controls.publish_style(old, style.clone()).unwrap());
    assert_eq!(*observed.borrow(), 1);

    assert!(controls.close(old));
    let new = controls.attach(Configuration::default(), |_| {});
    assert!(!controls.publish_style(old, style.clone()).unwrap());
    assert!(controls.publish_style(new, style).unwrap());
    assert_eq!(*observed.borrow(), 2);
}

#[test]
fn a_web_custom_control_keeps_multiline_while_disabling_the_platform_keyboard() {
    for (input_type, multiline) in [
        (TextInputType::Multiline, true),
        (TextInputType::Text, false),
    ] {
        let (mut controls, observed, _) = controls(true);
        let requested = original(input_type);
        let client = controls.attach(requested.clone(), |_| {});
        assert_eq!(client.get(), 1);
        let callbacks = observed.borrow();
        let [
            Update::Attach {
                client: actual,
                configuration,
            },
        ] = callbacks.as_slice()
        else {
            panic!("expected the single attach callback, got {callbacks:?}");
        };
        assert_eq!(*actual, client);
        assert_eq!(configuration, &requested);
        let mut expected = original(TextInputType::None).to_json();
        expected.get_mut("inputType").unwrap()["isMultiline"] = json!(multiline);
        assert_eq!(
            controls.take_platform_calls(),
            [wire("TextInput.setClient", json!([1, expected]))],
        );
    }
}
