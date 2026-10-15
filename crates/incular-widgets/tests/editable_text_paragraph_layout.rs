//! `EditableText::paragraph_layout` keeps the same geometry as the default
//! whole-text layout while laying out a long document per paragraph.

use incular_config::Constraints;
use incular_core::{Offset, Size};
use incular_rendering::PaintCommand;
use incular_text::{TextAffinity, TextEditingController, TextSelection};
use incular_widgets::internal::*;
use std::time::Instant;

fn size() -> Size {
    Size::new(320., 220.)
}

fn document() -> String {
    (0..60)
        .map(|index| {
            if index % 7 == 3 {
                format!("Line {index} is long enough to wrap across the field more than once at this width.")
            } else if index % 11 == 5 {
                String::new()
            } else {
                format!("Line {index}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

struct Field {
    tree: WidgetTree,
    id: ElementId,
    controller: TextEditingController,
}

impl Field {
    fn new(paragraph_layout: bool) -> Self {
        let controller = TextEditingController::with_text(document());
        let mut tree = WidgetTree::new();
        let id = tree
            .mount(Widget::from(
                EditableText::new(controller.clone())
                    .text_direction(incular_widgets::TextDirection::Ltr)
                    .multiline(true)
                    .size(size())
                    .paragraph_layout(paragraph_layout),
            ))
            .expect("mount field");
        let mut field = Self {
            tree,
            id,
            controller,
        };
        field.layout();
        field
    }

    fn layout(&mut self) {
        self.tree
            .layout(Constraints::tight(size()))
            .expect("layout");
    }

    fn extent(&self) -> (usize, TextAffinity) {
        (
            self.controller.value().selection.extent,
            self.controller.caret_affinity(),
        )
    }

    /// Painted glyph runs as (origin, first glyph id, glyph count).
    fn painted_runs(&mut self) -> Vec<(Offset, Option<u16>, usize)> {
        self.tree
            .paint()
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::GlyphRun { run, .. } => Some((
                    run.origin,
                    run.glyphs.first().map(|glyph| glyph.id),
                    run.glyphs.len(),
                )),
                _ => None,
            })
            .collect()
    }
}

fn both() -> (Field, Field) {
    (Field::new(false), Field::new(true))
}

#[test]
fn pointer_mapping_matches_the_whole_text_layout() {
    let (mut whole, mut paragraphs) = both();
    let now = Instant::now();
    for y in (0..21).map(|step| step as f32 * 10. + 4.) {
        for x in [0., 9., 40., 120., 300.] {
            whole
                .tree
                .text_field_set_caret(whole.id, Offset::new(x, y), false, now);
            paragraphs
                .tree
                .text_field_set_caret(paragraphs.id, Offset::new(x, y), false, now);
            assert_eq!(whole.extent(), paragraphs.extent(), "point ({x}, {y})");
        }
    }
}

#[test]
fn caret_movement_matches_the_whole_text_layout() {
    let (mut whole, mut paragraphs) = both();
    for field in [&mut whole, &mut paragraphs] {
        field.controller.set_selection(TextSelection::collapsed(5));
    }
    for step in 0..80 {
        let down = step < 50;
        let moved = (
            whole.tree.text_field_move_lines(whole.id, down, 1, false),
            paragraphs
                .tree
                .text_field_move_lines(paragraphs.id, down, 1, false),
        );
        assert_eq!(moved.0, moved.1, "line step {step}");
        assert_eq!(whole.extent(), paragraphs.extent(), "line step {step}");
        if step % 9 == 0 {
            for right in [true, false, true] {
                whole
                    .tree
                    .text_field_move_horizontal(whole.id, right, false);
                paragraphs
                    .tree
                    .text_field_move_horizontal(paragraphs.id, right, false);
                assert_eq!(whole.extent(), paragraphs.extent(), "visual step {step}");
            }
            let (offset, affinity) = whole.extent();
            for forward in [false, true] {
                assert_eq!(
                    whole
                        .tree
                        .text_field_line_boundary(whole.id, offset, affinity, forward, true),
                    paragraphs.tree.text_field_line_boundary(
                        paragraphs.id,
                        offset,
                        affinity,
                        forward,
                        true
                    ),
                );
            }
        }
    }
}

#[test]
fn painting_and_input_rects_match_after_edits() {
    let (mut whole, mut paragraphs) = both();
    let runs = whole.painted_runs();
    assert!(runs.len() > 5, "the viewport paints several lines");
    assert_eq!(runs, paragraphs.painted_runs());
    let text = document();
    let middle = text.len() / 2;
    let middle = (0..=middle)
        .rev()
        .find(|index| text.is_char_boundary(*index))
        .unwrap_or(0);
    let edits = [
        (0, "Inserted at the top\n"),
        (middle, "middle edit "),
        (text.len(), "\nappended"),
    ];
    for (at, insert) in edits {
        for field in [&mut whole, &mut paragraphs] {
            let mut edited = field.controller.text();
            let at = at.min(edited.len());
            edited.insert_str(at, insert);
            field.controller.set_text(edited);
            field
                .controller
                .set_selection(TextSelection::collapsed(at + insert.len()));
            field.layout();
        }
        assert_eq!(
            whole.painted_runs(),
            paragraphs.painted_runs(),
            "after {insert:?}"
        );
        let rect = |field: &Field| {
            field
                .tree
                .text_field_input_snapshot(field.id)
                .map(|snapshot| snapshot.caret_rect)
        };
        assert_eq!(rect(&whole), rect(&paragraphs), "after {insert:?}");
    }
}
