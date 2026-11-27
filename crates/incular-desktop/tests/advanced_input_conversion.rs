use incular_core::{
    InputEvent, NormalizedPressure, PointerContactMetadata, PointerDeviceKind,
    PointerSampleMetadata, SECONDARY_POINTER_BUTTON, StylusMetadata, StylusOrientation,
    TrackpadGesture, TrackpadGesturePhase,
};
use incular_desktop::winit_adapter::{
    touch_event_with_native_sample, trackpad_pinch_event, trackpad_rotation_event,
};
use incular_platform::{NativePointerSample, PhysicalSize, PlatformEvent, WindowMetrics};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, Touch, TouchPhase},
};

fn touch(phase: TouchPhase) -> Touch {
    Touch {
        device_id: DeviceId::dummy(),
        phase,
        location: PhysicalPosition::new(40.0, 20.0),
        force: None,
        id: 7,
    }
}

#[test]
fn native_stylus_metadata_preserves_inversion_barrel_hover_and_device_identity() {
    let sample = NativePointerSample {
        device: Some(99),
        kind: PointerDeviceKind::InvertedStylus,
        sample: PointerSampleMetadata {
            pressure: Some(NormalizedPressure::new(0.4).unwrap()),
            stylus: Some(StylusMetadata {
                orientation: StylusOrientation::from_tilt_degrees(30.0, -15.0),
                barrel_button: true,
            }),
            contact: PointerContactMetadata {
                raw_pressure: None,
                pressure_min: 0.,
                pressure_max: 1024.,
                distance: 4.,
                obscured: true,
                ..PointerContactMetadata::DEFAULT
            },
        },
        in_contact: Some(false),
    };
    let event = touch_event_with_native_sample(
        touch(TouchPhase::Moved),
        5,
        Some(sample),
        WindowMetrics::new(PhysicalSize::new(200, 100), 2.0),
    );
    let PlatformEvent::Input(InputEvent::PointerWithMetadata {
        pointer,
        device,
        kind,
        buttons,
        button,
        sample: actual,
        position,
        ..
    }) = event
    else {
        panic!("expected rich pointer event");
    };
    assert_eq!(pointer, 7);
    assert_eq!(device, 99);
    assert_eq!(kind, PointerDeviceKind::InvertedStylus);
    assert_eq!(buttons, SECONDARY_POINTER_BUTTON);
    assert_eq!(button, None);
    assert_eq!(actual, sample.sample);
    assert_eq!(position.x, 20.0);
    assert_eq!(position.y, 10.0);
}

#[test]
fn pinch_and_rotation_keep_native_phase_order_and_normalized_units() {
    let phases = [TouchPhase::Started, TouchPhase::Moved, TouchPhase::Ended];
    let expected = [
        TrackpadGesturePhase::Started,
        TrackpadGesturePhase::Updated,
        TrackpadGesturePhase::Ended,
    ];
    for (phase, expected) in phases.into_iter().zip(expected) {
        let PlatformEvent::Input(InputEvent::TrackpadGesture(TrackpadGesture::Pinch {
            device,
            phase,
            magnification_delta,
        })) = trackpad_pinch_event(3, 0.125, phase).unwrap()
        else {
            panic!("expected pinch");
        };
        assert_eq!(device, 3);
        assert_eq!(phase, expected);
        assert_eq!(magnification_delta, 0.125);
    }

    let PlatformEvent::Input(InputEvent::TrackpadGesture(TrackpadGesture::Rotation {
        phase,
        delta_radians,
        ..
    })) = trackpad_rotation_event(4, 180.0, TouchPhase::Moved).unwrap()
    else {
        panic!("expected rotation");
    };
    assert_eq!(phase, TrackpadGesturePhase::Updated);
    assert!((delta_radians - std::f32::consts::PI).abs() < 1e-6);

    assert!(trackpad_pinch_event(3, f64::NAN, TouchPhase::Moved).is_none());
}

#[test]
fn calibrated_touch_pressure_survives_routing_separately_from_gesture_force() {
    use std::{cell::Cell, rc::Rc};

    use incular_config::Constraints;
    use incular_core::{Color, Size};
    use incular_runtime::Runtime;
    use incular_widgets::{Listener, Widget};

    let metrics = WindowMetrics::new(PhysicalSize::new(200, 100), 2.0);
    let received = Rc::new(Cell::new(None));
    let observed = Rc::clone(&received);
    let widget = Listener::new(Widget::box_(Size::new(100., 50.), Color::WHITE))
        .on_pointer_move(move |event| observed.set(Some(event)));
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(100., 50.)))
        .unwrap();

    let down = touch_event_with_native_sample(touch(TouchPhase::Started), 5, None, metrics);
    let _ = runtime.handle_platform_event(down);

    let mut input = touch(TouchPhase::Moved);
    input.location.x += 2.;
    input.force = Some(winit::event::Force::Calibrated {
        force: 4.,
        max_possible_force: 10.,
        // Angle correction changes normalized gesture force, not the raw sample.
        altitude_angle: Some(std::f64::consts::FRAC_PI_6),
    });
    let converted = touch_event_with_native_sample(input, 5, None, metrics);
    let _ = runtime.handle_platform_event(converted);
    let event = received
        .get()
        .expect("listener sees the calibrated contact move");
    assert_eq!(event.pressure(), 4.);
    assert_eq!(event.sample.contact.pressure_min, 0.);
    assert_eq!(event.sample.contact.pressure_max, 10.);
    assert_eq!(event.sample.pressure, NormalizedPressure::new(0.8));
}

#[test]
fn native_calibration_is_not_mixed_with_winit_force_units() {
    let mut input = touch(TouchPhase::Moved);
    input.force = Some(winit::event::Force::Normalized(0.4));
    for raw_pressure in [None, Some(256.)] {
        let native = NativePointerSample {
            device: None,
            kind: PointerDeviceKind::Stylus,
            sample: PointerSampleMetadata::EMPTY.with_contact(PointerContactMetadata {
                raw_pressure,
                pressure_min: 0.,
                pressure_max: 1024.,
                ..PointerContactMetadata::DEFAULT
            }),
            in_contact: Some(true),
        };
        let event = touch_event_with_native_sample(
            input,
            5,
            Some(native),
            WindowMetrics::new(PhysicalSize::new(200, 100), 2.0),
        );
        let PlatformEvent::Input(InputEvent::PointerWithMetadata { sample, .. }) = event else {
            panic!("expected rich pointer event");
        };
        assert_eq!(sample.contact.raw_pressure, raw_pressure);
        assert_eq!(sample.contact.pressure_max, 1024.);
        assert_eq!(sample.pressure, NormalizedPressure::new(0.4));
    }
}
