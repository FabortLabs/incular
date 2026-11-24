//! Interpolation assertions from Flutter 3.50.0 animation/tween_test.dart.

use incular_animation::{
    AnimatableExt, ColorTween, Curve, CurveTween, IntTween, ReverseTween, StepTween,
};
use incular_core::Color;

#[test]
fn int_tween_rounds_at_the_upstream_samples() {
    // Upstream line 145.
    let tween = IntTween::new(5, 9);
    assert_eq!(tween.lerp(0.5), 7);
    assert_eq!(tween.lerp(0.7), 8);
}

#[test]
fn reverse_tween_samples_its_integer_parent_in_reverse() {
    // Upstream line 191.
    let tween = ReverseTween(IntTween::new(5, 9));
    assert_eq!(tween.transform(0.5), 7);
    assert_eq!(tween.transform(0.7), 6);
}

#[test]
fn color_tween_matches_black_to_white_under_the_original_matcher() {
    // Upstream line 197 compares normalized sRGB channels using
    // isSameColorAs's inclusive threshold of 0.004, not byte identity.
    let tween = ColorTween::new(Color::BLACK, Color::WHITE);
    for (progress, expected_gray) in [(0.0, 0_u8), (0.5, 127), (0.7, 178), (1.0, 255)] {
        let color = tween.lerp(progress);
        for (channel, actual, expected) in [
            ("red", color.red, expected_gray),
            ("green", color.green, expected_gray),
            ("blue", color.blue, expected_gray),
            ("alpha", color.alpha, 255),
        ] {
            let error = (f64::from(actual) - f64::from(expected)).abs() / 255.0;
            assert!(
                error <= 0.004,
                "{channel} at {progress}: {actual} vs {expected}, error {error}",
            );
        }
    }
}

#[test]
fn step_tween_floors_instead_of_rounding() {
    // Upstream line 205.
    let tween = StepTween { begin: 5, end: 9 };
    assert_eq!(tween.transform(0.5), 7);
    assert_eq!(tween.transform(0.7), 7);
}

#[test]
fn curve_tween_preserves_endpoints_and_exact_ease_in_midpoint() {
    // Upstream line 211.
    let tween = CurveTween(Curve::EaseIn);
    assert_eq!(tween.transform(0.0), 0.0);
    assert_eq!(tween.transform(0.5), 0.316_406_25);
    assert_eq!(tween.transform(1.0), 1.0);
}
