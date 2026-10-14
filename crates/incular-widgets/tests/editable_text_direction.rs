//! `EditableText` direction handling, checked through painted glyph positions.
//!
//! Flutter 3.41.7 (`packages/flutter/lib/src/widgets/editable_text.dart`,
//! `EditableTextState._textDirection`) resolves
//! `widget.textDirection ?? Directionality.of(context)` and hands it to the
//! `RenderEditable`, whose `TextPainter` makes `TextAlign.start`/`end`
//! physical. `TextAlign.left`/`right` never depend on direction.

use incular_config::Constraints;
use incular_core::{Color, Offset, Size};
use incular_rendering::PaintCommand;
use incular_text::{TextAlign, TextEditingController, TextSelection};
use incular_widgets::internal::*;
use incular_widgets::{Directionality, TextDirection};
use std::time::Instant;

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

fn caret_edges(list: &incular_rendering::DisplayList, color: incular_core::Color) -> (f32, f32) {
    let rect = list.commands().iter().find_map(|command| match command {
        PaintCommand::Rect { rect, color: paint } if *paint == color => Some(*rect),
        _ => None,
    });
    let rect = rect.expect("focused EditableText must paint its caret");
    (rect.origin.x, rect.origin.x + rect.size.width)
}

fn glyph_cluster_bounds(
    list: &incular_rendering::DisplayList,
    start: usize,
    end: usize,
) -> (f32, f32) {
    let mut stack = vec![Offset::ZERO];
    let mut bounds: Option<(f32, f32)> = None;
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
                for glyph in run
                    .glyphs
                    .iter()
                    .filter(|glyph| (start..end).contains(&(glyph.cluster as usize)))
                {
                    let left = shift.x + glyph.offset.x;
                    let right = left + glyph.advance;
                    bounds = Some(match bounds {
                        Some((min, max)) => (min.min(left), max.max(right)),
                        None => (left, right),
                    });
                }
            }
            _ => {}
        }
    }
    bounds.unwrap_or_else(|| panic!("no painted glyph cluster in {start}..{end}"))
}

fn colored_rects(list: &incular_rendering::DisplayList, color: Color) -> Vec<incular_core::Rect> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { rect, color: paint } if *paint == color => Some(*rect),
            _ => None,
        })
        .collect()
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
fn mixed_paragraph_run_order_follows_the_resolved_field_direction() {
    let text = "English שלום";

    let mut ltr = WidgetTree::new();
    let ltr_root = ltr
        .mount(
            field(text, "", TextAlign::Start)
                .text_direction(TextDirection::Ltr)
                .into(),
        )
        .expect("mount LTR mixed field");
    ltr.layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
        .expect("layout LTR mixed field");
    let ltr_painted = ltr.paint();
    let ltr_english = glyph_cluster_bounds(&ltr_painted, 0, "English ".len());
    let ltr_hebrew = glyph_cluster_bounds(&ltr_painted, "English ".len(), text.len());
    assert!(
        ltr_english.1 <= ltr_hebrew.0,
        "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:856-863 puts the English run to the left of the Hebrew run in an LTR field: English {ltr_english:?}, Hebrew {ltr_hebrew:?}"
    );
    assert_eq!(
        ltr.text_controller(ltr_root)
            .expect("LTR controller")
            .text(),
        text,
        "the retained field must keep the mixed paragraph used for the direction check"
    );

    let mut rtl = WidgetTree::new();
    rtl.mount(
        field(text, "", TextAlign::Start)
            .text_direction(TextDirection::Rtl)
            .into(),
    )
    .expect("mount RTL mixed field");
    rtl.layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
        .expect("layout RTL mixed field");
    let rtl_painted = rtl.paint();
    let rtl_english = glyph_cluster_bounds(&rtl_painted, 0, "English ".len());
    let rtl_hebrew = glyph_cluster_bounds(&rtl_painted, "English ".len(), text.len());
    assert!(
        rtl_hebrew.1 <= rtl_english.0,
        "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:856-863 puts the Hebrew run to the left of the English run in an RTL field: English {rtl_english:?}, Hebrew {rtl_hebrew:?}"
    );
}

