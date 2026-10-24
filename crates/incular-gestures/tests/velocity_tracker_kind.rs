//! Flutter `velocityTrackerBuilder`: the iOS and macOS scroll view fling
//! trackers weight the last three sample-to-sample velocities.

use std::{cell::Cell, rc::Rc, time::Duration, time::Instant};

use incular_core::{Offset, PointerPhase};
use incular_gestures::*;

/// A pan through x = 0, 10, 30, 60 at 10 ms intervals, released at once.
/// The sample-to-sample velocities, oldest first, are 1000, 2000 and 3000
/// px/s.
fn release_velocity(kind: Option<VelocityTrackerKind>) -> f32 {
    let velocity = Rc::new(Cell::new(f32::NAN));
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_pan_end: Some({
            let velocity = velocity.clone();
            Rc::new(move |details: DragEndDetails| {
                velocity.set(details.velocity.pixels_per_second.x);
            })
        }),
        ..GestureCallbacks::default()
    });
    if let Some(kind) = kind {
        recognizer.set_velocity_tracker(kind);
    }
    let base = Instant::now();
    for (phase, x, millis) in [
        (PointerPhase::Down, 0., 0),
        (PointerPhase::Move, 10., 10),
        (PointerPhase::Move, 30., 20),
        (PointerPhase::Move, 60., 30),
        (PointerPhase::Up, 60., 30),
    ] {
        recognizer.handle(PointerEvent {
            pointer: 1,
            position: Offset::new(x, 0.),
            phase,
            time: base + Duration::from_millis(millis),
        });
    }
    velocity.get()
}

#[test]
fn the_default_tracker_is_the_least_squares_fit() {
    assert_eq!(
        VelocityTrackerKind::default(),
        VelocityTrackerKind::LeastSquares
    );
    assert_eq!(
        release_velocity(None),
        release_velocity(Some(VelocityTrackerKind::LeastSquares))
    );
}

#[test]
fn ios_tracker_weights_older_velocities_most() {
    // 1000 * 0.6 + 2000 * 0.35 + 3000 * 0.05.
    assert!((release_velocity(Some(VelocityTrackerKind::IosScrollViewFling)) - 1450.).abs() < 0.5);
}

#[test]
fn macos_tracker_uses_its_own_weights() {
    // 1000 * 0.15 + 2000 * 0.65 + 3000 * 0.2.
    assert!(
        (release_velocity(Some(VelocityTrackerKind::MacOsScrollViewFling)) - 2050.).abs() < 0.5
    );
}
