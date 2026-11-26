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

#[test]
fn refocus_rejects_an_evicted_list_editor_but_accepts_the_new_mount() {
    use incular_widgets::{ListView, ScrollController, internal::Key};
    let controller = TextEditingController::with_text("draft");
    let node = FocusNode::new();
    let scroll = ScrollController::new();
    let key = Key::from("virtual editor");
    let list = ListView::builder(50, {
        let (controller, node, key) = (controller.clone(), node.clone(), key.clone());
        move |index| {
            if index == 0 {
                Widget::from(field(&controller, &node, false)).with_key(key.clone())
            } else {
                SizedBox::expand().into()
            }
        }
    })
    .item_extent(40.)
    .controller(scroll.clone());
    let mut runtime = Runtime::new(list.into()).unwrap();
    frame(&mut runtime);
    let original = runtime.tree().element_with_key(&key).unwrap();
    assert!(runtime.request_text_input_focus_for(original));
    frame(&mut runtime);
    let old_client = attached(&mut runtime);
    assert!(platform_call(
        &mut runtime,
        "onConnectionClosed",
        old_client
    ));
    frame(&mut runtime);
    assert!(!node.has_focus());
    assert!(runtime.take_text_input_commands().is_empty());

    assert!(scroll.jump_to(scroll.max_offset()));
    frame(&mut runtime);
    assert!(runtime.tree().element_with_key(&key).is_none());
    assert!(!runtime.request_text_input_focus_for(original));
    assert!(!platform_call(&mut runtime, "onFocusReceived", old_client));

    assert!(scroll.jump_to(0.));
    frame(&mut runtime);
    let remounted = runtime.tree().element_with_key(&key).unwrap();
    assert_ne!(original, remounted);
    assert!(!platform_call(&mut runtime, "onFocusReceived", old_client));
    assert!(runtime.request_text_input_focus_for(remounted));
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    assert_ne!(attached(&mut runtime), old_client);
    assert_eq!(controller.text(), "draft");
}

#[test]
fn user_taps_reopen_the_closed_connection_and_edit_the_selected_field() {
    use incular_core::{InputEvent, Offset, PointerPhase};
    for next_row in [false, true] {
        let controller = TextEditingController::new();
        let [first, second] = [FocusNode::new(), FocusNode::new()];
        // The editable value is shared, but these are separate retained
        // focus nodes; a focus attachment cannot own two retained fields.
        let fields = Column::new([
            Widget::from(field(&controller, &first, false)),
            Widget::from(field(&controller, &second, false)),
        ]);
        let mut runtime = Runtime::new(fields.into()).unwrap();
        frame(&mut runtime);
        let tap = |runtime: &mut Runtime, y| {
            for phase in [PointerPhase::Down, PointerPhase::Up] {
                let _ = runtime.handle_input(InputEvent::Pointer {
                    phase,
                    position: Offset::new(15., y),
                });
            }
            frame(runtime);
        };
        tap(&mut runtime, 20.);
        let prior = attached(&mut runtime);
        assert!(first.has_focus());
        controller.set_text("test3");
        frame(&mut runtime);
        assert!(runtime.take_text_input_commands().iter().any(|command| {
            matches!(command, TextInputCommand::Update { client, state }
                if *client == prior && state.text == "test3")
        }));

        assert!(platform_call(&mut runtime, "onConnectionClosed", prior));
        frame(&mut runtime);
        assert!(!first.has_focus());
        assert!(runtime.take_text_input_commands().is_empty());

        tap(&mut runtime, if next_row { 60. } else { 20. });
        let reopened = attached(&mut runtime);
        assert_ne!(reopened, prior);
        assert_eq!(first.has_primary_focus(), !next_row);
        assert_eq!(second.has_primary_focus(), next_row);
        let text = if next_row { "650123123" } else { "test2" };
        controller.set_text(text);
        frame(&mut runtime);
        assert!(runtime.take_text_input_commands().iter().any(|command| {
            matches!(command, TextInputCommand::Update { client, state }
                if *client == reopened && state.text == text)
        }));
        assert!(!platform_call(&mut runtime, "onConnectionClosed", prior));
    }
}

