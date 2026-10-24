//! Flutter's `EditableTextState._scroll` and `_scrollToDocumentBoundary` for
//! the macOS/iOS `DefaultTextEditingShortcuts`: PageUp/PageDown scroll a
//! field by a page and Home/End bring the text's edges into view, all
//! without moving the selection.

use incular_config::Constraints;
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset, PointerPhase, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    EditableText, Widget,
    internal::{TextEditingController, TextSelection},
};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(300., 120.))
}

fn mount(text: &str, multiline: bool) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text(text);
    let mut field = EditableText::new(controller.clone())
        .text_direction(incular_widgets::TextDirection::Ltr)
        .size(Size::new(300., if multiline { 120. } else { 32. }));
    if multiline {
        field = field.multiline(true);
    }
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::MacOs);
    runtime.run_frame(bounds()).unwrap();
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(5., 5.),
        });
    }
    controller.set_selection(TextSelection::collapsed(0));
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller)
}

fn press(runtime: &mut Runtime, code: Code) {
    let event = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Unidentified), code);
    let _ = runtime.handle_input(InputEvent::Key(event));
    runtime.run_frame(bounds()).unwrap();
}

fn scroll(runtime: &Runtime) -> Offset {
    let field = runtime.focused_element().expect("focused field");
    runtime
        .tree()
        .text_field_scroll_offset(field)
        .expect("text field")
}

#[test]
fn page_keys_scroll_a_multiline_field_by_eight_tenths_of_its_viewport() {
    let text = (1..=40)
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let (mut runtime, controller) = mount(&text, true);
    assert_eq!(scroll(&runtime).y, 0.);
    // The viewport is the 120 px field less its 8 px vertical padding on
    // each side: 104 px, so a page is 0.8 x 104 = 83.2 px.
    press(&mut runtime, Code::PageDown);
    assert!((scroll(&runtime).y - 83.2).abs() < 1e-3);
    press(&mut runtime, Code::PageDown);
    assert!((scroll(&runtime).y - 166.4).abs() < 1e-3);
    press(&mut runtime, Code::PageUp);
    assert!((scroll(&runtime).y - 83.2).abs() < 1e-3);
    // The selection never moves, and repaints keep the scroll offset.
    assert_eq!(controller.selection(), TextSelection::collapsed(0));
    runtime.run_frame(bounds()).unwrap();
    assert!((scroll(&runtime).y - 83.2).abs() < 1e-3);
}

#[test]
fn home_and_end_bring_the_text_edges_into_view() {
    let text = (1..=40)
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let (mut runtime, controller) = mount(&text, true);
    press(&mut runtime, Code::End);
    let bottom = scroll(&runtime).y;
    assert!(bottom > 166.4, "the last line is in view");
    // Already in view: nothing changes.
    press(&mut runtime, Code::End);
    assert_eq!(scroll(&runtime).y, bottom);
    press(&mut runtime, Code::Home);
    assert_eq!(scroll(&runtime).y, 0.);
    assert_eq!(controller.selection(), TextSelection::collapsed(0));
}

#[test]
fn page_keys_jump_a_single_line_field_to_its_scroll_extent() {
    let text = "word ".repeat(80);
    let (mut runtime, controller) = mount(&text, false);
    assert_eq!(scroll(&runtime).x, 0.);
    // `maxLines == 1`: both directions jump to maxScrollExtent.
    press(&mut runtime, Code::PageUp);
    let end = scroll(&runtime).x;
    assert!(end > 0.);
    press(&mut runtime, Code::PageDown);
    assert_eq!(scroll(&runtime).x, end);
    press(&mut runtime, Code::Home);
    assert_eq!(scroll(&runtime).x, 0.);
    assert_eq!(controller.selection(), TextSelection::collapsed(0));
}
