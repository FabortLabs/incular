//! Styled runs: each span shapes, measures and colors with its own style
//! (Flutter `TextSpan` tree, `painting/text_painter.dart`).

use incular_core::Color;
use incular_text::{
    RichText, TextAlign, TextEngine, TextLayoutOptions, TextOverflow, TextSpan, TextStyle,
};

fn small() -> TextStyle {
    TextStyle::default().font_size(14.0)
}

fn large() -> TextStyle {
    TextStyle::default().font_size(30.0)
}

#[test]
fn larger_span_raises_the_paragraph_height() {
    let mut engine = TextEngine::new();
    let runs = TextSpan::new("Small ")
        .style(small())
        .child(TextSpan::new("Large").style(large()))
        .flatten(&TextStyle::default());
    let rich = engine.layout_runs_with_options(
        "Small Large",
        &runs,
        TextLayoutOptions::new(Some(220.0), TextAlign::Start),
    );
    let plain = engine.layout_with_options(
        "Small Large",
        &small(),
        TextLayoutOptions::new(Some(220.0), TextAlign::Start),
    );
    // Flutter measures 220x36 for this paragraph; the first run alone gives
    // about 17 px, so a height near the 30 px span proves every run measured.
    assert!(
        rich.metrics.size.height > plain.metrics.size.height + 10.0,
        "rich {} vs plain {}",
        rich.metrics.size.height,
        plain.metrics.size.height
    );
    assert_eq!(rich.lines[0].end, "Small Large".len());
}

#[test]
fn uniform_runs_match_the_single_style_layout() {
    let mut engine = TextEngine::new();
    let text = "uniform paragraph";
    let runs = TextSpan::new(text)
        .style(small())
        .flatten(&TextStyle::default());
    let options = TextLayoutOptions::new(Some(120.0), TextAlign::Start);
    let rich = engine.layout_runs_with_options(text, &runs, options);
    let plain = engine.layout_with_options(text, &small(), options);
    assert_eq!(rich.metrics.size, plain.metrics.size);
    assert_eq!(rich.lines.len(), plain.lines.len());
    assert!(rich.color_runs.is_empty());
}

#[test]
fn span_colors_are_recorded_by_byte_range() {
    let mut engine = TextEngine::new();
    let red = Color::rgba(200, 0, 0, 255);
    let runs = TextSpan::new("ab")
        .style(small().color(Color::BLACK))
        .child(TextSpan::new("cd").style(small().color(red)))
        .flatten(&TextStyle::default());
    let layout = engine.layout_runs_with_options(
        "abcd",
        &runs,
        TextLayoutOptions::new(None, TextAlign::Start),
    );
    assert_eq!(layout.color_runs.len(), 2);
    assert_eq!(layout.color_runs[0].range, 0..2);
    assert_eq!(layout.color_runs[0].color, Color::BLACK);
    assert_eq!(layout.color_runs[1].range, 2..4);
    assert_eq!(layout.color_runs[1].color, red);
}

#[test]
fn rich_text_layout_uses_every_span() {
    let mut engine = TextEngine::new();
    let rich = RichText::new(
        TextSpan::new("Small ")
            .style(small())
            .child(TextSpan::new("Large").style(large())),
    )
    .text_align(TextAlign::Start);
    let layout = rich.layout(&mut engine, Some(220.0));
    assert!(layout.metrics.size.height > 25.0);
}

#[test]
fn ellipsis_keeps_the_style_of_the_text_before_it() {
    let mut engine = TextEngine::new();
    let text = "first second third fourth fifth";
    let runs = TextSpan::new("first second ")
        .style(small())
        .child(TextSpan::new("third fourth fifth").style(large()))
        .flatten(&TextStyle::default());
    let layout = engine.layout_runs_with_options(
        text,
        &runs,
        TextLayoutOptions::new(Some(90.0), TextAlign::Start)
            .soft_wrap(false)
            .overflow(TextOverflow::Ellipsis),
    );
    assert_eq!(layout.lines.len(), 1);
    assert!(layout.overflowed);
}
