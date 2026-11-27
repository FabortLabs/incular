use incular_core::{Color, HslColor, HsvColor, Lerp};

const SHORTEST: [(f64, f64, f32, f64); 13] = [
    (300., 60., 0.25, 330.),
    (300., 60., 0.5, 0.),
    (300., 60., 0.75, 30.),
    (60., 300., 0.25, 30.),
    (60., 300., 0.5, 0.),
    (60., 300., 0.75, 330.),
    (350., 10., 0.5, 0.),
    (10., 350., 0.5, 0.),
    (20., 200., 0.5, 110.),
    (0., 90., 0.5, 45.),
    (0., 360., 0.5, 0.),
    (300., 60., 1.5, 120.),
    (300., 60., -0.5, 240.),
];

const ANTIPODAL: [(f64, f64, f32, f64); 4] = [
    (0., 180., 0.5, 90.),
    (180., 0., 0.5, 90.),
    (60., 240., 0.75, 195.),
    (240., 60., 0.25, 195.),
];

#[test]
fn hsv_short_arc_handles_both_seam_directions_whole_turns_and_extrapolation() {
    for (a, b, t, expected) in SHORTEST {
        let start = HsvColor::new(a, 1., 1., 1.);
        let end = HsvColor::new(b, 1., 1., 1.);
        assert_eq!(start.lerp(&end, t).hue, expected, "{a} → {b} at {t}");
    }
}

#[test]
fn hsl_short_arc_handles_both_seam_directions_whole_turns_and_extrapolation() {
    for (a, b, t, expected) in SHORTEST {
        let start = HslColor::new(a, 0.5, 0.5, 1.);
        let end = HslColor::new(b, 0.5, 0.5, 1.);
        assert_eq!(start.lerp(&end, t).hue, expected, "{a} → {b} at {t}");
    }
}

#[test]
fn hsv_half_turn_keeps_the_raw_direction_and_reverses_cleanly() {
    for (a, b, t, expected) in ANTIPODAL {
        let start = HsvColor::new(a, 1., 1., 1.);
        let end = HsvColor::new(b, 1., 1., 1.);
        assert_eq!(start.lerp(&end, t).hue, expected);
    }
}

#[test]
fn hsl_half_turn_keeps_the_raw_direction_and_reverses_cleanly() {
    for (a, b, t, expected) in ANTIPODAL {
        let start = HslColor::new(a, 0.5, 0.5, 1.);
        let end = HslColor::new(b, 0.5, 0.5, 1.);
        assert_eq!(start.lerp(&end, t).hue, expected);
    }
}

#[test]
fn color_channels_clamp_after_extrapolation_without_clamping_hue_motion() {
    let from_hsv = HsvColor::new(300., 0.25, 0.25, 0.75);
    let to_hsv = HsvColor::new(60., 0.75, 0.75, 0.25);
    assert_eq!(from_hsv.lerp(&to_hsv, 1.5), HsvColor::new(120., 1., 1., 0.));
    assert_eq!(
        from_hsv.lerp(&to_hsv, -0.5),
        HsvColor::new(240., 0., 0., 1.)
    );

    let from_hsl = HslColor::new(300., 0.25, 0.25, 0.75);
    let to_hsl = HslColor::new(60., 0.75, 0.75, 0.25);
    assert_eq!(from_hsl.lerp(&to_hsl, 1.5), HslColor::new(120., 1., 1., 0.));
    assert_eq!(
        from_hsl.lerp(&to_hsl, -0.5),
        HslColor::new(240., 0., 0., 1.)
    );

    assert_eq!(
        HsvColor::new(350., 1., 1., 1.)
            .lerp(&HsvColor::new(10., 1., 1., 1.), 0.5)
            .to_color(),
        Color::rgba(255, 0, 0, 255),
    );
    assert_eq!(
        HslColor::new(350., 1., 0.5, 1.)
            .lerp(&HslColor::new(10., 1., 0.5, 1.), 0.5)
            .to_color(),
        Color::rgba(255, 0, 0, 255),
    );
}
