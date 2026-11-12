//! Values captured from Flutter 3.41.7 (`flutter test` over
//! `package:flutter/animation.dart`): curves, `fling`, `repeat(count:)`,
//! `animateTo`/`animateBack` statuses and `CurvedAnimation`'s direction
//! latch. Frames follow `WidgetTester.pump`: the ticker's first frame is at
//! elapsed zero, then 16 ms apart.

// Reference values are printed by Flutter with nine decimals.
#![allow(clippy::excessive_precision)]

use std::time::Duration;

use incular_animation::{
    AnimationController, AnimationSource, AnimationStatus, Curve, CurvedAnimation, RepeatOptions,
};
use incular_core::time::Instant;

const SAMPLES: [f32; 5] = [0.1, 0.25, 0.5, 0.75, 0.9];

fn assert_curve(name: &str, curve: Curve, expected: [f32; 5]) {
    for (t, expected) in SAMPLES.into_iter().zip(expected) {
        let actual = curve.apply(t);
        assert!(
            (actual - expected).abs() < 2e-3,
            "{name}({t}) = {actual}, Flutter {expected}"
        );
    }
}

#[test]
fn curves_match_flutter() {
    assert_curve(
        "decelerate",
        Curve::Decelerate,
        [0.19, 0.4375, 0.75, 0.9375, 0.99],
    );
    assert_curve(
        "fastLinearToSlowEaseIn",
        Curve::FastLinearToSlowEaseIn,
        [0.591172, 0.913997, 0.988370, 0.999083, 0.999953],
    );
    assert_curve(
        "fastEaseInToSlowEaseOut",
        Curve::FastEaseInToSlowEaseOut,
        [0.238569, 0.666823, 0.942346, 0.991563, 0.997621],
    );
    assert_curve(
        "easeInToLinear",
        Curve::EaseInToLinear,
        [0.005117, 0.017364, 0.080196, 0.376278, 0.739474],
    );
    assert_curve(
        "linearToEaseOut",
        Curve::LinearToEaseOut,
        [0.260526, 0.623722, 0.919804, 0.982636, 0.994883],
    );
    assert_curve(
        "easeInOutCubicEmphasized",
        Curve::EaseInOutCubicEmphasized,
        [0.093717, 0.771493, 0.950837, 0.991445, 0.998832],
    );
    assert_curve(
        "slowMiddle",
        Curve::SlowMiddle,
        [0.286198, 0.428316, 0.5, 0.571684, 0.713802],
    );
    assert_curve(
        "Interval(0.2, 0.7, easeIn)",
        Curve::interval(0.2, 0.7, &Curve::EaseIn),
        [0.0, 0.016937, 0.429893, 1.0, 1.0],
    );
    assert_curve(
        "Threshold(0.4)",
        Curve::Threshold(0.4),
        [0.0, 0.0, 1.0, 1.0, 1.0],
    );
    assert_curve(
        "SawTooth(3)",
        Curve::SawTooth(3),
        [0.3, 0.75, 0.5, 0.25, 0.7],
    );
    assert_curve(
        "Split(0.3)",
        Curve::split(0.3, &Curve::Linear, &Curve::EaseOutCubic),
        [0.1, 0.25, 0.760461, 0.962055, 0.995058],
    );
    assert_curve(
        "ease.flipped",
        Curve::Ease.flipped(),
        [0.005758, 0.039451, 0.198008, 0.590505, 0.904607],
    );
    assert_curve(
        "easeInSine.flipped",
        Curve::EaseInSine.flipped(),
        [0.129198, 0.358221, 0.708879, 0.929444, 0.988943],
    );
    assert_curve(
        "elasticIn.flipped",
        Curve::EaseInElastic.flipped(),
        [1.0, 1.125, 1.0, 0.996094, 1.0],
    );
    assert_curve(
        "ElasticOutCurve(0.3)",
        Curve::ElasticOut(0.3),
        [1.25, 0.911612, 1.015625, 1.005524, 0.998047],
    );
}

