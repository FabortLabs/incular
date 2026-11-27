use incular_core::{Color, HslColor, HsvColor};

fn check(start: f64, end: f64, expected: &[u32], mut evaluate: impl FnMut(f64) -> Color) {
    let mut factor = start;
    let mut actual = Vec::new();
    // Match Dart's repeated double additions, including which side of a byte
    // rounding tie a sample lands on. Multiplying an index by 0.1 changes that.
    while factor < end {
        actual.push(evaluate(factor).to_argb32());
        factor += 0.1;
    }
    assert_eq!(
        actual
            .iter()
            .map(|value| format!("{value:08x}"))
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|value| format!("{value:08x}"))
            .collect::<Vec<_>>()
    );
}

fn hsv(a: HsvColor, b: HsvColor, t: f64) -> Color {
    HsvColor::lerp_ref(Some(&a), Some(&b), t)
        .unwrap()
        .to_color()
}

fn hsl(a: HslColor, b: HslColor, t: f64) -> Color {
    HslColor::lerp_ref(Some(&a), Some(&b), t)
        .unwrap()
        .to_color()
}

#[test]
fn hsv_hue_interpolation_and_extrapolation_preserve_exact_bytes() {
    check(
        -0.5,
        1.5,
        &[
            0xff8000ff, 0xffcc00ff, 0xffff00e6, 0xffff0099, 0xffff004c, 0xffff0000, 0xffff4c00,
            0xffff9900, 0xffffe600, 0xffccff00, 0xff80ff00, 0xff33ff00, 0xff00ff19, 0xff00ff66,
            0xff00ffb2, 0xff00ffff, 0xff00b3ff, 0xff0066ff, 0xff001aff, 0xff3300ff,
        ],
        |t| {
            hsv(
                HsvColor::new(0., 1., 1., 1.),
                HsvColor::new(180., 1., 1., 1.),
                t,
            )
        },
    );
}

#[test]
fn hsv_saturation_interpolation_clamps_and_preserves_exact_bytes() {
    check(
        -0.1,
        1.1,
        &[
            0xffffffff, 0xffffffff, 0xffffe6e6, 0xffffcccc, 0xffffb3b3, 0xffff9999, 0xffff8080,
            0xffff6666, 0xffff4d4d, 0xffff3333, 0xffff1a1a, 0xffff0000, 0xffff0000,
        ],
        |t| {
            hsv(
                HsvColor::new(0., 0., 1., 1.),
                HsvColor::new(0., 1., 1., 1.),
                t,
            )
        },
    );
}

#[test]
fn hsv_value_interpolation_clamps_and_preserves_exact_bytes() {
    check(
        -0.1,
        1.1,
        &[
            0xff000000, 0xff000000, 0xff1a0000, 0xff330000, 0xff4d0000, 0xff660000, 0xff800000,
            0xff990000, 0xffb30000, 0xffcc0000, 0xffe50000, 0xffff0000, 0xffff0000,
        ],
        |t| {
            hsv(
                HsvColor::new(0., 1., 0., 1.),
                HsvColor::new(0., 1., 1., 1.),
                t,
            )
        },
    );
}

#[test]
fn hsl_hue_interpolation_and_extrapolation_preserve_exact_bytes() {
    check(
        -0.5,
        1.5,
        &[
            0xff8040bf, 0xffa640bf, 0xffbf40b3, 0xffbf408c, 0xffbf4066, 0xffbf4040, 0xffbf6640,
            0xffbf8c40, 0xffbfb340, 0xffa6bf40, 0xff80bf40, 0xff59bf40, 0xff40bf4c, 0xff40bf73,
            0xff40bf99, 0xff40bfbf, 0xff4099bf, 0xff4073bf, 0xff404dbf, 0xff5940bf,
        ],
        |t| {
            hsl(
                HslColor::new(0., 0.5, 0.5, 1.),
                HslColor::new(180., 0.5, 0.5, 1.),
                t,
            )
        },
    );
}

#[test]
fn hsl_saturation_interpolation_clamps_and_preserves_exact_bytes() {
    check(
        -0.1,
        1.1,
        &[
            0xff808080, 0xff808080, 0xff8c7373, 0xff996666, 0xffa65959, 0xffb34d4d, 0xffbf4040,
            0xffcc3333, 0xffd92626, 0xffe51a1a, 0xfff20d0d, 0xffff0000, 0xffff0000,
        ],
        |t| {
            hsl(
                HslColor::new(0., 0., 0.5, 1.),
                HslColor::new(0., 1., 0.5, 1.),
                t,
            )
        },
    );
}

#[test]
fn hsl_lightness_interpolation_clamps_and_preserves_exact_bytes() {
    check(
        -0.1,
        1.1,
        &[
            0xff000000, 0xff000000, 0xff260d0d, 0xff4d1a1a, 0xff732626, 0xff993333, 0xffbf4040,
            0xffcc6666, 0xffd98c8c, 0xffe6b3b3, 0xfff2d9d9, 0xffffffff, 0xffffffff,
        ],
        |t| {
            hsl(
                HslColor::new(0., 0.5, 0., 1.),
                HslColor::new(0., 0.5, 1., 1.),
                t,
            )
        },
    );
}
