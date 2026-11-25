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

fn assert_one_line_debug(value: impl std::fmt::Debug) {
    let text = format!("{value:?}");
    assert!(!text.is_empty() && !text.contains('\n'), "{text:?}");
}

#[test]
fn chained_tween_maps_the_controllers_initial_progress() {
    use incular_animation::{AnimationController, TweenValue};
    use std::time::Duration;

    let outer = TweenValue::new(0.30_f64, 0.50);
    assert_one_line_debug(outer);
    let chain = outer.chain(TweenValue::new(0.50_f32, 1.0));
    let controller = AnimationController::new(Duration::from_millis(100));
    assert_eq!(chain.transform(controller.value()), 0.40);
    assert_one_line_debug(chain);
}

#[test]
fn animated_and_driven_tweens_observe_the_controller_after_attachment() {
    use incular_animation::{AnimationController, DoubleAnimation, TweenValue};
    use std::time::Duration;

    let controller = AnimationController::new(Duration::from_millis(100));
    let tween = TweenValue::new(0.30_f64, 0.50);
    let from_tween = tween.animate(&controller);
    let from_controller = DoubleAnimation::from(&controller).drive(tween);
    assert_eq!((from_tween.value(), from_controller.value()), (0.30, 0.30));
    controller.set_value(0.50);
    assert_eq!((from_tween.value(), from_controller.value()), (0.40, 0.40));
}

#[test]
fn size_tween_interpolates_each_axis_and_has_one_line_diagnostics() {
    use incular_animation::SizeTween;
    use incular_core::Size;

    let tween = SizeTween::new(Size::ZERO, Size::new(20., 30.));
    assert_eq!(tween.lerp(0.5), Size::new(10., 15.));
    assert_one_line_debug(tween);
}

#[test]
fn rect_tween_preserves_rectangle_midpoint_and_one_line_diagnostics() {
    use incular_animation::RectTween;
    use incular_core::{Lerp, Offset, Rect, Size};

    let rect =
        |x, y, width, height| Rect::from_origin_size(Offset::new(x, y), Size::new(width, height));
    let a = rect(5., 3., 7., 11.);
    let b = rect(8., 12., 14., 18.);
    let tween = RectTween::new(a, b);
    assert_eq!(tween.transform(0.5), a.lerp(&b, 0.5));
    assert_eq!(tween.transform(0.5), rect(6.5, 7.5, 10.5, 14.5));
    assert_one_line_debug(tween);
}

#[test]
fn constant_tween_has_the_same_value_at_both_ends_and_throughout() {
    use incular_animation::ConstantTween;
    let tween = ConstantTween(100.0_f64);
    assert_eq!((*tween.begin(), *tween.end()), (100., 100.));
    for t in [0., 0.5, 1.] {
        assert_eq!(tween.lerp(t), 100.);
    }
}

#[test]
fn projected_matrix_tween_compares_xy_translation_scale_and_z_rotation() {
    use incular_animation::Matrix4Tween;
    use incular_core::Transform;

    let identity = Transform::IDENTITY;
    let end = Transform::from_coefficients([0.5, 0., 0., 1., 6., -8.]);
    let tween = Matrix4Tween::new(identity, end);
    assert_eq!(tween.transform(0.0), identity);
    assert_eq!(tween.transform(1.0), end);
    assert_eq!(
        tween.transform(0.5).coefficients(),
        [0.75, 0., 0., 1., 3., -4.]
    );

    let end = Transform::rotation(1.);
    let rotation = Matrix4Tween::new(identity, end);
    assert_eq!(rotation.transform(0.0), identity);
    assert_eq!(rotation.transform(1.0), end);
    for (actual, expected) in rotation
        .transform(0.5)
        .coefficients()
        .into_iter()
        .zip(Transform::rotation(0.5).coefficients())
    {
        assert!((actual - expected).abs() <= 1e-10);
    }
}
