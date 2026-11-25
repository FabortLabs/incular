//! Flutter 3.50.0 text style merge and interpolation behavior.

use incular_core::{Color, Lerp, Offset};
use incular_text::{FontFamily, TextShadow, TextStyle};

#[test]
fn text_style_lerp_interpolates_each_shadow_component() {
    let shadow = |distance, gray| {
        TextShadow::new(
            Color::rgba(gray, gray, gray, 255),
            Offset::new(distance, distance),
            distance,
        )
    };
    let a = TextStyle::new().shadows([shadow(1.0, 0), shadow(2.0, 0x11)]);
    let b = TextStyle::new().shadows([shadow(3.0, 0x22), shadow(4.0, 0x33)]);
    let mid = a.lerp(&b, 0.5);
    assert_eq!(mid.shadows.len(), 2);
    assert_eq!(
        mid.shadows.as_ref(),
        &[shadow(2.0, 0x11), shadow(3.0, 0x22)]
    );
}

#[test]
#[cfg(debug_assertions)]
fn text_style_rejects_nan_height_in_debug() {
    assert!(TextStyle::new().height(Some(1.2)).line_height.is_some());
    assert!(std::panic::catch_unwind(|| TextStyle::new().height(Some(f32::NAN))).is_err());
}

#[test]
fn explicitly_zero_letter_spacing_overrides_inherited_spacing() {
    let inherited = TextStyle::new().letter_spacing(4.0);
    assert_eq!(inherited.merge(&TextStyle::new()).letter_spacing, 4.0);
    assert_eq!(
        inherited
            .merge(&TextStyle::new().letter_spacing(0.0))
            .letter_spacing,
        0.0
    );
}

#[test]
fn explicitly_empty_fallback_families_clear_inherited_fallbacks() {
    let inherited = TextStyle::new().fallback_families(["Fallback"]);
    assert_eq!(
        inherited
            .merge(&TextStyle::new())
            .fallback_families
            .as_ref(),
        &[FontFamily::named("Fallback")]
    );
    assert!(
        inherited
            .merge(&TextStyle::new().fallback_families([] as [&str; 0]))
            .fallback_families
            .is_empty()
    );
}

#[test]
fn unpaired_text_shadows_scale_geometry_but_not_their_color() {
    let color = Color::rgba(10, 20, 30, 128);
    let only = TextShadow::new(color, Offset::new(-8., 12.), 4.);
    let a = TextStyle::new().shadow(only);
    let b = TextStyle::new();
    assert_eq!(
        a.lerp(&b, 0.25).shadows.as_ref(),
        &[TextShadow::new(color, Offset::new(-6., 9.), 3.)],
    );
    assert_eq!(
        b.lerp(&a, 0.25).shadows.as_ref(),
        &[TextShadow::new(color, Offset::new(-2., 3.), 1.)],
    );
    assert_eq!(
        a.lerp(&b, 1.).shadows.as_ref(),
        &[TextShadow::new(color, Offset::ZERO, 0.)],
    );
    assert!(std::sync::Arc::ptr_eq(
        &a.shadows,
        &a.lerp(&a, 0.125).shadows
    ));
}
