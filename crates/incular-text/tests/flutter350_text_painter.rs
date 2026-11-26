use incular_config::TextDirection;
use incular_core::Size;
use incular_text::{
    InlineSpan, TextAlign, TextEngine, TextPainter, TextPainterError, TextRange, TextSpan,
    TextStyle, TextWidthBasis, WidgetSpan,
};
use std::sync::Arc;

fn painter(span: TextSpan, direction: TextDirection) -> TextPainter {
    let mut painter = TextPainter::new();
    painter.set_text(Some(span));
    painter.set_text_direction(Some(direction));
    painter
}

#[test]
fn layout_requires_explicit_text_direction_even_for_empty_text() {
    let mut missing = TextPainter::new();
    missing.set_text(Some(TextSpan::empty()));
    assert_eq!(
        missing.layout(0., f32::INFINITY),
        Err(TextPainterError::MissingTextDirection)
    );
    assert_eq!(missing.size(), Err(TextPainterError::NeedsLayout));
    missing.set_text_direction(Some(TextDirection::Rtl));
    missing.layout(0., f32::INFINITY).unwrap();
    assert_eq!(missing.size().unwrap().width, 0.);
}

#[test]
fn disposal_releases_a_measurement_and_cannot_be_repeated() {
    let mut painter = painter(TextSpan::new("text"), TextDirection::Ltr);
    assert!(!painter.debug_disposed());
    painter.layout(0., f32::INFINITY).unwrap();
    painter.dispose().unwrap();
    assert!(painter.debug_disposed());
    assert_eq!(painter.size(), Err(TextPainterError::Disposed));
    assert_eq!(painter.dispose(), Err(TextPainterError::Disposed));
}

#[test]
fn plain_text_keeps_inline_placeholders_before_and_after_layout() {
    fn mixed(before: &str, after: &str) -> TextSpan {
        TextSpan::empty().children([
            InlineSpan::from(TextSpan::new(before)),
            InlineSpan::from(WidgetSpan::sized(1, Size::new(50., 30.))),
            InlineSpan::from(TextSpan::new(after)),
        ])
    }

    let mut painter = TextPainter::new();
    painter.set_text_direction(Some(TextDirection::Ltr));
    assert_eq!(painter.plain_text(), "");
    for (before, after, flattened) in [
        ("before\n", "after", "before\n\u{fffc}after"),
        ("be\nfo\nre\n", "af\nter", "be\nfo\nre\n\u{fffc}af\nter"),
    ] {
        painter.set_text(Some(mixed(before, after)));
        assert_eq!(painter.plain_text(), flattened);
        assert_eq!(painter.size(), Err(TextPainterError::NeedsLayout));
        painter.layout(0., f32::INFINITY).unwrap();
        assert_eq!(painter.plain_text(), flattened);
        assert_eq!(painter.shaped_layout().unwrap().placeholders.len(), 1);
    }
}

#[test]
fn consecutive_family_emoji_are_independent_word_boundaries() {
    let emoji = "👨‍👩‍👦";
    let text = emoji.repeat(3);
    let mut painter = painter(TextSpan::new(&text), TextDirection::Ltr);
    painter.layout(0., f32::INFINITY).unwrap();
    let range = painter.word_boundary(emoji.len()).unwrap();
    assert_eq!((range.start, range.end), (emoji.len(), emoji.len() * 2));
    assert_eq!(text[..range.start].encode_utf16().count(), 8);
    assert_eq!(text[..range.end].encode_utf16().count(), 16);
    assert_eq!(
        painter.word_boundary(text.len()).unwrap(),
        TextRange::collapsed(text.len())
    );
}

fn inter() -> (TextEngine, TextStyle) {
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as(
        "Incular Painter Test",
        include_bytes!("../../incular-web/fonts/Inter-Regular.ttf").to_vec(),
    ));
    let style = TextStyle::default()
        .font_family("Incular Painter Test")
        .font_size(10.);
    (engine, style)
}

