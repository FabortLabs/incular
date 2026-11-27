use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_platform::{
    FlutterChannelMicrotask, FlutterInboundMessenger, FlutterScribbleClient, TextInputCommand,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use incular_runtime::{FlutterTextInputBinding, Runtime};
use incular_text::TextEditingController;
use incular_widgets::{EditableText, FocusNode, SizedBox};
use serde_json::{Value, json};

const CHANNEL: &str = "flutter/textinput";

struct Host {
    inbound: FlutterInboundMessenger,
    tasks: Rc<RefCell<VecDeque<FlutterChannelMicrotask>>>,
}

impl Host {
    fn new() -> Self {
        let tasks = Rc::new(RefCell::new(VecDeque::new()));
        let inbound = FlutterInboundMessenger::new({
            let tasks = tasks.clone();
            move |task| tasks.borrow_mut().push_back(task)
        });
        Self { inbound, tasks }
    }

    fn send(&self, method: &str, arguments: Value) -> Option<Vec<u8>> {
        let bytes = JsonMethodCodec::encode_method_call(&MethodCall {
            method: method.into(),
            arguments,
        })
        .unwrap();
        let reply = Rc::new(RefCell::new(Vec::new()));
        self.inbound.push(CHANNEL, Some(bytes), {
            let reply = reply.clone();
            move |payload| reply.borrow_mut().push(payload)
        });
        loop {
            let task = self.tasks.borrow_mut().pop_front();
            let Some(task) = task else { break };
            task();
        }
        let mut replies = reply.take();
        assert_eq!(
            replies.len(),
            1,
            "each received method has one binary reply"
        );
        replies.pop().unwrap()
    }
}

struct ScribbleTarget;

impl FlutterScribbleClient for ScribbleTarget {
    fn bounds(&self) -> Rect {
        Rect::from_origin_size(Offset::new(4., 5.), Size::new(10., 20.))
    }

    fn on_scribble_focus(&self, _position: Offset) {}
}

#[test]
fn startup_text_input_channel_returns_an_encoded_null_instead_of_no_handler() {
    let runtime = Rc::new(RefCell::new(
        Runtime::new(SizedBox::shrink().into()).unwrap(),
    ));
    let host = Host::new();
    let binding = FlutterTextInputBinding::install(&host.inbound, &runtime);

    let bytes = host
        .send("TextInput.requestElementsInRect", Value::Null)
        .unwrap();
    assert_eq!(bytes, b"[null]");
    assert_eq!(
        JsonMethodCodec::decode_envelope(&bytes).unwrap(),
        Value::Null
    );
    assert!(runtime.borrow_mut().take_text_input_commands().is_empty());

    binding
        .scribble_registry()
        .borrow_mut()
        .register_element("result", Rc::new(ScribbleTarget));
    let scribble = host
        .send(
            "TextInputClient.requestElementsInRect",
            json!([0, 0, 40, 40]),
        )
        .unwrap();
    assert_eq!(
        JsonMethodCodec::decode_envelope(&scribble).unwrap(),
        json!([["result", 4.0, 5.0, 10.0, 20.0]])
    );
    drop(runtime);
    assert_eq!(
        host.send("TextInput.requestElementsInRect", Value::Null),
        None
    );
}

#[test]
fn the_same_receiver_routes_full_editing_state_only_to_the_current_attachment() {
    let text = TextEditingController::with_text("old");
    let focus = FocusNode::new();
    let runtime = Rc::new(RefCell::new(
        Runtime::new(
            EditableText::new(text.clone())
                .text_direction(TextDirection::Ltr)
                .focus_node(focus.clone())
                .into(),
        )
        .unwrap(),
    ));
    let host = Host::new();
    let _binding = FlutterTextInputBinding::install(&host.inbound, &runtime);
    assert_eq!(
        host.send("TextInput.requestElementsInRect", Value::Null)
            .unwrap(),
        b"[null]"
    );

    focus.request_focus();
    runtime
        .borrow_mut()
        .run_frame(Constraints::tight(Size::new(180., 50.)))
        .unwrap();
    let client = runtime
        .borrow_mut()
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client.get()),
            _ => None,
        })
        .expect("the requested focus attaches the editor");
    let method = "TextInputClient.updateEditingState";
    let state = json!({"text": "new", "selectionBase": 3, "selectionExtent": 3});
    assert_eq!(
        host.send(method, json!([client + 1, state])).unwrap(),
        b"[null]"
    );
    assert_eq!(text.text(), "old");
    assert_eq!(
        host.send(method, json!([client, state])).unwrap(),
        b"[null]"
    );
    assert_eq!(text.text(), "new");
    assert_eq!(host.send("TextInputClient.unknown", json!([client])), None);
}