#[test]
fn focused_off_cache_editor_releases_only_its_automatic_keep_alive() {
    use incular_widgets::{KeepAlive, ListView, ScrollController, internal::Key};
    for explicit in [false, true] {
        let text = TextEditingController::with_text("draft");
        let focus = FocusNode::new();
        let scroll = ScrollController::new();
        let key = Key::from("kept editor");
        let list = ListView::builder(60, {
            let (text, focus, key) = (text.clone(), focus.clone(), key.clone());
            move |index| {
                if index == 0 {
                    let row = Widget::from(Column::new([Widget::from(field(&text, &focus, true))]))
                        .with_key(key.clone());
                    KeepAlive::new(explicit, row).into_child()
                } else {
                    SizedBox::expand().into()
                }
            }
        })
        .item_extent(40.)
        .controller(scroll.clone());
        let mut runtime = Runtime::new(list.into()).unwrap();
        frame(&mut runtime);
        assert!(focus.has_primary_focus());
        let client = attached(&mut runtime);
        let row = runtime.tree().element_with_key(&key).unwrap();
        let editor = runtime.focused_element().unwrap();

        assert!(scroll.jump_to(scroll.max_offset()));
        frame(&mut runtime);
        assert_eq!(runtime.tree().element_with_key(&key), Some(row));
        assert_eq!(runtime.focused_element(), Some(editor));
        assert!(focus.has_primary_focus());
        let _ = runtime.take_text_input_commands();

        assert!(platform_call(&mut runtime, "onConnectionClosed", client));
        frame(&mut runtime);
        assert!(!focus.has_focus());
        assert_eq!(
            runtime.tree().element_with_key(&key),
            explicit.then_some(row)
        );
        assert!(runtime.take_text_input_commands().is_empty());
        assert_eq!(text.text(), "draft");
    }
}

#[test]
fn a_focused_keyed_editor_survives_rebuild_and_skipped_layout_after_a_preceding_row() {
    use incular_text::{TextAffinity, TextAlign, TextSelection};
    use incular_widgets::{ListView, ScrollController, internal::Key};

    for fixed in [false, true] {
        let text = TextEditingController::with_text("draft");
        let focus = FocusNode::new();
        let scroll = ScrollController::new();
        let key = Key::from("keyed editor");
        let build = |prepend| -> Widget {
            let editor = field(&text, &focus, true)
                .multiline(true)
                .text_align(if prepend {
                    TextAlign::Right
                } else {
                    TextAlign::Left
                })
                .show_cursor(prepend);
            let mut items = Vec::new();
            if prepend {
                items.extend((0..if fixed { 150 } else { 1 }).map(|_| {
                    Widget::from(SizedBox::from_dimensions(
                        None,
                        Some(if fixed { 40. } else { 6000. }),
                        None::<Widget>,
                    ))
                }));
            }
            items.push(Widget::from(editor).with_key(key.clone()));
            let view = ListView::new(items).controller(scroll.clone());
            if fixed {
                view.item_extent(40.).into()
            } else {
                view.into()
            }
        };
        let mut runtime = Runtime::new(build(false)).unwrap();
        frame(&mut runtime);
        let client = attached(&mut runtime);
        let editor = runtime.tree().element_with_key(&key).unwrap();
        assert!(focus.has_primary_focus());
        let root = runtime.tree().root().unwrap();
        runtime.schedule_update(root, build(true)).unwrap();
        frame(&mut runtime);
        assert_eq!(
            runtime.tree().element_with_key(&key),
            Some(editor),
            "fixed {fixed}"
        );
        assert_eq!(runtime.focused_element(), Some(editor), "fixed {fixed}");
        assert!(focus.has_primary_focus());
        assert_eq!(scroll.offset(), 0.);
        assert_eq!(text.text(), "draft");
        assert!(!runtime.take_text_input_commands().iter().any(|command| {
            matches!(
                command,
                TextInputCommand::SetClient { .. } | TextInputCommand::Clear { .. }
            )
        }));

        assert!(scroll.jump_to(scroll.max_offset()));
        frame(&mut runtime);
        assert_eq!(runtime.tree().element_with_key(&key), Some(editor));
        text.set_selection(TextSelection::collapsed(text.text().len()));
        frame(&mut runtime);
        let caret = runtime
            .tree()
            .text_field_offset_rect(editor, text.text().len(), TextAffinity::Downstream)
            .unwrap();
        assert!(
            caret.origin.x > 120.,
            "the updated right alignment applies on reentry: {caret:?}"
        );
        assert!(platform_call(&mut runtime, "onConnectionClosed", client));
        frame(&mut runtime);
        assert!(!focus.has_focus());
    }
}

