use incular_core::{
    NormalizedPressure, Offset, PRIMARY_POINTER_BUTTON as PRIMARY, PointerContactMetadata,
    PointerDeviceKind as Device, PointerPanZoomPhase, PointerSampleMetadata,
    SECONDARY_POINTER_BUTTON as SECONDARY, TERTIARY_POINTER_BUTTON as TERTIARY, time::Instant,
};
use incular_gestures::{
    ConvertedPointerChange as Change, ConvertedPointerKind as Kind, PointerData,
    PointerDataChange as Data, PointerDataSignal as Signal, PointerEventConverter, RawPointerEvent,
};

const RATIO: f64 = 2.5;

fn contact(kind: Kind) -> (Change, RawPointerEvent) {
    match kind {
        Kind::Contact { change, event } => (change, event),
        other => panic!("expected a contact, got {other:?}"),
    }
}

fn five_records(kind: Device, press: u32, motion: u32) -> [PointerData; 5] {
    let now = Instant::now();
    [
        (Data::Add, 0),
        (Data::Hover, 0),
        (Data::Down, press),
        (Data::Move, motion),
        (Data::Up, 0),
    ]
    .map(|(change, buttons)| PointerData {
        kind,
        device: 24,
        pointer: 7,
        buttons,
        physical_position: Offset::new(25., 25.),
        ..PointerData::new(change, now)
    })
}

fn check_button_sequence(kind: Device, press: u32, motion: u32, expected: [u32; 5]) {
    let converted: Vec<_> =
        PointerEventConverter::expand(five_records(kind, press, motion), |_| Some(RATIO))
            .map(|event| contact(event.kind))
            .collect();
    assert_eq!(
        converted
            .iter()
            .map(|(change, _)| *change)
            .collect::<Vec<_>>(),
        [
            Change::Add,
            Change::Hover,
            Change::Down,
            Change::Move,
            Change::Up
        ]
    );
    assert_eq!(
        converted
            .iter()
            .map(|(_, event)| event.buttons)
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        converted
            .iter()
            .map(|(_, event)| event.down)
            .collect::<Vec<_>>(),
        [false, false, true, true, false]
    );
    assert_eq!(
        converted
            .iter()
            .map(|(_, event)| event.pointer)
            .collect::<Vec<_>>(),
        [0, 0, 7, 7, 7]
    );
    for (_, event) in converted {
        assert_eq!(event.kind, kind);
        assert_eq!(event.device, 24);
        assert_eq!(event.position, Offset::new(10., 10.));
    }
}

#[test]
fn buttonless_touch_down_and_move_synthesize_primary_but_hover_does_not() {
    check_button_sequence(Device::Touch, 0, 0, [0, 0, PRIMARY, PRIMARY, 0]);
}

#[test]
fn an_explicit_touch_button_is_not_augmented_with_primary() {
    check_button_sequence(
        Device::Touch,
        SECONDARY,
        SECONDARY,
        [0, 0, SECONDARY, SECONDARY, 0],
    );
}

#[test]
fn both_stylus_kinds_synthesize_only_when_the_barrel_mask_is_absent() {
    for device in [Device::Stylus, Device::InvertedStylus] {
        check_button_sequence(device, 0, TERTIARY, [0, 0, PRIMARY, TERTIARY, 0]);
    }
}

#[test]
fn an_unknown_contact_uses_primary_only_as_a_fallback() {
    check_button_sequence(Device::Unknown, 0, SECONDARY, [0, 0, PRIMARY, SECONDARY, 0]);
}

#[test]
fn a_mouse_preserves_even_an_empty_mask_and_tracks_chords_verbatim() {
    check_button_sequence(
        Device::Mouse,
        TERTIARY,
        TERTIARY | SECONDARY,
        [0, 0, TERTIARY, TERTIARY | SECONDARY, 0],
    );
    for kind in [Device::Mouse, Device::Trackpad] {
        check_button_sequence(kind, 0, 0, [0; 5]);
    }
}