#[test]
fn flipping_twice_restores_a_curve() {
    for curve in [Curve::Ease, Curve::EaseInElastic, Curve::Decelerate] {
        assert_eq!(curve.flipped().flipped(), curve);
    }
    // A cubic flips to the named cubic with mirrored control points.
    assert_eq!(Curve::EaseIn.flipped(), Curve::EaseOut);
    assert_eq!(Curve::EaseInToLinear.flipped(), Curve::LinearToEaseOut);
}

fn frames(start: Instant, count: usize) -> impl Iterator<Item = Instant> {
    (0..count).map(move |frame| start + Duration::from_millis(16 * frame as u64))
}

#[test]
fn fling_matches_flutter() {
    let controller = AnimationController::new(Duration::ZERO);
    controller.fling_on_next_tick(2.0);
    let expected = [
        0.0, 0.073483, 0.194129, 0.326920, 0.453637, 0.565921, 0.660992, 0.739046,
    ];
    let start = Instant::now();
    let mut animating_frames = 0;
    for (index, now) in frames(start, 200).enumerate() {
        controller.tick(now);
        if let Some(expected) = expected.get(index) {
            assert!(
                (controller.value() - expected).abs() < 1e-3,
                "frame {index}: {} vs Flutter {expected}",
                controller.value()
            );
        }
        if !controller.is_animating() {
            break;
        }
        animating_frames += 1;
    }
    assert_eq!(animating_frames, 19, "Flutter completes on the 20th frame");
    assert_eq!(controller.status(), AnimationStatus::Completed);
    assert_eq!(controller.value(), 1.0);
}

#[test]
fn repeat_with_a_count_matches_flutter() {
    let controller = AnimationController::new(Duration::from_millis(100)).with_value(0.25);
    controller.repeat_on_next_tick(RepeatOptions::default().reverse(true).count(2));
    let values = [
        0.25, 0.41, 0.57, 0.73, 0.89, 0.95, 0.79, 0.63, 0.47, 0.31, 0.15, 0.01, 0.01, 0.01,
    ];
    let statuses = [
        "forward",
        "forward",
        "forward",
        "forward",
        "forward",
        "reverse",
        "reverse",
        "reverse",
        "reverse",
        "reverse",
        "reverse",
        "completed",
        "completed",
        "completed",
    ];
    for (index, now) in frames(Instant::now(), values.len()).enumerate() {
        controller.tick(now);
        assert!(
            (controller.value() - values[index]).abs() < 1e-4,
            "frame {index}: {} vs Flutter {}",
            controller.value(),
            values[index]
        );
        let status = match controller.status() {
            AnimationStatus::Forward => "forward",
            AnimationStatus::Reverse => "reverse",
            AnimationStatus::Completed => "completed",
            AnimationStatus::Dismissed => "dismissed",
        };
        assert_eq!(status, statuses[index], "frame {index}");
    }
}

#[test]
fn animate_to_and_back_end_by_direction_as_in_flutter() {
    let controller = AnimationController::new(Duration::from_millis(100)).with_value(1.0);
    controller.animate_to_on_next_tick(0.2);
    // `animateTo` always runs forward, even downwards.
    assert_eq!(controller.status(), AnimationStatus::Forward);
    let start = Instant::now();
    controller.tick(start);
    controller.tick(start + Duration::from_millis(50));
    assert_eq!(controller.status(), AnimationStatus::Forward);
    assert!((controller.value() - 0.5).abs() < 1e-5);
    controller.tick(start + Duration::from_millis(110));
    assert_eq!(controller.status(), AnimationStatus::Completed);
    assert!((controller.value() - 0.2).abs() < 1e-5);
    let later = start + Duration::from_millis(200);
    controller.animate_back_curved(later, 0.5, Some(Duration::from_millis(40)), Curve::Linear);
    controller.tick(later + Duration::from_millis(50));
    assert_eq!(controller.status(), AnimationStatus::Dismissed);
    assert!((controller.value() - 0.5).abs() < 1e-5);
}

