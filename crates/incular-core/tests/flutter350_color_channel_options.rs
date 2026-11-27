use std::hash::{DefaultHasher, Hash, Hasher};

use incular_core::{Color, HslColor, HsvColor, Lerp};

fn digest(color: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    color.hash(&mut hasher);
    hasher.finish()
}

fn close(actual: f64, expected: f64) {
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

#[test]
fn optional_models_borrow_identical_input_and_preserve_hue_and_rgb_on_a_fade() {
    use std::borrow::Cow;

    assert!(HslColor::lerp_ref(None, None, 0.).is_none());
    assert!(HsvColor::lerp_ref(None, None, 0.).is_none());

    let hsl = HslColor::new(360., 0.4, 0.6, 0.5);
    let hsv = HsvColor::new(360., 0.4, 0.6, 0.5);
    let Cow::Borrowed(hsl_same) = HslColor::lerp_ref(Some(&hsl), Some(&hsl), 0.5).unwrap() else {
        panic!("identical HSL must be borrowed");
    };
    let Cow::Borrowed(hsv_same) = HsvColor::lerp_ref(Some(&hsv), Some(&hsv), 0.5).unwrap() else {
        panic!("identical HSV must be borrowed");
    };
    assert!(std::ptr::eq(hsl_same, &hsl));
    assert!(std::ptr::eq(hsv_same, &hsv));

    assert_eq!(
        *HslColor::lerp_ref(None, Some(&hsl), 0.25).unwrap(),
        hsl.with_alpha(0.125)
    );
    assert_eq!(
        *HsvColor::lerp_ref(None, Some(&hsv), 0.25).unwrap(),
        hsv.with_alpha(0.125)
    );
    assert_eq!(
        *HslColor::lerp_ref(Some(&hsl), None, 0.25).unwrap(),
        hsl.with_alpha(0.375)
    );
    assert_eq!(
        *HsvColor::lerp_ref(Some(&hsv), None, 0.25).unwrap(),
        hsv.with_alpha(0.375)
    );
    assert_eq!(
        *HslColor::lerp_ref(None, Some(&hsl), 1.5).unwrap(),
        hsl.with_alpha(0.75)
    );
    assert_eq!(
        *HsvColor::lerp_ref(Some(&hsv), None, -0.5).unwrap(),
        hsv.with_alpha(0.75)
    );

    let hsl_end = hsl.with_hue(20.);
    let hsv_end = hsv.with_hue(20.);
    assert_eq!(
        *HslColor::lerp_ref(Some(&hsl), Some(&hsl_end), 0.5).unwrap(),
        hsl.lerp(&hsl_end, 0.5)
    );
    assert_eq!(
        *HsvColor::lerp_ref(Some(&hsv), Some(&hsv_end), 0.5).unwrap(),
        hsv.lerp(&hsv_end, 0.5)
    );
}
