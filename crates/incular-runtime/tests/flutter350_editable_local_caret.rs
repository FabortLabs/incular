use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, Color, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset, Rect, Size,
};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_text::{TextAffinity, TextAlign, TextEditingController, TextSelection, TextStyle};
use incular_widgets::{EditableText, Padding};

const CURSOR: Color = Color::rgba(255, 0, 0, 255);

fn painted_caret(list: &DisplayList) -> Rect {
    list.commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::Rect {
                rect,
                color: CURSOR,
            } => Some(*rect),
            _ => None,
        })
        .expect("shown focused caret")
}

fn centered_multiline_editor(controller: &TextEditingController, font_size: f32) -> EditableText {
    EditableText::new(controller.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .text_align(TextAlign::Center)
        .style(
            TextStyle::new()
                .font_size(font_size)
                .line_height_multiplier(1.),
        )
        .max_lines(None)
        .cursor_color(CURSOR)
}

#[test]
fn caret_rect_keeps_its_height_and_top_while_deleting_to_empty() {
    let controller = TextEditingController::with_text("12");
    controller.set_selection(TextSelection::collapsed(2));
    let field = centered_multiline_editor(&controller, 20.);
    let mut runtime = Runtime::new(Padding::all(8., field).into()).unwrap();
    runtime.set_debug_deterministic_cursor(true);
    let view = Constraints::tight(Size::new(800., 120.));
    runtime.run_frame(view).unwrap();
    let id = runtime.focused_element().unwrap();
    let local = |runtime: &Runtime, pos| {
        runtime
            .tree()
            .editable_local_caret_rect(id, pos, TextAffinity::Downstream)
            .unwrap()
    };
    let initial = local(&runtime, 2);
    assert_eq!(initial.origin.y, 0.);
    for len in [1, 0, 0] {
        let key = KeyboardKey::Named(NamedKey::Backspace);
        let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_down(
            key.clone(),
            Code::Backspace,
        )));
        let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_up(key, Code::Backspace)));
        let (frame, _) = runtime.run_frame(view).unwrap();
        assert_eq!(controller.text().len(), len);
        let current = local(&runtime, len);
        assert_eq!(current.size, initial.size);
        assert_eq!(current.origin.y, initial.origin.y);
        assert!(current.origin.x < initial.origin.x);
        assert_eq!(painted_caret(&frame), current);
        let global = runtime
            .tree()
            .text_field_offset_rect(id, len, TextAffinity::Downstream)
            .unwrap();
        assert_eq!(global.origin, current.origin + Offset::new(8., 8.));
    }
    assert!(controller.text().is_empty());
}

#[test]
fn centered_multiline_caret_at_many_trailing_spaces_stays_inside_the_editor() {
    let text = format!("test{}", " ".repeat(1000));
    let controller = TextEditingController::with_text(text.clone());
    controller
        .set_selection_with_affinity(TextSelection::collapsed(text.len()), TextAffinity::Upstream);
    let field = centered_multiline_editor(&controller, 17.)
        .cursor_width(13.)
        .cursor_height(Some(17.));
    let mut runtime = Runtime::new(field.into()).unwrap();
    runtime.set_debug_deterministic_cursor(true);
    let (frame, _) = runtime
        .run_frame(Constraints::tight(Size::new(800., 120.)))
        .unwrap();
    let expected = Rect::from_origin_size(Offset::new(786., 0.), Size::new(13., 17.));
    assert_eq!(painted_caret(&frame), expected);
    let id = runtime.focused_element().unwrap();
    let caret = runtime
        .tree()
        .editable_local_caret_rect(id, text.len(), TextAffinity::Upstream);
    assert_eq!(caret, Some(expected));
}