#[test]
fn rtl_neutral_punctuation_caret_and_pointer_round_trip() {
    let cursor = Color::rgba(255, 0, 0, 255);
    let text = "שלום, 42";
    let controller = TextEditingController::with_text(text);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Rtl)
                .text_align(TextAlign::Start)
                .cursor_color(cursor)
                .size(Size::new(WIDTH, HEIGHT))
                .into(),
        )
        .expect("mount RTL neutral punctuation field");
    let constraints = Constraints::tight(Size::new(WIDTH, HEIGHT));
    tree.layout(constraints).expect("layout RTL neutral field");
    tree.set_focused(root, true, Instant::now())
        .expect("focus RTL neutral field");

    let offsets = [0, "שלום".len(), "שלום,".len()];
    let mut caret_x = Vec::with_capacity(offsets.len());
    for offset in offsets {
        controller.set_selection(TextSelection::collapsed(offset));
        controller.reset_caret(Instant::now());
        tree.layout(constraints)
            .expect("relayout RTL neutral punctuation caret");
        let (left, right) = caret_edges(&tree.paint(), cursor);
        assert!(
            left >= LEFT_EDGE && right <= WIDTH,
            "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1817-1829 keeps the RTL caret for neutral punctuation inside the field: offset {offset}, rect {left}..{right}"
        );
        caret_x.push((offset, left));
    }

    assert_ne!(
        caret_x[1].1, caret_x[2].1,
        "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:1463-1464 gives the neutral comma distinct caret stops before and after the punctuation"
    );
    for (offset, x) in caret_x {
        assert!(
            tree.text_field_set_caret(
                root,
                Offset::new(x + 0.25, HEIGHT / 2.0),
                false,
                Instant::now(),
            ),
            "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 maps a pointer placed on the RTL caret for logical offset {offset}"
        );
        assert_eq!(
            controller.selection().extent,
            offset,
            "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:1706-1710 reverses the neutral-punctuation caret position back to logical offset {offset}"
        );
    }
}

#[test]
fn mixed_paragraph_selection_round_trips_through_rtl_caret_pointer_geometry() {
    let cursor = Color::rgba(255, 0, 0, 255);
    let selection_color = Color::rgba(0, 255, 0, 255);
    let text = "English שלום";
    let controller = TextEditingController::with_text(text);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Rtl)
                .text_align(TextAlign::Start)
                .cursor_color(cursor)
                .selection_color(selection_color)
                .size(Size::new(WIDTH, HEIGHT))
                .into(),
        )
        .expect("mount RTL mixed selection field");
    let constraints = Constraints::tight(Size::new(WIDTH, HEIGHT));
    tree.layout(constraints).expect("layout RTL mixed field");
    tree.set_focused(root, true, Instant::now())
        .expect("focus RTL mixed field");

    let caret_at =
        |tree: &mut WidgetTree, controller: &TextEditingController, offset: usize| -> f32 {
            controller.set_selection(TextSelection::collapsed(offset));
            controller.reset_caret(Instant::now());
            tree.layout(constraints)
                .expect("relayout mixed selection field");
            caret_edges(&tree.paint(), cursor).0
        };
    let start_x = caret_at(&mut tree, &controller, 0);
    let end_x = caret_at(&mut tree, &controller, text.len());
    assert!(
        start_x > end_x,
        "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:856-863 and 1447-1464 place the RTL mixed paragraph's logical start to the right of its logical end: start {start_x}, end {end_x}"
    );

    controller.set_selection(TextSelection::collapsed(0));
    controller.reset_caret(Instant::now());
    tree.layout(constraints)
        .expect("restore mixed selection anchor");
    assert!(
        tree.text_field_set_caret(
            root,
            Offset::new(end_x + 0.25, HEIGHT / 2.0),
            true,
            Instant::now(),
        ),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 extends a selection at the visual RTL logical end"
    );
    assert_eq!(
        controller.selection(),
        TextSelection::new(0, text.len()),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1721-1781 preserves the logical base and extent when a mixed RTL selection is extended through its painted caret"
    );
    tree.layout(constraints)
        .expect("paint mixed forward selection");
    assert!(
        !colored_rects(&tree.paint(), selection_color).is_empty(),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1304-1315 paints a mixed-direction selection after the caret round trip"
    );

    controller.set_selection(TextSelection::collapsed(text.len()));
    controller.reset_caret(Instant::now());
    tree.layout(constraints)
        .expect("restore reverse selection anchor");
    assert!(
        tree.text_field_set_caret(
            root,
            Offset::new(start_x + 0.25, HEIGHT / 2.0),
            true,
            Instant::now(),
        ),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 extends a reverse selection to the visual RTL logical start"
    );
    assert_eq!(
        controller.selection(),
        TextSelection::new(text.len(), 0),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1721-1781 preserves a reverse logical selection through the mixed RTL caret geometry"
    );
}

