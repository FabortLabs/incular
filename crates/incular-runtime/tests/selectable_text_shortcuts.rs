//! Flutter's `SelectableText` is a read-only `EditableText`: it uses the
//! platform's `DefaultTextEditingShortcuts` for selection movement, copy
//! and select-all, while the editing intents stay disabled.

use incular_config::Constraints;
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, PointerPhase, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{Widget, internal::SelectionAreaController};

fn mount(platform: TextShortcutPlatform, text: &str) -> (Runtime, SelectionAreaController) {
    let controller = SelectionAreaController::new();
    let mut runtime = Runtime::new(Widget::selection_area(
        controller.clone(),
        Widget::selectable_text_styled(
            text,
            incular_text::TextStyle::default(),
            incular_text::TextAlign::Start,
        ),
    ))
    .unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime
        .run_frame(Constraints::tight(Size::new(400., 40.)))
        .unwrap();
    let area = runtime.tree().root().unwrap();
    let label = runtime.tree().children(area).unwrap()[0];
    let bounds = runtime.tree().element_bounds(label).unwrap();
    // A click at the start places a caret there and focuses the text.
    let start = bounds.origin + incular_core::Offset::new(0.5, bounds.size.height * 0.5);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: start,
        });
    }
    assert_eq!(runtime.focused_element(), Some(label));
    (runtime, controller)
}

fn press(runtime: &mut Runtime, code: Code, modifiers: Modifiers) {
    let mut event = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Unidentified), code);
    event.modifiers = modifiers;
    let _ = runtime.handle_input(InputEvent::Key(event));
}

#[test]
fn windows_word_and_line_selection() {
    let (mut runtime, controller) = mount(TextShortcutPlatform::Windows, "hello big world");
    // Ctrl+Shift+Right selects to the end of the next word.
    press(
        &mut runtime,
        Code::ArrowRight,
        Modifiers::CONTROL | Modifiers::SHIFT,
    );
    assert_eq!(controller.selected_text(), "hello");
    press(
        &mut runtime,
        Code::ArrowRight,
        Modifiers::CONTROL | Modifiers::SHIFT,
    );
    assert_eq!(controller.selected_text(), "hello big");
    // Shift+End extends to the end of the line.
    press(&mut runtime, Code::End, Modifiers::SHIFT);
    assert_eq!(controller.selected_text(), "hello big world");
    // A plain Left collapses the selection to its start.
    press(&mut runtime, Code::ArrowLeft, Modifiers::empty());
    assert_eq!(controller.selected_text(), "");
    // Ctrl+A selects everything.
    press(&mut runtime, Code::KeyA, Modifiers::CONTROL);
    assert_eq!(controller.selected_text(), "hello big world");
}

#[test]
fn mac_uses_option_for_words_and_command_for_lines() {
    let (mut runtime, controller) = mount(TextShortcutPlatform::MacOs, "hello big world");
    press(
        &mut runtime,
        Code::ArrowRight,
        Modifiers::ALT | Modifiers::SHIFT,
    );
    assert_eq!(controller.selected_text(), "hello");
    // Control+Shift+Arrow is unbound on macOS.
    press(
        &mut runtime,
        Code::ArrowRight,
        Modifiers::CONTROL | Modifiers::SHIFT,
    );
    assert_eq!(controller.selected_text(), "hello");
    // Command+Shift+Right expands to the end of the line.
    press(
        &mut runtime,
        Code::ArrowRight,
        Modifiers::META | Modifiers::SHIFT,
    );
    assert_eq!(controller.selected_text(), "hello big world");
    // Command+A selects all; Control+A is not select-all on macOS.
    press(&mut runtime, Code::ArrowLeft, Modifiers::empty());
    press(&mut runtime, Code::KeyA, Modifiers::META);
    assert_eq!(controller.selected_text(), "hello big world");
}

#[test]
fn editing_keys_do_nothing() {
    let (mut runtime, controller) = mount(TextShortcutPlatform::Windows, "hello");
    press(&mut runtime, Code::KeyA, Modifiers::CONTROL);
    for (code, modifiers) in [
        (Code::Backspace, Modifiers::empty()),
        (Code::Delete, Modifiers::empty()),
        (Code::KeyX, Modifiers::CONTROL),
        (Code::KeyV, Modifiers::CONTROL),
        (Code::KeyZ, Modifiers::CONTROL),
    ] {
        press(&mut runtime, code, modifiers);
        assert_eq!(controller.selected_text(), "hello");
    }
}
