//! Painted horizontal caret placement with the pinned RenderEditable gap.
use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size, time::Instant};
use incular_rendering::PaintCommand;
use incular_text::{TextAlign, TextEditingController};
use incular_widgets::{EditableText, internal::WidgetTree};

#[test]
fn empty_caret_uses_the_reserved_margin_in_both_directions_and_line_modes() {
    let size = Size::new(200., 60.);
    let caret = Color::rgba(21, 125, 252, 255);
    let mut mismatches = Vec::new();
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        for max_lines in [1, 2] {
            for (align, expected) in [
                (TextAlign::Left, 0.),
                (TextAlign::Center, 98.5),
                (TextAlign::Right, 197.),
            ] {
                let mut tree = WidgetTree::new();
                let controller = TextEditingController::new();
                let field = EditableText::new(controller.clone())
                    .size(size)
                    .max_lines(Some(max_lines))
                    .text_direction(direction)
                    .text_align(align)
                    .cursor_color(caret);
                let root = tree.mount(field.into()).unwrap();
                tree.layout(Constraints::tight(size)).unwrap();
                let now = Instant::now();
                tree.set_focused(root, true, now).unwrap();
                controller.reset_caret(now);
                let list = tree.paint();
                let painted = list
                    .commands()
                    .iter()
                    .find_map(|command| match command {
                        PaintCommand::Rect { rect, color } if *color == caret => Some(*rect),
                        _ => None,
                    })
                    .expect("the focused empty field must have a visible caret");
                assert_eq!(painted.size.width, 2.);
                if painted.origin.x != expected {
                    mismatches.push((direction, max_lines, align, painted.origin.x, expected));
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:?}");
}
