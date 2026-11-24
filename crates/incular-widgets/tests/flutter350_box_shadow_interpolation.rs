//! Value checkpoints from Flutter 3.50 painting/box_painter_test.dart and
//! the pinned box_shadow_extrapolation_test.dart probe.

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
    // Flutter lerp(null, one, .25) and lerp(one, null, .25) delegate to scale.
    let two = one.scale(0.25);
    let three = one.scale(0.75);
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
    let three = one.scale(0.75);
    let four = one.scale(0.25);
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
