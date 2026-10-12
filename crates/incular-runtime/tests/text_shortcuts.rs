//! Flutter 3.41 `DefaultTextEditingShortcuts` through real key dispatch.
//!
//! Each platform has its own table (`widgets/default_text_editing_shortcuts.dart`):
//! Windows, Linux and Android share `_commonShortcuts`, macOS and iOS use
//! `_macShortcuts`. The actions are `EditableTextState`'s
//! (`widgets/editable_text.dart`): word movement skips spaces
//! (`WordBoundary.moveByWordBoundary`), a non-collapsed selection collapses
//! on a plain character arrow, `Up`/`Down` on a single line jump to the
//! document edges.

use incular_config::Constraints;
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, PointerPhase, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    EditableText, Widget,
    internal::{TextEditingController, TextSelection},
};

fn key(code: Code, modifiers: Modifiers) -> InputEvent {
    let mut event = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Unidentified), code);
    event.modifiers = modifiers;
    InputEvent::Key(event)
}

fn mount(
    platform: TextShortcutPlatform,
    text: &str,
    selection: TextSelection,
    multiline: bool,
) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text(text);
    let mut field = EditableText::new(controller.clone())
        .text_direction(incular_widgets::TextDirection::Ltr)
        .size(Size::new(300., if multiline { 120. } else { 32. }));
    if multiline {
        field = field.multiline(true);
    }
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime
        .run_frame(Constraints::tight(Size::new(300., 120.)))
        .unwrap();
    let _ = runtime.handle_input(InputEvent::Pointer {
        phase: PointerPhase::Down,
        position: Offset::new(5., 5.),
    });
    let _ = runtime.handle_input(InputEvent::Pointer {
        phase: PointerPhase::Up,
        position: Offset::new(5., 5.),
    });
    assert!(
        runtime.focused_element().is_some(),
        "press focuses the field"
    );
    controller.set_selection(selection);
    (runtime, controller)
}

fn press(runtime: &mut Runtime, code: Code, modifiers: Modifiers) {
    let _ = runtime.handle_input(key(code, modifiers));
    runtime
        .run_frame(Constraints::tight(Size::new(300., 120.)))
        .unwrap();
}

fn caret(controller: &TextEditingController) -> (usize, usize) {
    let selection = controller.selection();
    (selection.base, selection.extent)
}

const PLAIN: Modifiers = Modifiers::empty();

#[test]
fn windows_control_arrow_moves_to_the_end_of_the_next_word() {
    // moveByWordBoundary skips the spaces between words: from 0 in
    // "hello world" Ctrl+Right lands at the end of "hello", and again at the
    // end of "world" (not at the start of the second word).
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::Windows,
        "hello world",
        TextSelection::collapsed(0),
        false,
    );
    press(&mut runtime, Code::ArrowRight, Modifiers::CONTROL);
    assert_eq!(caret(&controller), (5, 5));
    press(&mut runtime, Code::ArrowRight, Modifiers::CONTROL);
    assert_eq!(caret(&controller), (11, 11));
    press(&mut runtime, Code::ArrowLeft, Modifiers::CONTROL);
    assert_eq!(caret(&controller), (6, 6));
    press(&mut runtime, Code::ArrowLeft, Modifiers::CONTROL);
    assert_eq!(caret(&controller), (0, 0));
}

#[test]
fn windows_shift_control_arrow_extends_by_word_from_the_base() {
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::Windows,
        "hello world",
        TextSelection::collapsed(0),
        false,
    );
    press(
        &mut runtime,
        Code::ArrowRight,
        Modifiers::CONTROL | Modifiers::SHIFT,
    );
    assert_eq!(caret(&controller), (0, 5));
    press(
        &mut runtime,
        Code::ArrowRight,
        Modifiers::CONTROL | Modifiers::SHIFT,
    );
    assert_eq!(caret(&controller), (0, 11));
}

#[test]
fn windows_control_backspace_and_delete_remove_a_word() {
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::Windows,
        "hello world",
        TextSelection::collapsed(11),
        false,
    );
    press(&mut runtime, Code::Backspace, Modifiers::CONTROL);
    assert_eq!(controller.text(), "hello ");
    controller.set_selection(TextSelection::collapsed(0));
    press(&mut runtime, Code::Delete, Modifiers::CONTROL);
    assert_eq!(controller.text(), " ");
}