#[test]
fn longest_line_geometry_distinguishes_width_and_alignment_offsets() {
    let (mut engine, style) = inter();
    let mut p = painter(
        TextSpan::new("aaaaaa\naaaaaa").style(style),
        TextDirection::Ltr,
    );
    p.set_text_align(TextAlign::Center);
    p.set_text_width_basis(TextWidthBasis::LongestLine);
    p.layout_with_engine(&mut engine, 0., 100.).unwrap();
    let metrics = p.line_metrics().unwrap();
    assert_eq!(metrics.len(), 2);
    assert_eq!(metrics[0].width, p.size().unwrap().width);
    assert!(p.paint_offset().unwrap().x < 0.);
    assert!(
        metrics.iter().all(|line| line.left.abs() < 0.0001),
        "{metrics:?}"
    );
    assert!(Arc::ptr_eq(metrics, p.line_metrics().unwrap()));
    let previously = metrics.clone();
    let requests = engine.diagnostics().layouts_requested;
    p.layout_with_engine(&mut engine, 0., 100.).unwrap();
    assert_eq!(engine.diagnostics().layouts_requested, requests);
    assert!(Arc::ptr_eq(&previously, p.line_metrics().unwrap()));
    let (mut separate, _) = inter();
    p.layout_with_engine(&mut separate, 0., 100.).unwrap();
    assert!(!Arc::ptr_eq(&previously, p.line_metrics().unwrap()));
}

#[test]
fn reflow_replaces_cached_metrics_instead_of_leaving_the_old_lines() {
    let (mut engine, style) = inter();
    let mut p = painter(
        TextSpan::new("word1 word2 word3").style(style),
        TextDirection::Ltr,
    );
    for align in [
        TextAlign::Start,
        TextAlign::End,
        TextAlign::Center,
        TextAlign::Justify,
        TextAlign::Left,
        TextAlign::Right,
    ] {
        p.set_text_align(align);
        p.set_text_width_basis(TextWidthBasis::LongestLine);
        p.layout_with_engine(&mut engine, 0., 44.).unwrap();
        let wrapped = p.line_metrics().unwrap().clone();
        assert!(wrapped.len() > 1);
        assert!(Arc::ptr_eq(&wrapped, p.line_metrics().unwrap()));
        p.layout_with_engine(&mut engine, 0., 1_000.).unwrap();
        assert_eq!(p.line_metrics().unwrap().len(), 1);
        assert!(!Arc::ptr_eq(&wrapped, p.line_metrics().unwrap()));
    }
}

#[test]
fn flutter_ahem_line_metrics_when_the_pinned_font_is_available() {
    let Ok(path) = std::env::var("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let font = std::fs::read(path).expect("pinned Ahem font");
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Pinned Ahem", font));
    let make = |text: &str, size: f32| {
        TextSpan::new(text).style(
            TextStyle::default()
                .font_family("Pinned Ahem")
                .font_size(size),
        )
    };
    let mut p = painter(make("aaaaaa\naaaaaa", 10.), TextDirection::Ltr);
    p.set_text_align(TextAlign::Center);
    p.set_text_width_basis(TextWidthBasis::LongestLine);
    p.layout_with_engine(&mut engine, 0., 100.).unwrap();
    assert_eq!(p.size().unwrap().width, 60.);
    assert!(p.line_metrics().unwrap().iter().all(|line| line.left == 0.));

    p.set_text(Some(make("word1 word2 word3", 14.)));
    p.layout_with_engine(&mut engine, 0., 80.).unwrap();
    assert_eq!(p.line_metrics().unwrap().len(), 3);
    p.layout_with_engine(&mut engine, 0., 1000.).unwrap();
    assert_eq!(p.line_metrics().unwrap().len(), 1);

    p.set_text(Some(make("word1 word2 word3", 10.)));
    for align in [
        TextAlign::Start,
        TextAlign::End,
        TextAlign::Center,
        TextAlign::Justify,
        TextAlign::Left,
        TextAlign::Right,
    ] {
        p.set_text_align(align);
        p.layout_with_engine(&mut engine, 0., 130.).unwrap();
        let lines = p.line_metrics().unwrap().clone();
        assert_eq!(lines.len(), 2);
        assert!(Arc::ptr_eq(&lines, p.line_metrics().unwrap()));
        p.layout_with_engine(&mut engine, 0., 1000.).unwrap();
        assert_eq!(p.line_metrics().unwrap().len(), 1);
    }

    p.set_text_align(TextAlign::Justify);
    p.set_text(Some(make(&"A".repeat(100), 10.)));
    for (max, expected) in [(1000., 1000.), (100., 100.), (1000., 1000.)] {
        p.layout_with_engine(&mut engine, 0., max).unwrap();
        assert_eq!(p.size().unwrap().width, expected);
    }

    p.set_text(Some(make("12345", 1.25)));
    p.layout_with_engine(&mut engine, 0., 6.25).unwrap();
    assert_eq!(p.max_intrinsic_width().unwrap(), 6.25);
    assert_eq!(p.line_metrics().unwrap().len(), 1);
    assert_eq!(p.line_metrics().unwrap()[0].width, 6.25);
}
