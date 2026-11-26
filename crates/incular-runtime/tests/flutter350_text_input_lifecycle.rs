//! A native connection ends without submitting and may be reopened only for
//! the same eligible mounted editor.
use incular_config::Constraints;
use incular_core::Size;
use incular_platform::{PlatformEvent, TextInputClientId, TextInputCommand};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::TextEditingController;
use incular_widgets::{Column, EditableText, FocusNode, Offstage, SizedBox, TextDirection, Widget};
use std::{cell::Cell, rc::Rc};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(180., 80.)))
        .unwrap();
}

fn attached(runtime: &mut Runtime) -> TextInputClientId {
    let commands = runtime.take_text_input_commands();
    let client = commands
        .iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .expect("native client attached");
    assert!(
        commands
            .iter()
            .any(|command| *command == TextInputCommand::Show { client })
    );
    client
}

fn platform_call(runtime: &mut Runtime, name: &str, client: TextInputClientId) -> bool {
    let data = serde_json::json!({
        "method": format!("TextInputClient.{name}"),
        "args": [client.get()],
    });
    runtime
        .handle_flutter_text_input_lifecycle_call(data.to_string().as_bytes())
        .expect("valid channel message")
        .expect("lifecycle method")
}

fn field(controller: &TextEditingController, node: &FocusNode, autofocus: bool) -> EditableText {
    EditableText::new(controller.clone())
        .focus_node(node.clone())
        .autofocus(autofocus)
        .text_direction(TextDirection::Ltr)
        .size(Size::new(180., 40.))
}

#[test]
fn closing_the_connection_is_silent_and_focus_received_reattaches_with_a_new_id() {
    let controller = TextEditingController::with_text("test");
    let node = FocusNode::new();
    let completed = Rc::new(Cell::new(false));
    let submitted = Rc::new(Cell::new(false));
    let [c, s] = [completed.clone(), submitted.clone()];
    let mut runtime = Runtime::new(
        field(&controller, &node, true)
            .on_editing_complete(move || c.set(true))
            .on_submit(move |_| s.set(true))
            .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    frame(&mut runtime);
    let first = attached(&mut runtime);
    assert!(node.has_primary_focus());
    assert!(!platform_call(&mut runtime, "onFocusReceived", first));

    assert!(platform_call(&mut runtime, "onConnectionClosed", first));
    frame(&mut runtime);
    assert!(!node.has_focus());
    assert_eq!(controller.text(), "test");
    assert!(!completed.get());
    assert!(!submitted.get());
    assert!(runtime.take_text_input_commands().is_empty());
    assert!(!platform_call(&mut runtime, "onConnectionClosed", first));

    assert!(platform_call(&mut runtime, "onFocusReceived", first));
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    let next = attached(&mut runtime);
    assert_ne!(first, next);
    assert!(!platform_call(&mut runtime, "onConnectionClosed", first));
    assert!(!platform_call(&mut runtime, "onFocusReceived", first));
    assert!(node.has_primary_focus());
    assert!(runtime.take_text_input_commands().is_empty());
}

#[test]
fn close_then_refocus_respects_retained_offstage_and_unmounted_editors() {
    let controller = TextEditingController::new();
    let node = FocusNode::new();
    let editor: Widget = field(&controller, &node, true).into();
    let mut runtime =
        Runtime::new(Column::new([Widget::from(Offstage::new(editor))]).into()).unwrap();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    let client = attached(&mut runtime);
    let _ = runtime.handle_platform_event(PlatformEvent::TextInputConnectionClosed(client));
    frame(&mut runtime);
    assert!(!node.has_focus());
    assert!(runtime.take_text_input_commands().is_empty());
    assert!(platform_call(&mut runtime, "onFocusReceived", client));
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    let next = attached(&mut runtime);
    assert!(platform_call(&mut runtime, "onConnectionClosed", next));
    frame(&mut runtime);
    assert!(!node.has_focus());

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, Column::new([Widget::from(SizedBox::shrink())]).into())
        .unwrap();
    frame(&mut runtime);
    assert!(!platform_call(&mut runtime, "onFocusReceived", next));
    frame(&mut runtime);
    assert!(!node.has_focus());
    assert!(runtime.take_text_input_commands().is_empty());
}

#[test]
fn a_delayed_host_message_does_not_steal_focus_from_another_editor() {
    let [a, b] = [TextEditingController::new(), TextEditingController::new()];
    let [first_node, next_node] = [FocusNode::new(), FocusNode::new()];
    let first: Widget = field(&a, &first_node, true).into();
    let next: Widget = field(&b, &next_node, false).into();
    let mut runtime = Runtime::new(Column::new([first, next]).into()).unwrap();
    frame(&mut runtime);
    let first_client = attached(&mut runtime);

    next_node.request_focus();
    frame(&mut runtime);
    let next_client = attached(&mut runtime);
    assert_ne!(first_client, next_client);
    assert!(next_node.has_primary_focus());
    assert!(!platform_call(
        &mut runtime,
        "onFocusReceived",
        first_client
    ));
    assert!(!platform_call(
        &mut runtime,
        "onConnectionClosed",
        first_client
    ));
    assert!(next_node.has_primary_focus());
    assert!(runtime.take_text_input_commands().is_empty());

    assert!(platform_call(
        &mut runtime,
        "onConnectionClosed",
        next_client
    ));
    frame(&mut runtime);
    next_node.set_can_request_focus(false);
    assert!(!platform_call(&mut runtime, "onFocusReceived", next_client));
    frame(&mut runtime);
    assert!(!next_node.has_focus());
    assert!(runtime.take_text_input_commands().is_empty());
}