#[test]
fn list_builder_and_separated_key_finders_observe_only_retained_items_on_rebuild() {
    use incular_widgets::{ListView, ScrollController, internal::Key};
    for separated in [false, true] {
        let calls = Rc::new(Cell::new(0));
        let scroll = ScrollController::new();
        let build = |count| -> Widget {
            let large_item = |index: usize| {
                Widget::from(SizedBox::from_dimensions(None, Some(2000.), None::<Widget>))
                    .with_key(Key::String(index.to_string()))
            };
            let list = if separated {
                ListView::separated(count, large_item, |_| SizedBox::shrink())
            } else {
                ListView::builder(count, large_item)
            };
            list.controller(scroll.clone())
                .find_child_index_callback({
                    let calls = calls.clone();
                    move |key| {
                        assert_eq!(key, &Key::from("0"));
                        calls.set(calls.get() + 1);
                        None
                    }
                })
                .into()
        };
        let mut runtime = Runtime::new(build(7)).unwrap();
        frame(&mut runtime);
        assert_eq!(calls.get(), 0);
        let root = runtime.tree().root().unwrap();
        runtime.schedule_update(root, build(77)).unwrap();
        frame(&mut runtime);
        assert_eq!(calls.get(), 1);
        assert!(runtime.tree().element_with_key(&Key::from("0")).is_some());
        assert!(scroll.jump_to(16.));
        frame(&mut runtime);
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn separated_item_lookup_excludes_separators_when_reattributing_a_parked_editor() {
    use incular_widgets::{ListView, ScrollController, internal::Key};
    let controller = TextEditingController::with_text("draft");
    let focus = FocusNode::new();
    let scroll = ScrollController::new();
    let key = Key::from("focused");
    let build = |slot| {
        ListView::separated(
            100,
            {
                let (controller, focus, key) = (controller.clone(), focus.clone(), key.clone());
                move |index| {
                    if index == slot {
                        Widget::from(field(&controller, &focus, true)).with_key(key.clone())
                    } else {
                        SizedBox::from_dimensions(None, Some(40.), None::<Widget>).into()
                    }
                }
            },
            |_| SizedBox::from_dimensions(None, Some(2.), None::<Widget>),
        )
        .find_item_index_callback({
            let key = key.clone();
            move |candidate| (candidate == &key).then_some(slot)
        })
        .controller(scroll.clone())
        .into()
    };
    let mut runtime = Runtime::new(build(0)).unwrap();
    frame(&mut runtime);
    let original = runtime.tree().element_with_key(&key).unwrap();
    let client = attached(&mut runtime);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(90)).unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.tree().element_with_key(&key), Some(original));
    assert_eq!(runtime.focused_element(), Some(original));
    assert_eq!(scroll.offset(), 0.);
    assert_eq!(controller.text(), "draft");
    assert!(platform_call(&mut runtime, "onConnectionClosed", client));
    frame(&mut runtime);
    assert!(runtime.tree().element_with_key(&key).is_none());
}