#[test]
fn add_hover_remove_and_readd_are_distinct_records_for_the_same_device() {
    let now = Instant::now();
    let data =
        [Data::Add, Data::Hover, Data::Remove, Data::Add, Data::Hover].map(|change| PointerData {
            device: 24,
            ..PointerData::new(change, now)
        });
    let events: Vec<_> = PointerEventConverter::expand(data, |_| Some(RATIO))
        .map(|event| {
            let (change, raw) = contact(event.kind);
            assert_eq!(raw.device, 24);
            change
        })
        .collect();
    assert_eq!(
        events,
        [
            Change::Add,
            Change::Hover,
            Change::Remove,
            Change::Add,
            Change::Hover
        ]
    );
}

#[test]
fn all_nonfinite_scroll_components_and_a_zero_view_scale_are_dropped() {
    let now = Instant::now();
    let data = PointerData {
        device: 24,
        ..PointerData::new(Data::Add, now)
    };
    assert!(matches!(
        PointerEventConverter::expand([data], |_| Some(RATIO))
            .next()
            .unwrap()
            .kind,
        Kind::Contact {
            change: Change::Add,
            ..
        }
    ));
    let malformed = [f32::INFINITY, f32::NAN, f32::NEG_INFINITY]
        .into_iter()
        .flat_map(|bad| {
            [Offset::new(bad, 10.), Offset::new(10., bad)].map(|delta| PointerData {
                signal: Signal::Scroll(delta),
                ..data
            })
        });
    assert_eq!(
        PointerEventConverter::expand(malformed, |_| Some(RATIO)).count(),
        0
    );
    let valid = PointerData {
        signal: Signal::Scroll(Offset::new(10., 10.)),
        ..data
    };
    let event = PointerEventConverter::expand([valid], |_| Some(RATIO))
        .next()
        .unwrap();
    assert!(matches!(event.kind, Kind::Scroll(raw) if raw.scroll_delta == Offset::new(4., 4.)));
    assert_eq!(
        PointerEventConverter::expand([valid], |_| Some(0.)).count(),
        0
    );
}

#[test]
fn even_zero_scroll_is_a_signal_not_a_hover() {
    let now = Instant::now();
    let events: Vec<_> = PointerEventConverter::expand(
        [
            PointerData::new(Data::Add, now),
            PointerData {
                signal: Signal::Scroll(Offset::ZERO),
                ..PointerData::new(Data::Hover, now)
            },
        ],
        |_| Some(RATIO),
    )
    .collect();
    assert!(matches!(events.as_slice(), [
        first, second
    ] if matches!(first.kind, Kind::Contact { change: Change::Add, .. })
        && matches!(second.kind, Kind::Scroll(raw) if raw.scroll_delta == Offset::ZERO)));
}

