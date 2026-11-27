//! Borrowed projections of the exact gesture packet shapes in events_test.dart.

use incular_core::{
    Matrix4, NormalizedPressure, Offset, PointerContactMetadata, PointerDeviceKind,
    PointerSampleMetadata, time::Instant,
};
use incular_gestures::{
    ConvertedPointerKind, OffsetPair, PointerData, PointerDataChange, PointerDataSignal,
    PointerEventConverter,
};

#[test]
fn converted_contacts_signals_and_pan_zoom_project_without_replacing_the_original() {
    use PointerDataChange as Change;
    use PointerDataSignal as Signal;

    let transform = Matrix4::scale(2., 2., 1.).multiply(Matrix4::translation(10., 20., 0.));
    let position = Offset::new(20., 30.);
    let local_position = Offset::new(60., 100.);
    let delta = Offset::new(5., 5.);
    let kinds = [
        (Change::Add, Signal::None, PointerDeviceKind::Mouse),
        (Change::Cancel, Signal::None, PointerDeviceKind::Mouse),
        (Change::Down, Signal::None, PointerDeviceKind::Mouse),
        (Change::Hover, Signal::None, PointerDeviceKind::Mouse),
        (Change::Move, Signal::None, PointerDeviceKind::Mouse),
        (Change::Remove, Signal::None, PointerDeviceKind::Mouse),
        (
            Change::Down,
            Signal::Scroll(Offset::ZERO),
            PointerDeviceKind::Mouse,
        ),
        (
            Change::PanZoomStart,
            Signal::None,
            PointerDeviceKind::Trackpad,
        ),
        (
            Change::PanZoomUpdate {
                physical_pan: Offset::new(4., 6.),
                physical_pan_delta: Offset::new(1., -1.),
                scale: 1.25,
                rotation: 0.5,
            },
            Signal::None,
            PointerDeviceKind::Trackpad,
        ),
        (
            Change::PanZoomEnd,
            Signal::None,
            PointerDeviceKind::Trackpad,
        ),
        (Change::Up, Signal::None, PointerDeviceKind::Mouse),
    ];
    let now = Instant::now();
    for (change, signal, kind) in kinds {
        let source = PointerEventConverter::expand(
            [PointerData {
                view_id: 5,
                embedder_id: 6,
                platform_data: 10,
                kind,
                device: 1,
                pointer: 45,
                physical_position: position,
                physical_delta: delta,
                buttons: 4,
                synthesized: true,
                sample: PointerSampleMetadata::new(NormalizedPressure::new(0.5), None)
                    .with_contact(PointerContactMetadata {
                        raw_pressure: None,
                        obscured: true,
                        pressure_min: 10.,
                        pressure_max: 60.,
                        distance: 12.,
                        distance_max: 24.,
                        size: 10.,
                        radius_major: 33.,
                        radius_minor: 44.,
                        radius_min: 10.,
                        radius_max: 50.,
                        orientation: 2.,
                        tilt: 4.,
                    }),
                signal,
                ..PointerData::new(change, now)
            }],
            |_| Some(1.),
        )
        .next()
        .unwrap();
        let saved = source;
        let unprojected = source.projection(None);
        assert!(unprojected.original().is_none());
        assert!(unprojected.transform().is_none());
        assert_eq!(
            unprojected.coordinates().position,
            OffsetPair::new(position, position)
        );
        let projected = source.projection(Some(&transform));
        assert!(std::ptr::eq(projected.source(), &source));
        assert!(std::ptr::eq(projected.original().unwrap(), &source));
        assert!(std::ptr::eq(projected.transform().unwrap(), &transform));
        assert_eq!(*projected.source(), saved);
        let coordinates = projected.coordinates();
        assert_eq!(
            coordinates.position,
            OffsetPair::new(local_position, position)
        );
        let expected_delta = if matches!(change, Change::Hover | Change::Move) {
            delta
        } else {
            Offset::ZERO
        };
        assert_eq!(
            coordinates.delta,
            OffsetPair::new(expected_delta + expected_delta, expected_delta)
        );
        let second = Matrix4::translation(-5., -7., 0.);
        assert_eq!(
            source
                .projection(Some(&second))
                .coordinates()
                .position
                .local,
            Offset::new(15., 23.)
        );

        if let Change::PanZoomUpdate { .. } = change {
            let pan = projected
                .pan_coordinates()
                .expect("only a pan/zoom update has pan");
            assert_eq!(
                pan.position,
                OffsetPair::new(Offset::new(28., 52.), Offset::new(4., 6.))
            );
            assert_eq!(
                pan.delta,
                OffsetPair::new(Offset::new(2., -2.), Offset::new(1., -1.))
            );
        } else {
            assert!(projected.pan_coordinates().is_none());
        }

        if let ConvertedPointerKind::Contact { event, .. } = source.kind {
            let raw = event.transformed_matrix4(Some(transform));
            assert_eq!(raw.local_position, coordinates.position.local);
            assert_eq!(raw.local_delta, coordinates.delta.local);
            assert_eq!(raw.in_global_coordinates(), event);
            if matches!(change, Change::Hover | Change::Move) {
                for boundary in [event.as_mouse_enter(), event.as_mouse_exit()] {
                    let projected = boundary.transformed_matrix4(Some(transform));
                    assert_eq!(projected.local_position, local_position);
                    assert_eq!(projected.local_delta, Offset::new(10., 10.));
                    assert_eq!(projected.in_global_coordinates(), boundary);
                }
            }
        }
    }
}

#[test]
fn projecting_a_pan_delta_uses_both_ends_with_projective_geometry() {
    use PointerDataChange as Change;

    let mut matrix = *Matrix4::IDENTITY.columns();
    matrix[3] = 0.25;
    let matrix = Matrix4::from_columns(matrix);
    let now = Instant::now();
    let source = PointerEventConverter::expand(
        [PointerData {
            kind: PointerDeviceKind::Trackpad,
            ..PointerData::new(
                Change::PanZoomUpdate {
                    physical_pan: Offset::new(4., 0.),
                    physical_pan_delta: Offset::new(2., 0.),
                    scale: 1.,
                    rotation: 0.,
                },
                now,
            )
        }],
        |_| Some(1.),
    )
    .next()
    .unwrap();
    let pan = source.projection(Some(&matrix)).pan_coordinates().unwrap();
    assert_eq!(
        pan.position,
        OffsetPair::new(Offset::new(2., 0.), Offset::new(4., 0.))
    );
    assert!((pan.delta.local.x - 2. / 3.).abs() < 0.000_001);
    assert_eq!(pan.delta.global, Offset::new(2., 0.));
}
