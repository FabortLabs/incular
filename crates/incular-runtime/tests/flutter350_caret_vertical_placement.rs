use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_text::{TextEditingController, TextStyle};
use incular_widgets::EditableText;

const CURSOR: Color = Color::rgba(255, 0, 0, 255);

fn caret_rect(cursor_height: f32) -> Rect {
    let mut runtime = Runtime::new(
        EditableText::new(TextEditingController::new())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::new().font_size(16.).line_height_multiplier(1.))
            .cursor_width(4.)
            .cursor_height(Some(cursor_height))
            .cursor_color(CURSOR)
            .into(),
    )
    .unwrap();
    let (list, _) = runtime
        .run_frame(Constraints::tight(Size::new(200., 16.)))
        .unwrap();
    list.commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::Rect {
                rect,
                color: CURSOR,
            } => Some(*rect),
            _ => None,
        })
        .expect("cursor paint in the focused editor")
}

#[test]
fn a_short_caret_centers_vertically_on_the_line() {
    assert_eq!(
        caret_rect(12.),
        Rect::from_origin_size(Offset::new(0., 2.), Size::new(4., 12.)),
    );
}

#[test]
fn a_tall_caret_can_extend_above_the_line_to_remain_centered() {
    assert_eq!(
        caret_rect(24.),
        Rect::from_origin_size(Offset::new(0., -4.), Size::new(4., 24.)),
    );
}