#[test]
fn all_changes_use_their_own_view_ratio_and_keep_their_view_id() {
    let start_id = 987_654;
    let now = Instant::now();
    let data = [
        Data::Cancel,
        Data::Add,
        Data::Remove,
        Data::Hover,
        Data::Down,
        Data::Move,
        Data::Up,
        Data::PanZoomStart,
        Data::PanZoomUpdate {
            physical_pan: Offset::new(6., 9.),
            physical_pan_delta: Offset::new(3., 0.),
            scale: 1.25,
            rotation: 0.2,
        },
        Data::PanZoomEnd,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, change)| PointerData {
        view_id: start_id + i as u64,
        physical_position: Offset::new(180., 120.),
        device: 4,
        pointer: 12,
        embedder_id: 32,
        ..PointerData::new(change, now)
    });
    let mut ratio_requests = Vec::new();
    let events: Vec<_> = PointerEventConverter::expand(data, |id| {
        ratio_requests.push(id);
        Some((id - start_id + 1) as f64)
    })
    .collect();
    let expected: Vec<_> = (start_id..start_id + 10).collect();
    assert_eq!(ratio_requests, expected);
    assert_eq!(
        events.iter().map(|event| event.view_id).collect::<Vec<_>>(),
        expected
    );
    for (i, converted) in events.iter().enumerate() {
        assert_eq!(converted.embedder_id, 32);
        let position = match converted.kind {
            Kind::Contact { event, .. } => event.position,
            Kind::PanZoom { event, .. } => event.position,
            other => panic!("unexpected change {other:?}"),
        };
        let ratio = (i + 1) as f32;
        assert_eq!(position, Offset::new(180. / ratio, 120. / ratio));
    }
    assert!(matches!(events[7].kind, Kind::PanZoom { event, .. }
        if event.phase == PointerPanZoomPhase::Start && event.pointer == 12));
    assert!(matches!(events[8].kind, Kind::PanZoom { event, pan_delta }
    if pan_delta == Offset::new(3. / 9., 0.)
    && event.phase == PointerPanZoomPhase::Update {
        pan: Offset::new(6. / 9., 1.), scale: 1.25, rotation: 0.2
    }));
    assert!(matches!(events[9].kind, Kind::PanZoom { event, .. }
        if event.phase == PointerPanZoomPhase::End));
}

#[test]
fn a_lazy_packet_ignores_unknown_signals_and_missing_views_without_disturbing_contacts() {
    use std::cell::RefCell;
    let now = Instant::now();
    let source = PointerData {
        view_id: 5,
        pointer: 2,
        platform_data: 112,
        physical_position: Offset::new(25., 50.),
        physical_delta: Offset::new(-5., 10.),
        synthesized: true,
        sample: PointerSampleMetadata {
            pressure: NormalizedPressure::new(0.5),
            contact: PointerContactMetadata {
                radius_major: 15.,
                radius_minor: 10.,
                radius_min: 5.,
                radius_max: 20.,
                size: 0.8,
                ..PointerContactMetadata::DEFAULT
            },
            ..PointerSampleMetadata::EMPTY
        },
        ..PointerData::new(Data::Move, now)
    };
    let looked_up = RefCell::new(Vec::new());
    let packet = [
        PointerData {
            view_id: 1,
            signal: Signal::Unknown,
            ..source
        },
        PointerData {
            view_id: 2,
            ..source
        },
        source,
        PointerData {
            view_id: 6,
            change: Data::Hover,
            ..source
        },
    ];
    let mut expanded = PointerEventConverter::expand(packet, |id| {
        looked_up.borrow_mut().push(id);
        (id >= 5).then_some(RATIO)
    });
    assert!(looked_up.borrow().is_empty());
    let first = expanded.next().unwrap();
    assert_eq!(*looked_up.borrow(), [2, 5]);
    let (change, raw) = contact(first.kind);
    assert_eq!(change, Change::Move);
    assert_eq!(first.platform_data, 112);
    assert_eq!(raw.position, Offset::new(10., 20.));
    assert_eq!(raw.delta, Offset::new(-2., 4.));
    assert_eq!(raw.local_delta, raw.delta);
    assert_eq!(raw.sample.pressure, NormalizedPressure::new(0.5));
    assert_eq!(raw.sample.contact.radius_major, 6.);
    assert_eq!(raw.sample.contact.radius_minor, 4.);
    assert_eq!(raw.sample.contact.radius_min, 2.);
    assert_eq!(raw.sample.contact.radius_max, 8.);
    assert_eq!(raw.sample.contact.size, 0.8);
    assert!(raw.synthesized);
    let second = expanded.next().unwrap();
    let (change, raw) = contact(second.kind);
    assert_eq!(change, Change::Hover);
    assert_eq!(second.platform_data, 0);
    assert!(!raw.down);
    assert_eq!(raw.buttons, 0);
    assert!(expanded.next().is_none());
    assert_eq!(*looked_up.borrow(), [2, 5, 6]);
}
