//! Public geometry assertions from Flutter 3.50.0 test/animation/curves_test.dart.

use incular_animation::{CatmullRomCurve, CatmullRomSpline, Curve, Curve2D};
use incular_core::Offset;

fn close_to(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= 1e-6,
        "{context}: expected {expected} ± 1e-6, got {actual}",
    );
}

#[test]
fn threshold_step_occurs_at_the_threshold() {
    // Upstream line 28.
    let step = Curve::Threshold(0.25);
    assert_eq!(
        [0.0, 0.24, 0.25, 0.26, 1.0].map(|t| step.apply(t)),
        [0.0, 0.0, 1.0, 1.0, 1.0],
    );
}

#[test]
fn split_interpolates_across_its_join() {
    // Upstream line 278. Flutter's default branches are linear and easeOutCubic.
    let curve = Curve::split(0.3, &Curve::Linear, &Curve::EaseOutCubic);
    for t in [0.0, 0.1, 0.25, 0.3, 1.0] {
        assert_eq!(curve.apply(t), t);
    }
    close_to(f64::from(curve.apply(0.5)), 0.760_461, "Split(0.5)");
    close_to(f64::from(curve.apply(0.75)), 0.962_055, "Split(0.75)");
}

#[test]
fn catmull_rom_spline_interpolates_with_explicit_handles() {
    // Upstream line 291; Incular prepares segments eagerly, which this
    // registration does not observe.
    let spline = CatmullRomSpline::with_handles(
        &[
            Offset::ZERO,
            Offset::new(0.01, 0.25),
            Offset::new(0.2, 0.25),
            Offset::new(0.33, 0.25),
            Offset::new(0.5, 1.0),
            Offset::new(0.66, 0.75),
            Offset::new(1.0, 1.0),
        ],
        0.0,
        Some(Offset::new(0.0, -0.3)),
        Some(Offset::new(1.3, 1.3)),
    );
    for (t, x, y) in [
        (0.0, 0.0, 0.0),
        (0.25, 0.096_694_5, 0.262_680_6),
        (0.5, 0.33, 0.25),
        (0.75, 0.570_260, 0.883_085),
        (1.0, 1.0, 1.0),
    ] {
        let actual = spline.transform(t);
        close_to(f64::from(actual.x), x, &format!("spline x({t})"));
        close_to(f64::from(actual.y), y, &format!("spline y({t})"));
    }
}

#[test]
fn catmull_rom_spline_rejects_invalid_inputs() {
    // Upstream line 318 rejects short inputs and invalid tension when
    // constructing, and non-finite inputs on deferred segment initialization.
    let zeros = [Offset::ZERO; 4];
    for count in 0..=3 {
        assert!(std::panic::catch_unwind(|| CatmullRomSpline::new(&zeros[..count], 0.0)).is_err());
    }
    for tension in [-1.0, 2.0] {
        assert!(std::panic::catch_unwind(|| CatmullRomSpline::new(&zeros, tension)).is_err());
    }

    // Incular initializes eagerly, so check for rejection at construction.
    // Do not feed non-finite coefficients to the adaptive sampler.
    let mut accepted = Vec::new();
    for (label, invalid) in [
        ("control point x", Offset::new(f32::INFINITY, 0.0)),
        ("control point y", Offset::new(0.0, f32::INFINITY)),
    ] {
        let mut points = zeros;
        points[0] = invalid;
        if CatmullRomSpline::try_new(&points, 0.0).is_ok() {
            accepted.push(label);
        }
    }
    let invalid = Offset::new(0.0, f32::INFINITY);
    for (label, start, end) in [
        ("start handle", Some(invalid), None),
        ("end handle", None, Some(invalid)),
    ] {
        if CatmullRomSpline::try_with_handles(&zeros, 0.0, start, end).is_ok() {
            accepted.push(label);
        }
    }
    assert!(
        accepted.is_empty(),
        "non-finite input accepted: {accepted:?}"
    );
}

#[test]
fn catmull_rom_curve_rejects_non_functional_control_points() {
    // Upstream line 511.
    let zeros = [Offset::ZERO; 2];
    let rejects_construction =
        |points: &[Offset]| std::panic::catch_unwind(|| CatmullRomCurve::new(points, 0.0)).is_err();
    for count in 0..=2 {
        assert!(rejects_construction(&zeros[..count]));
    }
    for points in [
        &[Offset::new(0.2, 0.25), Offset::new(0.01, 0.25)][..],
        &[Offset::new(0.2, 0.25), Offset::new(1.01, 0.25)],
        &[
            Offset::new(0.05, 0.50),
            Offset::new(0.50, 0.50),
            Offset::new(0.75, 0.75),
        ],
        &[
            Offset::new(0.25, 0.25),
            Offset::new(0.50, 0.50),
            Offset::new(0.95, 0.51),
        ],
        &[Offset::new(0.5, 0.05), Offset::new(0.5, 0.95)],
    ] {
        assert!(
            CatmullRomCurve::validate_control_points(points, 0.0).is_err(),
            "{points:?}"
        );
        assert!(rejects_construction(points), "{points:?}");
    }
}

