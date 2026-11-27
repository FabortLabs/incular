use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

use incular_core::{Color, Offset};
use incular_text::{FontFeature, FontVariation, TextLeadingDistribution, TextShadow, TextStyle};

fn hash(style: &TextStyle) -> u64 {
    BuildHasherDefault::<DefaultHasher>::default().hash_one(style)
}

#[test]
fn text_style_hashes_fallback_order_and_leading_distribution() {
    let make = |name: &str| {
        TextStyle::new()
            .fallback_families([name])
            .shadows([TextShadow::default()])
            .font_feature_settings(&[FontFeature::new(*b"abcd", 1)])
            .font_variation_settings(&[FontVariation::new(*b"wght", 123.)])
    };
    let roboto = make("Roboto");
    let noto = make("Noto");
    assert_eq!(hash(&roboto), hash(&roboto.clone()));
    assert_ne!(hash(&roboto), hash(&noto));
    let even = TextStyle::new().leading_distribution(TextLeadingDistribution::Even);
    let proportional = TextStyle::new().leading_distribution(TextLeadingDistribution::Proportional);
    assert_eq!(hash(&even), hash(&even.clone()));
    assert_ne!(hash(&even), hash(&proportional));
}

#[test]
fn hashes_match_for_signed_zero_and_different_debug_provenance() {
    let make = |zero: f32, label| {
        TextStyle::new()
            .debug_label(label)
            .letter_spacing(zero)
            .shadows([TextShadow {
                color: Color::BLACK,
                offset: Offset::new(zero, zero),
                blur_radius: zero,
            }])
    };
    let left = make(0., "left");
    let right = make(-0., "right");
    assert_eq!(left, right);
    assert_eq!(hash(&left), hash(&right));
}
