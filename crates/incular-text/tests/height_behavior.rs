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
    approx(trimmed_top.metrics.size.height, LINE - half_leading);
    // Even leading is split in halves: ascent + descent + 2 * half = line.
    let trimmed_bottom = layout("Hg", even, true, false);
    approx(trimmed_bottom.metrics.baseline, full.metrics.baseline);
    approx(trimmed_bottom.metrics.size.height, LINE - half_leading);
    let both = layout("Hg", even, false, false);
    approx(both.metrics.size.height, LINE - 2. * half_leading);

    // Only the outer edges of a two-line paragraph lose leading.
    let lines = layout("H\ng", even, false, false);
    assert_eq!(lines.lines.len(), 2);
    approx(lines.metrics.size.height, 2. * LINE - 2. * half_leading);
    approx(lines.lines[0].baseline, ascent);
    approx(lines.lines[1].baseline, ascent + LINE);
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
    // Glyphs move with the baseline.
    let shift = proportional.metrics.baseline - full_even.metrics.baseline;
    let glyph = |layout: &TextLayout| layout.lines[0].runs[0].origin.y;
    approx(glyph(&proportional) - glyph(&full_even), shift);
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
