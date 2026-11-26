use incular_core::{ChangeImpact, Lerp};
use incular_text::{FontVariation as Axis, TextAlign, TextEngine, TextStyle, lerp_font_variations};
use std::borrow::Cow;
use std::sync::Arc;

#[test]
fn nullable_and_empty_axis_lists_keep_the_selected_endpoint() {
    let weight = [Axis::weight(100.0)];
    for a in [None, Some([].as_slice()), Some(weight.as_slice())] {
        for b in [None, Some([].as_slice()), Some(weight.as_slice())] {
            for t in [0.0, 0.25, 0.5, 1.0] {
                if a == Some(&weight) && b == Some(&weight) && t != 0.0 && t != 1.0 {
                    continue;
                }
                match (lerp_font_variations(a, b, t), if t < 0.5 { a } else { b }) {
                    (None, None) => {}
                    (Some(Cow::Borrowed(got)), Some(want)) => {
                        assert!(std::ptr::eq(got, want));
                    }
                    other => panic!("wrong endpoint at {t}: {other:?}"),
                }
            }
        }
    }
}

#[test]
fn font_variations_match_by_axis_while_preserving_the_positional_prefix() {
    let [w100, w120, w150, w200, w300, w1000] =
        [100., 120., 150., 200., 300., 1000.].map(Axis::weight);
    let [sn80, sn40, s0, sp40, sp80] = [-80., -40., 0., 40., 80.].map(Axis::slant);
    let cases: &[(&[Axis], &[Axis], f32, &[Axis])] = &[
        (&[w100], &[w200], 0.0, &[w100]),
        (&[w100], &[w200], 0.2, &[w120]),
        (&[w100], &[w200], 0.5, &[w150]),
        (&[w100], &[w200], 2.0, &[w300]),
        (&[w100, w1000], &[w300], 0.0, &[w100, w1000]),
        (&[w100, w1000], &[w300], 0.5, &[w200]),
        (&[w100, w1000], &[w300], 1.0, &[w300]),
        (&[w100, w1000], &[], 0.5, &[]),
        (&[w100, sn80], &[w300, sp80], 0.5, &[w200, s0]),
        (&[sn80, w100], &[w300, sp80], 0.0, &[sn80, w100]),
        (&[sn80, w100], &[w300, sp80], 0.5, &[s0, w200]),
        (&[sn80, w100], &[w300, sp80], 1.0, &[w300, sp80]),
        (&[sn80, w100, sp80], &[w300, sp80], 0.5, &[sp80, w200]),
        (&[sn80, w100], &[w300], 0.5, &[w200]),
        (&[sn80], &[w300], 0.0, &[sn80]),
        (&[sn80], &[w300], 0.1, &[sn80]),
        (&[sn80], &[w300], 0.5, &[w300]),
        (&[sn80], &[w300], 0.9, &[w300]),
        (&[sn80], &[w300], 1.0, &[w300]),
        (
            &[sn40, s0, w100],
            &[sp40, w300, sp80],
            0.5,
            &[s0, sp40, w200],
        ),
    ];
    for &(a, b, t, expected) in cases {
        assert_eq!(
            lerp_font_variations(Some(a), Some(b), t).as_deref(),
            Some(expected),
            "{a:?}, {b:?}, {t}"
        );
    }
}

#[test]
fn signed_axis_bounds_and_mismatched_tags_follow_flutter() {
    let a = Axis::new(*b"TEST", 0.0);
    let b = Axis::new(*b"TEST", 16_384.0);
    assert_eq!(a.lerp(&b, -3.0).value, -32_768.0);
    assert_eq!(
        a.lerp(&b, 3.0).value,
        f32::from_bits(32_768.0_f32.to_bits() - 1)
    );
    let c = Axis::slant(-20.0);
    assert_eq!(a.lerp(&c, 0.499), a);
    assert_eq!(a.lerp(&c, 0.5), c);
}

#[test]
fn text_style_tweens_raw_and_typed_axis_values_without_rounding() {
    let a = TextStyle::default().font_variations("'wght' 100.03125");
    let b = TextStyle::default().font_variation_settings(&[Axis::weight(300.09375)]);
    assert_eq!(b.font_variations.as_deref(), Some("\"wght\" 300.09375"));
    assert!(Arc::ptr_eq(
        a.lerp(&b, 0.0).font_variations.as_ref().unwrap(),
        a.font_variations.as_ref().unwrap()
    ));
    assert_eq!(
        a.lerp(&b, 0.499).change_impact(&a.lerp(&b, 0.5)),
        ChangeImpact::Layout
    );
    assert_eq!(
        a.lerp(&b, 0.5).font_variations.as_deref(),
        Some("\"wght\" 200.0625")
    );
    assert_eq!(
        a.lerp(&b, 2.0).font_variations.as_deref(),
        Some("\"wght\" 500.15625")
    );
    let unsupported = TextStyle::default().font_variations("normal");
    for (t, expected) in [(0.49, &unsupported), (0.5, &b)] {
        assert_eq!(
            unsupported.lerp(&b, t).font_variations,
            expected.font_variations
        );
    }
    let empty = TextStyle::default().font_variation_settings(&[]);
    assert_eq!(empty.lerp(&b, 0.49).font_variations.as_deref(), Some(""));
    assert_eq!(empty.lerp(&b, 0.5).font_variations, b.font_variations);
}

#[test]
#[ignore = "requires the pinned Flutter SDK font fixture"]
fn tween_updates_glyph_layout_at_real_intermediate_weights() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let font = repo
        .join(".tmp/flutter/engine/src/flutter/testing/resources/RobotoSlab-VariableFont_wght.ttf");
    let mut engine = TextEngine::default();
    assert!(engine.register_font_bytes_as("ParityVariable", std::fs::read(font).unwrap()));
    let font = |value| {
        TextStyle::default()
            .font_family("ParityVariable")
            .font_size(48.0)
            .font_variation_settings(&[Axis::weight(value)])
    };
    let a = font(100.);
    let b = font(900.);
    let mut widths = Vec::new();
    for (t, w) in [
        (0., 100.),
        (0.25, 300.),
        (0.499, 499.2),
        (0.5, 500.),
        (0.75, 700.),
        (1., 900.),
    ] {
        let actual = engine.layout(
            "Flutter variation WWW",
            &a.lerp(&b, t),
            None,
            TextAlign::Left,
        );
        let direct = engine.layout("Flutter variation WWW", &font(w), None, TextAlign::Left);
        assert_eq!(actual.lines[0].width, direct.lines[0].width, "weight {w}");
        widths.push(actual.lines[0].width);
    }
    assert!(
        widths[0] < widths[1]
            && widths[1] < widths[3]
            && widths[3] < widths[4]
            && widths[4] < widths[5],
        "{widths:?}"
    );
    assert!(
        (widths[3] - widths[2]).abs() < (widths[3] - widths[1]).abs(),
        "{widths:?}"
    );
}
