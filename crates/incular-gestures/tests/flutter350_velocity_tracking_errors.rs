use std::time::Duration;

use incular_core::{Offset, time::Instant};
use incular_gestures::{Velocity, VelocityTracker, VelocityTrackerError, VelocityTrackerKind};

#[test]
fn scroll_view_tracker_requires_at_least_one_sample() {
    let now = Instant::now();
    for kind in [
        VelocityTrackerKind::IosScrollViewFling,
        VelocityTrackerKind::MacOsScrollViewFling,
    ] {
        let error = VelocityTracker::new(kind).try_velocity_at(now).unwrap_err();
        assert_eq!(error, VelocityTrackerError::MissingSamples);
        assert!(error.to_string().contains("at least 1 point"));
    }
    assert_eq!(
        VelocityTracker::default().try_velocity_at(now),
        Ok(Velocity::ZERO)
    );
}

#[test]
fn an_earlier_timestamp_is_rejected_without_polluting_newer_samples() {
    let start = Instant::now();
    let now = start + Duration::from_secs(3600);
    let mut tracker = VelocityTracker::new(VelocityTrackerKind::IosScrollViewFling);
    tracker.try_add_position(now, Offset::ZERO).unwrap();
    assert_eq!(tracker.try_velocity_at(now), Ok(Velocity::ZERO));
    let old = tracker
        .try_add_position(start + Duration::from_secs(1), Offset::new(100., 100.))
        .unwrap_err();
    assert_eq!(old, VelocityTrackerError::OutOfOrderTimestamp);
    assert!(old.to_string().contains("has a smaller timestamp"));

    let next = now + Duration::from_secs(1);
    tracker
        .try_add_position(next, Offset::new(0., -1.))
        .unwrap();
    assert_eq!(
        tracker.try_velocity_at(next),
        Ok(Velocity::new(Offset::new(0., -0.05))),
    );
    tracker
        .try_add_position(next, Offset::new(0., -1.))
        .unwrap();
}

#[test]
fn progressive_scroll_history_can_be_estimated_after_every_sample() {
    let start = Instant::now();
    let mut tracker = VelocityTracker::new(VelocityTrackerKind::IosScrollViewFling);
    for (index, expected) in [0., -0.05, -0.4, -1., -1.].into_iter().enumerate() {
        let count = index + 1;
        let time = start + Duration::from_secs(count as u64);
        tracker
            .try_add_position(time, Offset::new(0., -(count as f32)))
            .unwrap();
        let actual = tracker.try_velocity_at(time).unwrap().pixels_per_second;
        assert_eq!(actual.x, 0.);
        assert!((actual.y - expected).abs() < 0.0001);
    }
}
