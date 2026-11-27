use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    InputEvent, NormalizedPressure, Offset, PointerContactMetadata, PointerDeviceKind,
    PointerPhase, PointerSampleMetadata, Size, StylusMetadata, StylusOrientation,
};
use incular_platform::{HoverDeviceChange, HoverDeviceEvent, PlatformEvent};
use incular_runtime::Runtime;
use incular_widgets::{MouseRegion, Positioned, SizedBox, Stack};

#[test]
fn moving_a_pressed_stylus_across_a_region_resets_only_contact_pressure() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let enter = events.clone();
    let exit = events.clone();
    let region = MouseRegion::new(SizedBox::square(100.))
        .on_enter(move |event| enter.borrow_mut().push(event))
        .on_exit(move |event| exit.borrow_mut().push(event));
    let root = Stack::new([Positioned::new(region).left(100.).top(100.)])
        .text_direction(TextDirection::Ltr);
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();

    let stylus = StylusMetadata {
        orientation: Some(StylusOrientation {
            altitude: 0.8,
            azimuth: 1.1,
        }),
        barrel_button: false,
    };
    let contact = PointerContactMetadata {
        pressure_min: 0.,
        pressure_max: 1024.,
        distance: 11.,
        distance_max: 110.,
        size: 0.4,
        radius_major: 5.,
        radius_minor: 4.,
        radius_min: 1.,
        radius_max: 8.,
        orientation: 1.1,
        tilt: 0.8,
        obscured: true,
    };
    let _ = runtime.handle_platform_event(PlatformEvent::HoverDevice(HoverDeviceEvent {
        pointer: 34,
        device: 10,
        kind: PointerDeviceKind::Stylus,
        position: Offset::new(50., 150.),
        change: HoverDeviceChange::Added,
    }));
    for (x, phase, pressure) in [
        (60., PointerPhase::Down, 0.5),
        (130., PointerPhase::Move, 0.55),
        (220., PointerPhase::Move, 0.7),
    ] {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 34,
            device: 10,
            kind: PointerDeviceKind::Stylus,
            position: Offset::new(x, 150.),
            phase,
            buttons: 1,
            button: (phase == PointerPhase::Down).then_some(1),
            sample: PointerSampleMetadata {
                pressure: NormalizedPressure::new(pressure),
                stylus: Some(stylus),
                contact,
            },
        });
    }
    let events = events.borrow();
    assert_eq!(events.len(), 2);
    for (event, phase, x, delta) in [
        (events[0], PointerPhase::Enter, 130., 70.),
        (events[1], PointerPhase::Exit, 220., 90.),
    ] {
        assert_eq!(event.pointer, 34);
        assert_eq!(event.device, 10);
        assert_eq!(event.kind, PointerDeviceKind::Stylus);
        assert_eq!(event.buttons, 1);
        assert!(event.down);
        assert_eq!(event.phase, phase);
        assert_eq!(event.position, Offset::new(x, 150.));
        assert_eq!(event.local_position, Offset::new(x - 100., 50.));
        assert_eq!(event.delta, Offset::new(delta, 0.));
        assert_eq!(event.local_delta, Offset::new(delta, 0.));
        assert_eq!(event.sample.stylus, Some(stylus));
        assert_eq!(event.sample.contact, contact);
        assert_eq!(event.sample.pressure.map(NormalizedPressure::get), Some(0.));
    }
}

#[test]
fn portable_boundary_constructors_keep_hover_and_pressed_samples_distinct() {
    use incular_widgets::RawPointerEvent;

    for down in [false, true] {
        let time = incular_core::time::Instant::now();
        let mut source = RawPointerEvent::new(
            if down { 34 } else { 0 },
            10,
            PointerDeviceKind::Mouse,
            7,
            Offset::new(101., 202.),
            PointerPhase::Move,
            time,
        )
        .with_sample(PointerSampleMetadata {
            pressure: NormalizedPressure::new(0.65),
            stylus: None,
            ..PointerSampleMetadata::EMPTY
        });
        source.down = down;
        source.synthesized = true;
        source.delta = Offset::new(5., 6.);
        source.local_position = Offset::new(51., 102.);
        source.local_delta = Offset::new(2.5, 3.);
        for (actual, phase) in [
            (source.as_mouse_enter(), PointerPhase::Enter),
            (source.as_mouse_exit(), PointerPhase::Exit),
        ] {
            let mut expected = source;
            expected.phase = phase;
            expected.sample.pressure = NormalizedPressure::new(0.);
            assert_eq!(actual, expected);
        }
    }
}

// Four exact fromMouseEvent registrations in the pinned events_test.dart.
#[test]
fn constructor_boundary_cases_keep_all_source_contact_geometry() {
    use incular_widgets::RawPointerEvent;

    let time = incular_core::time::Instant::now();
    for pressed in [false, true] {
        let mut pointer = RawPointerEvent::new(
            0,
            10,
            PointerDeviceKind::Unknown,
            7,
            Offset::new(101., 202.),
            PointerPhase::Move,
            time,
        )
        .with_sample(
            PointerSampleMetadata::new(NormalizedPressure::new(0.7), None).with_contact(
                PointerContactMetadata {
                    pressure_min: 1.1,
                    pressure_max: 2.1,
                    distance: if pressed { 0. } else { 11. },
                    distance_max: 110.,
                    size: 11.,
                    radius_major: 11.,
                    radius_minor: 9.,
                    radius_min: 1.1,
                    radius_max: 22.,
                    orientation: 1.1,
                    tilt: 1.1,
                    obscured: true,
                },
            ),
        );
        pointer.down = pressed;
        pointer.synthesized = true;

        for (event, phase) in [
            (pointer.as_mouse_enter(), PointerPhase::Enter),
            (pointer.as_mouse_exit(), PointerPhase::Exit),
        ] {
            let mut expected = pointer;
            expected.phase = phase;
            expected.sample.pressure = NormalizedPressure::new(0.);
            assert_eq!(event, expected);
        }
    }
}

#[test]
fn empty_contact_metadata_uses_flutter_defaults_without_assuming_hardware_force() {
    let sample = PointerSampleMetadata::default();
    assert_eq!(sample, PointerSampleMetadata::EMPTY);
    assert_eq!(sample.contact.pressure_min, 1.);
    assert_eq!(sample.contact.pressure_max, 1.);
    assert_eq!(sample.contact.distance, 0.);
    assert_eq!(sample.contact.distance_max, 0.);
    assert_eq!(sample.contact.size, 0.);
    assert_eq!(sample.contact.radius_major, 0.);
    assert_eq!(sample.contact.radius_minor, 0.);
    assert_eq!(sample.contact.radius_min, 0.);
    assert_eq!(sample.contact.radius_max, 0.);
    assert_eq!(sample.contact.orientation, 0.);
    assert_eq!(sample.contact.tilt, 0.);
    assert!(!sample.contact.obscured);
    assert_eq!(sample.pressure, None);
    assert_eq!(sample.stylus, None);
}