#[test]
fn arabic_hebrew_and_neutral_text_use_the_resolved_field_direction() {
    for (label, text) in [
        ("Arabic", "مرحبا"),
        ("Hebrew", "שלום"),
        ("neutral", "123 - 45"),
    ] {
        let ltr = directed(text, TextAlign::Start, TextDirection::Ltr);
        assert!(
            (ltr.0 - LEFT_EDGE).abs() < TOLERANCE,
            "Flutter 3.41.7 packages/flutter/lib/src/widgets/editable_text.dart:1138-1142 and packages/flutter/lib/src/painting/text_painter.dart:856-859 resolve {label} TextAlign.start against the LTR field direction: {ltr:?}"
        );
        let rtl = directed(text, TextAlign::Start, TextDirection::Rtl);
        assert!(
            (rtl.1 - RIGHT_EDGE).abs() < TOLERANCE,
            "Flutter 3.41.7 packages/flutter/lib/src/widgets/editable_text.dart:1138-1142 and packages/flutter/lib/src/painting/text_painter.dart:856-859 resolve {label} TextAlign.start against the RTL field direction: {rtl:?}"
        );
    }
}

#[test]
fn rtl_caret_pointer_and_native_input_share_the_painted_geometry() {
    let cursor = incular_core::Color::rgba(255, 0, 0, 255);
    let controller = TextEditingController::with_text("שלום");
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Rtl)
                .text_align(TextAlign::Start)
                .size(Size::new(WIDTH, HEIGHT))
                .cursor_color(cursor)
                .into(),
        )
        .expect("mount RTL field");
    tree.layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
        .expect("layout RTL field");
    tree.set_focused(root, true, Instant::now())
        .expect("focus RTL field");
    controller.set_selection(TextSelection::collapsed(0));
    controller.reset_caret(Instant::now());

    let painted = tree.paint();
    let glyph = glyph_bounds(&painted);
    let caret = caret_edges(&painted, cursor);
    assert!(
        (caret.0 - glyph.1).abs() < TOLERANCE,
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1817-1829 places the logical-start RTL caret at the painted right edge: caret {caret:?}, glyph {glyph:?}"
    );

    let snapshot = tree
        .text_field_input_snapshot(root)
        .expect("native text input snapshot");
    assert!(
        (snapshot.caret_rect.origin.x - caret.0).abs() < TOLERANCE,
        "Flutter 3.41.7 packages/flutter/lib/src/widgets/editable_text.dart:4945 and packages/flutter/lib/src/rendering/editable.dart:1817-1829 expose the same RTL caret rectangle to native input: snapshot {:?}, painted {:?}",
        snapshot.caret_rect,
        caret
    );
    assert!(
        snapshot.caret_rect.origin.x >= 0.0
            && snapshot.caret_rect.origin.x + snapshot.caret_rect.size.width <= WIDTH,
        "Flutter 3.41.7 packages/flutter/lib/src/widgets/editable_text.dart:4941-4945 keeps the native caret rectangle in the visible field: {:?}",
        snapshot.caret_rect
    );

    let now = Instant::now();
    assert!(
        tree.text_field_set_caret(root, Offset::new(glyph.1 - 1.0, HEIGHT / 2.0), false, now),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 maps a pointer near the visual RTL right edge"
    );
    assert_eq!(
        controller.selection().extent,
        0,
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 maps the visual RTL right edge to logical offset zero"
    );
    assert!(
        tree.text_field_set_caret(root, Offset::new(glyph.0 + 1.0, HEIGHT / 2.0), false, now),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 maps a pointer near the visual RTL left edge"
    );
    assert_eq!(
        controller.selection().extent,
        "שלום".len(),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 maps the visual RTL left edge to logical text end"
    );
}

