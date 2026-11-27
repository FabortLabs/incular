//! Platform-neutral input values.

use crate::{Offset, Size};
mod flutter_keyboard;
pub use flutter_keyboard::{LogicalKeyboardKey, PhysicalKeyboardKey};

/// Whether a logical key label is exactly one C0 or C1 control character,
/// matching Flutter's `LogicalKeyboardKey.isControlCharacter`.
/// Empty labels, key names, and multi-character labels do not qualify.
#[must_use]
pub fn is_control_key_label(label: &str) -> bool {
    let mut chars = label.chars();
    matches!(chars.next(), Some('\0'..='\x1f' | '\x7f'..='\u{9f}')) && chars.next().is_none()
}

/// Pressure normalized to the closed interval `[0, 1]`.
///
/// Native adapters normalize once at their boundary. Values outside a native
/// device's documented range are clamped, while non-finite values and invalid
/// ranges remain unavailable instead of entering retained input state.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct NormalizedPressure(f32);

impl NormalizedPressure {
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        value
            .is_finite()
            .then(|| Self(value.clamp(0.0, 1.0) as f32))
    }

    #[must_use]
    pub fn from_range(value: f64, maximum: f64) -> Option<Self> {
        (value.is_finite() && maximum.is_finite() && maximum > 0.0)
            .then(|| Self((value / maximum).clamp(0.0, 1.0) as f32))
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Portable stylus orientation derived from native pen tilt.
///
/// `altitude` is radians above the tablet plane: `0` is parallel and `PI/2`
/// is perpendicular. `azimuth` is radians in `[0, TAU)`, clockwise from the
/// positive logical X axis in Incular's screen coordinate system (+Y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StylusOrientation {
    pub altitude: f32,
    pub azimuth: f32,
}

impl StylusOrientation {
    /// Converts Windows-style signed X/Y tilt angles (degrees from the surface
    /// normal) into Incular's altitude/azimuth representation.
    #[must_use]
    pub fn from_tilt_degrees(tilt_x: f64, tilt_y: f64) -> Option<Self> {
        if !tilt_x.is_finite()
            || !tilt_y.is_finite()
            || !(-90.0..=90.0).contains(&tilt_x)
            || !(-90.0..=90.0).contains(&tilt_y)
        {
            return None;
        }
        let x = tilt_x.to_radians().tan();
        let y = tilt_y.to_radians().tan();
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let altitude = 1.0_f64.atan2(x.hypot(y));
        let azimuth = y.atan2(x).rem_euclid(std::f64::consts::TAU);
        Some(Self {
            altitude: altitude as f32,
            azimuth: azimuth as f32,
        })
    }
}

/// Stylus-only sample data with semantics shared by supported native backends.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StylusMetadata {
    pub orientation: Option<StylusOrientation>,
    /// Whether the pen's barrel/secondary switch is currently depressed.
    pub barrel_button: bool,
}

/// Flutter-style contact measurements supplied by the pointer source.
/// Distances and radii are in logical pixels, orientation and tilt in radians.
/// Pressure bounds describe the source rather than the normalized force used
/// for gesture thresholds; enter and exit events retain those bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerContactMetadata {
    /// Raw pressure in device units, independent of normalized gesture force.
    pub raw_pressure: Option<f32>,
    pub pressure_min: f32,
    pub pressure_max: f32,
    pub distance: f32,
    pub distance_max: f32,
    pub size: f32,
    pub radius_major: f32,
    pub radius_minor: f32,
    pub radius_min: f32,
    pub radius_max: f32,
    pub orientation: f32,
    pub tilt: f32,
    pub obscured: bool,
}

impl PointerContactMetadata {
    /// Pointer defaults when no backend measurement is supplied.
    pub const DEFAULT: Self = Self {
        raw_pressure: None,
        pressure_min: 1.0,
        pressure_max: 1.0,
        distance: 0.0,
        distance_max: 0.0,
        size: 0.0,
        radius_major: 0.0,
        radius_minor: 0.0,
        radius_min: 0.0,
        radius_max: 0.0,
        orientation: 0.0,
        tilt: 0.0,
        obscured: false,
    };
}

impl Default for PointerContactMetadata {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Optional high-fidelity data attached to one pointer sample.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointerSampleMetadata {
    pub pressure: Option<NormalizedPressure>,
    pub stylus: Option<StylusMetadata>,
    pub contact: PointerContactMetadata,
}

impl PointerSampleMetadata {
    pub const EMPTY: Self = Self {
        pressure: None,
        stylus: None,
        contact: PointerContactMetadata::DEFAULT,
    };

    /// Builds the original normalized-pressure and stylus sample.
    #[must_use]
    pub const fn new(
        pressure: Option<NormalizedPressure>,
        stylus: Option<StylusMetadata>,
    ) -> Self {
        Self { pressure, stylus, ..Self::EMPTY }
    }