#[test]
fn precomputed_spline_rejects_the_original_invalid_inputs() {
    use incular_animation::CatmullRomSplineError;

    let zeros = [Offset::ZERO; 4];
    let precompute_fails = |points: &[Offset], tension, start, end| {
        std::panic::catch_unwind(|| CatmullRomSpline::precompute(points, tension, start, end))
            .is_err()
    };
    for count in 0..=3 {
        assert!(precompute_fails(&zeros[..count], 0., None, None));
    }
    for tension in [-1., 2., f32::NAN] {
        assert!(precompute_fails(&zeros, tension, None, None));
    }
    for invalid in [
        Offset::new(f32::INFINITY, 0.),
        Offset::new(0., f32::INFINITY),
        Offset::new(0., f32::NAN),
    ] {
        let mut points = zeros;
        points[0] = invalid;
        assert!(precompute_fails(&points, 0., None, None));
        assert_eq!(
            CatmullRomSpline::try_new(&points, 0.),
            Err(CatmullRomSplineError::NonFiniteControlPoint(0))
        );
    }
    let invalid = Some(Offset::new(0., f32::INFINITY));
    assert!(precompute_fails(&zeros, 0., invalid, None));
    assert!(precompute_fails(&zeros, 0., None, invalid));
    assert_eq!(
        CatmullRomSpline::try_with_handles(&zeros, 0., invalid, None),
        Err(CatmullRomSplineError::NonFiniteStartHandle),
    );
    assert_eq!(
        CatmullRomSpline::try_with_handles(&zeros, 0., None, invalid),
        Err(CatmullRomSplineError::NonFiniteEndHandle),
    );
    assert_eq!(
        CatmullRomSpline::try_new(&zeros, 0.),
        Err(CatmullRomSplineError::DegenerateSegment(0)),
    );
}

#[test]
fn precomputed_spline_and_timing_curve_match_original_samples() {
    let spline = CatmullRomSpline::precompute(
        &[
            Offset::ZERO,
            Offset::new(0.01, 0.25),
            Offset::new(0.2, 0.25),
            Offset::new(0.33, 0.25),
            Offset::new(0.5, 1.),
            Offset::new(0.66, 0.75),
            Offset::new(1., 1.),
        ],
        0.,
        Some(Offset::new(0., -0.3)),
        Some(Offset::new(1.3, 1.3)),
    );
    for (t, x, y) in [
        (0., 0., 0.),
        (0.25, 0.096_694_5, 0.262_680_6),
        (0.5, 0.33, 0.25),
        (0.75, 0.570_260, 0.883_085),
        (1., 1., 1.),
    ] {
        let actual = spline.transform(t);
        close_to(f64::from(actual.x), x, "precomputed x");
        close_to(f64::from(actual.y), y, "precomputed y");
    }

    let timing = CatmullRomCurve::precompute(
        &[
            Offset::new(0.2, 0.25),
            Offset::new(0.33, 0.25),
            Offset::new(0.5, 1.0),
            Offset::new(0.8, 0.75),
        ],
        0.,
    );
    for (t, expected) in [
        (0., 0.),
        (0.01, 0.012_874_734_350_170_863),
        (0.2, 0.249_896_460_452_775_42),
        (0.33, 0.250_037_698_527_661),
        (0.5, 0.999_905_732_323_593_9),
        (0.6, 0.935_729_496_453_662_1),
        (0.8, 0.750_042_340_237_803_4),
        (1., 1.),
    ] {
        close_to(
            f64::from(timing.transform(t)),
            expected,
            "precomputed timing",
        );
    }
}

#[test]
fn precomputed_timing_curve_rejects_non_functional_or_non_finite_inputs() {
    let rejects = |points: &[Offset]| {
        std::panic::catch_unwind(|| CatmullRomCurve::precompute(points, 0.)).is_err()
    };
    let zero = Offset::ZERO;
    for points in [
        &[][..],
        &[zero],
        &[zero, zero],
        &[Offset::new(0.2, 0.25), Offset::new(0.01, 0.25)],
        &[Offset::new(0.2, 0.25), Offset::new(1.01, 0.25)],
        &[
            Offset::new(0.05, 0.50),
            Offset::new(0.50, 0.50),
            Offset::new(0.75, 0.75),
        ],
        &[
            Offset::new(0.25, 0.25),
            Offset::new(0.50, 0.50),
            Offset::new(0.95, 0.51),
        ],
        &[Offset::new(0.5, 0.05), Offset::new(0.5, 0.95)],
        &[Offset::new(0.25, f32::NAN), Offset::new(0.5, 0.6)],
    ] {
        assert!(rejects(points), "{points:?}");
    }
}

#[test]
fn adaptive_sampling_fails_fast_on_non_finite_curves_and_invalid_budgets() {
    struct BrokenCurve;
    impl Curve2D for BrokenCurve {
        fn transform_xy(&self, t: f64) -> (f64, f64) {
            (t, if t == 0. || t == 1. { t } else { f64::NAN })
        }
    }
    assert!(std::panic::catch_unwind(|| BrokenCurve.generate_samples(0., 1., 1e-10)).is_err());
    assert!(std::panic::catch_unwind(|| BrokenCurve.generate_samples(0., 1., 0.)).is_err());
    assert!(std::panic::catch_unwind(|| BrokenCurve.generate_samples(1., 0., 1e-10)).is_err());
}
