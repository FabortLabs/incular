//! Backfills finding A01 and selected pinned animation_style_test.dart cases.
use incular_animation::{AnimationStyle, Curve};
use std::time::Duration;

const ONE: Duration = Duration::from_secs(1);
const TWO: Duration = Duration::from_secs(2);
const A: AnimationStyle = AnimationStyle::new()
    .curve(Curve::Ease)
    .duration(ONE)
    .reverse_curve(Curve::Ease)
    .reverse_duration(ONE);
const B: AnimationStyle = AnimationStyle::new()
    .curve(Curve::Linear)
    .duration(TWO)
    .reverse_curve(Curve::Linear)
    .reverse_duration(TWO);

#[test]
fn empty_and_all_fields_override_controls() {
    assert_eq!(
        AnimationStyle::default().merge(AnimationStyle::new()),
        AnimationStyle::new()
    );
    assert_eq!(A.merge(B), B);
}
#[test]
fn merge_keeps_unset_reverse_fields() {
    let patch = AnimationStyle::new().curve(Curve::Linear).duration(TWO);
    assert_eq!(
        A.merge(patch),
        patch.reverse_curve(Curve::Ease).reverse_duration(ONE)
    );
}
#[test]
fn lerp_preserves_empty_equal_and_explicit_endpoints() {
    assert_eq!(AnimationStyle::lerp(None, None, 0.), None);
    assert_eq!(AnimationStyle::lerp(Some(A), Some(A), 0.5), Some(A));
    assert_eq!(AnimationStyle::lerp(Some(A), Some(B), 0.), Some(A));
    assert_eq!(AnimationStyle::lerp(Some(A), Some(B), 1.), Some(B));
}
#[test]
fn lerp_rounds_both_interpolated_durations_to_microseconds() {
    let actual = (1..=5)
        .map(|i| {
            let style = AnimationStyle::lerp(Some(A), Some(B), i as f32 / 6.).unwrap();
            [
                style.duration.unwrap().as_micros(),
                style.reverse_duration.unwrap().as_micros(),
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        [1_166_667, 1_333_333, 1_500_000, 1_666_667, 1_833_333].map(|v| [v, v])
    );
}
#[test]
fn lerp_blends_forward_and_reverse_curve_outputs() {
    let mut failures = Vec::new();
    for k in 1..=5 {
        let blend = k as f32 / 6.;
        let style = AnimationStyle::lerp(Some(A), Some(B), blend).unwrap();
        for i in 1..=5 {
            let t = i as f32 / 6.;
            let expected = Curve::Ease.apply(t) * (1. - blend) + Curve::Linear.apply(t) * blend;
            let actual = [
                style.curve.unwrap().apply(t),
                style.reverse_curve.unwrap().apply(t),
            ];
            if actual
                .into_iter()
                .any(|sample| (sample - expected).abs() > 1e-6)
            {
                failures.push((blend, t, actual, expected));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "style blend, time, actual, expected: {failures:?}"
    );
}
#[test]
fn null_duration_interpolates_as_zero_between_endpoints() {
    let a = AnimationStyle::new().duration(TWO);
    let b = AnimationStyle::new();
    let actual =
        [0., 0.25, 0.5, 1.].map(|t| AnimationStyle::lerp(Some(a), Some(b), t).unwrap().duration);
    assert_eq!(
        actual,
        [
            Some(TWO),
            Some(Duration::from_millis(1500)),
            Some(ONE),
            None
        ]
    );
}

#[test]
fn durations_round_independently_and_preserve_nullable_style_endpoints() {
    let first = AnimationStyle::new()
        .duration(Duration::from_micros(1))
        .reverse_duration(Duration::from_micros(3));
    let second = AnimationStyle::new().duration(Duration::from_micros(4));
    let midpoint = AnimationStyle::lerp(Some(first), Some(second), 0.5).unwrap();
    assert_eq!(midpoint.duration, Some(Duration::from_micros(3)));
    assert_eq!(midpoint.reverse_duration, Some(Duration::from_micros(2)));
    assert_eq!(
        AnimationStyle::lerp(None, Some(second), 0.),
        Some(AnimationStyle::new())
    );
    assert_eq!(
        AnimationStyle::lerp(Some(first), None, 1.),
        Some(AnimationStyle::new())
    );
    assert_eq!(
        AnimationStyle::lerp(Some(first), Some(second), 1.),
        Some(second)
    );
    assert_eq!(
        AnimationStyle::lerp(Some(first), None, 1.5)
            .unwrap()
            .duration,
        Some(Duration::ZERO),
        "negative extrapolation cannot wrap through unsigned Rust Duration"
    );
}
