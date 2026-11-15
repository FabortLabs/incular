//! Flutter's slop helpers (`gestures/events.dart` `computeHitSlop`,
//! `computePanSlop`, `computeScaleSlop`): how far a pointer may move before
//! it stops being a tap or starts a drag, by device kind and the platform's
//! [`DeviceGestureSettings`].

use incular_core::DeviceGestureSettings;

use crate::constants::{
    PAN_SLOP, PRECISE_POINTER_HIT_SLOP, PRECISE_POINTER_PAN_SLOP, PRECISE_POINTER_SCALE_SLOP,
    SCALE_SLOP, TOUCH_SLOP,
};
use crate::details::PointerDeviceKind;

/// Flutter `computeHitSlop`: the tap tolerance and single-axis drag
/// acceptance distance. Mice use `kPrecisePointerHitSlop`; every other kind
/// uses the settings' touch slop, or `kTouchSlop` when it is unset.
#[must_use]
pub fn compute_hit_slop(kind: PointerDeviceKind, settings: DeviceGestureSettings) -> f32 {
    match kind {
        PointerDeviceKind::Mouse => PRECISE_POINTER_HIT_SLOP,
        PointerDeviceKind::Touch
        | PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Trackpad
        | PointerDeviceKind::Unknown => settings.touch_slop.unwrap_or(TOUCH_SLOP),
    }
}

/// Flutter `computePanSlop`: the pan acceptance distance. Mice use
/// `kPrecisePointerPanSlop`; every other kind uses the settings' pan slop
/// (twice the touch slop), or `kPanSlop` when it is unset.
#[must_use]
pub fn compute_pan_slop(kind: PointerDeviceKind, settings: DeviceGestureSettings) -> f32 {
    match kind {
        PointerDeviceKind::Mouse => PRECISE_POINTER_PAN_SLOP,
        PointerDeviceKind::Touch
        | PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Trackpad
        | PointerDeviceKind::Unknown => settings.pan_slop().unwrap_or(PAN_SLOP),
    }
}

/// Flutter `computeScaleSlop`: how far the span must change before a scale
/// is accepted. Flutter's settings carry no scale slop, so only the device
/// kind matters.
#[must_use]
pub fn compute_scale_slop(kind: PointerDeviceKind) -> f32 {
    match kind {
        PointerDeviceKind::Mouse => PRECISE_POINTER_SCALE_SLOP,
        PointerDeviceKind::Touch
        | PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Trackpad
        | PointerDeviceKind::Unknown => SCALE_SLOP,
    }
}