#[test]
fn curved_animation_latches_its_direction_until_rest() {
    let controller = AnimationController::new(Duration::from_millis(100));
    let curved =
        CurvedAnimation::new(controller.clone(), Curve::EaseIn).reverse_curve(Curve::EaseOut);
    let start = Instant::now();
    controller.forward_on_next_tick();
    controller.tick(start);
    controller.tick(start + Duration::from_millis(40));
    assert!(
        (curved.value() - 0.214525).abs() < 1e-3,
        "{}",
        curved.value()
    );
    // Turning around mid-run keeps the forward curve.
    let reversed = start + Duration::from_millis(56);
    controller.reverse_on_next_tick();
    controller.tick(reversed);
    controller.tick(reversed + Duration::from_millis(10));
    assert!(
        (curved.value() - 0.129707).abs() < 1e-3,
        "{}",
        curved.value()
    );
    // Once it rests, a new reverse run uses the reverse curve.
    controller.tick(reversed + Duration::from_millis(110));
    assert_eq!(controller.status(), AnimationStatus::Dismissed);
    controller.set_value(1.0);
    let again = reversed + Duration::from_millis(126);
    controller.reverse_on_next_tick();
    controller.tick(again);
    controller.tick(again + Duration::from_millis(30));
    assert!((controller.value() - 0.7).abs() < 1e-4);
    assert!(
        (curved.value() - 0.870293).abs() < 1e-3,
        "{}",
        curved.value()
    );
}

const CATMULL_SAMPLES: [f32; 9] = [0.05, 0.1, 0.25, 0.4, 0.5, 0.6, 0.75, 0.9, 0.95];

#[test]
fn catmull_rom_curves_match_flutter() {
    use incular_core::Offset;
    let curve = Curve::catmull_rom(
        &[
            Offset::new(0.2, 0.6),
            Offset::new(0.5, 0.3),
            Offset::new(0.8, 0.9),
        ],
        0.0,
    );
    let expected = [
        0.194214495,
        0.411526521,
        0.577478289,
        0.374780546,
        0.300002652,
        0.429619109,
        0.823600822,
        0.961734681,
        0.979417464,
    ];
    for (t, expected) in CATMULL_SAMPLES.into_iter().zip(expected) {
        let actual = curve.apply(t);
        assert!(
            (actual - expected).abs() < 2e-6,
            "CR({t}) = {actual}, Flutter {expected}"
        );
    }
    let tense = Curve::catmull_rom(&[Offset::new(0.25, 0.1), Offset::new(0.6, 0.85)], 0.4);
    let expected = [
        0.016763479,
        0.032431860,
        0.100107812,
        0.419742253,
        0.668143336,
        0.849999304,
        0.922181016,
        0.968806956,
        0.983652298,
    ];
    for (t, expected) in CATMULL_SAMPLES.into_iter().zip(expected) {
        let actual = tense.apply(t);
        assert!(
            (actual - expected).abs() < 2e-6,
            "CR1({t}) = {actual}, Flutter {expected}"
        );
    }
    // The same points give the same interned curve.
    assert_eq!(
        tense,
        Curve::catmull_rom(&[Offset::new(0.25, 0.1), Offset::new(0.6, 0.85)], 0.4)
    );
    assert!(
        incular_animation::CatmullRomCurve::validate_control_points(
            &[Offset::new(0.5, 1.5), Offset::new(0.4, 0.2)],
            0.0
        )
        .is_err()
    );
}

