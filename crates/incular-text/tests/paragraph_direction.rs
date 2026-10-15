//! Paragraph base-direction coverage for the Parley-backed text engine.
//!
//! Flutter 3.41.7's `TextPainter.textDirection` is used both to resolve
//! `TextAlign.start` and to disambiguate bidirectional text.  In particular,
//! an English phrase followed by Hebrew is ordered differently in LTR and RTL
//! contexts (`packages/flutter/lib/src/painting/text_painter.dart:856-863`).

use incular_config::TextDirection;
use incular_text::{TextAffinity, TextAlign, TextEngine, TextLayout, TextLayoutOptions, TextStyle};
use std::sync::Arc;

const MIXED: &str = "English שלום";
const HEBREW_START: usize = "English ".len();
const HEBREW_END: usize = MIXED.len();

fn options(direction: Option<TextDirection>) -> TextLayoutOptions {
    let mut options = TextLayoutOptions::new(None, TextAlign::Start);
    options.text_direction = direction;
    options
}

fn layout(engine: &mut TextEngine, direction: Option<TextDirection>) -> Arc<TextLayout> {
    engine.layout_with_options(MIXED, &TextStyle::default(), options(direction))
}

fn region_bounds(layout: &TextLayout, start: usize, end: usize) -> (f32, f32) {
    let line = layout.lines.first().expect("mixed paragraph has one line");
    let mut bounds: Option<(f32, f32)> = None;
    for span in line
        .clusters
        .iter()
        .filter(|span| span.start < end && start < span.end)
    {
        bounds = Some(match bounds {
            Some((left, right)) => (left.min(span.left), right.max(span.right)),
            None => (span.left, span.right),
        });
    }
    bounds.unwrap_or_else(|| panic!("paragraph has no cluster in {start}..{end}"))
}

fn caret_x(layout: &TextLayout, offset: usize) -> f32 {
    layout
        .line_caret_positions(0)
        .iter()
        .filter(|position| position.offset == offset)
        .map(|position| position.x)
        .min_by(f32::total_cmp)
        .unwrap_or_else(|| panic!("paragraph has no caret stop at byte offset {offset}"))
}

#[test]
fn mixed_paragraph_run_order_follows_the_resolved_base_direction() {
    let mut engine = TextEngine::new();
    let ltr = layout(&mut engine, Some(TextDirection::Ltr));
    let rtl = layout(&mut engine, Some(TextDirection::Rtl));

    let ltr_english = region_bounds(&ltr, 0, HEBREW_START);
    let ltr_hebrew = region_bounds(&ltr, HEBREW_START, HEBREW_END);
    assert!(
        ltr_english.1 <= ltr_hebrew.0,
        "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:856-863 puts the English run to the left of the Hebrew run in an LTR paragraph: English {ltr_english:?}, Hebrew {ltr_hebrew:?}"
    );

    let rtl_english = region_bounds(&rtl, 0, HEBREW_START);
    let rtl_hebrew = region_bounds(&rtl, HEBREW_START, HEBREW_END);
    assert!(
        rtl_hebrew.1 <= rtl_english.0,
        "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:856-863 puts the Hebrew run to the left of the English run in an RTL paragraph: English {rtl_english:?}, Hebrew {rtl_hebrew:?}"
    );
}

#[test]
fn mixed_paragraph_caret_stops_reverse_with_the_base_direction() {
    let mut engine = TextEngine::new();
    let ltr = layout(&mut engine, Some(TextDirection::Ltr));
    let rtl = layout(&mut engine, Some(TextDirection::Rtl));

    let ltr_start = caret_x(&ltr, 0);
    let ltr_end = caret_x(&ltr, MIXED.len());
    assert!(
        ltr_start < ltr_end,
        "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:1447-1464 keeps the LTR logical start before the logical end: {ltr_start}..{ltr_end}"
    );
    let rtl_start = caret_x(&rtl, 0);
    let rtl_end = caret_x(&rtl, MIXED.len());
    assert!(
        rtl_start > rtl_end,
        "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:1447-1464 places the RTL logical start after the logical end: {rtl_start}..{rtl_end}"
    );

    let ltr_stops = ltr.line_caret_positions(0);
    let rtl_stops = rtl.line_caret_positions(0);
    assert!(
        ltr_stops.iter().any(|stop| {
            stop.offset == HEBREW_START && stop.affinity == TextAffinity::Downstream
        }),
        "the LTR mixed paragraph must retain a downstream stop at the Hebrew run boundary"
    );
    assert!(
        rtl_stops.iter().any(|stop| {
            stop.offset == HEBREW_START && stop.affinity == TextAffinity::Downstream
        }),
        "the RTL mixed paragraph must retain a downstream stop at the Hebrew run boundary"
    );
}

#[test]
fn mixed_paragraph_selection_spans_round_trip_to_bidi_caret_edges() {
    let mut engine = TextEngine::new();
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        let layout = layout(&mut engine, Some(direction));
        let line = layout.lines.first().expect("mixed paragraph has one line");
        let spans = line.selection_spans(HEBREW_START, HEBREW_END);
        assert_eq!(
            spans.len(),
            1,
            "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:1658-1710 returns one visual box for the contiguous Hebrew selection in {direction:?}: {spans:?}"
        );
        let (left, right) = spans[0];
        let start_x = layout
            .line_caret_positions(0)
            .iter()
            .filter(|position| position.offset == HEBREW_START)
            .map(|position| position.x)
            .fold(f32::INFINITY, f32::min);
        let end_x = layout
            .line_caret_positions(0)
            .iter()
            .filter(|position| position.offset == HEBREW_END)
            .map(|position| position.x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            left <= start_x && start_x <= right && left <= end_x && end_x <= right,
            "Flutter 3.41.7 packages/flutter/lib/src/rendering/editable.dart:1721-1781 keeps selection endpoints on the selected visual span in {direction:?}: span {spans:?}, stops {start_x} and {end_x}"
        );
    }
}

#[test]
fn changing_base_direction_changes_layout_cache_identity() {
    assert_eq!(
        TextLayoutOptions::default().text_direction,
        None,
        "the generic text engine keeps Parley's automatic base-direction default"
    );
    let mut engine = TextEngine::new();
    let ltr = layout(&mut engine, Some(TextDirection::Ltr));
    let ltr_hit = layout(&mut engine, Some(TextDirection::Ltr));
    assert!(
        Arc::ptr_eq(&ltr, &ltr_hit),
        "a repeated direction and text must reuse the retained paragraph layout"
    );

    let rtl = layout(&mut engine, Some(TextDirection::Rtl));
    assert!(
        !Arc::ptr_eq(&ltr, &rtl),
        "Flutter 3.41.7 packages/flutter/lib/src/painting/text_painter.dart:869-875 marks a text-direction change as a new layout"
    );

    let automatic = layout(&mut engine, None);
    let automatic_english = region_bounds(&automatic, 0, HEBREW_START);
    let automatic_hebrew = region_bounds(&automatic, HEBREW_START, HEBREW_END);
    assert!(
        automatic_english.1 <= automatic_hebrew.0,
        "Parley BaseDirection::Auto keeps the existing LTR inference for an English-leading paragraph: English {automatic_english:?}, Hebrew {automatic_hebrew:?}"
    );
    assert!(
        !Arc::ptr_eq(&ltr, &automatic) && !Arc::ptr_eq(&rtl, &automatic),
        "automatic base direction must remain a distinct cache key from an explicitly resolved direction"
    );
}
