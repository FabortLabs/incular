//! Backfills finding A01 and selected pinned animation_style_test.dart cases.
use incular_animation::{AnimationStyle, Curve};
use std::time::Duration;

const ONE: Duration = Duration::from_secs(1);
const TWO: Duration = Duration::from_secs(2);
const A: AnimationStyle = AnimationStyle {
    curve: Some(Curve::Ease),
    duration: Some(ONE),
    reverse_curve: Some(Curve::Ease),
    reverse_duration: Some(ONE),
};
const B: AnimationStyle = AnimationStyle {
    curve: Some(Curve::Linear),
    duration: Some(TWO),
    reverse_curve: Some(Curve::Linear),
    reverse_duration: Some(TWO),
};

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
        A.merge(patch.clone()),
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
                style.curve.as_ref().unwrap().apply(t),
                style.reverse_curve.as_ref().unwrap().apply(t),
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
    let actual = [0., 0.25, 0.5, 1.].map(|t| {
        AnimationStyle::lerp(Some(a.clone()), Some(b.clone()), t)
            .unwrap()
            .duration
    });
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
    let midpoint = AnimationStyle::lerp(Some(first.clone()), Some(second.clone()), 0.5).unwrap();
    assert_eq!(midpoint.duration, Some(Duration::from_micros(3)));
    assert_eq!(midpoint.reverse_duration, Some(Duration::from_micros(2)));
    assert_eq!(
        AnimationStyle::lerp(None, Some(second.clone()), 0.),
        Some(AnimationStyle::new())
    );
    assert_eq!(
        AnimationStyle::lerp(Some(first.clone()), None, 1.),
        Some(AnimationStyle::new())
    );
    assert_eq!(
        AnimationStyle::lerp(Some(first.clone()), Some(second.clone()), 1.),
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

#[test]
fn curve_blends_keep_nested_outputs_and_release_owned_storage() {
    use std::sync::Arc;
    let a = AnimationStyle::new()
        .curve(Curve::Ease)
        .reverse_curve(Curve::EaseIn);
    let mid = AnimationStyle::lerp(Some(a), None, 0.25).unwrap();
    let Curve::Blend(entry) = mid.curve.as_ref().unwrap() else {
        panic!("an interior style owns its blended curve");
    };
    let allocation = Arc::downgrade(entry);
    let nested = AnimationStyle::lerp(
        Some(mid.clone()),
        Some(AnimationStyle::new().curve(Curve::EaseInBounce)),
        0.75,
    )
    .unwrap();
    for t in [0., 0.2, 0.5, 0.8, 1.] {
        let inner = Curve::Ease.apply(t) * 0.75 + Curve::Linear.apply(t) * 0.25;
        let expected = inner * 0.25 + Curve::EaseInBounce.apply(t) * 0.75;
        assert!((nested.curve.as_ref().unwrap().apply(t) - expected).abs() < 1e-6);
        let reverse = Curve::EaseIn.apply(t) * 0.75 + Curve::Linear.apply(t) * 0.25;
        let expected_reverse = reverse * 0.25 + Curve::Linear.apply(t) * 0.75;
        assert!((nested.reverse_curve.as_ref().unwrap().apply(t) - expected_reverse).abs() < 1e-6);
        let flipped = nested.curve.as_ref().unwrap().clone().flipped();
        assert!(
            (flipped.apply(t) - (1. - nested.curve.as_ref().unwrap().apply(1. - t))).abs() < 1e-5
        );
    }
    drop(mid);
    assert!(
        allocation.upgrade().is_some(),
        "nested curve retains its source"
    );
    drop(nested);
    assert!(
        allocation.upgrade().is_none(),
        "no global allocation retains the blend"
    );
}

#[test]
fn an_unset_animation_style_omits_all_normal_diagnostic_properties() {
    assert!(AnimationStyle::default().diagnostic_properties().next().is_none());
    let actual = AnimationStyle::NO_ANIMATION
        .diagnostic_properties()
        .map(|property| property.to_string())
        .collect::<Vec<_>>();
    assert_eq!(actual, ["duration: 0:00:00.000000", "reverseDuration: 0:00:00.000000"]);
}

#[test]
fn animation_style_diagnostics_preserve_the_typed_overrides_and_pinned_labels() {
    use incular_animation::AnimationStyleProperty;

    let style = AnimationStyle::new()
        .curve(Curve::EaseInOut)
        .duration(Duration::from_secs(1))
        .reverse_curve(Curve::EaseInOutBounce)
        .reverse_duration(Duration::from_secs(2));
    let actual = style.diagnostic_properties().collect::<Vec<_>>();
    assert_eq!(
        actual,
        [
            AnimationStyleProperty::Curve(&Curve::EaseInOut),
            AnimationStyleProperty::Duration(Duration::from_secs(1)),
            AnimationStyleProperty::ReverseCurve(&Curve::EaseInOutBounce),
            AnimationStyleProperty::ReverseDuration(Duration::from_secs(2)),
        ]
    );
    assert_eq!(
        actual.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [
            "curve: Cubic(0.42, 0.00, 0.58, 1.00)",
            "duration: 0:00:01.000000",
            "reverseCurve: _BounceInOutCurve",
            "reverseDuration: 0:00:02.000000",
        ]
    );

    assert_eq!(
        AnimationStyleProperty::Duration(Duration::new(3661, 5_007_500)).to_string(),
        "duration: 1:01:01.005007"
    );
}
