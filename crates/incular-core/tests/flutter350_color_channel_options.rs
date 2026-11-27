use std::hash::{DefaultHasher, Hash, Hasher};

use incular_core::{Color, HslColor, HsvColor, Lerp};

fn digest(color: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    color.hash(&mut hasher);
    hasher.finish()
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

#[test]
fn hsv_channel_replacements_leave_other_values_and_pixel_conversion_unchanged() {
    let color = HsvColor::new(28., 0.3, 0.6, 0.7);
    let diagnostic = format!("{color:?}");
    assert!(diagnostic.starts_with("HsvColor"));
    assert_eq!(diagnostic.lines().count(), 1);
    assert_eq!(digest(color), digest(HsvColor::new(28., 0.3, 0.6, 0.7)));
    assert_eq!(color.with_alpha(0.8), HsvColor::new(28., 0.3, 0.6, 0.8));
    assert_eq!(color.with_hue(123.), HsvColor::new(123., 0.3, 0.6, 0.7));
    assert_eq!(
        color.with_saturation(0.9),
        HsvColor::new(28., 0.9, 0.6, 0.7)
    );
    assert_eq!(color.with_value(0.1), HsvColor::new(28., 0.3, 0.1, 0.7));
    assert_eq!(color.to_color(), Color::rgba(0x99, 0x81, 0x6b, 0xb3));
    let mixed = color.lerp(&HsvColor::new(128., 0.7, 0.2, 0.3), 0.25);
    for (a, b) in [
        (mixed.alpha, 0.6),
        (mixed.hue, 53.),
        (mixed.saturation, 0.4),
        (mixed.value, 0.5),
    ] {
        close(a, b);
    }
}

#[test]
fn hsl_channel_replacements_leave_other_values_and_pixel_conversion_unchanged() {
    let color = HslColor::new(28., 0.3, 0.6, 0.7);
    let diagnostic = format!("{color:?}");
    assert!(diagnostic.starts_with("HslColor"));
    assert_eq!(diagnostic.lines().count(), 1);
    assert_eq!(digest(color), digest(HslColor::new(28., 0.3, 0.6, 0.7)));
    assert_eq!(color.with_alpha(0.8), HslColor::new(28., 0.3, 0.6, 0.8));
    assert_eq!(color.with_hue(123.), HslColor::new(123., 0.3, 0.6, 0.7));
    assert_eq!(
        color.with_saturation(0.9),
        HslColor::new(28., 0.9, 0.6, 0.7)
    );
    assert_eq!(color.with_lightness(0.1), HslColor::new(28., 0.3, 0.1, 0.7));
    assert_eq!(color.to_color(), Color::rgba(0xb8, 0x97, 0x7a, 0xb3));
    let mixed = color.lerp(&HslColor::new(128., 0.7, 0.2, 0.3), 0.25);
    for (a, b) in [
        (mixed.alpha, 0.6),
        (mixed.hue, 53.),
        (mixed.saturation, 0.4),
        (mixed.lightness, 0.5),
    ] {
        close(a, b);
    }
}

#[test]
fn signed_zero_still_hashes_like_its_equal_positive_zero() {
    assert_eq!(
        digest(HslColor::new(-0., 0., 0., 0.)),
        digest(HslColor::new(0., 0., 0., 0.))
    );
    assert_eq!(
        digest(HsvColor::new(-0., 0., 0., 0.)),
        digest(HsvColor::new(0., 0., 0., 0.))
    );
}
