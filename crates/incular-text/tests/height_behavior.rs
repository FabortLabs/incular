//! Flutter's `TextHeightBehavior` and `TextLeadingDistribution` for an
//! explicit line height.

use incular_text::{
    LineHeight, TextAlign, TextEngine, TextHeightBehavior, TextLayout, TextLayoutOptions,
    TextLeadingDistribution, TextStyle,
};
use std::sync::Arc;

const LINE: f32 = 30.;

fn style(distribution: Option<TextLeadingDistribution>) -> TextStyle {
    TextStyle {
        size: 10.,
        line_height: Some(LineHeight::Absolute(LINE)),
        leading_distribution: distribution,
        ..TextStyle::default()
    }
}

fn layout(
    text: &str,
    distribution: Option<TextLeadingDistribution>,
    first: bool,
    last: bool,
) -> Arc<TextLayout> {
    TextEngine::new().layout_with_options(
        text,
        &style(distribution),
        TextLayoutOptions::new(None, TextAlign::Start).height_behavior(TextHeightBehavior::new(
            first,
            last,
            TextLeadingDistribution::Proportional,
        )),
    )
}

fn approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-3,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn trimming_the_first_ascent_and_last_descent_removes_their_leading() {
    let even = Some(TextLeadingDistribution::Even);
    let full = layout("Hg", even, true, true);
    approx(full.metrics.size.height, LINE);
    // Without the first ascent's leading the baseline sits at the ascent.
    let trimmed_top = layout("Hg", even, false, true);
    let ascent = trimmed_top.metrics.baseline;
    let half_leading = full.metrics.baseline - ascent;
    // Line heights round to whole pixels (SkParagraph
    // `InternalLineMetrics::height`); the baseline does not.
    approx(
        trimmed_top.metrics.size.height,
        (LINE - half_leading).round(),
    );
    // Even leading is split in halves: ascent + descent + 2 * half = line.
    let trimmed_bottom = layout("Hg", even, true, false);
    approx(trimmed_bottom.metrics.baseline, full.metrics.baseline);
    approx(
        trimmed_bottom.metrics.size.height,
        (LINE - half_leading).round(),
    );
    let both = layout("Hg", even, false, false);
    approx(both.metrics.size.height, (LINE - 2. * half_leading).round());

    // Only the outer edges of a two-line paragraph lose leading.
    let lines = layout("H\ng", even, false, false);
    assert_eq!(lines.lines.len(), 2);
    let first_line = (LINE - half_leading).round();
    approx(lines.metrics.size.height, 2. * first_line);
    approx(lines.lines[0].baseline, ascent);
    approx(lines.lines[1].baseline, first_line + half_leading + ascent);
}

#[test]
fn proportional_leading_follows_the_ascent_to_descent_ratio() {
    let even = layout("Hg", Some(TextLeadingDistribution::Even), false, true);
    let full_even = layout("Hg", Some(TextLeadingDistribution::Even), true, true);
    let ascent = even.metrics.baseline;
    let descent = LINE - 2. * (full_even.metrics.baseline - ascent) - ascent;
    // `TextStyle.leadingDistribution` unset: the behavior's proportional
    // distribution places the baseline at line * ascent / (ascent + descent).
    let proportional = layout("Hg", None, true, true);
    approx(
        proportional.metrics.baseline,
        LINE * ascent / (ascent + descent),
    );
    approx(proportional.metrics.size.height, LINE);
    // The style's distribution wins over the behavior's.
    approx(
        full_even.metrics.baseline,
        ascent + (LINE - ascent - descent) / 2.,
    );
    // Glyphs move with the baseline, painting on its whole-pixel rounding.
    let glyph = |layout: &TextLayout| {
        let run = &layout.lines[0].runs[0];
        run.origin.y - run.glyphs[0].offset.y
    };
    approx(glyph(&proportional), proportional.metrics.baseline.round());
    approx(glyph(&full_even), full_even.metrics.baseline.round());
}

#[test]
fn without_an_explicit_height_the_behavior_changes_nothing() {
    let mut engine = TextEngine::new();
    let plain = TextStyle::default();
    let normal = engine.layout("Hg", &plain, None, TextAlign::Start);
    let trimmed =
        engine.layout_with_options(
            "Hg",
            &plain,
            TextLayoutOptions::new(None, TextAlign::Start).height_behavior(
                TextHeightBehavior::new(false, false, TextLeadingDistribution::Even),
            ),
        );
    assert_eq!(normal.metrics, trimmed.metrics);
}

fn inter(size: f32) -> TextStyle {
    TextStyle {
        size,
        family: incular_text::FontFamily::Named("Inter".into()),
        ..TextStyle::default()
    }
}

fn inter_engine() -> TextEngine {
    let mut engine = TextEngine::new();
    engine.register_font_bytes(
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../incular-web/fonts/Inter-Regular.ttf"
        ))
        .expect("read Inter-Regular.ttf"),
    );
    engine
}

/// SkParagraph rounds a line's height (`InternalLineMetrics::height`) but
/// keeps its baseline at the unrounded ascent. Scenario
/// `layout__baseline__cross_axis`: Flutter places 12 px Inter's baseline at
/// 11.625 with a 15 px line, and 24 px Inter's at 23.25.
#[test]
fn line_heights_round_while_the_baseline_keeps_the_exact_ascent() {
    let mut engine = inter_engine();
    let small = engine.layout_with_options(
        "Inter 12",
        &inter(12.),
        TextLayoutOptions::new(None, TextAlign::Start),
    );
    approx(small.metrics.baseline, 11.625);
    approx(small.metrics.size.height, 15.);
    // The glyphs themselves paint on the rounded ascent, as Flutter's
    // raster does (pixel scenes `text@1.25`, `text@2.0`).
    let run = &small.lines[0].runs[0];
    approx(run.origin.y - run.glyphs[0].offset.y, 12.);
    let large = engine.layout_with_options(
        "Inter 24",
        &inter(24.),
        TextLayoutOptions::new(None, TextAlign::Start),
    );
    approx(large.metrics.baseline, 23.25);
    approx(large.metrics.size.height.fract(), 0.);
    // Scenario `text__text_height_behavior_first_ascent`: 14 px Inter at
    // height 2 without the first ascent's leading is 19 px tall in Flutter.
    let trimmed = engine.layout_with_options(
        "alpha",
        &TextStyle {
            line_height: Some(LineHeight::Multiplier(2.)),
            ..inter(14.)
        },
        TextLayoutOptions::new(Some(200.), TextAlign::Start).height_behavior(
            TextHeightBehavior::new(false, true, TextLeadingDistribution::Proportional),
        ),
    );
    approx(trimmed.metrics.size.height, 19.);
}