#[test]
fn catmull_rom_spline_samples_match_flutter() {
    use incular_animation::{CatmullRomSpline, Curve2D};
    use incular_core::Offset;
    let spline = CatmullRomSpline::new(
        &[
            Offset::new(0.0, 0.0),
            Offset::new(0.3, 0.8),
            Offset::new(0.7, 0.2),
            Offset::new(1.0, 1.0),
        ],
        0.0,
    );
    let expected = [
        (0.043650355, 0.134484124),
        (0.085554112, 0.287712409),
        (0.215076143, 0.706500914),
        (0.374282194, 0.741362391),
        (0.5, 0.5),
        (0.625717806, 0.258637609),
        (0.784923857, 0.293499086),
        (0.914445888, 0.712287591),
        (0.956349645, 0.865515876),
    ];
    for (t, (x, y)) in CATMULL_SAMPLES.into_iter().zip(expected) {
        let (ax, ay) = spline.transform_xy(f64::from(t));
        assert!(
            (ax - x).abs() < 1e-6 && (ay - y).abs() < 1e-6,
            "{t}: ({ax}, {ay})"
        );
    }
    // Flutter's adaptive sampler, with the Dart VM's seeded `Random`.
    let samples = spline.generate_samples(0.0, 1.0, 1e-10);
    assert_eq!(samples.len(), 77);
    let expected = [
        (0.0, 0.0, 0.0),
        (0.013458729, 0.012002456, 0.033485742),
        (0.028435509, 0.025122218, 0.073286386),
        (0.044855085, 0.039263670, 0.119520571),
        (0.061099482, 0.053053117, 0.167420010),
        (0.074642658, 0.064432164, 0.208614374),
    ];
    for (sample, (t, x, y)) in samples.iter().zip(expected) {
        assert!(
            (sample.t - t).abs() < 1e-8
                && (sample.x - x).abs() < 1e-8
                && (sample.y - y).abs() < 1e-8,
            "{sample:?} vs Flutter ({t}, {x}, {y})"
        );
    }
    // Bisection lands within its 1e-6 limit; the f32 control points move
    // the exact crossing slightly from Dart's doubles.
    assert!((spline.find_inverse(0.5) - 0.499998093).abs() < 1e-5);
}

#[test]
fn an_interpolation_completes_on_the_frame_after_its_end() {
    // Flutter `_InterpolationSimulation.isDone` is strict: at the exact end
    // the value is there but the status still runs.
    let controller = AnimationController::new(Duration::from_millis(100));
    let start = Instant::now();
    controller.forward(start);
    controller.tick(start + Duration::from_millis(100));
    assert_eq!(controller.value(), 1.0);
    assert_eq!(controller.status(), AnimationStatus::Forward);
    controller.tick(start + Duration::from_millis(116));
    assert_eq!(controller.status(), AnimationStatus::Completed);
}

#[test]
fn ticker_futures_complete_or_cancel_as_in_flutter() {
    use std::{cell::RefCell, rc::Rc};
    let log = Rc::new(RefCell::new(Vec::new()));
    let controller = AnimationController::new(Duration::from_millis(100));
    let start = Instant::now();
    let first = controller.forward(start);
    first.when_complete_or_cancel({
        let log = log.clone();
        move || log.borrow_mut().push("first done-or-cancel")
    });
    first.when_complete({
        let log = log.clone();
        move || log.borrow_mut().push("first complete")
    });
    controller.tick(start + Duration::from_millis(40));
    let second = controller.reverse(start + Duration::from_millis(40));
    assert!(
        first.is_canceled(),
        "a new run cancels the old one's future"
    );
    second.when_complete({
        let log = log.clone();
        move || log.borrow_mut().push("second complete")
    });
    controller.tick(start + Duration::from_millis(80));
    assert!(second.is_pending());
    controller.tick(start + Duration::from_millis(96));
    assert!(second.is_complete());
    assert_eq!(
        log.borrow().as_slice(),
        &["first done-or-cancel", "second complete"]
    );
    // `stop` cancels; a zero-length run is complete at once.
    let third = controller.forward(start + Duration::from_millis(200));
    controller.stop();
    assert!(third.is_canceled());
    assert!(controller.animate_to(start, 0.0).is_complete());
}

#[test]
fn ticker_future_or_cancel_reports_the_outcome() {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};
    let controller = AnimationController::new(Duration::from_millis(100));
    let start = Instant::now();
    let canceled = controller.forward(start).or_cancel();
    // The new run cancels the first.
    let running = controller.animate_to(start, 0.5);
    let mut context = Context::from_waker(Waker::noop());
    assert_eq!(
        pin!(canceled).poll(&mut context),
        Poll::Ready(Err(incular_animation::TickerCanceled))
    );
    let mut running = pin!(running);
    assert_eq!(running.as_mut().poll(&mut context), Poll::Pending);
    controller.tick(start + Duration::from_millis(51));
    assert_eq!(running.as_mut().poll(&mut context), Poll::Ready(()));
}
