//! Flutter's `ExtendSelectionByCharacterIntent` moves in the logical order
//! of the string, not affected by the writing direction
//! (`DirectionalTextEditingIntent.forward`): Right is forward and Left
//! backward, by grapheme cluster, even in right-to-left text.

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, PointerPhase, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    EditableText, Widget,
    internal::{TextEditingController, TextSelection},
};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(300., 32.))
}

fn mount(text: &str, direction: TextDirection, caret: usize) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text(text);
    let field = EditableText::new(controller.clone())
        .text_direction(direction)
        .size(Size::new(300., 32.));
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Windows);
    runtime.run_frame(bounds()).unwrap();
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(5., 5.),
        });
    }
    controller.set_selection(TextSelection::collapsed(caret));
    (runtime, controller)
}

fn press(runtime: &mut Runtime, code: Code, modifiers: Modifiers) {
    let mut event = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Unidentified), code);
    event.modifiers = modifiers;
    let _ = runtime.handle_input(InputEvent::Key(event));
    runtime.run_frame(bounds()).unwrap();
}

#[test]
fn right_moves_forward_in_right_to_left_text() {
    // Each Hebrew letter is two UTF-8 bytes.
    let (mut runtime, controller) = mount("שלום", TextDirection::Rtl, 0);
    press(&mut runtime, Code::ArrowRight, Modifiers::empty());
    assert_eq!(controller.selection(), TextSelection::collapsed(2));
    press(&mut runtime, Code::ArrowRight, Modifiers::empty());
    assert_eq!(controller.selection(), TextSelection::collapsed(4));
    press(&mut runtime, Code::ArrowLeft, Modifiers::empty());
    assert_eq!(controller.selection(), TextSelection::collapsed(2));
}

#[test]
fn mixed_direction_text_is_crossed_in_logical_order() {
    // "ab " then two Hebrew letters: bytes 0..3 are Latin, 3..7 Hebrew.
    let (mut runtime, controller) = mount("ab של", TextDirection::Ltr, 3);
    press(&mut runtime, Code::ArrowRight, Modifiers::empty());
    assert_eq!(controller.selection(), TextSelection::collapsed(5));
    press(&mut runtime, Code::ArrowRight, Modifiers::SHIFT);
    assert_eq!(controller.selection(), TextSelection::new(5, 7));
    press(&mut runtime, Code::ArrowLeft, Modifiers::SHIFT);
    assert_eq!(controller.selection(), TextSelection::new(5, 5));
}
