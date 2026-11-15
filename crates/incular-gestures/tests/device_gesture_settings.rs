//! Flutter `DeviceGestureSettings` (`gestures/gesture_settings.dart`) and the
//! settings-aware slop helpers of `gestures/events.dart`: a platform touch
//! slop replaces `kTouchSlop` for touch-class pointers and doubles into the
//! pan slop; mice keep the precise-pointer slops (gestures-013).

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_core::{Offset, PointerPhase};
use incular_gestures::constants::{
    PAN_SLOP, PRECISE_POINTER_HIT_SLOP, PRECISE_POINTER_PAN_SLOP, TOUCH_SLOP,
};
use incular_gestures::*;

const TOUCH_KINDS: [PointerDeviceKind; 5] = [
    PointerDeviceKind::Touch,
    PointerDeviceKind::Stylus,
    PointerDeviceKind::InvertedStylus,
    PointerDeviceKind::Trackpad,
    PointerDeviceKind::Unknown,
];

#[test]
fn unset_settings_keep_flutter_default_slops() {
    let settings = DeviceGestureSettings::default();
    assert_eq!(settings.touch_slop, None);
    assert_eq!(settings.pan_slop(), None);
    for kind in TOUCH_KINDS {
        assert_eq!(compute_hit_slop(kind, settings), TOUCH_SLOP);
        assert_eq!(compute_pan_slop(kind, settings), PAN_SLOP);
    }
}

#[test]
fn touch_slop_overrides_touch_class_kinds_and_doubles_into_the_pan_slop() {
    let settings = DeviceGestureSettings::new(Some(8.));
    assert_eq!(settings.pan_slop(), Some(16.));
    for kind in TOUCH_KINDS {
        assert_eq!(compute_hit_slop(kind, settings), 8.);
        assert_eq!(compute_pan_slop(kind, settings), 16.);
    }
    // `computeHitSlop` and `computePanSlop` ignore the settings for mice.
    assert_eq!(
        compute_hit_slop(PointerDeviceKind::Mouse, settings),
        PRECISE_POINTER_HIT_SLOP
    );
    assert_eq!(
        compute_pan_slop(PointerDeviceKind::Mouse, settings),
        PRECISE_POINTER_PAN_SLOP
    );
}

#[test]
fn from_physical_divides_by_the_device_pixel_ratio() {
    // `DeviceGestureSettings.fromView`: physicalTouchSlop / devicePixelRatio.
    assert_eq!(
        DeviceGestureSettings::from_physical(Some(24.), 2.0).touch_slop,
        Some(12.)
    );
    assert_eq!(
        DeviceGestureSettings::from_physical(None, 2.0),
        DeviceGestureSettings::default()
    );
    for (slop, ratio) in [(f32::NAN, 2.0), (-1., 2.0), (24., 0.0), (24., f64::NAN)] {
        assert_eq!(
            DeviceGestureSettings::from_physical(Some(slop), ratio),
            DeviceGestureSettings::default(),
            "slop {slop} at ratio {ratio}"
        );
    }
}

fn event(position: Offset, phase: PointerPhase, base: Instant, millis: u64) -> PointerEvent {
    PointerEvent {
        pointer: 1,
        position,
        phase,
        time: base + Duration::from_millis(millis),
    }
}

/// Presses, moves `distance` and reports whether the long press was
/// cancelled for moving beyond the hit slop.
fn long_press_cancelled_after(distance: f32, settings: Option<DeviceGestureSettings>) -> bool {
    let cancelled = Rc::new(Cell::new(false));
    let callbacks = GestureCallbacks {
        on_long_press_down: Some(Rc::new(|_: LongPressDownDetails| {})),
        on_long_press_cancel: Some({
            let cancelled = cancelled.clone();
            Rc::new(move || cancelled.set(true))
        }),
        on_long_press: Some(Rc::new(|| {})),
        ..GestureCallbacks::default()
    };
    let mut recognizer = PointerGestureRecognizer::new(callbacks);
    if let Some(settings) = settings {
        recognizer.set_gesture_settings(settings);
    }
    let base = Instant::now();
    let origin = Offset::new(8., 8.);
    recognizer.handle(event(origin, PointerPhase::Down, base, 0));
    recognizer.handle(event(
        origin + Offset::new(0., distance),
        PointerPhase::Move,
        base,
        20,
    ));
    cancelled.get()
}

#[test]
fn pointer_recognizer_uses_the_settings_touch_slop() {
    // Moving 10 stays within kTouchSlop (18) by default.
    assert!(!long_press_cancelled_after(10., None));
    // A platform touch slop of 6 makes the same movement leave the slop.
    let settings = DeviceGestureSettings::new(Some(6.));
    assert!(long_press_cancelled_after(10., Some(settings)));
    assert!(!long_press_cancelled_after(6., Some(settings)));
    let recognizer = {
        let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks::default());
        recognizer.set_gesture_settings(settings);
        recognizer
    };
    assert_eq!(recognizer.gesture_settings(), settings);
}

#[test]
fn scale_recognizer_uses_the_settings_pan_slop_for_the_focal_point() {
    let started = Rc::new(Cell::new(false));
    let mut detector = ScaleGestureDetector::with_callbacks(
        None,
        Some({
            let started = started.clone();
            Rc::new(move |_: ScaleStartDetails| started.set(true))
        }),
        None,
    )
    .with_gesture_settings(DeviceGestureSettings::new(Some(5.)));
    let raw = |x: f32, phase| {
        RawPointerEvent::new(
            1,
            0,
            PointerDeviceKind::Touch,
            0,
            Offset::new(x, 0.),
            phase,
            Instant::now(),
        )
    };
    detector.handle_raw(raw(0., PointerPhase::Down));
    // The pan slop is twice the touch slop: 10, not kPanSlop (36).
    detector.handle_raw(raw(10., PointerPhase::Move));
    assert!(!started.get());
    detector.handle_raw(raw(11., PointerPhase::Move));
    assert!(started.get());
}
