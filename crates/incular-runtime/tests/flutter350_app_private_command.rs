use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{TextInputClientId, TextInputCommand};
use incular_runtime::{FlutterTextInputPrivateCommandError, Runtime, TextEditingCallback};
use incular_text::TextEditingController;
use incular_widgets::{Column, EditableText, FocusNode, Widget};
use serde_json::{Map, Value, json};

type Calls = Rc<RefCell<Vec<(String, Value)>>>;

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(180., 80.)))
        .unwrap();
}

fn attached(runtime: &mut Runtime) -> TextInputClientId {
    runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .expect("a real editor was attached")
}

fn field(controller: &TextEditingController, node: &FocusNode, autofocus: bool) -> EditableText {
    EditableText::new(controller.clone())
        .focus_node(node.clone())
        .autofocus(autofocus)
        .text_direction(TextDirection::Ltr)
        .size(Size::new(180., 40.))
}

fn listener(calls: Calls) -> impl Fn(&str, &Map<String, Value>) {
    move |action, data| {
        calls
            .borrow_mut()
            .push((action.to_owned(), Value::Object(data.clone())));
    }
}

fn send(
    runtime: &Runtime,
    client: TextInputClientId,
    payload: Value,
) -> Result<Option<bool>, FlutterTextInputPrivateCommandError> {
    let data = serde_json::to_vec(&json!({
        "method": "TextInputClient.performPrivateCommand",
        "args": [client.get(), payload],
    }))
    .unwrap();
    runtime.handle_flutter_text_input_private_command_call(&data)
}

#[test]
fn forwards_original_strings_floats_arrays_and_empty_data_without_editing() {
    let controller = TextEditingController::with_text("test3");
    let node = FocusNode::new();
    let calls = Calls::default();
    let editor = field(&controller, &node, true).on_app_private_command(listener(calls.clone()));
    let mut runtime = Runtime::new(editor.into()).unwrap();
    frame(&mut runtime);
    let client = attached(&mut runtime);
    let original = controller.with_value(Clone::clone);
    assert!(calls.borrow().is_empty());

    let test_data = [
        json!({"input_context": "abcdefg"}),
        json!({"input_context": 0.5}),
        json!({"input_context": ["abc", "efg"]}),
        json!({"input_context": "abc"}),
        json!({"input_context": [0.5, 0.8]}),
    ];
    for data in &test_data {
        assert_eq!(
            send(
                &runtime,
                client,
                json!({"action": "actionCommand", "data": data}),
            )
            .unwrap(),
            Some(true)
        );
    }
    for payload in [
        json!({"action": "actionCommand"}),
        json!({"action": "actionCommand", "data": null}),
    ] {
        assert_eq!(send(&runtime, client, payload).unwrap(), Some(true));
    }
    let expected = test_data
        .into_iter()
        .chain([json!({}), json!({})])
        .map(|data| ("actionCommand".to_owned(), data))
        .collect::<Vec<_>>();
    assert_eq!(*calls.borrow(), expected);
    assert_eq!(controller.with_value(Clone::clone), original);
    assert!(node.has_primary_focus());
    assert!(runtime.take_text_input_commands().is_empty());
}

#[test]
fn stale_or_malformed_payloads_never_call_the_current_listener() {
    let [first, second] = [FocusNode::new(), FocusNode::new()];
    let controller = TextEditingController::new();
    let [first_calls, second_calls] = [Calls::default(), Calls::default()];
    let editor = |node, autofocus, calls| {
        Widget::from(field(&controller, node, autofocus).on_app_private_command(listener(calls)))
    };
    let mut runtime = Runtime::new(
        Column::new([
            editor(&first, true, first_calls.clone()),
            editor(&second, false, second_calls.clone()),
        ])
        .into(),
    )
    .unwrap();
    frame(&mut runtime);
    let stale = attached(&mut runtime);
    second.request_focus();
    frame(&mut runtime);
    let active = attached(&mut runtime);
    assert_ne!(stale, active);

    assert_eq!(send(&runtime, stale, json!(false)).unwrap(), Some(false));
    assert!(matches!(
        send(&runtime, active, json!(false)),
        Err(FlutterTextInputPrivateCommandError::InvalidArguments)
    ));
    assert!(matches!(
        send(&runtime, active, json!({"action": null})),
        Err(FlutterTextInputPrivateCommandError::InvalidAction)
    ));
    assert!(matches!(
        send(
            &runtime,
            active,
            json!({"action": "actionCommand", "data": [1]})
        ),
        Err(FlutterTextInputPrivateCommandError::InvalidData)
    ));
    assert!(first_calls.borrow().is_empty());
    assert!(second_calls.borrow().is_empty());

    assert_eq!(
        send(&runtime, active, json!({"action": "actionCommand"})).unwrap(),
        Some(true)
    );
    assert!(first_calls.borrow().is_empty());
    assert_eq!(second_calls.borrow().len(), 1);
    assert!(second.has_primary_focus());
}

#[test]
fn rebuilding_replaces_only_the_listener_on_the_retained_attachment() {
    let controller = TextEditingController::new();
    let node = FocusNode::new();
    let [before, after] = [Calls::default(), Calls::default()];
    let build = |calls| field(&controller, &node, true).on_app_private_command(listener(calls));
    let mut runtime = Runtime::new(build(before.clone()).into()).unwrap();
    frame(&mut runtime);
    let client = attached(&mut runtime);
    let root = runtime.tree().root().unwrap();

    runtime
        .schedule_update(root, build(after.clone()).into())
        .unwrap();
    frame(&mut runtime);
    assert!(
        !runtime
            .take_text_input_commands()
            .iter()
            .any(|command| { matches!(command, TextInputCommand::SetClient { .. }) })
    );

    assert_eq!(
        send(&runtime, client, json!({"action": "actionCommand"})).unwrap(),
        Some(true)
    );
    assert!(before.borrow().is_empty());
    assert_eq!(after.borrow().len(), 1);
    assert!(node.has_primary_focus());
}

#[test]
fn listener_panic_is_reported_and_does_not_close_the_connection() {
    let controller = TextEditingController::new();
    let node = FocusNode::new();
    let editor = field(&controller, &node, true)
        .on_app_private_command(|_, _| panic!("expected callback failure"));
    let mut runtime = Runtime::new(editor.into()).unwrap();
    let failures = Rc::new(RefCell::new(Vec::new()));
    let output = failures.clone();
    runtime.observe_text_editing_error(move |error| output.borrow_mut().push(error.clone()));
    frame(&mut runtime);
    let client = attached(&mut runtime);

    assert_eq!(
        send(&runtime, client, json!({"action": "actionCommand"})).unwrap(),
        Some(true)
    );
    let failures = failures.borrow();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].callback, TextEditingCallback::AppPrivateCommand);
    assert_eq!(failures[0].message, "expected callback failure");
    assert!(node.has_primary_focus());
    assert!(runtime.take_text_input_commands().is_empty());
}
