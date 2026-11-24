//! Authored public pan companion to Flutter's standalone tracker tests.
//! This does not port the private standalone Incular tracker or raw data.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_core::{Offset, PointerPhase, time::Instant};
use incular_gestures::{
    GestureCallbacks, GestureDecision, GestureRecognizer, PointerDeviceKind,
    PointerGestureRecognizer, RawPointerEvent,
};

fn release_velocity(sample_gap: u64, release_delay: u64) -> Offset {
    let velocity = Rc::new(Cell::new(None));
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_pan_end: Some({
            let velocity = Rc::clone(&velocity);
            Rc::new(move |details| velocity.set(Some(details.velocity.pixels_per_second)))
        }),
        ..GestureCallbacks::default()
    });
    let base = Instant::now();
    let samples = [
        (PointerPhase::Down, 250.0, 250.0, 0),
        (PointerPhase::Move, 260.0, 255.0, 10),
        (PointerPhase::Move, 100.0, 150.0, 10 + sample_gap),
        (PointerPhase::Move, 107.0, 189.0, 20 + sample_gap),
        (PointerPhase::Move, 114.0, 228.0, 30 + sample_gap),
        (
            PointerPhase::Up,
            114.0,
            228.0,
            30 + sample_gap + release_delay,
        ),
    ];
    for (phase, x, y, millis) in samples {
        let decision = recognizer.observe_raw(RawPointerEvent::new(
            1,
            1,
            PointerDeviceKind::Touch,
            u32::from(phase != PointerPhase::Up),
            Offset::new(x, y),
            phase,
            base + Duration::from_millis(millis),
        ));
        if let GestureDecision::Accept(action) = decision {
            recognizer.dispatch(action);
        }
    }
    velocity.get().expect("the pan ended")
}

fn assert_velocity(actual: Offset, expected: Offset, context: &str) {
    const TOLERANCE: f32 = 0.01;
    assert!(
        (actual.x - expected.x).abs() <= TOLERANCE && (actual.y - expected.y).abs() <= TOLERANCE,
        "{context}: expected {expected:?} within {TOLERANCE} px/s per axis, got {actual:?}",
    );
}

#[test]
fn a_gap_over_40_ms_discards_samples_before_the_gap() {
    // With a 40 ms gap all five samples contribute to the quadratic fit.
    assert_velocity(
        release_velocity(40, 0),
        Offset::new(-39.098_06, 3842.779),
        "gap 40 ms",
    );
    // With a 41 ms gap only the three trailing, constant-speed samples fit.
    assert_velocity(
        release_velocity(41, 0),
        Offset::new(700.0, 3900.0),
        "gap 41 ms",
    );
}

#[test]
fn release_velocity_is_zero_when_the_last_sample_is_over_40_ms_old() {
    assert_velocity(
        release_velocity(41, 40),
        Offset::new(700.0, 3900.0),
        "release after 40 ms",
    );
    assert_eq!(release_velocity(41, 41), Offset::ZERO);
}
