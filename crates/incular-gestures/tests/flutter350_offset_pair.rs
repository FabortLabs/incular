use incular_core::{Offset, PointerPhase, Transform, time::Instant};
use incular_gestures::{OffsetPair, PointerDeviceKind, RawPointerEvent};

#[test]
fn offset_pair_arithmetic_keeps_coordinate_spaces_separate() {
    let a = OffsetPair::new(Offset::new(10., 20.), Offset::new(30., 40.));
    let b = OffsetPair::new(Offset::new(50., 60.), Offset::new(70., 80.));
    assert_eq!(a.local, Offset::new(10., 20.));
    assert_eq!(a.global, Offset::new(30., 40.));
    assert_eq!(
        b + a,
        OffsetPair::new(Offset::new(60., 80.), Offset::new(100., 120.))
    );
    assert_eq!(
        b - a,
        OffsetPair::new(Offset::new(40., 40.), Offset::new(40., 40.))
    );
    assert_eq!(a + OffsetPair::ZERO, a);
}

#[test]
fn event_pairs_read_the_existing_position_and_delta_in_each_space() {
    let mut event = RawPointerEvent::new(
        4,
        1,
        PointerDeviceKind::Touch,
        1,
        Offset::new(30., 40.),
        PointerPhase::Move,
        Instant::now(),
    );
    event.delta = Offset::new(4., 3.);
    let transformed = event.transformed(Some(
        Transform::translation(Offset::new(-5., 7.)).then(Transform::scale_non_uniform(0.5, -2.)),
    ));
    assert_eq!(
        OffsetPair::from_event_position(&transformed),
        OffsetPair::new(Offset::new(10., -73.), Offset::new(30., 40.))
    );
    assert_eq!(
        OffsetPair::from_event_delta(&transformed),
        OffsetPair::new(Offset::new(2., -6.), Offset::new(4., 3.))
    );
}
