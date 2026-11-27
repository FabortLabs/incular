use incular_core::{Color, HslColor, HsvColor};

fn argb(bits: u32) -> Color {
    let [a, r, g, b] = bits.to_be_bytes();
    Color::rgba(r, g, b, a)
}

fn require_close(original: [f32; 4], decoded: [f32; 4]) {
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
        let hue = 36. * i as f32;
        let model = HsvColor::new(hue, 1., 1., 1.);
        let encoded = model.to_color();
        assert_eq!(encoded, argb(packed), "{hue} degrees");
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
        let hue = 36. * i as f32;
        let model = HslColor::new(hue, 0.5, 0.5, 1.);
        let encoded = model.to_color();
        assert_eq!(encoded, argb(packed), "{hue} degrees");
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