#[test]
fn empty_placeholder_and_caret_follow_start_direction() {
    let cursor = incular_core::Color::rgba(255, 0, 0, 255);
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        let mut tree = WidgetTree::new();
        let root = tree
            .mount(
                field("", "name", TextAlign::Start)
                    .text_direction(direction)
                    .cursor_color(cursor)
                    .into(),
            )
            .expect("mount empty placeholder field");
        tree.layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
            .expect("layout empty placeholder field");
        tree.set_focused(root, true, Instant::now())
            .expect("focus empty placeholder field");
        tree.text_controller(root)
            .expect("empty placeholder controller")
            .reset_caret(Instant::now());
        let painted = tree.paint();
        let glyph = glyph_bounds(&painted);
        let caret = caret_edges(&painted, cursor);
        let (expected_edge, expected_caret) = match direction {
            TextDirection::Ltr => (glyph.0, glyph.0),
            TextDirection::Rtl => (glyph.1, glyph.1),
        };
        assert!(
            (expected_edge
                - if direction == TextDirection::Ltr {
                    LEFT_EDGE
                } else {
                    RIGHT_EDGE
                })
            .abs()
                < TOLERANCE,
            "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:856-859 aligns the empty placeholder with the resolved {:?} start edge: {glyph:?}",
            direction
        );
        assert!(
            (caret.0 - expected_caret).abs() < TOLERANCE,
            "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1817-1829 places an empty field's caret at the same {:?} placeholder edge: caret {caret:?}, placeholder {glyph:?}",
            direction
        );
        let snapshot = tree
            .text_field_input_snapshot(root)
            .expect("empty placeholder native snapshot");
        assert!(
            (snapshot.caret_rect.origin.x - caret.0).abs() < TOLERANCE,
            "Flutter 3.41.7 packages/flutter/lib/src/widgets/editable_text.dart:4941-4945 reports the same empty {:?} caret geometry to native input: {:?} vs {:?}",
            direction,
            snapshot.caret_rect,
            caret
        );
    }
}

#[test]
fn rtl_overflow_keeps_each_caret_visible_and_pointer_mappable() {
    let cursor = incular_core::Color::rgba(255, 0, 0, 255);
    let text = "שלום אבגדה ".repeat(16);
    let text_len = text.len();
    let controller = TextEditingController::with_text(text);
    controller.set_selection(TextSelection::collapsed(0));
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Rtl)
                .text_align(TextAlign::Start)
                .cursor_color(cursor)
                .size(Size::new(96., HEIGHT))
                .into(),
        )
        .expect("mount overflowing RTL field");
    tree.layout(Constraints::tight(Size::new(96., HEIGHT)))
        .expect("layout overflowing RTL field");
    tree.set_focused(root, true, Instant::now())
        .expect("focus overflowing RTL field");
    controller.reset_caret(Instant::now());

    let start_painted = tree.paint();
    let start_caret = caret_edges(&start_painted, cursor);
    assert!(
        start_caret.0 >= 0.0 && start_caret.1 <= 96.0,
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1279 and 1817-1829 keep the RTL logical-start caret visible after overflow scrolling: {start_caret:?}"
    );
    let start_snapshot = tree
        .text_field_input_snapshot(root)
        .expect("RTL overflow start snapshot");
    assert!(
        start_snapshot.caret_rect.origin.x >= 0.0
            && start_snapshot.caret_rect.origin.x + start_snapshot.caret_rect.size.width <= 96.0,
        "Flutter 3.41.7 packages/flutter/lib/src/widgets/editable_text.dart:4941-4945 keeps the native RTL logical-start caret visible: {:?}",
        start_snapshot.caret_rect
    );
    assert!(
        tree.text_field_at(Offset::new(start_caret.0 + 0.25, HEIGHT / 2.0)) == Some(root),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1984-1988 hit-tests the visible RTL caret inside the field"
    );
    assert!(
        tree.text_field_set_caret(
            root,
            Offset::new(start_caret.0 + 0.25, HEIGHT / 2.0),
            false,
            Instant::now(),
        ),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 maps the visible RTL logical-start caret"
    );
    assert_eq!(
        controller.selection().extent,
        0,
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 keeps the visible RTL start caret at logical offset zero"
    );

    controller.set_selection(TextSelection::collapsed(text_len));
    controller.reset_caret(Instant::now());
    tree.layout(Constraints::tight(Size::new(96., HEIGHT)))
        .expect("relayout overflowing RTL field at logical end");
    let end_painted = tree.paint();
    let end_caret = caret_edges(&end_painted, cursor);
    assert!(
        end_caret.0 >= 0.0 && end_caret.1 <= 96.0,
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1279 and 1817-1829 keep the RTL logical-end caret visible after overflow scrolling: {end_caret:?}"
    );
    assert!(
        end_caret.0 + 5.0 < start_caret.0,
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1279 and 2614-2615 move the overflow viewport between the RTL logical-start and logical-end carets: start {start_caret:?}, end {end_caret:?}"
    );
    let end_snapshot = tree
        .text_field_input_snapshot(root)
        .expect("RTL overflow end snapshot");
    assert!(
        end_snapshot.caret_rect.origin.x >= 0.0
            && end_snapshot.caret_rect.origin.x + end_snapshot.caret_rect.size.width <= 96.0,
        "Flutter 3.41.7 packages/flutter/lib/src/widgets/editable_text.dart:4941-4945 keeps the native RTL logical-end caret visible: {:?}",
        end_snapshot.caret_rect
    );
    assert!(
        tree.text_field_at(Offset::new(end_caret.0 + 0.25, HEIGHT / 2.0)) == Some(root),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1984-1988 hit-tests the visible RTL end caret inside the field"
    );
    assert!(
        tree.text_field_set_caret(
            root,
            Offset::new(end_caret.0 + 0.25, HEIGHT / 2.0),
            false,
            Instant::now(),
        ),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 maps the visible RTL logical-end caret"
    );
    assert_eq!(
        controller.selection().extent,
        text_len,
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1801-1803 keeps the visible RTL end caret at logical text end"
    );
}

