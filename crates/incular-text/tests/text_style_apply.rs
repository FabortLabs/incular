use incular_core::{Color, Lerp, Offset};
use incular_text::{
    FontFamily, FontFeature, FontStyle, FontVariation, FontWeight, LineHeight, TextAlign,
    TextBaseline, TextEngine, TextLeadingDistribution, TextMetricAdjustment as Adjust, TextShadow,
    TextStyle, TextStyleApply as Apply,
};

#[test]
fn apply_replaces_typed_options_without_unsetting_other_fields() {
    let base = TextStyle::new()
        .font_size(10.)
        .shadows([])
        .font_style(FontStyle::Normal)
        .font_feature_settings(&[])
        .font_variation_settings(&[])
        .text_baseline(TextBaseline::Alphabetic)
        .leading_distribution(TextLeadingDistribution::Even);
    let untouched = base.apply_options(Apply::default());
    assert_eq!(untouched, base);
    assert_eq!(untouched.font_features.as_deref(), Some(""));
    assert_eq!(untouched.font_variations.as_deref(), Some(""));
    assert!(untouched.locale.is_none());

    let shadows = [TextShadow::new(Color::BLACK, Offset::ZERO, 2.)];
    let features = [FontFeature::enable(*b"test")];
    let variations = [FontVariation::new(*b"test", 100.)];
    let fallback = [FontFamily::named("Fallback")];
    let got = base.apply_options(Apply {
        font_family: Some(FontFamily::named("Primary")),
        font_family_fallback: Some(&fallback),
        shadows: Some(&shadows),
        font_style: Some(FontStyle::Italic),
        locale: Some("es"),
        font_features: Some(&features),
        font_variations: Some(&variations),
        text_baseline: Some(TextBaseline::Ideographic),
        leading_distribution: Some(TextLeadingDistribution::Proportional),
        ..Apply::default()
    });
    assert_eq!(got.size, 10.);
    assert_eq!(got.family.name(), "Primary");
    assert_eq!(got.fallback_families.as_ref(), &fallback);
    assert_eq!(got.shadows.as_ref(), &shadows);
    assert_eq!(got.style, FontStyle::Italic);
    assert_eq!(got.locale.as_deref(), Some("es"));
    assert_eq!(got.font_features.as_deref(), Some("\"test\" 1"));
    assert_eq!(got.font_variations.as_deref(), Some("\"test\" 100.0"));
    assert_eq!(got.text_baseline, Some(TextBaseline::Ideographic));
    assert_eq!(
        got.leading_distribution,
        Some(TextLeadingDistribution::Proportional)
    );

    let emptied = got.merge(&got.apply_options(Apply {
        font_family_fallback: Some(&[]),
        shadows: Some(&[]),
        font_features: Some(&[]),
        font_variations: Some(&[]),
        ..Apply::default()
    }));
    assert!(emptied.fallback_families.is_empty());
    assert!(emptied.shadows.is_empty());
    assert_eq!(emptied.font_features.as_deref(), Some(""));
    assert_eq!(emptied.font_variations.as_deref(), Some(""));
}

#[test]
fn each_metric_multiplies_before_adding_its_delta() {
    let style = TextStyle::new()
        .font_size(10.)
        .letter_spacing(2.)
        .word_spacing(Some(-2.))
        .height(Some(2.))
        .decoration_thickness(4.)
        .apply_options(Apply {
            font_size: Adjust::new(-2., 30.),
            letter_spacing: Adjust::new(3., -8.),
            word_spacing: Adjust::new(2., 1.),
            height: Adjust::new(2., 1.),
            decoration_thickness: Adjust::new(3., -1.),
            ..Apply::default()
        });
    assert_eq!(style.size, 10.);
    assert_eq!(style.letter_spacing, -2.);
    assert_eq!(style.word_spacing, Some(-3.));
    assert_eq!(style.line_height, Some(LineHeight::Multiplier(5.)));
    assert_eq!(style.decoration_thickness, Some(11.));
    let legacy =
        TextStyle::new()
            .font_size(10.)
            .apply(None, None, Some(-2.), Some(30.), None, None);
    assert_eq!(legacy.size, 10.);
}

#[test]
fn weight_adjustment_clamps_the_legacy_weight_index() {
    let style = TextStyle::new().font_weight(FontWeight::new(580));
    for (delta, expected) in [(0, 500), (-2, 300), (i32::MIN, 100), (i32::MAX, 900)] {
        assert_eq!(
            style
                .apply_options(Apply {
                    font_weight_delta: delta,
                    ..Apply::default()
                })
                .weight
                .value(),
            expected
        );
    }
}

#[test]
fn natural_height_is_not_scaled_by_apply_and_renders_at_the_font_metrics() {
    let natural = TextStyle::new().height(Some(0.));
    let scale = Apply {
        height: Adjust::new(1000., 1000.),
        ..Apply::default()
    };
    assert_eq!(
        TextStyle::new().apply_options(scale.clone()).line_height,
        None
    );
    let adj = natural.apply_options(scale);
    assert_eq!(adj.line_height, Some(LineHeight::Normal));

    let base = TextStyle::new().font_size(20.);
    let parent = base.clone().height(Some(5.));
    let overridden = parent.merge(&adj);
    let mut engine = TextEngine::new();
    let mut advance = |style| {
        let layout = engine.layout("H\nH", style, None, TextAlign::Left);
        layout.lines[1].baseline - layout.lines[0].baseline
    };
    assert_eq!(advance(&parent), 100.);
    assert_eq!(advance(&overridden), advance(&base));
    assert_eq!(
        natural.lerp(&base.height(Some(2.)), 0.25).line_height,
        Some(LineHeight::Multiplier(0.5))
    );
}

#[test]
#[cfg(debug_assertions)]
fn adjusting_an_absent_weight_or_numeric_field_is_a_debug_error() {
    for options in [
        Apply {
            font_weight_delta: 1,
            ..Apply::default()
        },
        Apply {
            word_spacing: Adjust::new(1., 1.),
            ..Apply::default()
        },
        Apply {
            font_size: Adjust::new(2., 0.),
            ..Apply::default()
        },
        Apply {
            letter_spacing: Adjust::new(1., 1.),
            ..Apply::default()
        },
        Apply {
            decoration_thickness: Adjust::new(2., 0.),
            ..Apply::default()
        },
    ] {
        assert!(std::panic::catch_unwind(|| TextStyle::new().apply_options(options)).is_err());
    }
}