    /// Replaces the source contact metadata without modifying normalized force.
    #[must_use]
    pub const fn with_contact(mut self, contact: PointerContactMetadata) -> Self {
        self.contact = contact;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrackpadGesturePhase {
    Started,
    Updated,
    Ended,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrackpadGestureKind {
    Pinch,
    Rotation,
    Pan,
    SmartMagnify,
    Pressure,
}

/// Phase of the absolute pointer pan/zoom protocol.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerPanZoomPhase {
    Start,
    Update {
        /// Translation relative to the gesture's start, in global coordinates.
        pan: Offset,
        /// Multiplicative scale relative to the gesture's start (initially 1).
        scale: f32,
        /// Rotation in radians relative to the gesture's start.
        rotation: f32,
    },
    End,
}

/// A trackpad gesture stream. Unlike a platform delta, each update describes
/// the full motion since Start; do not sum successive updates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerPanZoomEvent {
    pub device: u64,
    pub pointer: u64,
    pub position: Offset,
    pub phase: PointerPanZoomPhase,
    /// Timestamp from the same monotonic clock as the runtime.
    pub time: crate::time::Instant,
    pub synthesized: bool,
}

/// Aggregate native trackpad gesture. These events deliberately do not contain
/// synthetic touch contacts: platforms that expose only magnification,
/// rotation, or force keep those aggregate semantics intact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrackpadGesture {
    Pinch {
        device: u64,
        phase: TrackpadGesturePhase,
        magnification_delta: f32,
    },
    Rotation {
        device: u64,
        phase: TrackpadGesturePhase,
        /// Counter-clockwise rotation in radians for this native update.
        delta_radians: f32,
    },
    Pan {
        device: u64,
        phase: TrackpadGesturePhase,
        delta: Offset,
    },
    SmartMagnify {
        device: u64,
    },
    Pressure {
        device: u64,
        pressure: NormalizedPressure,
        stage: i64,
    },
}

impl TrackpadGesture {
    #[must_use]
    pub const fn device(self) -> u64 {
        match self {
            Self::Pinch { device, .. }
            | Self::Rotation { device, .. }
            | Self::Pan { device, .. }
            | Self::SmartMagnify { device }
            | Self::Pressure { device, .. } => device,
        }
    }

    #[must_use]
    pub const fn kind(self) -> TrackpadGestureKind {
        match self {
            Self::Pinch { .. } => TrackpadGestureKind::Pinch,
            Self::Rotation { .. } => TrackpadGestureKind::Rotation,
            Self::Pan { .. } => TrackpadGestureKind::Pan,
            Self::SmartMagnify { .. } => TrackpadGestureKind::SmartMagnify,
            Self::Pressure { .. } => TrackpadGestureKind::Pressure,
        }
    }

    #[must_use]
    pub const fn phase(self) -> Option<TrackpadGesturePhase> {
        match self {
            Self::Pinch { phase, .. } | Self::Rotation { phase, .. } | Self::Pan { phase, .. } => {
                Some(phase)
            }
            Self::SmartMagnify { .. } | Self::Pressure { .. } => None,
        }
    }
}

/// Standardized keyboard values shared by every platform adapter.
///
/// [`Code`] identifies the physical key position; [`Key`] is the logical key
/// value after the operating system's layout processing.  Incular does not
/// maintain a parallel keyboard vocabulary.
pub use keyboard_types::{Code, Key, KeyState, KeyboardEvent, Location, Modifiers, NamedKey};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerPhase {
    /// A hover-capable pointer entered the native surface. No widget-local
    /// position is implied until a subsequent [`Self::Move`] arrives.
    Enter,
    Move,
    Down,
    Up,
    Cancel,
    /// A hover-capable pointer left the native surface.
    Exit,
}

/// Physical device category for pointer input shared by platform adapters and
/// raw widget routing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PointerDeviceKind {
    #[default]
    Mouse,
    Touch,
    Stylus,
    InvertedStylus,
    Trackpad,
    Unknown,
}

/// Flutter's `DeviceGestureSettings` (`gestures/gesture_settings.dart`):
/// platform gesture tolerances in logical pixels. A `None` slop keeps the
/// framework default (`kTouchSlop`, `kPanSlop`), which is what Flutter's
/// desktop and web embedders report.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DeviceGestureSettings {
    /// Flutter `touchSlop`: how far a touch may move before it counts as a
    /// drag.
    pub touch_slop: Option<f32>,
}

impl DeviceGestureSettings {
    /// Settings with the given touch slop.
    #[must_use]
    pub const fn new(touch_slop: Option<f32>) -> Self {
        Self { touch_slop }
    }

