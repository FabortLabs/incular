use std::sync::Arc;

use incular_animation::Curve;
use incular_core::Offset;

fn one_line(curve: &Curve) {
    assert_eq!(format!("{curve:?}").lines().count(), 1);
}

fn bounds(curve: &Curve) -> (f32, f32) {
    (0..=10)
        .map(|i| curve.apply(i as f32 / 10.))
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
            (min.min(value), max.max(value))
        })
}

#[test]
fn common_curves_have_short_debug_descriptions() {
    for curve in [
        Curve::Linear,
        Curve::SawTooth(3),
        Curve::interval(0.25, 0.75, &Curve::Linear),
        Curve::interval(0.25, 0.75, &Curve::Ease),
        Curve::split(0.25, &Curve::Ease, &Curve::EaseOutCubic),
    ] {
        one_line(&curve);
    }
}

#[test]
fn flipped_ease_reverses_the_value_and_preserves_endpoints() {
    let ease = Curve::Ease;
    let flipped = ease.clone().flipped();
    one_line(&flipped);
    assert!(flipped.apply(0.) < 0.001);
    assert!(flipped.apply(0.5) < ease.apply(0.5));
    assert!(flipped.apply(1.) > 0.999);
}

#[test]
fn elastic_in_out_and_in_out_overshoot_the_correct_edges() {
    for (curve, below, above) in [
        (Curve::EaseInElastic, true, false),
        (Curve::EaseOutElastic, false, true),
        (Curve::EaseInOutElastic, true, true),
    ] {
        one_line(&curve);
        let (min, max) = bounds(&curve);
        assert_eq!(min < 0., below);
        assert_eq!(max > 1., above);
    }
}

#[test]
fn back_in_out_and_in_out_overshoot_the_correct_edges() {
    for (curve, below, above) in [
        (Curve::EaseInBack, true, false),
        (Curve::EaseOutBack, false, true),
        (Curve::EaseInOutBack, true, true),
    ] {
        one_line(&curve);
        let (min, max) = bounds(&curve);
        assert_eq!(min < 0., below);
        assert_eq!(max > 1., above);
    }
}

#[test]
fn decelerate_stays_between_endpoints_and_moves_more_early() {
    let curve = Curve::Decelerate;
    one_line(&curve);
    let (min, max) = bounds(&curve);
    assert_eq!((min, max), (0., 1.));
    let early = curve.apply(0.2) - curve.apply(0.);
    let late = curve.apply(1.) - curve.apply(0.8);
    assert!(late < early);
}

#[test]
fn live_catmull_rom_curves_share_samples_without_a_permanent_retention() {
    let points = [
        Offset::new(0.2, 0.25),
        Offset::new(0.33, 0.25),
        Offset::new(0.5, 1.0),
        Offset::new(0.8, 0.75),
    ];
    let first = Curve::catmull_rom(&points, 0.125);
    let second = Curve::catmull_rom(&points, 0.125);
    let (Curve::CatmullRom(a), Curve::CatmullRom(b)) = (&first, &second) else {
        panic!("expected sampled timing curves")
    };
    assert!(Arc::ptr_eq(a, b));
    assert_eq!(first.apply(0.25), second.apply(0.25));
    let probe = Arc::downgrade(a);
    drop(first);
    assert!(probe.upgrade().is_some());
    drop(second);
    assert!(probe.upgrade().is_none());
}

#[test]
fn an_owned_flipped_curve_is_freed_with_its_last_reference() {
    let source = Curve::SawTooth(3);
    let flipped = source.clone().flipped();
    let Curve::Flipped(owned) = &flipped else {
        panic!("the generic path should own the curve")
    };
    let weak = Arc::downgrade(owned);
    let clone = flipped.clone();
    for t in [0.05, 0.25, 0.5, 0.95] {
        assert_eq!(flipped.apply(t), 1.0 - source.apply(1.0 - t));
    }
    assert_eq!(flipped.flipped(), source);
    assert!(weak.upgrade().is_some());
    drop(clone);
    assert!(weak.upgrade().is_none());
}
