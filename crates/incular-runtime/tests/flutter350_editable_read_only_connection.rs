use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset, PointerPhase, Size,
};
use incular_platform::{PlatformEvent, TextInputCommand, TextInputState};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextEditingValue, TextSelection};
use incular_widgets::{
    CallbackTextInputFormatter, Column, EditableText, FocusNode, FocusScope, Widget,
};

const TEXT: &str = "Lorem ipsum dolor sit amet";
const PLATFORMS: [(&str, TextShortcutPlatform); 6] = [
    ("Android", TextShortcutPlatform::Android),
    (
        "Fuchsia (shared Android mapping)",
        TextShortcutPlatform::Android,
    ),
    ("iOS", TextShortcutPlatform::Ios),
    ("Linux", TextShortcutPlatform::Linux),
    ("macOS", TextShortcutPlatform::MacOs),
    ("Windows", TextShortcutPlatform::Windows),
];

fn editor(controller: &TextEditingController, focus: &FocusNode, read_only: bool) -> EditableText {
    EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .read_only(read_only)
        .size(Size::new(400., 32.))
        .text_direction(TextDirection::Ltr)
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(400., 200.)))
        .unwrap();
}

fn click(runtime: &mut Runtime) {
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(3., 5.),
        });
    }
    frame(runtime);
}

#[test]
fn a_read_only_native_editor_can_focus_and_select_without_attaching_an_ime() {
    for (platform, shortcuts) in PLATFORMS {
        let text = TextEditingController::with_text(TEXT);
        let focus = FocusNode::new();
        let mut runtime = Runtime::new(editor(&text, &focus, true).into()).unwrap();
        runtime.set_text_shortcut_platform(shortcuts);
        frame(&mut runtime);
        click(&mut runtime);
        assert!(focus.has_primary_focus(), "{platform}");
        text.set_selection(TextSelection::new(0, 5));
        frame(&mut runtime);
        assert_eq!(text.selection(), TextSelection::new(0, 5));
        assert_eq!(text.text(), TEXT);
        assert!(runtime.take_text_input_commands().is_empty(), "{platform}");
        assert!(runtime.flutter_text_input_configuration().is_none());
    }
}

#[test]
fn rejected_native_updates_neither_mutate_nor_invoke_the_formatter() {
    let text = TextEditingController::with_text(TEXT);
    let focus = FocusNode::new();
    let calls = Rc::new(Cell::new(0));
    let fmt_calls = calls.clone();
    let mut runtime = Runtime::new(
        editor(&text, &focus, true)
            .input_formatter(CallbackTextInputFormatter::new(
                move |_: &TextEditingValue, next: &TextEditingValue| {
                    fmt_calls.set(fmt_calls.get() + 1);
                    next.clone()
                },
            ))
            .into(),
    )
    .unwrap();
    frame(&mut runtime);
    click(&mut runtime);
    text.set_selection(TextSelection::new(0, 5));
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "Foo bar".into(),
        selection_start: 0,
        selection_end: 3,
        composing: Some((3, 4)),
    }));
    frame(&mut runtime);
    assert!(focus.has_primary_focus());
    assert_eq!(text.text(), TEXT);
    assert_eq!(text.selection(), TextSelection::new(0, 5));
    assert_eq!(text.composing(), None);
    assert_eq!(calls.get(), 0);
    assert!(runtime.take_text_input_commands().is_empty());
}

#[test]
fn removing_read_only_attaches_without_remounting_and_restoring_it_closes_the_client() {
    let text = TextEditingController::with_text(TEXT);
    let focus = FocusNode::new();
    let mut runtime = Runtime::new(editor(&text, &focus, true).into()).unwrap();
    frame(&mut runtime);
    click(&mut runtime);
    let field = runtime.focused_element().unwrap();
    assert!(runtime.take_text_input_commands().is_empty());
    let selection = TextSelection::new(0, 5);
    text.set_selection(selection);
    runtime
        .schedule_update(field, editor(&text, &focus, false).into())
        .unwrap();
    frame(&mut runtime);
    let updates = runtime.take_text_input_commands();
    let (client, config) = runtime.flutter_text_input_configuration().unwrap();
    assert!(!config.read_only);
    assert!(updates.iter().any(|update| matches!(
        update, TextInputCommand::SetClient { configuration, state, .. }
            if configuration.client == client && state.text == TEXT
                && state.selection_start == 0 && state.selection_end == 5
    )));
    assert!(updates.iter().any(|update| matches!(update,
        TextInputCommand::Show { client: shown } if *shown == client
    )));
    assert_eq!(runtime.focused_element(), Some(field));
    assert!(focus.has_primary_focus());
    runtime
        .schedule_update(field, editor(&text, &focus, true).into())
        .unwrap();
    frame(&mut runtime);
    let updates = runtime.take_text_input_commands();
    assert!(updates.iter().any(|update| matches!(update,
        TextInputCommand::Clear { client: cleared } if *cleared == client
    )));
    assert!(
        !updates
            .iter()
            .any(|update| matches!(update, TextInputCommand::Show { .. }))
    );
    assert_eq!(runtime.focused_element(), Some(field));
    assert!(focus.has_primary_focus());
    assert_eq!(text.selection(), selection);
}

#[test]
fn tab_cycles_through_both_editable_and_read_only_editors_on_every_native_platform() {
    for (platform, shortcuts) in PLATFORMS {
        let first = FocusNode::new();
        let second = FocusNode::new();
        let mut runtime = Runtime::new(
            FocusScope::new(
                Column::new([
                    Widget::from(
                        editor(&TextEditingController::new(), &first, false).autofocus(true),
                    ),
                    Widget::from(editor(&TextEditingController::new(), &second, true)),
                ])
                .text_direction(TextDirection::Ltr),
            )
            .into(),
        )
        .unwrap();
        runtime.set_text_shortcut_platform(shortcuts);
        frame(&mut runtime);
        assert!(first.has_primary_focus(), "{platform}");
        assert!(!second.has_primary_focus(), "{platform}");
        for (to, from) in [(&second, &first), (&first, &second)] {
            let key = KeyboardKey::Named(NamedKey::Tab);
            let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_down(
                key.clone(),
                Code::Tab,
            )));
            let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_up(key, Code::Tab)));
            frame(&mut runtime);
            assert!(to.has_primary_focus(), "{platform}");
            assert!(!from.has_primary_focus(), "{platform}");
        }
    }
}
