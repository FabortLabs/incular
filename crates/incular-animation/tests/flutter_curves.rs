//! Named curves against Flutter 3.41.7 `animation/curves.dart`.
//!
//! Every named cubic curve is `Cubic(a, b, c, d)` with the control points
//! below, evaluated by bisection to within 0.001 (`Cubic.transformInternal`).
//! The table is generated from the framework source, not from memory.

use incular_animation::Curve;

const CUBICS: &[(Curve, [f32; 4], &str)] = &[
    (Curve::Ease, [0.25, 0.1, 0.25, 1.0], "Curves.ease"),
    (Curve::EaseIn, [0.42, 0.0, 1.0, 1.0], "Curves.easeIn"),
    (Curve::EaseOut, [0.0, 0.0, 0.58, 1.0], "Curves.easeOut"),
    (Curve::EaseInOut, [0.42, 0.0, 0.58, 1.0], "Curves.easeInOut"),
    (
        Curve::EaseInSine,
        [0.47, 0.0, 0.745, 0.715],
        "Curves.easeInSine",
    ),
    (
        Curve::EaseOutSine,
        [0.39, 0.575, 0.565, 1.0],
        "Curves.easeOutSine",
    ),
    (
        Curve::EaseInOutSine,
        [0.445, 0.05, 0.55, 0.95],
        "Curves.easeInOutSine",
    ),
    (
        Curve::EaseInQuad,
        [0.55, 0.085, 0.68, 0.53],
        "Curves.easeInQuad",
    ),
    (
        Curve::EaseOutQuad,
        [0.25, 0.46, 0.45, 0.94],
        "Curves.easeOutQuad",
    ),
    (
        Curve::EaseInOutQuad,
        [0.455, 0.03, 0.515, 0.955],
        "Curves.easeInOutQuad",
    ),
    (
        Curve::EaseInCubic,
        [0.55, 0.055, 0.675, 0.19],
        "Curves.easeInCubic",
    ),
    (
        Curve::EaseOutCubic,
        [0.215, 0.61, 0.355, 1.0],
        "Curves.easeOutCubic",
    ),
    (
        Curve::EaseInOutCubic,
        [0.645, 0.045, 0.355, 1.0],
        "Curves.easeInOutCubic",
    ),
    (
        Curve::EaseInQuart,
        [0.895, 0.03, 0.685, 0.22],
        "Curves.easeInQuart",
    ),
    (
        Curve::EaseOutQuart,
        [0.165, 0.84, 0.44, 1.0],
        "Curves.easeOutQuart",
    ),
    (
        Curve::EaseInOutQuart,
        [0.77, 0.0, 0.175, 1.0],
        "Curves.easeInOutQuart",
    ),
    (
        Curve::EaseInQuint,
        [0.755, 0.05, 0.855, 0.06],
        "Curves.easeInQuint",
    ),
    (
        Curve::EaseOutQuint,
        [0.23, 1.0, 0.32, 1.0],
        "Curves.easeOutQuint",
    ),
    (
        Curve::EaseInOutQuint,
        [0.86, 0.0, 0.07, 1.0],
        "Curves.easeInOutQuint",
    ),
    (
        Curve::EaseInExpo,
        [0.95, 0.05, 0.795, 0.035],
        "Curves.easeInExpo",
    ),
    (
        Curve::EaseOutExpo,
        [0.19, 1.0, 0.22, 1.0],
        "Curves.easeOutExpo",
    ),
    (
        Curve::EaseInOutExpo,
        [1.0, 0.0, 0.0, 1.0],
        "Curves.easeInOutExpo",
    ),
    (
        Curve::EaseInCirc,
        [0.6, 0.04, 0.98, 0.335],
        "Curves.easeInCirc",
    ),
    (
        Curve::EaseOutCirc,
        [0.075, 0.82, 0.165, 1.0],
        "Curves.easeOutCirc",
    ),
    (
        Curve::EaseInOutCirc,
        [0.785, 0.135, 0.15, 0.86],
        "Curves.easeInOutCirc",
    ),
    (
        Curve::EaseInBack,
        [0.6, -0.28, 0.735, 0.045],
        "Curves.easeInBack",
    ),
    (
        Curve::EaseOutBack,
        [0.175, 0.885, 0.32, 1.275],
        "Curves.easeOutBack",
    ),
    (
        Curve::EaseInOutBack,
        [0.68, -0.55, 0.265, 1.55],
        "Curves.easeInOutBack",
    ),
    (
        Curve::FastOutSlowIn,
        [0.4, 0.0, 0.2, 1.0],
        "Curves.fastOutSlowIn",
    ),
];

#[test]
fn named_cubic_curves_use_flutters_control_points() {
    for (curve, [x1, y1, x2, y2], name) in CUBICS {
        let reference = Curve::cubic_bezier(*x1, *y1, *x2, *y2);
        for step in 0..=100 {
            let t = step as f32 / 100.0;
            assert_eq!(
                curve.apply(t),
                reference.apply(t),
                "{name} must equal Cubic({x1}, {y1}, {x2}, {y2}) at t={t}"
            );
        }
    }
}

#[test]
fn cubic_curves_map_endpoints_exactly_and_may_overshoot() {
    for (curve, _, name) in CUBICS {
        assert_eq!(curve.apply(0.0), 0.0, "{name} starts at 0");
        assert_eq!(curve.apply(1.0), 1.0, "{name} ends at 1");
    }
    // Curves.easeInBack dips below 0 and Curves.easeOutBack rises above 1:
    // Flutter's Curve.transform does not clamp its output.
    assert!(Curve::EaseInBack.apply(0.2) < 0.0);
    assert!(Curve::EaseOutBack.apply(0.8) > 1.0);
}

#[test]
fn elastic_curves_follow_flutter_period_point_four() {
    // ElasticOutCurve: 2^(-10 t) * sin((t - s) * 2pi / period) + 1, s = period / 4.
    let period = 0.4_f32;
    let s = period / 4.0;
    for t in [0.1_f32, 0.3, 0.5, 0.7, 0.9] {
        let expected =
            (2.0_f32).powf(-10.0 * t) * ((t - s) * std::f32::consts::TAU / period).sin() + 1.0;
        let actual = Curve::EaseOutElastic.apply(t);
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "Curves.elasticOut({t}): {actual} vs {expected}"
        );
    }
    assert!(
        Curve::EaseOutElastic.apply(0.2) > 1.0,
        "elasticOut overshoots"
    );
}
