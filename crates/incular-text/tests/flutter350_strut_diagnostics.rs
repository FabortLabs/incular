//! Flutter 3.50.0 painting/strut_style_test.dart:9.
use incular_text::{StrutStyle, TextLeadingDistribution};

#[test]
fn strut_diagnostics_keep_absent_flags_distinct_from_explicit_false() {
    let base = StrutStyle::new().font_family("Serif").font_size(14.);
    assert_eq!(base.to_string(), "StrutStyle(family: Serif, size: 14.0)");
    let forced = base.clone().force_strut_height(true);
    assert_eq!(forced.font_family.as_ref().unwrap().name(), "Serif");
    assert_eq!(forced.font_size, Some(14.));
    assert_eq!(forced, forced.clone());
    assert_eq!(
        forced.to_string(),
        "StrutStyle(family: Serif, size: 14.0, <strut height forced>)"
    );
    assert_eq!(
        base.force_strut_height(false).to_string(),
        "StrutStyle(family: Serif, size: 14.0, <strut height normal>)"
    );
    assert_eq!(StrutStyle::new().to_string(), "StrutStyle");
    assert_eq!(
        StrutStyle::new().force_strut_height(false).to_string(),
        "StrutStyle(<strut height normal>)"
    );
    assert_eq!(
        StrutStyle::new().force_strut_height(true).to_string(),
        "StrutStyle(<strut height forced>)"
    );
}

#[test]
fn leading_distribution_is_named_when_the_height_is_explicit() {
    let even = StrutStyle::new()
        .height(14.)
        .leading_distribution(TextLeadingDistribution::Even);
    let proportional = StrutStyle::new()
        .height(14.)
        .leading_distribution(TextLeadingDistribution::Proportional);
    assert_eq!(
        even.to_string(),
        "StrutStyle(height: 14.0x, leadingDistribution: even)"
    );
    assert_eq!(
        proportional.to_string(),
        "StrutStyle(height: 14.0x, leadingDistribution: proportional)"
    );
    assert_ne!(even, proportional);
    assert_eq!(
        StrutStyle::new()
            .leading_distribution(TextLeadingDistribution::Even)
            .to_string(),
        "StrutStyle"
    );
}
