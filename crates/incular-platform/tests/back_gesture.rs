//! Flutter's `PredictiveBackEvent` (support-002).

use incular_core::Offset;
use incular_platform::{PredictiveBackEvent, SwipeEdge};

#[test]
fn android_back_events_convert_to_logical_pixels_and_edges() {
    let event = PredictiveBackEvent::from_android(60., 300., 0.4, 1, 3.);
    assert_eq!(event.touch_offset, Some(Offset::new(20., 100.)));
    assert!((event.progress - 0.4).abs() < 1e-6);
    assert_eq!(event.swipe_edge, SwipeEdge::Right);
    assert!(!event.is_button_event());
    assert_eq!(
        PredictiveBackEvent::from_android(0., 0., 0.2, 0, 0.).swipe_edge,
        SwipeEdge::Left
    );
}

#[test]
fn progress_is_clamped_and_button_events_are_recognised() {
    assert_eq!(
        PredictiveBackEvent::new(None, 2., SwipeEdge::Left).progress,
        1.
    );
    assert_eq!(
        PredictiveBackEvent::new(None, f32::NAN, SwipeEdge::Left).progress,
        0.
    );
    assert!(PredictiveBackEvent::new(None, 0.5, SwipeEdge::Left).is_button_event());
    // Flutter: zero progress at the origin is a button, any progress is not.
    assert!(PredictiveBackEvent::new(Some(Offset::ZERO), 0., SwipeEdge::Left).is_button_event());
    assert!(!PredictiveBackEvent::new(Some(Offset::ZERO), 0.1, SwipeEdge::Left).is_button_event());
}