#[test]
fn closing_or_switching_keyboard_clients_never_replays_stale_composition() {
    use incular_platform::{TextInputState, TextInputType};
    use incular_text::{TextRange, TextSelection};
    use incular_widgets::TextInputTypeHint;

    let shared = TextEditingController::new();
    let [first, second] = [FocusNode::new(), FocusNode::new()];
    let root: Widget = Column::new([
        Widget::from(field(&shared, &first, true)),
        Widget::from(field(&shared, &second, false).input_type(TextInputTypeHint::Phone)),
    ])
    .into();
    let mut runtime = Runtime::new(root).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    frame(&mut runtime);
    let retired = attached(&mut runtime);
    let editing = TextInputState {
        text: "draft".into(),
        selection_start: 3,
        selection_end: 3,
        composing: Some((1, 4)),
    };
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(editing.clone()));
    frame(&mut runtime);
    assert_eq!(shared.composing(), Some(TextRange::new(1, 4)));
    runtime.take_text_input_commands();

    assert!(platform_call(&mut runtime, "onConnectionClosed", retired));
    frame(&mut runtime);
    assert!(!first.has_focus());
    assert_eq!(shared.text(), "draft");
    assert_eq!(shared.selection(), TextSelection::collapsed(3));
    assert_eq!(
        shared.composing(),
        None,
        "the host closed its composing session"
    );
    assert!(runtime.take_text_input_commands().is_empty());

    let _ = second.request_focus();
    frame(&mut runtime);
    assert!(second.has_primary_focus());
    let reshow = runtime.take_text_input_commands();
    let active = reshow
        .iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient {
                configuration,
                state,
                ..
            } if configuration.input_type == TextInputType::Phone => {
                assert_eq!(state.text, "draft");
                assert_eq!(state.composing, None);
                Some(configuration.client)
            }
            _ => None,
        })
        .expect("the second field receives a phone input client");
    assert_ne!(active, retired);
    assert!(reshow.contains(&TextInputCommand::Show { client: active }));
    assert!(!platform_call(&mut runtime, "onConnectionClosed", retired));

    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(editing));
    frame(&mut runtime);
    assert_eq!(shared.composing(), Some(TextRange::new(1, 4)));
    runtime.take_text_input_commands();
    let _ = first.request_focus();
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    assert_eq!(shared.text(), "draft");
    assert_eq!(
        shared.composing(),
        None,
        "focus transfer ends the old composition too"
    );
    assert!(runtime.take_text_input_commands().iter().any(|command| {
        matches!(command, TextInputCommand::SetClient { configuration, state, .. }
            if configuration.client != active && state.composing.is_none() && state.selection_end == 3)
    }));
}

#[test]
fn selection_updates_send_the_correct_composing_range_to_the_existing_client() {
    use incular_platform::TextInputState;
    use incular_text::TextSelection;

    let controller = TextEditingController::new();
    let node = FocusNode::new();
    let mut runtime = Runtime::new(field(&controller, &node, true).into()).unwrap();
    frame(&mut runtime);
    let client = attached(&mut runtime);
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "foo composing bar".into(),
        selection_start: 4,
        selection_end: 4,
        composing: Some((4, 12)),
    }));
    frame(&mut runtime);
    runtime.take_text_input_commands();
    for (selection, composing) in [
        (TextSelection::new(7, 5), Some((4, 12))),
        (TextSelection::new(14, 7), None),
    ] {
        controller.set_selection(selection);
        frame(&mut runtime);
        let commands = runtime.take_text_input_commands();
        assert!(node.has_primary_focus());
        assert!(
            commands.iter().any(|command| {
                matches!(command, TextInputCommand::Update { client: id, state }
                if *id == client && state.selection_start == selection.base
                    && state.selection_end == selection.extent && state.composing == composing)
            }),
            "{commands:?}"
        );
        assert!(!commands.iter().any(|command| matches!(
            command,
            TextInputCommand::SetClient { .. } | TextInputCommand::Show { .. }
        )));
    }
}
