//! Inline placeholder boxes (Flutter `WidgetSpan` / `PlaceholderSpan`) and
//! paragraph struts (Flutter `StrutStyle`).

use incular_core::Size;
use incular_text::{
    FontFamily, RichText, StrutStyle, TextAlign, TextEngine, TextGeometry, TextLayoutOptions,
    TextPlaceholder, TextSpan, TextStyle, WidgetSpan, WidgetSpanAlignment,
    register_application_font,
};

fn inter() -> TextStyle {
    // Application fonts are registered per thread.
    thread_local! {
        static FONT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if !FONT.replace(true) {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../harness/flutter/fonts/Inter-Regular.ttf"
        ))
        .expect("read harness/flutter/fonts/Inter-Regular.ttf");
        register_application_font(bytes);
    }
    TextStyle::new()
        .family(FontFamily::named("Inter"))
        .size(14.)
}

fn widget_paragraph(alignment: WidgetSpanAlignment) -> RichText {
    RichText::new(
        TextSpan::new("before ")
            .style(inter())
            .child(WidgetSpan::sized(7, Size::new(40., 40.)).alignment(alignment))
            .child(TextSpan::new(" after")),
    )
}

#[test]
fn a_middle_widget_span_reserves_its_box_in_the_line() {
    let mut engine = TextEngine::new();
    let paragraph = widget_paragraph(WidgetSpanAlignment::Middle);
    let layout = paragraph.layout(&mut engine, Some(220.));
    // text__widget_span_layout: Flutter's paragraph is 40 px tall, the
    // middle-aligned 40x40 box overflowing the 14 px font on both sides
    // (SkParagraph Run::updateMetrics kMiddle).
    assert_eq!(layout.metrics.size.height, 40.);
    assert_eq!(layout.placeholders.len(), 1);
    let placed = layout.placeholders[0];
    assert_eq!(placed.id, 7);
    assert_eq!(placed.index, "before ".len());
    assert_eq!(placed.rect.size, Size::new(40., 40.));
    assert!(placed.rect.origin.y.abs() < 0.01, "{placed:?}");

    // The box starts where the caret after "before " sits.
    let plain = engine.layout("before after", &inter(), None, TextAlign::Start);
    let start = plain
        .line_caret_positions(0)
        .iter()
        .find(|caret| caret.offset == "before ".len())
        .expect("caret after the space")
        .x;
    assert!(
        (placed.rect.origin.x - start).abs() < 0.01,
        "box starts after the preceding text: {placed:?} vs {start}"
    );
    // The placeholder character itself adds no advance beyond its box.
    let words = engine.layout("before  after", &inter(), None, TextAlign::Start);
    assert!(
        (layout.metrics.size.width - (words.metrics.size.width + 40.)).abs() < 0.01,
        "{} vs {}",
        layout.metrics.size.width,
        words.metrics.size.width + 40.
    );
}

#[test]
fn baseline_alignments_place_the_box_against_the_baseline() {
    let mut engine = TextEngine::new();
    let below = widget_paragraph(WidgetSpanAlignment::BelowBaseline).layout(&mut engine, None);
    let placed = below.placeholders[0];
    assert!((placed.rect.origin.y - below.lines[0].baseline).abs() < 0.01);

    let above = widget_paragraph(WidgetSpanAlignment::AboveBaseline).layout(&mut engine, None);
    let placed = above.placeholders[0];
    assert!((placed.rect.origin.y + 40. - above.lines[0].baseline).abs() < 0.01);

    // Baseline alignment with an explicit child baseline puts that baseline
    // on the text baseline.
    let text = "a\u{fffc}b";
    let runs = TextSpan::new(text)
        .style(inter())
        .flatten(&TextStyle::default());
    let layout = engine.layout_rich_with_options(
        text,
        &runs,
        &[
            TextPlaceholder::new(1, 1, Size::new(10., 30.), WidgetSpanAlignment::Baseline)
                .baseline(Some(20.)),
        ],
        TextLayoutOptions::new(None, TextAlign::Start),
    );
    let placed = layout.placeholders[0];
    assert!((placed.rect.origin.y + 20. - layout.lines[0].baseline).abs() < 0.01);
}

#[test]
fn caret_stops_after_a_placeholder_follow_its_box() {
    let mut engine = TextEngine::new();
    let layout = widget_paragraph(WidgetSpanAlignment::Middle).layout(&mut engine, None);
    let placed = layout.placeholders[0];
    let after = layout
        .line_caret_positions(0)
        .iter()
        .find(|caret| caret.offset == placed.index + '\u{fffc}'.len_utf8())
        .copied()
        .expect("caret after the placeholder");
    assert!(
        (after.x - (placed.rect.origin.x + 40.)).abs() < 0.01,
        "{after:?} vs {placed:?}"
    );
}

#[test]
fn a_forced_strut_sets_every_line_height() {
    let mut engine = TextEngine::new();
    let base = inter();
    let strut = engine.resolve_strut(
        &StrutStyle::new()
            .font_family("Inter")
            .font_size(30.)
            .force_strut_height(true),
        &base,
    );
    let options = TextLayoutOptions::new(None, TextAlign::Start).strut(Some(strut));
    let layout = engine.layout_with_options("one\ntwo", &base, options);
    let line = (strut.ascent + strut.descent).round();
    assert_eq!(layout.metrics.size.height, line * 2.);
    assert!((layout.lines[0].baseline - strut.ascent).abs() < 0.01);

    // Without forcing, a smaller strut never shrinks a line.
    let small = engine.resolve_strut(&StrutStyle::new().font_size(4.), &base);
    let plain = engine.layout_with_options("one", &base, TextLayoutOptions::default());
    let struck = engine.layout_with_options(
        "one",
        &base,
        TextLayoutOptions::default().strut(Some(small)),
    );
    assert_eq!(struck.metrics.size, plain.metrics.size);

    // A strut height multiplies the strut font size (StrutStyle.height).
    let tall = engine.resolve_strut(&StrutStyle::new().font_size(10.).height(3.), &base);
    assert!((tall.ascent + tall.descent - 30.).abs() < 0.01);
}
