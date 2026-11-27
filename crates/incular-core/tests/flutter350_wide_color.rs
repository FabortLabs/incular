use incular_core::{Color, ColorSpace, FloatColor};

fn channels(color: FloatColor) -> [f64; 3] {
    [color.red, color.green, color.blue]
}

fn close(color: FloatColor, expected: [f64; 3]) {
    for (actual, expected) in channels(color).into_iter().zip(expected) {
        assert!((actual - expected).abs() <= 0.01, "{actual} != {expected}");
    }
}

#[test]
fn p3_teal_converts_in_linear_light_and_preserves_negative_red() {
    let teal = FloatColor::new(
        30. / 255.,
        202. / 255.,
        211. / 255.,
        1.,
        ColorSpace::DisplayP3,
    );
    let extended = teal.with_color_space(ColorSpace::ExtendedSrgb);
    assert_eq!(extended.color_space, ColorSpace::ExtendedSrgb);
    close(extended, [-0.376, 0.807, 0.837]);
    assert_eq!(extended.alpha, 1.);
}

#[test]
fn pure_p3_green_preserves_both_negative_components_and_superwhite_green() {
    let green = FloatColor::new(0., 1., 0., 1., ColorSpace::DisplayP3);
    let extended = green.with_color_space(ColorSpace::ExtendedSrgb);
    close(extended, [-0.512, 1.018, -0.311]);
    assert!(extended.red < 0. && extended.green > 1. && extended.blue < 0.);
}

#[test]
fn srgb_through_display_p3_returns_to_the_original_unquantized_channels() {
    let source = FloatColor::new(0.5, 0.3, 0.8, 1., ColorSpace::Srgb);
    let p3 = source.with_color_space(ColorSpace::DisplayP3);
    assert_eq!(p3.color_space, ColorSpace::DisplayP3);
    let extended = p3.with_color_space(ColorSpace::ExtendedSrgb);
    close(extended, channels(source));
}

#[test]
fn compact_output_clips_after_converting_and_uses_straight_alpha() {
    let teal = FloatColor::new(
        30. / 255.,
        202. / 255.,
        211. / 255.,
        0.5,
        ColorSpace::DisplayP3,
    );
    let unclipped = teal.with_color_space(ColorSpace::ExtendedSrgb);
    close(unclipped, [-0.376, 0.807, 0.837]);
    assert_eq!(unclipped.alpha, 0.5);
    assert_eq!(teal.to_srgba8(), unclipped.to_srgba8());
    let packed = teal.to_srgba8();
    assert_eq!(packed.to_argb32(), 0x8000ced5);
    assert_eq!(
        FloatColor::from(Color::from_argb32(0x801ecad3))
            .to_srgba8()
            .to_argb32(),
        0x801ecad3
    );
}
