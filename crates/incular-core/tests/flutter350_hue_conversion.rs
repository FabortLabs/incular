use incular_core::{Color, HslColor, HsvColor};

fn argb(bits: u32) -> Color {
    let [a, r, g, b] = bits.to_be_bytes();
    Color::rgba(r, g, b, a)
}

fn require_close(original: [f64; 4], decoded: [f64; 4]) {
    // The upstream matcher scales hue by 360 and takes the largest
    // component error, not a wrapped angular or Euclidean distance.
    for (a, b) in original.into_iter().zip(decoded) {
        assert!((a - b).abs() <= 0.01, "{original:?} vs {decoded:?}");
    }
}

#[test]
fn hsv_hue_sweep_matches_every_argb_pixel_and_its_reverse_conversion() {
    let expected = [
        0xffff0000, 0xffff9900, 0xffccff00, 0xff33ff00, 0xff00ff66, 0xff00ffff, 0xff0066ff,
        0xff3300ff, 0xffcc00ff, 0xffff0099, 0xffff0000,
    ];
    for (i, packed) in expected.into_iter().enumerate() {
        let hue = 36. * i as f64;
        let model = HsvColor::new(hue, 1., 1., 1.);
        let encoded = model.to_color();
        assert_eq!(encoded, argb(packed), "{hue} degrees");
        assert_eq!(Color::from_ahsv(1., hue, 1., 1.), argb(packed));
        if hue != 360. {
            let decoded = HsvColor::from_color(encoded);
            require_close(
                [hue / 360., 1., 1., 1.],
                [
                    decoded.hue / 360.,
                    decoded.saturation,
                    decoded.value,
                    decoded.alpha,
                ],
            );
        }
    }
}

#[test]
fn hsl_hue_sweep_matches_every_argb_pixel_and_its_reverse_conversion() {
    let expected = [
        0xffbf4040, 0xffbf8c40, 0xffa6bf40, 0xff59bf40, 0xff40bf73, 0xff40bfbf, 0xff4073bf,
        0xff5940bf, 0xffa640bf, 0xffbf408c, 0xffbf4040,
    ];
    for (i, packed) in expected.into_iter().enumerate() {
        let hue = 36. * i as f64;
        let model = HslColor::new(hue, 0.5, 0.5, 1.);
        let encoded = model.to_color();
        assert_eq!(encoded, argb(packed), "{hue} degrees");
        assert_eq!(Color::from_ahsl(1., hue, 0.5, 0.5), argb(packed));
        if hue != 360. {
            let decoded = HslColor::from_color(encoded);
            require_close(
                [hue / 360., 0.5, 0.5, 1.],
                [
                    decoded.hue / 360.,
                    decoded.saturation,
                    decoded.lightness,
                    decoded.alpha,
                ],
            );
        }
    }
}

#[test]
fn pink_decodes_to_saturated_hsl_near_330_degrees() {
    let color = HslColor::from_color(Color::rgba(255, 51, 152, 255));
    assert_eq!(color.alpha, 1.);
    assert_eq!(color.saturation, 1.);
    assert!((color.hue - 330.).abs() <= 0.3, "{color:?}");
    assert!((color.lightness - 0.6).abs() <= 0.01);
}

#[test]
fn white_black_and_gray_have_no_saturation_and_zero_hue() {
    let white = HslColor::from_color(argb(0xffffffff));
    assert_eq!(white, HslColor::new(0., 0., 1., 1.));
    let black = HslColor::from_color(argb(0xff000000));
    assert_eq!(black, HslColor::new(0., 0., 0., 1.));
    let gray = HslColor::from_color(argb(0xff808080));
    assert_eq!((gray.hue, gray.saturation, gray.alpha), (0., 0., 1.));
    assert!((gray.lightness - 0.5).abs() <= 0.01);
}

#[test]
fn precise_hsv_saturation_and_value_sweeps_match_the_pinned_bytes() {
    let channels = [
        (
            false,
            [
                0xffffffff, 0xffffe6e6, 0xffffcccc, 0xffffb3b3, 0xffff9999, 0xffff8080, 0xffff6666,
                0xffff4d4d, 0xffff3333, 0xffff1a1a, 0xffff0000,
            ],
        ),
        (
            true,
            [
                0xff000000, 0xff1a0000, 0xff330000, 0xff4d0000, 0xff660000, 0xff800000, 0xff990000,
                0xffb30000, 0xffcc0000, 0xffe50000, 0xffff0000,
            ],
        ),
    ];
    for (varies_value, expected) in channels {
        let mut t = 0f64;
        for (sample, packed) in expected.into_iter().enumerate() {
            let (s, v) = if varies_value { (1., t) } else { (t, 1.) };
            let encoded = Color::from_ahsv(1., 0., s, v);
            assert_eq!(encoded, argb(packed), "sample {sample} at {t}");
            assert_eq!(HsvColor::new(0., s, v, 1.).to_color(), encoded);
            if !varies_value || (0.01..=0.99).contains(&t) {
                let back = HsvColor::from_color(encoded);
                require_close(
                    [0., s, v, 1.],
                    [back.hue / 360., back.saturation, back.value, back.alpha],
                );
            }
            t += 0.1;
        }
    }
}

#[test]
fn precise_hsl_saturation_and_lightness_sweeps_match_the_pinned_bytes() {
    let channels = [
        (
            false,
            [
                0xff808080, 0xff8c7373, 0xff996666, 0xffa65959, 0xffb34d4d, 0xffbf4040, 0xffcc3333,
                0xffd92626, 0xffe51a1a, 0xfff20d0d, 0xffff0000,
            ],
        ),
        (
            true,
            [
                0xff000000, 0xff260d0d, 0xff4d1a1a, 0xff732626, 0xff993333, 0xffbf4040, 0xffcc6666,
                0xffd98c8c, 0xffe6b3b3, 0xfff2d9d9, 0xffffffff,
            ],
        ),
    ];
    for (varies_lightness, expected) in channels {
        let mut t = 0f64;
        for (sample, packed) in expected.into_iter().enumerate() {
            let (s, l) = if varies_lightness { (0.5, t) } else { (t, 0.5) };
            let encoded = Color::from_ahsl(1., 0., s, l);
            assert_eq!(encoded, argb(packed), "sample {sample} at {t}");
            assert_eq!(HslColor::new(0., s, l, 1.).to_color(), encoded);
            if !varies_lightness || (0.01..=0.99).contains(&t) {
                let back = HslColor::from_color(encoded);
                require_close(
                    [0., s, l, 1.],
                    [back.hue / 360., back.saturation, back.lightness, back.alpha],
                );
            }
            t += 0.1;
        }
    }
}
