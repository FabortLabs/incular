use incular_core::{Color, Lerp};
use incular_text::{FontStyle, FontWeight, LineHeight, TextStyle};

#[test]
fn one_sided_numeric_fields_never_lerp_from_a_synthetic_default() {
    let absent = TextStyle::default();
    let defined = TextStyle::default()
        .font_size(24.)
        .letter_spacing(3.)
        .word_spacing(Some(-2.))
        .line_height_multiplier(1.6)
        .decoration_thickness(2.);
    for t in [-1., 0., 0.01, 0.499, 0.5, 0.99, 1., 2.] {
        for style in [absent.lerp(&defined, t), defined.lerp(&absent, t)] {
            assert_eq!(style.size, 24.);
            assert_eq!(style.letter_spacing, 3.);
            assert_eq!(style.word_spacing, Some(-2.));
            assert_eq!(style.line_height, Some(LineHeight::Multiplier(1.6)));
            assert_eq!(style.decoration_thickness, Some(2.));
            assert!(style.has_explicit_size);
            assert!(style.has_explicit_letter_spacing);
        }
    }
    let identical = absent.lerp(&absent, 0.5);
    assert_eq!(identical, absent);
    assert!(!identical.has_explicit_size);
    assert!(identical.word_spacing.is_none());

    let zero = TextStyle::new()
        .letter_spacing(0.)
        .line_height_multiplier(0.);
    let tween = zero.lerp(&absent, 0.75);
    assert!(tween.has_explicit_letter_spacing);
    assert_eq!(tween.line_height, Some(LineHeight::Multiplier(0.)));
}

#[test]
fn discrete_font_overrides_inherit_only_on_the_unspecified_side() {
    let parent = TextStyle::new()
        .font_family("Ambient")
        .font_style(FontStyle::Italic);
    let absent = TextStyle::new();
    let defined = TextStyle::new()
        .font_family("Named")
        .font_style(FontStyle::Normal);
    for (from, to, early, late) in [
        (&absent, &defined, &parent, &defined),
        (&defined, &absent, &defined, &parent),
    ] {
        for (t, expected) in [(0.499, early), (0.5, late)] {
            let style = parent.merge(&from.lerp(to, t));
            assert_eq!(style.family, expected.family);
            assert_eq!(style.style, expected.style);
        }
    }
}

#[test]
fn a_single_color_fades_in_alpha_and_never_blends_toward_white() {
    let absent = TextStyle::new();
    let plain = Color::rgba(17, 51, 85, 255);
    let decoration = Color::rgba(34, 68, 102, 255);
    let style = TextStyle::new().color(plain).decoration_color(decoration);
    for (t, alpha) in [(0., 0), (0.25, 64), (0.5, 128), (0.75, 191), (1., 255)] {
        let expected = Color { alpha, ..plain };
        let expected_decoration = Color {
            alpha,
            ..decoration
        };
        for mixed in [absent.lerp(&style, t), style.lerp(&absent, 1.0 - t)] {
            assert!(mixed.has_explicit_color);
            assert_eq!(mixed.color, expected);
            assert_eq!(mixed.decoration_color, Some(expected_decoration));
        }
    }
}

#[test]
fn two_numeric_values_extrapolate_and_immutable_overrides_remain_explicit() {
    let a = TextStyle::new()
        .font_size(20.)
        .letter_spacing(1.)
        .word_spacing(Some(2.))
        .line_height_multiplier(1.)
        .decoration_thickness(3.);
    let b = TextStyle::new()
        .font_size(30.)
        .letter_spacing(3.)
        .word_spacing(Some(6.))
        .line_height_multiplier(2.)
        .decoration_thickness(5.);
    for (t, size, letter, word, height, thickness) in
        [(-0.5, 15., 0., 0., 0.5, 2.), (1.5, 35., 4., 8., 2.5, 6.)]
    {
        let actual = a.lerp(&b, t);
        assert_eq!(actual.size, size);
        assert_eq!(actual.letter_spacing, letter);
        assert_eq!(actual.word_spacing, Some(word));
        assert_eq!(actual.line_height, Some(LineHeight::Multiplier(height)));
        assert_eq!(actual.decoration_thickness, Some(thickness));
    }
    let default = TextStyle::new();
    let explicit = default
        .with_font_size(default.size)
        .with_font_weight(FontWeight::NORMAL);
    let parent = TextStyle::new()
        .font_size(30.)
        .font_weight(FontWeight::BOLD);
    let merged = parent.merge(&explicit);
    assert_eq!(merged.size, default.size);
    assert_eq!(merged.weight, FontWeight::NORMAL);
}

#[test]
#[cfg(debug_assertions)]
fn mixed_inheritance_reports_metrics_that_would_jump() {
    let from = TextStyle::new().font_size(18.);
    let to = TextStyle::new().inherit(false);
    let error = std::panic::catch_unwind(|| from.lerp(&to, 0.5)).unwrap_err();
    let message = error.downcast_ref::<String>().unwrap();
    assert!(message.contains("Failed to interpolate TextStyles with different inherit values."));
    assert!(message.contains("color, backgroundColor, letterSpacing, wordSpacing, height, decorationColor, decorationThickness"));
    assert!(!message.contains("fontSize"));
    let unspecified = TextStyle::new();
    let error = std::panic::catch_unwind(|| unspecified.lerp(&to, 0.5)).unwrap_err();
    assert!(error.downcast_ref::<String>().unwrap().contains("fontSize"));
    assert_eq!(unspecified.lerp(&unspecified, 0.5), unspecified);
}

#[test]
fn mixed_inheritance_is_valid_if_at_least_one_endpoint_sets_each_metric() {
    let from = TextStyle::new();
    let to = TextStyle::new()
        .inherit(false)
        .color(Color::rgba(0x65, 0x43, 0x21, 0x87))
        .background_color(Color::rgba(0x34, 0x56, 0x78, 0x12))
        .font_size(20.)
        .letter_spacing(1.)
        .word_spacing(Some(1.))
        .line_height_multiplier(20.)
        .decoration_color(Color::rgba(0x11, 0x11, 0x11, 0x11))
        .decoration_thickness(5.);
    assert_eq!(from.lerp(&to, 1.), to);
    assert!(from.lerp(&to, 0.49).inherit);
    assert!(!from.lerp(&to, 0.5).inherit);
}
