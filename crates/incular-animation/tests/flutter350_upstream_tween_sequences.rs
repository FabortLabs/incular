//! Observable scalar assertions from Flutter 3.50.0 animation/animations_test.dart.

use std::time::Duration;

use incular_animation::{AnimationController, Curve, DoubleAnimation, TweenSequence, TweenValue};

// The upstream cases use direct value assignment, never a timed run.
fn check_sequence(sequence: TweenSequence<f32>, initial: f32, samples: &[(f32, f32)]) {
    let controller = AnimationController::new(Duration::ZERO);
    let animation = DoubleAnimation::from(&controller).drive(sequence);
    assert_eq!(animation.value(), initial, "initial value");
    for &(progress, expected) in samples {
        controller.set_value(progress);
        assert_eq!(animation.value(), expected, "progress {progress}");
    }
}

#[test]
fn weighted_tween_sequence() {
    // Upstream line 414.
    let sequence = TweenSequence::new()
        .push(TweenValue::new(5.0, 10.0), 4.0)
        .push(TweenValue::new(10.0, 10.0), 2.0)
        .push(TweenValue::new(10.0, 5.0), 4.0);
    check_sequence(
        sequence,
        5.0,
        &[(0.2, 7.5), (0.4, 10.0), (0.6, 10.0), (0.8, 7.5), (1.0, 5.0)],
    );
}

#[test]
fn weighted_tween_sequence_with_segment_local_curves() {
    // Upstream line 441.
    let sequence = TweenSequence::new()
        .push(
            TweenValue::curved(5.0, 10.0, Curve::interval(0.5, 1.0, &Curve::Linear)),
            4.0,
        )
        .push(TweenValue::new(10.0, 10.0), 2.0)
        .push(
            TweenValue::curved(10.0, 5.0, Curve::interval(0.0, 0.5, &Curve::Linear)),
            4.0,
        );
    check_sequence(
        sequence,
        5.0,
        &[(0.2, 5.0), (0.4, 10.0), (0.6, 10.0), (0.8, 5.0), (1.0, 5.0)],
    );
}

#[test]
fn tween_sequence_with_one_segment() {
    // Upstream line 484.
    let sequence = TweenSequence::new().push(TweenValue::new(5.0, 10.0), 1.0);
    check_sequence(sequence, 5.0, &[(0.5, 7.5), (1.0, 10.0)]);
}

#[test]
fn raw_sequence_input_policy_is_distinct_from_flutter() {
    // An additional API characterization, not a transplanted upstream test.
    let empty = TweenSequence::<f32>::new();
    assert_eq!(empty.evaluate(0.5), None);
    let sequence = empty
        .push(TweenValue::new(0.0, 1.0), 0.0)
        .push(TweenValue::new(0.0, 1.0), -1.0)
        .push(TweenValue::new(5.0, 10.0), 1.0);
    assert_eq!(sequence.len(), 1);
    assert_eq!(sequence.evaluate(-0.01), Some(5.0));
    assert_eq!(sequence.evaluate(1.01), Some(10.0));
}
