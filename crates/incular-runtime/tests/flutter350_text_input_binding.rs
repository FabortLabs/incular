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

#[test]
fn the_mounted_flutter_keyboard_configuration_tracks_its_view_without_leaking_to_another() {
    use incular_runtime::FlutterDeltaTextInputConnection;
    use incular_text::TextSelection;

    let controller = TextEditingController::with_text("one two");
    controller.set_selection(TextSelection::collapsed(3));
    let focus = FocusNode::new();
    let editor = |multiline, interactive| {
        EditableText::new(controller.clone())
            .focus_node(focus.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .multiline(multiline)
            .enable_interactive_selection(interactive)
    };
    let runtime = Rc::new(RefCell::new(
        Runtime::new(editor(false, true).into()).unwrap(),
    ));
    let host = Host::new();
    assert_eq!(
        runtime
            .borrow()
            .flutter_text_input_configuration()
            .unwrap()
            .1
            .view_id,
        None
    );
    let _binding = FlutterTextInputBinding::install_for_view(&host.inbound, &runtime, 77);
    let constraints = Constraints::tight(Size::new(260., 80.));
    let frame =
        |runtime: &Rc<RefCell<Runtime>>| runtime.borrow_mut().run_frame(constraints).unwrap();
    frame(&runtime);
    let field = runtime.borrow().focused_element().unwrap();
    let (client, initial) = runtime.borrow().flutter_text_input_configuration().unwrap();
    assert_eq!(initial.view_id, Some(77));
    assert!(initial.enable_interactive_selection);
    assert!(!initial.enable_delta_model);
    let initial_commands = runtime.borrow_mut().take_text_input_commands();
    assert_eq!(initial_commands.iter().filter(|command| {
        matches!(command, TextInputCommand::SetClient { configuration, .. } if configuration.client == client)
    }).count(), 1);

    let other = Rc::new(RefCell::new(
        Runtime::new(
            EditableText::new(TextEditingController::with_text("other"))
                .autofocus(true)
                .text_direction(TextDirection::Ltr)
                .into(),
        )
        .unwrap(),
    ));
    let other_host = Host::new();
    let _other_binding = FlutterTextInputBinding::install_for_view(&other_host.inbound, &other, 99);
    frame(&other);
    assert_eq!(
        other
            .borrow()
            .flutter_text_input_configuration()
            .unwrap()
            .1
            .view_id,
        Some(99)
    );

    // A view-only change is real configuration work, even though the native
    // command and editor values otherwise compare equal.
    runtime
        .borrow_mut()
        .set_flutter_text_input_view_id(Some(88));
    assert!(runtime.borrow().frame_requested());
    frame(&runtime);
    let commands = runtime.borrow_mut().take_text_input_commands();
    assert_eq!(commands.len(), 1);
    assert!(
        matches!(&commands[0], TextInputCommand::SetClient { configuration, .. } if configuration.client == client)
    );
    runtime
        .borrow_mut()
        .schedule_update(field, editor(true, false).into())
        .unwrap();
    frame(&runtime);
    let (same_client, moved) = runtime.borrow().flutter_text_input_configuration().unwrap();
    assert_eq!(same_client, client);
    assert_eq!(moved.view_id, Some(88));
    assert!(!moved.enable_interactive_selection);
    let encoded = FlutterDeltaTextInputConnection::new(client)
        .encode_set_client_with_config(&moved)
        .unwrap();
    let wire = JsonMethodCodec::decode_method_call(&encoded).unwrap();
    assert_eq!(wire.method, "TextInput.setClient");
    assert_eq!(wire.arguments[1]["viewId"], 88);
    assert_eq!(wire.arguments[1]["enableInteractiveSelection"], false);
    assert_eq!(
        wire.arguments[1]["inputType"]["name"],
        "TextInputType.multiline"
    );
    assert_eq!(
        other
            .borrow()
            .flutter_text_input_configuration()
            .unwrap()
            .1
            .view_id,
        Some(99)
    );
    assert_eq!(runtime.borrow().focused_element(), Some(field));
    assert_eq!(controller.selection(), TextSelection::collapsed(3));
    assert!(focus.has_primary_focus());

    runtime.borrow_mut().take_text_input_commands();
    runtime
        .borrow_mut()
        .set_flutter_text_input_view_id(Some(123));
    runtime
        .borrow_mut()
        .set_flutter_text_input_view_id(Some(88));
    frame(&runtime);
    assert!(runtime.borrow_mut().take_text_input_commands().is_empty());
}

#[test]
fn only_selection_changes_during_a_live_scribble_interaction_report_handwriting() {
    use incular_runtime::TextShortcutPlatform;
    use incular_text::{SelectionChangedCause, TextSelection};

    let text = TextEditingController::with_text("Lorem ipsum dolor sit amet");
    let focus = FocusNode::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let runtime = Rc::new(RefCell::new(
        Runtime::new(
            EditableText::new(text.clone())
                .text_direction(TextDirection::Ltr)
                .focus_node(focus.clone())
                .on_selection_changed({
                    let changes = changes.clone();
                    move |selection, cause| changes.borrow_mut().push((selection, cause))
                })
                .into(),
        )
        .unwrap(),
    ));
    runtime
        .borrow_mut()
        .set_text_shortcut_platform(TextShortcutPlatform::Ios);
    let host = Host::new();
    let _binding = FlutterTextInputBinding::install(&host.inbound, &runtime);
    let layout = || {
        runtime
            .borrow_mut()
            .run_frame(Constraints::tight(Size::new(300., 100.)))
            .unwrap();
    };
    focus.request_focus();
    layout();
    let client = runtime
        .borrow()
        .flutter_text_input_configuration()
        .unwrap()
        .0
        .get();
    changes.borrow_mut().clear();
    let set = |value, base, extent| {
        assert_eq!(
            host.send(
                "TextInputClient.updateEditingState",
                json!([client, {"text": value, "selectionBase": base, "selectionExtent": extent}])
            )
            .unwrap(),
            b"[null]"
        );
        layout();
    };

    set("Lorem ipsum dolor sit amet", 2, 3);
    assert_eq!(
        changes.take(),
        [(TextSelection::new(2, 3), SelectionChangedCause::Keyboard)]
    );
    assert_eq!(
        host.send("TextInputClient.scribbleInteractionBegan", Value::Null)
            .unwrap(),
        b"[null]"
    );
    set("Lorem ipsum dolor sit amet", 3, 4);
    assert_eq!(
        changes.take(),
        [(
            TextSelection::new(3, 4),
            SelectionChangedCause::StylusHandwriting
        )]
    );
    set("Lorem ipsum dolor sit amet", 3, 4);
    assert!(
        changes.borrow().is_empty(),
        "repeating the value has no selection event"
    );

    // Replacing actual text stays a keyboard edit even inside Scribble.
    set("Lorem ipsum dolor sit amet!", 5, 5);
    assert_eq!(
        changes.take(),
        [(TextSelection::collapsed(5), SelectionChangedCause::Keyboard)]
    );
    assert_eq!(
        host.send("TextInputClient.scribbleInteractionFinished", Value::Null)
            .unwrap(),
        b"[null]"
    );
    set("Lorem ipsum dolor sit amet!", 6, 7);
    assert_eq!(
        changes.take(),
        [(TextSelection::new(6, 7), SelectionChangedCause::Keyboard)]
    );
}

#[test]
fn repeated_toolbar_show_keeps_the_same_mounted_overlay_during_handwriting() {
    assert_duplicate_toolbar_show_is_inert(true);
}

#[test]
fn repeated_toolbar_show_keeps_the_same_mounted_overlay_after_autofocus() {
    assert_duplicate_toolbar_show_is_inert(false);
}

fn assert_duplicate_toolbar_show_is_inert(handwriting: bool) {
    use incular_runtime::TextShortcutPlatform;
    use incular_widgets::{Widget, internal::Key};

    const TOOLBAR: &str = "scribble-toolbar";
    let text = TextEditingController::with_text("Lorem ipsum dolor sit amet");
    let focus = FocusNode::new();
    let runtime = Rc::new(RefCell::new(
        Runtime::new(
            EditableText::new(text)
                .text_direction(TextDirection::Ltr)
                .focus_node(focus.clone())
                .autofocus(!handwriting)
                .context_menu_builder(|_, _| {
                    Widget::from(SizedBox::new().width(20.).height(10.)).with_key(TOOLBAR)
                })
                .into(),
        )
        .unwrap(),
    ));
    runtime
        .borrow_mut()
        .set_text_shortcut_platform(TextShortcutPlatform::Ios);
    let host = Host::new();
    let _binding = FlutterTextInputBinding::install(&host.inbound, &runtime);
    let layout = || {
        runtime
            .borrow_mut()
            .run_frame(Constraints::tight(Size::new(300., 100.)))
            .unwrap();
    };
    let toolbar = || {
        runtime
            .borrow()
            .tree()
            .element_with_key(&Key::from(TOOLBAR))
    };
    layout();
    assert!(toolbar().is_none());
    if handwriting {
        focus.request_focus();
        layout();
    }
    assert!(focus.has_primary_focus());
    let client = runtime
        .borrow()
        .flutter_text_input_configuration()
        .unwrap()
        .0
        .get();
    if handwriting {
        assert_eq!(
            host.send("TextInputClient.scribbleInteractionBegan", Value::Null)
                .unwrap(),
            b"[null]"
        );
        assert_eq!(
            host.send(
                "TextInputClient.updateEditingState",
                json!([client, {"text": "Lorem ipsum dolor sit amet", "selectionBase": 3, "selectionExtent": 4}])
            ).unwrap(),
            b"[null]"
        );
        layout();
        assert!(
            toolbar().is_none(),
            "handwriting selection does not open the toolbar by itself"
        );
    }

    let show = JsonMethodCodec::encode_method_call(&MethodCall {
        method: "TextInputClient.showToolbar".into(),
        arguments: json!([client]),
    })
    .unwrap();
    assert_eq!(
        runtime
            .borrow_mut()
            .handle_flutter_text_toolbar_call(&show)
            .unwrap(),
        Some(true)
    );
    layout();
    let mounted = toolbar().unwrap();
    assert_eq!(
        runtime
            .borrow_mut()
            .handle_flutter_text_toolbar_call(&show)
            .unwrap(),
        Some(false)
    );
    layout();
    assert_eq!(toolbar(), Some(mounted));
    layout();
    assert_eq!(toolbar(), Some(mounted));
    if handwriting {
        assert_eq!(
            host.send("TextInputClient.scribbleInteractionFinished", Value::Null)
                .unwrap(),
            b"[null]"
        );
    }
}

#[test]
fn native_handwriting_discovers_only_touchable_mounted_editors_and_focuses_their_caret() {
    use incular_core::Color;
    use incular_runtime::TextShortcutPlatform;
    use incular_text::{SelectionChangedCause, TextSelection};
    use incular_widgets::{ColoredBox, Positioned, Stack, Widget};

    let controller = TextEditingController::with_text("Lorem ipsum dolor sit amet");
    let focus = FocusNode::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let root = |read_only, handwriting, covered| {
        let editor = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .focus_node(focus.clone())
            .read_only(read_only)
            .stylus_handwriting_enabled(handwriting)
            .on_selection_changed({
                let changes = changes.clone();
                move |selection, cause| changes.borrow_mut().push((selection, cause))
            });
        let mut children = vec![Widget::from(Positioned::fill(editor))];
        if covered {
            children.push(Widget::from(Positioned::fill(ColoredBox::new(
                Color::rgba(0, 0, 0, 255),
                SizedBox::expand(),
            ))));
        }
        Widget::from(Stack::new(children))
    };
    let runtime = Rc::new(RefCell::new(
        Runtime::new(root(false, true, false)).unwrap(),
    ));
    runtime
        .borrow_mut()
        .set_text_shortcut_platform(TextShortcutPlatform::Ios);
    let host = Host::new();
    let _binding = FlutterTextInputBinding::install(&host.inbound, &runtime);
    let layout = || {
        runtime
            .borrow_mut()
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
    };
    let query = |rect| {
        let response = host
            .send("TextInputClient.requestElementsInRect", rect)
            .unwrap();
        JsonMethodCodec::decode_envelope(&response).unwrap()
    };
    let replace = |widget| {
        let mut runtime = runtime.borrow_mut();
        let id = runtime.tree().root().unwrap();
        runtime.schedule_update(id, widget).unwrap();
    };
    layout();
    assert!(!focus.has_focus());
    let found = query(json!([0, 0, 1, 1]));
    assert_eq!(found.as_array().unwrap().len(), 1);
    let first = found[0][0].as_str().unwrap();
    assert_eq!(found[0], json!([first, 0., 0., 800., 600.]));
    assert_eq!(query(json!([-1, -1, 1, 1])), json!([]));

    assert_eq!(
        host.send("TextInputClient.focusElement", json!([first, 0, 0]))
            .unwrap(),
        b"[null]"
    );
    layout();
    assert!(focus.has_primary_focus());
    assert_eq!(controller.selection(), TextSelection::collapsed(0));
    assert_eq!(
        changes.take(),
        [(
            TextSelection::collapsed(0),
            SelectionChangedCause::StylusHandwriting
        )]
    );
    assert!(
        runtime
            .borrow()
            .flutter_text_input_configuration()
            .is_some()
    );

    for widget in [
        root(true, true, false),
        root(false, true, true),
        root(false, false, false),
    ] {
        replace(widget);
        layout();
        assert_eq!(query(json!([0, 0, 1, 1])), json!([]));
    }
    replace(Stack::new([] as [Widget; 0]).into());
    layout();
    replace(root(false, true, false));
    layout();
    let replacement = query(json!([0, 0, 1, 1]));
    assert_ne!(replacement[0][0].as_str().unwrap(), first);
    assert_eq!(
        host.send("TextInputClient.focusElement", json!([first, 0, 0]))
            .unwrap(),
        b"[null]"
    );
    layout();
    assert!(
        !focus.has_focus(),
        "a retired target must not focus the replacement"
    );
}

#[test]
fn macos_read_only_binding_dispatches_selectors_but_rejects_text_updates() {
    use incular_runtime::TextShortcutPlatform;
    use incular_text::TextSelection;

    let text = TextEditingController::with_text("read only");
    text.set_selection(TextSelection::collapsed(0));
    let runtime = Rc::new(RefCell::new(
        Runtime::new(
            EditableText::new(text.clone())
                .read_only(true)
                .autofocus(true)
                .text_direction(TextDirection::Ltr)
                .into(),
        )
        .unwrap(),
    ));
    runtime
        .borrow_mut()
        .set_text_shortcut_platform(TextShortcutPlatform::MacOs);
    let host = Host::new();
    let _binding = FlutterTextInputBinding::install(&host.inbound, &runtime);
    runtime
        .borrow_mut()
        .run_frame(Constraints::tight(Size::new(320., 120.)))
        .unwrap();
    let (client, config) = runtime.borrow().flutter_text_input_configuration().unwrap();
    assert!(config.read_only);
    let reply = host
        .send(
            "TextInputClient.performSelectors",
            json!([client.get(), ["moveRight:", "deleteBackward:"]]),
        )
        .unwrap();
    assert_eq!(
        JsonMethodCodec::decode_envelope(&reply).unwrap(),
        Value::Null
    );
    assert_eq!(text.selection(), TextSelection::collapsed(1));
    assert_eq!(text.text(), "read only");
    let reply = host
        .send(
            "TextInputClient.updateEditingState",
            json!([client.get(), {"text":"no","selectionBase":0,"selectionExtent":2}]),
        )
        .unwrap();
    assert_eq!(
        JsonMethodCodec::decode_envelope(&reply).unwrap(),
        Value::Null
    );
    assert_eq!(text.selection(), TextSelection::collapsed(1));
    assert_eq!(text.text(), "read only");
}