#[test]
fn explicit_and_ambient_direction_updates_retain_field_identity() {
    let controller = TextEditingController::with_text("hi");
    let mut explicit = WidgetTree::new();
    let explicit_field = explicit
        .mount(
            EditableText::new(controller.clone())
                .text_align(TextAlign::Start)
                .text_direction(TextDirection::Ltr)
                .size(Size::new(WIDTH, HEIGHT))
                .into(),
        )
        .expect("mount explicitly directed field");
    explicit
        .layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
        .expect("layout explicitly directed field");
    let explicit_render = explicit
        .render_id(explicit_field)
        .expect("explicit field render");
    assert_left(
        glyph_bounds(&explicit.paint()),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:835-840 keeps the explicit LTR field at the left edge before update",
    );
    explicit
        .update(
            explicit_field,
            EditableText::new(controller.clone())
                .text_align(TextAlign::Start)
                .text_direction(TextDirection::Rtl)
                .size(Size::new(WIDTH, HEIGHT))
                .into(),
        )
        .expect("update explicit field direction");
    explicit
        .layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
        .expect("relayout explicit field direction");
    assert_eq!(
        explicit.render_id(explicit_field),
        Some(explicit_render),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:835-840 updates RenderEditable.textDirection in place"
    );
    assert_right(
        glyph_bounds(&explicit.paint()),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:835-840 re-lays out the retained field after explicit textDirection changes",
    );

    let ambient_controller = TextEditingController::with_text("hi");
    let mut ambient = WidgetTree::new();
    let ambient_root = ambient
        .mount(
            Directionality::new(
                TextDirection::Ltr,
                EditableText::new(ambient_controller.clone())
                    .text_align(TextAlign::Start)
                    .size(Size::new(WIDTH, HEIGHT)),
            )
            .into(),
        )
        .expect("mount ambient field");
    ambient
        .layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
        .expect("layout ambient field");
    let ambient_field = ambient
        .children(ambient_root)
        .and_then(|children| children.first().copied())
        .expect("ambient field child");
    let ambient_render = ambient
        .render_id(ambient_field)
        .expect("ambient field render");
    ambient
        .update(
            ambient_root,
            Directionality::new(
                TextDirection::Rtl,
                EditableText::new(ambient_controller)
                    .text_align(TextAlign::Start)
                    .size(Size::new(WIDTH, HEIGHT)),
            )
            .into(),
        )
        .expect("update ambient direction");
    ambient
        .layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
        .expect("relayout ambient direction");
    let updated_field = ambient
        .children(ambient_root)
        .and_then(|children| children.first().copied())
        .expect("updated ambient field child");
    assert_eq!(
        updated_field, ambient_field,
        "Flutter 3.41.7 packages/flutter/lib/src/widgets/basic.dart:222 Directionality.updateShouldNotify retains the EditableText element"
    );
    assert_eq!(
        ambient.render_id(updated_field),
        Some(ambient_render),
        "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:835-840 updates the ambient direction on the retained RenderEditable"
    );
    assert_right(
        glyph_bounds(&ambient.paint()),
        "Flutter 3.41.7 packages/flutter/lib/src/widgets/basic.dart:222 and packages/flutter/lib/src/widgets/editable_text.dart:4945 re-lay out the retained field after ambient direction changes",
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
