//! Minimal repro for explicit `EditableText.text_direction` and
//! `TextAlign::Start` with direction-neutral Latin text.
//!
//! Flutter 3.41.7's `EditableTextState._textDirection` (in
//! `packages/flutter/lib/src/widgets/editable_text.dart`) passes the resolved
//! direction to the text painter. `TextAlign.start` therefore puts `hi` at
//! the left edge for LTR and the right edge for RTL even though the glyphs
//! themselves carry no strong RTL direction.

use incular_config::Constraints;
use incular_core::{Offset, Size};
use incular_rendering::PaintCommand;
use incular_text::{TextAlign, TextEditingController};
use incular_widgets::TextDirection;
use incular_widgets::internal::*;

const FIELD_WIDTH: f32 = 200.0;
const FIELD_HEIGHT: f32 = 32.0;
const TEXT_INSET: f32 = 8.0;

fn glyph_bounds(list: &incular_rendering::DisplayList) -> (f32, f32) {
    let mut shift = Offset::ZERO;
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                shift = shift + transform.translation_offset();
            }
            PaintCommand::GlyphRun { run, .. } => {
                for glyph in run.glyphs.iter() {
                    let left = shift.x + glyph.offset.x;
                    min = min.min(left);
                    max = max.max(left + glyph.advance);
                }
            }
            _ => {}
        }
    }
    assert!(
        min.is_finite() && max.is_finite(),
        "the field must paint `hi`"
    );
    (min, max)
}

fn field_edges(direction: TextDirection) -> (f32, f32) {
    let mut tree = WidgetTree::new();
    tree.mount(
        EditableText::new(TextEditingController::with_text("hi"))
            .text_align(TextAlign::Start)
            .text_direction(direction)
            .size(Size::new(FIELD_WIDTH, FIELD_HEIGHT))
            .into(),
    )
    .expect("mount explicit-direction EditableText");
    tree.layout(Constraints::tight(Size::new(FIELD_WIDTH, FIELD_HEIGHT)))
        .expect("layout explicit-direction EditableText");
    glyph_bounds(&tree.paint())
}

#[test]
fn explicit_editable_text_direction_resolves_start_for_latin_text() {
    let (ltr_min, ltr_max) = field_edges(TextDirection::Ltr);
    let (rtl_min, rtl_max) = field_edges(TextDirection::Rtl);
    let advance = ltr_max - ltr_min;
    let right_edge = TEXT_INSET + (FIELD_WIDTH - 2.0 * TEXT_INSET);

    assert!(
        (ltr_min - TEXT_INSET).abs() < 1.5,
        "Flutter 3.41.7 EditableText.textDirection=ltr + TextAlign.start puts Latin `hi` at the left text edge: {ltr_min}"
    );
    assert!(
        (ltr_max - (TEXT_INSET + advance)).abs() < 1.5,
        "LTR glyph advance must be preserved: {ltr_min}..{ltr_max}"
    );
    assert!(
        (rtl_min - (right_edge - advance)).abs() < 1.5,
        "Flutter 3.41.7 EditableText.textDirection=rtl + TextAlign.start puts Latin `hi` at the right text edge: {rtl_min}..{rtl_max}, want {}..{}",
        right_edge - advance,
        right_edge
    );
    assert!(
        (rtl_max - right_edge).abs() < 1.5,
        "RTL start must end at the right text edge: {rtl_min}..{rtl_max}, want {}..{right_edge}",
        right_edge - advance
    );
}
