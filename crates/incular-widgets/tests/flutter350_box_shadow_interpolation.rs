//! Value checkpoints from Flutter 3.50 painting/box_painter_test.dart and
//! the pinned box_shadow_extrapolation_test.dart probe.

use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    ptr,
};

use incular_core::{Color, Lerp, Offset};
use incular_widgets::{BlurStyle, BoxShadow};

fn shadow(blur: f32, style: BlurStyle) -> BoxShadow {
    BoxShadow {
        blur_radius: blur,
        blur_style: style,
        ..BoxShadow::default()
    }
}

#[test]
fn upstream_box_shadow_control_values() {
    let one = shadow(4., BlurStyle::Normal);
    let two = BoxShadow::lerp_ref(None, Some(&one), 0.25)
        .unwrap()
        .into_owned();
    let three = BoxShadow::lerp_ref(Some(&one), None, 0.25)
        .unwrap()
        .into_owned();
    let fingerprint = |value: BoxShadow| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    };
    assert_eq!(one.to_string().lines().count(), 1);
    assert_ne!(fingerprint(one), fingerprint(two));
    assert_eq!(one, shadow(4., BlurStyle::Normal));
    assert_eq!([two.blur_radius, three.blur_radius], [1., 3.]);
    let four = two.lerp(&three, 0.5);
    assert_eq!(four.blur_radius, 2.);
    let expected = [four, one.scale(0.5)];
    assert_eq!(BoxShadow::lerp_list(&[two, one], &[three], 0.5), expected);
    assert_eq!(BoxShadow::lerp_list(&[two], &[three, one], 0.5), expected);
}

#[test]
fn upstream_box_shadow_blur_style_values() {
    let one = shadow(4., BlurStyle::Normal);
    let two = shadow(4., BlurStyle::Outer);
    let three = BoxShadow::lerp_ref(Some(&one), None, 0.25)
        .unwrap()
        .into_owned();
    let four = BoxShadow::lerp_ref(None, Some(&one), 0.25)
        .unwrap()
        .into_owned();
    let five = one.lerp(&two, 0.25);
    let six = shadow(0., BlurStyle::Solid).lerp(&two, 0.25);
    for (sample, expected) in [
        (one, BlurStyle::Normal),
        (two, BlurStyle::Outer),
        (three, BlurStyle::Normal),
        (four, BlurStyle::Normal),
        (five, BlurStyle::Outer),
        (six, BlurStyle::Solid),
    ] {
        assert_eq!(sample.blur_style, expected);
    }
    for (samples, first_style) in [
        (
            BoxShadow::lerp_list(&[two, one], &[three], 0.5),
            BlurStyle::Outer,
        ),
        (
            BoxShadow::lerp_list(&[six], &[three, one], 0.5),
            BlurStyle::Solid,
        ),
        (
            BoxShadow::lerp_list(&[three], &[six, one], 0.5),
            BlurStyle::Solid,
        ),
        (
            BoxShadow::lerp_list(&[three], &[two, one], 0.5),
            BlurStyle::Outer,
        ),
    ] {
        assert_eq!(
            [samples[0].blur_style, samples[1].blur_style],
            [first_style, BlurStyle::Normal]
        );
    }
}

#[test]
fn flutter_350_box_shadow_extrapolates_color_alongside_geometry() {
    let a = BoxShadow::new(Color::rgba(64, 64, 128, 255), Offset::new(2., 4.), 4., 4.);
    let b = BoxShadow::new(Color::rgba(128, 128, 64, 255), Offset::new(6., 8.), 8., 8.);
    let samples = [-0.5, 0.5, 1.5].map(|t| a.lerp(&b, t));
    assert_eq!(
        samples.map(|s| s.offset),
        [
            Offset::new(0., 2.),
            Offset::new(4., 6.),
            Offset::new(8., 10.)
        ]
    );
    assert_eq!(samples.map(|s| s.blur_radius), [2., 6., 10.]);
    assert_eq!(samples.map(|s| s.spread_radius), [2., 6., 10.]);
    assert_eq!(samples[1].color, Color::rgba(96, 96, 96, 255));
    assert_eq!(
        [samples[0].color, samples[2].color],
        [
            Color::rgba(32, 32, 160, 255),
            Color::rgba(160, 160, 32, 255)
        ]
    );
}

#[test]
fn optional_shadow_and_shadow_list_borrow_the_identical_originals() {
    assert!(BoxShadow::lerp_ref(None, None, 0.).is_none());
    assert!(BoxShadow::lerp_list_ref(None, None, 0.).is_none());
    let shadow = BoxShadow::default();
    let Some(Cow::Borrowed(same)) = BoxShadow::lerp_ref(Some(&shadow), Some(&shadow), 0.5) else {
        panic!("expected a borrowed shadow");
    };
    assert!(ptr::eq(same, &shadow));
    let shadows = [shadow];
    let Some(Cow::Borrowed(same)) = BoxShadow::lerp_list_ref(Some(&shadows), Some(&shadows), 0.5)
    else {
        panic!("expected a borrowed shadow list");
    };
    assert!(ptr::eq(same, &shadows[..]));
}

#[test]
fn a_shadow_description_keeps_blur_style_and_geometry() {
    let color = "Color(alpha: 1.0000, red: 0.0000, green: 0.0000, blue: 0.0000, colorSpace: ColorSpace.sRGB)";
    for (style, name) in [(BlurStyle::Normal, "normal"), (BlurStyle::Solid, "solid")] {
        assert_eq!(
            shadow(4., style).to_string(),
            format!("BoxShadow({color}, Offset(0.0, 0.0), 4.0, 0.0, BlurStyle.{name})")
        );
    }
}

#[test]
fn copying_each_shadow_property_leaves_all_others_untouched() {
    let original = BoxShadow::default();
    let color = Color::rgba(0x11, 0x22, 0x33, 0xff);
    let offset = Offset::new(1., 2.);
    let cases = [
        (
            original.copy_with(Some(color), None, None, None, None),
            BoxShadow { color, ..original },
        ),
        (
            original.copy_with(None, Some(offset), None, None, None),
            BoxShadow { offset, ..original },
        ),
        (
            original.copy_with(None, None, Some(123.), None, None),
            BoxShadow {
                blur_radius: 123.,
                ..original
            },
        ),
        (
            original.copy_with(None, None, None, Some(123.), None),
            BoxShadow {
                spread_radius: 123.,
                ..original
            },
        ),
        (
            original.copy_with(None, None, None, None, Some(BlurStyle::Outer)),
            BoxShadow {
                blur_style: BlurStyle::Outer,
                ..original
            },
        ),
    ];
    for (actual, expected) in cases {
        assert_ne!(expected, original);
        assert_eq!(actual, expected);
    }
}
