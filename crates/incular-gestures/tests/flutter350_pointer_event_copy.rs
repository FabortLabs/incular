//! Position and time copies of each logical contact sample from events_test.dart.

use std::time::Duration;

use incular_core::{
    Offset, PointerContactMetadata, PointerDeviceKind, PointerSampleMetadata, Transform,
    time::Instant,
};
use incular_gestures::{
    ConvertedPointerChange, ConvertedPointerKind, PointerData, PointerDataChange,
    PointerEventConverter,
};

#[test]
fn copying_pointer_motion_preserves_device_contact_and_phase_specific_metadata() {
    use ConvertedPointerChange as Change;
    use PointerDataChange as Data;

    let first = Instant::now();
    let second = first + Duration::from_secs(86_400);
    let samples = [
        (Data::Add, Change::Add, 0, false, false),
        (Data::Hover, Change::Hover, 7, false, true),
        (Data::Down, Change::Down, 7, true, false),
        (Data::Move, Change::Move, 7, true, true),
        (Data::Up, Change::Up, 7, false, false),
        (Data::Remove, Change::Remove, 0, false, false),
    ];
    for (physical_change, expected_change, expected_buttons, expected_down, expected_synthesized) in
        samples
    {
        let packet = PointerData {
            view_id: 25,
            embedder_id: 32,
            platform_data: 42,
            pointer: 1,
            device: 10,
            kind: PointerDeviceKind::Unknown,
            buttons: 7,
            physical_position: Offset::new(101., 202.),
            physical_delta: if physical_change == Data::Move {
                Offset::new(1., 2.)
            } else {
                Offset::ZERO
            },
            sample: PointerSampleMetadata::EMPTY.with_contact(PointerContactMetadata {
                obscured: true,
                pressure_min: 1.1,
                pressure_max: 2.1,
                distance: 11.,
                distance_max: 110.,
                size: 11.,
                radius_major: 11.,
                radius_minor: 9.,
                radius_min: 1.1,
                radius_max: 22.,
                orientation: 1.1,
                tilt: 1.1,
            }),
            synthesized: true,
            ..PointerData::new(physical_change, first)
        };
        let original = PointerEventConverter::expand([packet], |id| (id == 25).then_some(1.))
            .next()
            .unwrap();
        let ConvertedPointerKind::Contact { change, event } = original.kind else {
            panic!("expected a logical contact sample for {physical_change:?}");
        };
        let copied = event.with_global_motion(second, Offset::ZERO, event.delta);
        assert_eq!(event.time, first);
        assert_eq!(event.position, Offset::new(101., 202.));
        assert_eq!(copied.time, second);
        assert_eq!(copied.position, Offset::ZERO);
        assert_eq!(copied.local_position, Offset::ZERO);
        assert_eq!(change, expected_change);
        assert_eq!(copied.phase, event.phase);
        assert_eq!(copied.device, 10);
        assert_eq!(copied.kind, PointerDeviceKind::Unknown);
        assert_eq!(copied.buttons, expected_buttons);
        assert_eq!(copied.down, expected_down);
        assert_eq!(copied.synthesized, expected_synthesized);
        let device_only = matches!(change, Change::Add | Change::Remove | Change::Hover);
        assert_eq!(copied.pointer, u64::from(!device_only));
        let expected_delta = if change == Change::Move {
            Offset::new(1., 2.)
        } else {
            Offset::ZERO
        };
        assert_eq!(copied.delta, expected_delta);
        assert_eq!(copied.local_delta, expected_delta);
        assert_eq!(copied.sample.pressure, None);
        let metrics = copied.sample.contact;
        assert!(metrics.obscured);
        assert_eq!((metrics.pressure_min, metrics.pressure_max), (1.1, 2.1));
        assert_eq!(metrics.distance_max, 110.);
        assert_eq!((metrics.radius_min, metrics.radius_max), (1.1, 22.));
        let device_boundary = matches!(change, Change::Add | Change::Remove);
        assert_eq!(
            (metrics.size, metrics.radius_major, metrics.radius_minor),
            if device_boundary {
                (0., 0., 0.)
            } else {
                (11., 11., 9.)
            },
        );
        assert_eq!(
            metrics.distance,
            if matches!(change, Change::Add | Change::Hover) {
                11.
            } else {
                0.
            },
        );
        assert_eq!(
            (metrics.orientation, metrics.tilt),
            if change == Change::Remove {
                (0., 0.)
            } else {
                (1.1, 1.1)
            },
        );
        assert_eq!(
            original.platform_data,
            if change == Change::Move { 42 } else { 0 }
        );
    }
}

#[test]
fn copying_a_routed_event_drops_the_previous_local_coordinates_and_preserves_measurements() {
    use incular_core::NormalizedPressure;
    use incular_gestures::RawPointerEvent;

    let now = Instant::now();
    let delta = Offset::new(5., 3.);
    let sample = PointerSampleMetadata::new(NormalizedPressure::new(0.75), None);
    let original = RawPointerEvent::contact_move(
        1,
        2,
        PointerDeviceKind::Stylus,
        Offset::new(20., 40.),
        delta,
        now,
        None,
    )
    .with_sample(sample)
    .transformed(Some(Transform::translation(Offset::new(10., 20.))));
    assert_eq!(original.local_position, Offset::new(30., 60.));
    let moved = original.with_global_motion(now, Offset::new(3., 4.), delta);
    assert_eq!(moved.local_position, moved.position);
    assert_eq!(moved.local_delta, delta);
    assert_eq!(moved.sample, sample);
    let retargeted = moved.transformed(Some(Transform::translation(Offset::new(-20., -30.))));
    assert_eq!(retargeted.local_position, Offset::new(-17., -26.));
    assert_eq!(retargeted.local_delta, delta);
}
