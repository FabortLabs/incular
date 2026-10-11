//! `EditableText` direction handling, checked through painted glyph positions.
//!
//! Flutter 3.41.7 (`packages/flutter/lib/src/widgets/editable_text.dart`,
//! `EditableTextState._textDirection`) resolves
//! `widget.textDirection ?? Directionality.of(context)` and hands it to the
//! `RenderEditable`, whose `TextPainter` makes `TextAlign.start`/`end`
//! physical. `TextAlign.left`/`right` never depend on direction.

use incular_config::Constraints;
use incular_core::{Offset, Size};
use incular_rendering::PaintCommand;
use incular_text::{TextAlign, TextEditingController};
use incular_widgets::internal::*;
use incular_widgets::{Directionality, TextDirection};

const WIDTH: f32 = 200.0;
const HEIGHT: f32 = 32.0;
/// Horizontal text inset of the editor: the text box spans `8..192`.
const LEFT_EDGE: f32 = 8.0;
const RIGHT_EDGE: f32 = WIDTH - 8.0;
const TOLERANCE: f32 = 1.5;

fn glyph_bounds(list: &incular_rendering::DisplayList) -> (f32, f32) {
    let mut stack = vec![Offset::ZERO];
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let top = *stack.last().expect("transform stack");
                stack.push(top + transform.translation_offset());
            }
            PaintCommand::PopTransform => {
                stack.pop();
            }
            PaintCommand::GlyphRun { run, .. } => {
                let shift = *stack.last().expect("transform stack");
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
        "the field must paint text"
    );
    (min, max)
}

fn field(text: &str, placeholder: &str, align: TextAlign) -> EditableText {
    let mut field = EditableText::new(TextEditingController::with_text(text))
        .text_align(align)
        .size(Size::new(WIDTH, HEIGHT));
    if !placeholder.is_empty() {
        field = field.placeholder(placeholder);
    }
    field
}

fn paint_bounds(root: Widget) -> (f32, f32) {
    let mut tree = WidgetTree::new();
    tree.mount(root).expect("mount");
    tree.layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
        .expect("layout");
    glyph_bounds(&tree.paint())
}

fn directed(text: &str, align: TextAlign, direction: TextDirection) -> (f32, f32) {
    paint_bounds(field(text, "", align).text_direction(direction).into())
}

fn assert_left(bounds: (f32, f32), message: &str) {
    assert!(
        (bounds.0 - LEFT_EDGE).abs() < TOLERANCE,
        "{message}: expected the text to start at the left edge {LEFT_EDGE}, got {bounds:?}"
    );
}

fn assert_right(bounds: (f32, f32), message: &str) {
    assert!(
        (bounds.1 - RIGHT_EDGE).abs() < TOLERANCE,
        "{message}: expected the text to end at the right edge {RIGHT_EDGE}, got {bounds:?}"
    );
}

#[test]
fn start_and_end_follow_the_explicit_direction() {
    assert_left(
        directed("hi", TextAlign::Start, TextDirection::Ltr),
        "ltr + start (TextPainter resolves start against textDirection)",
    );
    assert_right(
        directed("hi", TextAlign::Start, TextDirection::Rtl),
        "rtl + start",
    );
    assert_right(
        directed("hi", TextAlign::End, TextDirection::Ltr),
        "ltr + end",
    );
    assert_left(
        directed("hi", TextAlign::End, TextDirection::Rtl),
        "rtl + end",
    );
}

#[test]
fn left_and_right_ignore_the_direction() {
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        assert_left(
            directed("hi", TextAlign::Left, direction),
            "TextAlign.left is physical",
        );
        assert_right(
            directed("hi", TextAlign::Right, direction),
            "TextAlign.right is physical",
        );
    }
}

#[test]
fn ambient_directionality_is_used_without_an_explicit_direction() {
    let inherited = paint_bounds(
        Directionality::new(TextDirection::Rtl, field("hi", "", TextAlign::Start)).into(),
    );
    assert_right(
        inherited,
        "EditableTextState._textDirection falls back to Directionality.of(context)",
    );
}

#[test]
fn explicit_direction_wins_over_the_ambient_one() {
    let explicit = paint_bounds(
        Directionality::new(
            TextDirection::Rtl,
            field("hi", "", TextAlign::Start).text_direction(TextDirection::Ltr),
        )
        .into(),
    );
    assert_left(
        explicit,
        "widget.textDirection takes precedence over Directionality.of",
    );
}

#[test]
fn placeholder_follows_the_direction() {
    let ltr = paint_bounds(
        field("", "name", TextAlign::Start)
            .text_direction(TextDirection::Ltr)
            .into(),
    );
    let rtl = paint_bounds(
        field("", "name", TextAlign::Start)
            .text_direction(TextDirection::Rtl)
            .into(),
    );
    assert_left(
        ltr,
        "hint text is laid out with the field's direction (ltr)",
    );
    assert_right(
        rtl,
        "hint text is laid out with the field's direction (rtl)",
    );
}

#[test]
fn strong_rtl_text_is_placed_by_the_field_direction_not_its_content() {
    // Flutter paints the paragraph with the widget's textDirection; Hebrew
    // content in an LTR field with TextAlign.start sits at the left edge.
    assert_left(
        directed("שלום", TextAlign::Start, TextDirection::Ltr),
        "Hebrew in an ltr field",
    );
    assert_right(
        directed("שלום", TextAlign::Start, TextDirection::Rtl),
        "Hebrew in an rtl field",
    );
}

#[test]
fn changing_the_ambient_direction_relayouts_the_field() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Directionality::new(TextDirection::Ltr, field("hi", "", TextAlign::Start)).into())
        .expect("mount");
    let constraints = Constraints::tight(Size::new(WIDTH, HEIGHT));
    tree.layout(constraints).expect("layout ltr");
    assert_left(glyph_bounds(&tree.paint()), "initial ltr direction");

    tree.update(
        root,
        Directionality::new(TextDirection::Rtl, field("hi", "", TextAlign::Start)).into(),
    )
    .expect("update to rtl");
    tree.layout(constraints).expect("layout rtl");
    assert_right(
        glyph_bounds(&tree.paint()),
        "Directionality.updateShouldNotify must re-lay-out the editor",
    );
}
