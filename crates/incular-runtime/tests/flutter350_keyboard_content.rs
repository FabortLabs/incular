use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{KeyboardInsertedContent, TextInputClientId, TextInputCommand};
use incular_runtime::{FlutterKeyboardContentError, Runtime};
use incular_text::TextEditingController;
use incular_widgets::{
    ContentInsertionConfiguration, DEFAULT_CONTENT_INSERTION_MIME_TYPES, EditableText,
};
use serde_json::{Value, json};

const URI: &str = "content://com.google.android.inputmethod.latin.fileprovider/test.gif";
const DATA: &[u8] = &[0, 1, 0, 1, 0, 1, 0, 0, 0];

fn mount(
    configuration: Option<ContentInsertionConfiguration>,
) -> (
    Runtime,
    TextEditingController,
    TextInputClientId,
    Option<Vec<String>>,
) {
    let controller = TextEditingController::with_text("test");
    let mut editor = EditableText::new(controller.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .size(Size::new(180., 40.));
    if let Some(configuration) = configuration {
        editor = editor.content_insertion_configuration(configuration);
    }
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(180., 80.)))
        .unwrap();
    let (id, traits) = runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| {
            if let TextInputCommand::SetClient { configuration, .. } = command {
                Some((configuration.client, configuration.traits))
            } else {
                None
            }
        })
        .expect("mounted editor must attach");
    (
        runtime,
        controller,
        id,
        traits.content_mime_types.as_deref().map(<[String]>::to_vec),
    )
}

fn send(
    runtime: &mut Runtime,
    sender: Value,
    action: &str,
    content: Value,
) -> Result<Option<bool>, FlutterKeyboardContentError> {
    let wire = json!({
        "method": "TextInputClient.performAction",
        "args": [sender, action, content],
    });
    runtime.handle_flutter_keyboard_content_call(wire.to_string().as_bytes())
}

fn commit(
    runtime: &mut Runtime,
    sender: Value,
    content: Value,
) -> Result<Option<bool>, FlutterKeyboardContentError> {
    send(runtime, sender, "TextInputAction.commitContent", content)
}

#[test]
fn mounted_editor_delivers_allowed_keyboard_content_without_changing_text() {
    let inserted = Rc::new(RefCell::new(Vec::<KeyboardInsertedContent>::new()));
    let seen = inserted.clone();
    let configuration =
        ContentInsertionConfiguration::new(move |content| seen.borrow_mut().push(content))
            .with_allowed_mime_types(["image/gif"])
            .unwrap();
    let (mut runtime, controller, client, advertised) = mount(Some(configuration));
    assert_eq!(advertised, Some(vec!["image/gif".into()]));
    assert!(inserted.borrow().is_empty());

    let sender = if cfg!(debug_assertions) {
        json!(-1)
    } else {
        json!(client.get())
    };
    assert_eq!(
        commit(
            &mut runtime,
            sender,
            json!({"mimeType":"image/gif", "uri":URI, "data": DATA})
        )
        .unwrap(),
        Some(true)
    );
    assert_eq!(controller.text(), "test");
    let inserted = inserted.borrow();
    assert_eq!(inserted.len(), 1);
    assert_eq!(inserted[0].uri, URI);
    assert_eq!(inserted[0].mime_type, "image/gif");
    assert_eq!(inserted[0].data.as_deref(), Some(DATA));
    assert!(inserted[0].has_data());
}

#[test]
fn default_allowlist_and_payload_validation_preserve_missing_and_empty_bytes() {
    let inserted = Rc::new(RefCell::new(Vec::<KeyboardInsertedContent>::new()));
    let seen = inserted.clone();
    let configuration =
        ContentInsertionConfiguration::new(move |content| seen.borrow_mut().push(content));
    assert_eq!(
        configuration.allowed_mime_types(),
        DEFAULT_CONTENT_INSERTION_MIME_TYPES
    );
    assert!(
        configuration
            .clone()
            .with_allowed_mime_types(Vec::<String>::new())
            .is_err()
    );
    let (mut runtime, controller, client, advertised) = mount(Some(configuration));
    assert_eq!(advertised.unwrap(), DEFAULT_CONTENT_INSERTION_MIME_TYPES);
    let sender = json!(client.get());
    assert_eq!(
        commit(
            &mut runtime,
            sender.clone(),
            json!({"mimeType":"image/png", "uri":"content://pending"})
        )
        .unwrap(),
        Some(true)
    );
    assert_eq!(
        commit(
            &mut runtime,
            sender.clone(),
            json!({"mimeType":"image/png", "uri":"content://empty", "data":[]})
        )
        .unwrap(),
        Some(true)
    );
    assert_eq!(
        commit(
            &mut runtime,
            sender.clone(),
            json!({"mimeType":"text/html", "uri":"content://blocked"})
        )
        .unwrap(),
        Some(false)
    );
    assert_eq!(
        commit(
            &mut runtime,
            json!(client.get() + 1),
            json!({"mimeType":"image/png", "uri":"content://stale"})
        )
        .unwrap(),
        Some(false)
    );
    assert!(matches!(
        commit(
            &mut runtime,
            sender.clone(),
            json!({"mimeType":"image/png", "uri":"content://wrong", "data":[0, "x"]})
        ),
        Err(FlutterKeyboardContentError::Content(
            incular_platform::KeyboardInsertedContentError::InvalidByte { index: 1 }
        ))
    ));
    assert_eq!(
        send(&mut runtime, sender, "TextInputAction.done", Value::Null).unwrap(),
        None
    );
    assert_eq!(controller.text(), "test");
    let inserted = inserted.borrow();
    assert_eq!(inserted.len(), 2);
    assert_eq!(inserted[0].data, None);
    assert_eq!(inserted[1].data, Some(Vec::new()));
    assert!(!inserted[0].has_data());
    assert!(!inserted[1].has_data());
}

#[test]
fn an_editor_without_content_insertion_configuration_accepts_no_rich_content() {
    let (mut runtime, controller, client, advertised) = mount(None);
    assert_eq!(advertised, None);
    assert_eq!(
        commit(
            &mut runtime,
            json!(client.get()),
            json!({"mimeType":"image/gif", "uri":URI, "data": DATA})
        )
        .unwrap(),
        Some(false)
    );
    assert_eq!(controller.text(), "test");
}

#[test]
fn commit_content_invokes_the_active_clients_handler_once() {
    let inserted = Rc::new(RefCell::new(Vec::<KeyboardInsertedContent>::new()));
    let seen = inserted.clone();
    let configuration =
        ContentInsertionConfiguration::new(move |content| seen.borrow_mut().push(content));
    let (mut runtime, _, client, _) = mount(Some(configuration));
    assert!(inserted.borrow().is_empty());
    assert_eq!(
        commit(
            &mut runtime,
            json!(client.get()),
            json!({"mimeType":"image/gif", "uri":URI, "data": DATA})
        )
        .unwrap(),
        Some(true)
    );
    assert_eq!(inserted.borrow().len(), 1);
    assert_eq!(inserted.borrow()[0].uri, URI);
}