    /// Flutter `DeviceGestureSettings.fromView`: converts the view's
    /// physical touch slop to logical pixels. A missing, non-finite or
    /// negative slop, or an invalid ratio, leaves the default.
    #[must_use]
    pub fn from_physical(physical_touch_slop: Option<f32>, device_pixel_ratio: f64) -> Self {
        let touch_slop = physical_touch_slop.and_then(|slop| {
            let ratio = device_pixel_ratio as f32;
            (slop.is_finite() && slop >= 0.0 && ratio.is_finite() && ratio > 0.0)
                .then_some(slop / ratio)
        });
        Self { touch_slop }
    }

    /// Flutter `panSlop`: twice the touch slop, when one is set.
    #[must_use]
    pub fn pan_slop(self) -> Option<f32> {
        self.touch_slop.map(|slop| slop * 2.0)
    }
}

/// Flutter `DragStartBehavior`: where a drag or scale gesture measures its
/// starting point from.
///
/// Widgets default to [`Self::Start`]; Flutter's bare `ScaleGestureRecognizer`
/// defaults to [`Self::Down`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DragStartBehavior {
    /// Measure from where the gesture was accepted (after any slop).
    #[default]
    Start,
    /// Measure from where the pointer first went down.
    Down,
}

/// Edge/corner selected for a compositor-owned native window resize gesture.
///
/// This lives in `incular-core` because retained widgets can describe resize
/// hit regions without depending on a platform window implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowResizeDirection {
    East,
    North,
    NorthEast,
    NorthWest,
    South,
    SouthEast,
    SouthWest,
    West,
}

/// Primary-button bit in Incular's portable pointer-button mask.
///
/// Raw pointer metadata follows Flutter's button-mask convention so the value
/// is stable across platform adapters. Zero remains valid for legacy adapters
/// that do not publish button metadata.
pub const PRIMARY_POINTER_BUTTON: u32 = 1 << 0;
/// Secondary/right mouse button.
pub const SECONDARY_POINTER_BUTTON: u32 = 1 << 1;
/// Tertiary/middle mouse button.
pub const TERTIARY_POINTER_BUTTON: u32 = 1 << 2;
/// Conventional browser/navigation back button.
pub const BACK_POINTER_BUTTON: u32 = 1 << 3;
/// Conventional browser/navigation forward button.
pub const FORWARD_POINTER_BUTTON: u32 = 1 << 4;

/// Returns the portable mask for an additional mouse button.
///
/// `index == 0` is the first button beyond primary/secondary/tertiary/back/
/// forward. The `u32` event mask has room for 27 such buttons; larger native
/// button indices are intentionally unrepresentable instead of colliding with
/// an existing bit.
#[must_use]
pub const fn additional_pointer_button_mask(index: u16) -> Option<u32> {
    let bit = 5_u32 + index as u32;
    if bit < u32::BITS {
        Some(1_u32 << bit)
    } else {
        None
    }
}

/// IME composition is separate from committed text. Byte ranges always refer
/// to valid UTF-8 boundaries in the preedit string when supplied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImeEvent {
    Preedit {
        text: String,
        selection: Option<(usize, usize)>,
    },
    Commit(String),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    Pointer {
        phase: PointerPhase,
        position: Offset,
    },
    /// Identified pointer input for touch, pen, and multi-contact touchpads.
    /// The existing [`Self::Pointer`] form is retained as pointer `0` for
    /// source compatibility with mouse-only adapters.
    PointerWithId {
        /// Stable for the duration of one pointer sequence.
        pointer: u64,
        phase: PointerPhase,
        position: Offset,
    },
    /// Fully identified pointer input. The two legacy pointer forms remain
    /// available for adapters that do not expose device/button metadata.
    PointerWithMetadata {
        /// Stable for the duration of one pointer sequence.
        pointer: u64,
        /// Stable identifier for the physical device, when available.
        device: u64,
        /// Physical category of the source device.
        kind: PointerDeviceKind,
        /// Pressed-button bit mask; zero is used for hover.
        buttons: u32,
        /// Button whose state changed for a Down/Up event. `None` is used for
        /// movement, surface enter/exit, cancellation, and adapters that do not
        /// expose the changed native button.
        button: Option<u32>,
        /// Optional pressure/orientation data supplied by the native device.
        sample: PointerSampleMetadata,
        phase: PointerPhase,
        position: Offset,
    },
    /// Native aggregate trackpad gesture. No fake pointer contacts are created
    /// from this event.
    TrackpadGesture(TrackpadGesture),
    /// Correlated trackpad pan/zoom reported with a real position and
    /// absolute motion by an adapter that supports this protocol.
    PointerPanZoom(PointerPanZoomEvent),
    /// Wheel input whose location comes from the latest pointer sample.
    Scroll {
        delta: Offset,
    },
    /// Wheel input with a location reported by the platform, in logical pixels.
    ScrollAt {
        position: Offset,
        delta: Offset,
    },
    Key(KeyboardEvent),
    Text(String),
    Ime(ImeEvent),
    WindowResized {
        size: Size,
        scale_factor: f64,
    },
}