#[test]
fn plain_arrow_collapses_a_selection_instead_of_moving() {
    // ExtendSelectionByCharacterIntent with ignoreNonCollapsedSelection
    // false: Left collapses to the selection start, Right to its end.
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::Windows,
        "hello world",
        TextSelection::new(2, 7),
        false,
    );
    press(&mut runtime, Code::ArrowLeft, PLAIN);
    assert_eq!(caret(&controller), (2, 2));
    controller.set_selection(TextSelection::new(7, 2));
    press(&mut runtime, Code::ArrowRight, PLAIN);
    assert_eq!(caret(&controller), (7, 7));
}

#[test]
fn single_line_vertical_arrows_jump_to_the_document_edges() {
    // _UpdateTextSelectionVerticallyAction: no adjacent line, so the caret
    // goes to the end (Down) or start (Up) of the text.
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::Windows,
        "hello world",
        TextSelection::collapsed(4),
        false,
    );
    press(&mut runtime, Code::ArrowDown, PLAIN);
    assert_eq!(caret(&controller), (11, 11));
    press(&mut runtime, Code::ArrowUp, PLAIN);
    assert_eq!(caret(&controller), (0, 0));
}

#[test]
fn control_y_is_not_a_redo_shortcut() {
    // Flutter's tables bind redo to Ctrl+Shift+Z (Cmd+Shift+Z on Apple).
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::Windows,
        "abc",
        TextSelection::collapsed(3),
        false,
    );
    controller.insert("d");
    press(&mut runtime, Code::KeyZ, Modifiers::CONTROL);
    assert_eq!(controller.text(), "abc");
    press(&mut runtime, Code::KeyY, Modifiers::CONTROL);
    assert_eq!(controller.text(), "abc", "Ctrl+Y does not redo");
    press(
        &mut runtime,
        Code::KeyZ,
        Modifiers::CONTROL | Modifiers::SHIFT,
    );
    assert_eq!(controller.text(), "abcd");
}

#[test]
fn mac_option_arrow_moves_by_word_and_command_arrow_by_line() {
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::MacOs,
        "hello world",
        TextSelection::collapsed(0),
        false,
    );
    press(&mut runtime, Code::ArrowRight, Modifiers::ALT);
    assert_eq!(caret(&controller), (5, 5));
    press(&mut runtime, Code::ArrowRight, Modifiers::ALT);
    assert_eq!(caret(&controller), (11, 11));
    press(&mut runtime, Code::ArrowLeft, Modifiers::META);
    assert_eq!(caret(&controller), (0, 0));
    press(&mut runtime, Code::ArrowRight, Modifiers::META);
    assert_eq!(caret(&controller), (11, 11));
}

#[test]
fn mac_control_arrow_is_not_a_word_move() {
    // On macOS Control is not bound to word movement (Option is).
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::MacOs,
        "hello world",
        TextSelection::collapsed(0),
        false,
    );
    press(&mut runtime, Code::ArrowRight, Modifiers::CONTROL);
    assert_eq!(caret(&controller), (0, 0));
}

#[test]
fn mac_option_backspace_deletes_a_word_and_command_backspace_the_line() {
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::MacOs,
        "hello world",
        TextSelection::collapsed(11),
        false,
    );
    press(&mut runtime, Code::Backspace, Modifiers::ALT);
    assert_eq!(controller.text(), "hello ");
    controller.set_selection(TextSelection::collapsed(6));
    press(&mut runtime, Code::Backspace, Modifiers::META);
    assert_eq!(controller.text(), "");
}

#[test]
fn mac_shift_option_arrow_collapses_at_reversal() {
    // ExtendSelectionToNextWordBoundaryOrCaretLocationIntent: extending
    // backward past the base collapses to the base instead of flipping.
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::MacOs,
        "hello world",
        TextSelection::new(5, 11),
        false,
    );
    press(
        &mut runtime,
        Code::ArrowLeft,
        Modifiers::ALT | Modifiers::SHIFT,
    );
    // Moving the extent back from 11 to the start of "world" stays on the
    // same side of the base.
    assert_eq!(caret(&controller), (5, 6));
    press(
        &mut runtime,
        Code::ArrowLeft,
        Modifiers::ALT | Modifiers::SHIFT,
    );
    // The next word boundary is before the base: the selection collapses.
    assert_eq!(caret(&controller), (5, 5));
}

