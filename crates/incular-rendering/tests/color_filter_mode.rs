//! `ColorFilter::mode` is Flutter's `ColorFilter.mode(color, mode)`
//! (`painting.dart`): the color is the blend source and each pixel the
//! destination. Its matrix must agree with the reference blend for every
//! mode it accepts, and refuse the modes a 4x5 matrix cannot express.
use incular_core::Color;
use incular_rendering::{
    BlendMode, ColorFilter, ColorFilterStage, EffectChain, blend_premultiplied,
};

fn premultiply(rgba: [f32; 4]) -> [f32; 4] {
    [
        rgba[0] * rgba[3],
        rgba[1] * rgba[3],
        rgba[2] * rgba[3],
        rgba[3],
    ]
}

#[test]
fn matrix_modes_match_the_reference_blend() {
    let color = Color::rgba(200, 40, 120, 153);
    let source = premultiply([200. / 255., 40. / 255., 120. / 255., 153. / 255.]);
    let destinations = [
        [0.2, 0.7, 0.4, 1.],
        [0.9, 0.1, 0.5, 0.5],
        [1., 1., 1., 0.25],
        [0., 0., 0., 1.],
    ];
    for mode in [
        BlendMode::Clear,
        BlendMode::Dst,
        BlendMode::Src,
        BlendMode::SrcIn,
        BlendMode::DstIn,
        BlendMode::SrcOut,
        BlendMode::DstOut,
        BlendMode::SrcAtop,
        BlendMode::Modulate,
    ] {
        let filter = ColorFilter::mode(color, mode).expect("matrix-expressible mode");
        for destination in destinations {
            let expected = blend_premultiplied(mode, source, premultiply(destination));
            let actual = premultiply(filter.apply(destination));
            for channel in 0..4 {
                assert!(
                    (expected[channel] - actual[channel]).abs() < 1e-4,
                    "{mode:?} over {destination:?}: expected {expected:?}, got {actual:?}"
                );
            }
        }
    }
}

#[test]
fn modes_without_a_matrix_form_are_refused() {
    for mode in [
        BlendMode::SrcOver,
        BlendMode::DstOver,
        BlendMode::DstAtop,
        BlendMode::Xor,
        BlendMode::Plus,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Difference,
    ] {
        assert!(
            ColorFilter::mode(Color::WHITE, mode).is_none(),
            "{mode:?} has no exact matrix form"
        );
    }
}

/// Flutter `ColorFilter.mode` for the modes no matrix expresses: the blend
/// runs as a per-pixel stage with the reference equations.
#[test]
fn blend_filters_execute_every_mode_through_a_stage() {
    let color = Color::rgba(200, 40, 90, 160);
    let source = incular_rendering::premultiplied_working_rgba(color);
    for mode in [
        BlendMode::SrcOver,
        BlendMode::DstOver,
        BlendMode::DstAtop,
        BlendMode::Xor,
        BlendMode::Plus,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Difference,
    ] {
        let filter = ColorFilter::blend(color, mode);
        assert_eq!(
            filter.stage(),
            ColorFilterStage::Blend { color, mode },
            "{mode:?}"
        );
        for destination in [[0.2, 0.6, 0.9, 1.0], [1.0, 1.0, 1.0, 0.5], [0.; 4]] {
            let expected = blend_premultiplied(mode, source, premultiply(destination));
            let actual = premultiply(filter.apply(destination));
            for channel in 0..4 {
                assert!(
                    (expected[channel] - actual[channel]).abs() < 1e-4,
                    "{mode:?} over {destination:?}: expected {expected:?}, got {actual:?}"
                );
            }
        }
    }
    // A mode with an exact matrix stays a matrix.
    assert_eq!(
        ColorFilter::blend(color, BlendMode::SrcIn).stage(),
        ColorFilterStage::None
    );
}

