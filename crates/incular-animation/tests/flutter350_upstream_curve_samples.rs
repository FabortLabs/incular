//! Public curve assertions from Flutter 3.50.0 animation/curves_test.dart.

use incular_animation::{CatmullRomCurve, Curve};
use incular_core::Offset;

#[test]
fn named_curves_stay_below_the_sampled_maximum_slope() {
    // Upstream line 45 intentionally excludes the expo midpoint.
    for curve in [
        Curve::Linear,
        Curve::Decelerate,
        Curve::FastOutSlowIn,
        Curve::SlowMiddle,
        Curve::EaseInBounce,
        Curve::EaseOutBounce,
        Curve::EaseInOutBounce,
        Curve::EaseOutElastic,
        Curve::EaseInOutElastic,
        Curve::Ease,
        Curve::EaseIn,
        Curve::EaseInSine,
        Curve::EaseInQuad,
        Curve::EaseInCubic,
        Curve::EaseInQuart,
        Curve::EaseInQuint,
        Curve::EaseInExpo,
        Curve::EaseInCirc,
        Curve::EaseOut,
        Curve::EaseOutSine,
        Curve::EaseOutQuad,
        Curve::EaseOutCubic,
        Curve::EaseInOutCubicEmphasized,
        Curve::EaseOutQuart,
        Curve::EaseOutQuint,
        Curve::EaseOutExpo,
        Curve::EaseOutCirc,
        Curve::EaseInOut,
        Curve::EaseInOutSine,
        Curve::EaseInOutQuad,
        Curve::EaseInOutCubic,
        Curve::EaseInOutQuart,
        Curve::EaseInOutQuint,
        Curve::EaseInOutCirc,
    ] {
        let step = 0.005_f64;
        let mut t = 0.0_f64;
        while t < 1.0 - step {
            let change =
                f64::from(curve.apply(t as f32)) - f64::from(curve.apply((t + step) as f32));
            assert!(change.abs() < 20.0 * step, "{curve:?} at {t}: {change}");
            t += step;
        }
    }
}

#[test]
fn bounce_variants_stay_inside_the_unit_interval() {
    // Upstream line 102 tests all eleven deciles, including both endpoints.
    for curve in [
        Curve::EaseInBounce,
        Curve::EaseOutBounce,
        Curve::EaseInOutBounce,
    ] {
        for step in 0..=10 {
            let t = step as f32 / 10.0;
            let value = curve.apply(t);
            assert!((0.0..=1.0).contains(&value), "{curve:?} at {t}: {value}");
        }
    }
}

#[test]
fn three_point_cubic_preserves_its_explicit_midpoint() {
    // Upstream line 172.
    let curve = Curve::ThreePointCubic {
        a1: (0.05, 0.0),
        b1: (0.133_333, 0.06),
        midpoint: (0.166_666, 0.4),
        a2: (0.208_333, 0.82),
        b2: (0.25, 1.0),
    };
    assert_eq!(curve.apply(0.166_666), 0.4);
}

#[test]
fn transforms_preserve_both_unit_endpoints() {
    // Upstream line 231.
    const EXPO: Curve = Curve::EaseInOutExpo;
    for curve in [
        Curve::SawTooth(2),
        Curve::interval(0.0, 1.0, &Curve::Linear),
        Curve::split(0.5, &Curve::Linear, &Curve::EaseOutCubic),
        Curve::Threshold(0.5),
        Curve::ElasticIn(0.4),
        Curve::ElasticOut(0.4),
        Curve::ElasticInOut(0.4),
        Curve::Linear,
        EXPO,
        Curve::EaseInOutCubicEmphasized,
        Curve::Flipped(std::sync::Arc::new(EXPO.clone())),
        Curve::Decelerate,
        Curve::EaseInBounce,
        Curve::EaseOutBounce,
        Curve::EaseInOutBounce,
    ] {
        assert_eq!(curve.apply(0.0), 0.0, "{curve:?}");
        assert_eq!(curve.apply(1.0), 1.0, "{curve:?}");
    }
}

#[test]
fn catmull_rom_timing_curve_matches_the_reference_samples() {
    // Upstream line 471 observes transform values, not when samples are
    // computed. Incular prepares the samples at construction.
    let curve = CatmullRomCurve::new(
        &[
            Offset::new(0.2, 0.25),
            Offset::new(0.33, 0.25),
            Offset::new(0.5, 1.0),
            Offset::new(0.8, 0.75),
        ],
        0.0,
    );
    for (t, expected) in [
        (0.0, 0.0),
        (0.01, 0.012_874_734_350_170_863),
        (0.2, 0.249_896_460_452_775_42),
        (0.33, 0.250_037_698_527_661),
        (0.5, 0.999_905_732_323_593_9),
        (0.6, 0.935_729_496_453_662_1),
        (0.8, 0.750_042_340_237_803_4),
        (1.0, 1.0),
    ] {
        let actual = f64::from(curve.transform(t));
        assert!(
            (actual - expected).abs() <= 1e-6,
            "t={t}: expected {expected} ± 1e-6, got {actual}",
        );
    }
}

#[test]
fn a_raw_cubic_clamps_overrange_samples_and_rejects_nan() {
    use incular_animation::CubicTransformError;

    let curve = Curve::EaseOut;
    assert_eq!(curve.try_cubic_transform_internal(2.), Ok(1.));
    assert_eq!(curve.try_cubic_transform_internal(-1.), Ok(0.));
    assert_eq!(curve.try_cubic_transform_internal(-0.).unwrap().to_bits(), 0);
    assert_eq!(curve.try_cubic_transform_internal(f32::INFINITY), Ok(1.));
    assert_eq!(curve.try_cubic_transform_internal(f32::NEG_INFINITY), Ok(0.));
    assert_eq!(
        curve.try_cubic_transform_internal(f32::NAN),
        Err(CubicTransformError::NaNProgress)
    );
    for progress in [0.25, 0.5, 0.75] {
        assert_eq!(curve.try_cubic_transform_internal(progress), Ok(curve.transform(progress)));
    }
    assert_eq!(
        Curve::Linear.try_cubic_transform_internal(0.5),
        Err(CubicTransformError::NotCubic)
    );
}

#[test]
#[cfg(debug_assertions)]
fn checked_curve_transform_rejects_invalid_progress_for_every_pinned_curve() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let curves = [
        Curve::SawTooth(2),
        Curve::interval(0., 1., &Curve::Linear),
        Curve::split(0., &Curve::Linear, &Curve::Linear),
        Curve::Threshold(0.5),
        Curve::ElasticIn(0.4),
        Curve::ElasticOut(0.4),
        Curve::cubic_bezier(0.42, 0., 0.58, 1.),
        Curve::EaseInOutCubicEmphasized,
        Curve::Decelerate,
        Curve::EaseInBounce,
        Curve::EaseOutBounce,
        Curve::EaseInOutBounce,
    ];
    for curve in curves {
        for invalid in [-0.0001, 1.0001] {
            assert!(catch_unwind(AssertUnwindSafe(|| curve.transform(invalid))).is_err(),
                "{curve:?} accepted {invalid}");
        }
        assert_eq!(curve.transform(0.), 0.);
        assert_eq!(curve.transform(1.), 1.);
        assert_eq!(curve.apply(-0.0001), 0.);
        assert_eq!(curve.apply(1.0001), 1.);
    }
}
