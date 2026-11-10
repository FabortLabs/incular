//! Flutter's `TextSelectionGestureDetectorBuilder` primary-click gestures on
//! desktop: a double click selects a word, a triple click selects the line
//! of a single-line field or the paragraph (Linux: the line) of a multiline
//! one, and a shift click extends (Windows, Linux) or expands (macOS) the
//! selection.

use incular_config::Constraints;
use incular_core::{
    Code, InputEvent, KeyState, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset,
    PointerPhase, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    EditableText, TextDirection, Widget,
    internal::{TextEditingController, TextSelection},
};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(400., 120.))
}

fn mount(
    text: &str,
    multiline: bool,
    platform: TextShortcutPlatform,
) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text(text);
    let mut field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(400., if multiline { 120. } else { 32. }));
    if multiline {
        field = field.multiline(true);
    }
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller)
}

fn click(runtime: &mut Runtime, position: Offset) {
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
    }
    runtime.run_frame(bounds()).unwrap();
}

fn shift(runtime: &mut Runtime, down: bool) {
    let mut event = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Shift), Code::ShiftLeft);
    if !down {
        event.state = KeyState::Up;
    }
    event.modifiers = if down {
        Modifiers::SHIFT
    } else {
        Modifiers::empty()
    };
    let _ = runtime.handle_input(InputEvent::Key(event));
}

#[test]
fn double_click_selects_the_word_under_the_pointer() {
    let (mut runtime, controller) = mount("hello big world", false, TextShortcutPlatform::Windows);
    click(&mut runtime, Offset::new(12., 10.));
    assert!(controller.selection().is_collapsed());
    click(&mut runtime, Offset::new(12., 10.));
    assert_eq!(controller.selection(), TextSelection::new(0, 5));
}

#[test]
fn triple_click_selects_a_single_line_field_entirely() {
    let (mut runtime, controller) = mount("hello big world", false, TextShortcutPlatform::Windows);
    for _ in 0..3 {
        click(&mut runtime, Offset::new(12., 10.));
    }
    assert_eq!(controller.selection(), TextSelection::new(0, 15));
}

#[test]
fn triple_click_selects_the_paragraph_of_a_multiline_field() {
    let text = "first paragraph\nsecond one";
    let (mut runtime, controller) = mount(text, true, TextShortcutPlatform::MacOs);
    for _ in 0..3 {
        click(&mut runtime, Offset::new(20., 12.));
    }
    let selection = controller.selection();
    assert_eq!(selection.start(), 0);
    assert!(
        selection.end() == 15 || selection.end() == 16,
        "the first paragraph, with or without its line break: {selection:?}"
    );
}

#[test]
fn windows_cycles_between_word_and_line_after_three_clicks() {
    let (mut runtime, controller) = mount("hello big world", false, TextShortcutPlatform::Windows);
    for _ in 0..4 {
        click(&mut runtime, Offset::new(12., 10.));
    }
    // Windows counts 1, 2, 3, 2, 3...: the fourth click is a double click.
    assert_eq!(controller.selection(), TextSelection::new(0, 5));
}

#[test]
fn shift_click_extends_from_the_base_on_windows() {
    let (mut runtime, controller) = mount("hello big world", false, TextShortcutPlatform::Windows);
    click(&mut runtime, Offset::new(12., 10.));
    let base = controller.selection().base;
    // Far enough away not to count as a consecutive tap.
    shift(&mut runtime, true);
    click(&mut runtime, Offset::new(300., 10.));
    shift(&mut runtime, false);
    let selection = controller.selection();
    assert_eq!(selection.base, base);
    assert_eq!(selection.extent, 15);
}

#[test]
fn shift_click_on_macos_expands_from_the_far_end() {
    let (mut runtime, controller) = mount("hello big world", false, TextShortcutPlatform::MacOs);
    // Focus the field far from the next click, so it is not a double click.
    click(&mut runtime, Offset::new(300., 10.));
    controller.set_selection(TextSelection::new(6, 9));
    shift(&mut runtime, true);
    // Clicking before the selection keeps its far end (9) as the base.
    click(&mut runtime, Offset::new(1., 10.));
    shift(&mut runtime, false);
    assert_eq!(controller.selection(), TextSelection::new(9, 0));
}