/// Flutter `ColorFilter.linearToSrgbGamma` / `srgbToLinearGamma`: the
/// IEC 61966-2-1 transfer functions on straight RGB, alpha unchanged.
#[test]
fn gamma_filters_apply_the_srgb_transfer_functions() {
    let encode = ColorFilter::linear_to_srgb_gamma();
    let decode = ColorFilter::srgb_to_linear_gamma();
    let encoded = encode.apply([0.5, 0.0, 1.0, 0.25]);
    assert!((encoded[0] - 0.735_356_7).abs() < 1e-5, "{encoded:?}");
    assert_eq!(encoded[1], 0.0);
    assert!(
        (encoded[2] - 1.0).abs() < 1e-6 && encoded[3] == 0.25,
        "{encoded:?}"
    );
    let decoded = decode.apply([0.5, 0.02, 1.0, 1.0]);
    assert!((decoded[0] - 0.214_041_14).abs() < 1e-5, "{decoded:?}");
    assert!((decoded[1] - 0.02 / 12.92).abs() < 1e-6, "{decoded:?}");
    let round_trip = decode.apply(encode.apply([0.3, 0.6, 0.9, 1.0]));
    for (channel, expected) in round_trip.iter().zip([0.3, 0.6, 0.9, 1.0]) {
        assert!((channel - expected).abs() < 1e-4, "{round_trip:?}");
    }
    assert!(!encode.is_identity());
}

/// A stage cannot run between two matrices, so such pairs do not fuse; a
/// stage-free filter after a staged one fuses behind the stage.
#[test]
fn staged_filters_fuse_only_when_order_is_kept() {
    let invert = ColorFilter::invert(1.0);
    let gamma = ColorFilter::linear_to_srgb_gamma();
    assert_eq!(invert.try_then(gamma), None);
    assert_eq!(ColorFilter::identity().try_then(gamma), Some(gamma));
    let fused = gamma.try_then(invert).expect("matrix after stage fuses");
    assert_eq!(fused.stage(), gamma.stage());
    let input = [0.4, 0.5, 0.6, 1.0];
    let expected = invert.apply(gamma.apply(input));
    let actual = fused.apply(input);
    for channel in 0..4 {
        assert!((expected[channel] - actual[channel]).abs() < 1e-5);
    }
    let chain = EffectChain::new()
        .color_filter(invert)
        .color_filter(gamma)
        .optimized();
    assert_eq!(chain.effects().len(), 2);
}

#[test]
fn nonseparable_modes_match_pinned_flutter_premultiplied_samples() {
    let src = Color::rgba(200, 40, 90, 160);
    let source = premultiply([200. / 255., 40. / 255., 90. / 255., 160. / 255.]);
    for (mode, opaque, translucent) in [
        (BlendMode::Hue, [148, 94, 142, 255], [137, 60, 99, 208]),
        (
            BlendMode::Saturation,
            [70, 121, 211, 255],
            [98, 73, 134, 208],
        ),
        (BlendMode::Color, [169, 84, 141, 255], [147, 55, 99, 208]),
        (
            BlendMode::Luminosity,
            [66, 106, 176, 255],
            [96, 66, 117, 208],
        ),
    ] {
        let filter = ColorFilter::blend(src, mode);
        for (alpha, expected) in [(1., opaque), (128. / 255., translucent)] {
            let dest = [80. / 255., 120. / 255., 190. / 255., alpha];
            let filtered = premultiply(filter.apply(dest));
            let blended = blend_premultiplied(mode, source, premultiply(dest));
            for candidate in [filtered, blended] {
                let rgba = candidate.map(|value| (value * 255.).round() as u8);
                assert!(
                    rgba.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                    "{mode:?} with destination alpha {alpha}: {rgba:?} != {expected:?}"
                );
            }
        }
        assert_eq!(blend_premultiplied(mode, source, [0.; 4]), source);
    }
}

#[test]
fn saturation_extrapolates_and_preserves_alpha() {
    let input = [64. / 255., 96. / 255., 128. / 255., 0.5];
    for (saturation, rgb) in [
        (-1., [119, 87, 55]),
        (0., [92, 92, 92]),
        (1., [64, 96, 128]),
        (2., [36, 100, 164]),
        (8., [0, 127, 255]),
    ] {
        let [r, g, b, a] = ColorFilter::saturation(saturation).apply(input);
        assert_eq!([r, g, b].map(|v| (v * 255.).round() as u8), rgb);
        assert_eq!(a, 0.5);
        assert_eq!(ColorFilter::saturation(saturation).apply([0.; 4]), [0.; 4]);
    }
    assert!(ColorFilter::saturation(1.).is_identity());
    assert_eq!(ColorFilter::saturation(0.), ColorFilter::grayscale(1.));
}