#[test]
fn mac_emacs_bindings_and_transpose() {
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::MacOs,
        "abcd",
        TextSelection::collapsed(1),
        false,
    );
    press(&mut runtime, Code::KeyE, Modifiers::CONTROL);
    assert_eq!(caret(&controller), (4, 4));
    press(&mut runtime, Code::KeyA, Modifiers::CONTROL);
    assert_eq!(caret(&controller), (0, 0));
    press(&mut runtime, Code::KeyF, Modifiers::CONTROL);
    assert_eq!(caret(&controller), (1, 1));
    // Control-T swaps the characters around the caret and moves past them.
    press(&mut runtime, Code::KeyT, Modifiers::CONTROL);
    assert_eq!(controller.text(), "bacd");
    assert_eq!(caret(&controller), (2, 2));
    // At the end it swaps the last two.
    controller.set_selection(TextSelection::collapsed(4));
    press(&mut runtime, Code::KeyT, Modifiers::CONTROL);
    assert_eq!(controller.text(), "badc");
}

#[test]
fn mac_command_shortcuts_replace_control_ones() {
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::MacOs,
        "abc",
        TextSelection::collapsed(0),
        false,
    );
    press(&mut runtime, Code::KeyA, Modifiers::META);
    assert_eq!(caret(&controller), (0, 3));
    controller.set_selection(TextSelection::collapsed(0));
    // Ctrl+A is "start of line" on macOS, not select-all.
    press(&mut runtime, Code::KeyA, Modifiers::CONTROL);
    assert_eq!(caret(&controller), (0, 0));
}

#[test]
fn mac_home_and_end_do_not_move_the_caret() {
    // On macOS Home/End scroll the document (ScrollToDocumentBoundaryIntent)
    // and leave the selection alone.
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::MacOs,
        "hello world",
        TextSelection::collapsed(4),
        false,
    );
    press(&mut runtime, Code::End, PLAIN);
    assert_eq!(caret(&controller), (4, 4));
    press(&mut runtime, Code::Home, PLAIN);
    assert_eq!(caret(&controller), (4, 4));
}

#[test]
fn windows_home_and_end_move_to_line_edges() {
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::Windows,
        "hello world",
        TextSelection::collapsed(4),
        false,
    );
    press(&mut runtime, Code::End, PLAIN);
    assert_eq!(caret(&controller), (11, 11));
    press(&mut runtime, Code::Home, PLAIN);
    assert_eq!(caret(&controller), (0, 0));
    press(&mut runtime, Code::End, Modifiers::SHIFT);
    assert_eq!(caret(&controller), (0, 11));
}

#[test]
fn multiline_vertical_arrows_move_between_lines_and_then_to_the_edges() {
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::Windows,
        "one\ntwo\nthree",
        TextSelection::collapsed(1),
        true,
    );
    press(&mut runtime, Code::ArrowDown, PLAIN);
    assert_eq!(caret(&controller), (5, 5), "same column on the second line");
    press(&mut runtime, Code::ArrowDown, PLAIN);
    assert_eq!(caret(&controller), (9, 9));
    // Last line: Flutter's vertical run cannot move, so the caret goes to
    // the end of the document.
    press(&mut runtime, Code::ArrowDown, PLAIN);
    assert_eq!(caret(&controller), (13, 13));
    controller.set_selection(TextSelection::collapsed(5));
    press(&mut runtime, Code::ArrowUp, PLAIN);
    press(&mut runtime, Code::ArrowUp, PLAIN);
    assert_eq!(caret(&controller), (0, 0));
}

#[test]
fn paragraph_boundaries_via_shift_control_arrows() {
    let (mut runtime, controller) = mount(
        TextShortcutPlatform::Windows,
        "one\ntwo\nthree",
        TextSelection::collapsed(1),
        true,
    );
    press(
        &mut runtime,
        Code::ArrowDown,
        Modifiers::CONTROL | Modifiers::SHIFT,
    );
    // The next paragraph boundary is after the line terminator.
    assert_eq!(caret(&controller), (1, 4));
}
