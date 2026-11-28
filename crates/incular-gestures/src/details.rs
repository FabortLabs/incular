use std::{
    ops::{Add, Sub},
    rc::Rc,
};

use incular_core::time::Instant;

use incular_core::{
    DragStartBehavior, KeyboardEvent, Matrix4, NormalizedPressure, Offset, PRIMARY_POINTER_BUTTON,
    Point64, PointerPhase, PointerSampleMetadata, SECONDARY_POINTER_BUTTON,
    TERTIARY_POINTER_BUTTON, TrackpadGesture, Transform,
};

use crate::focus::{FocusBehavior, FocusNode};
use crate::keyboard::{
    ActionInvocationSubscription, ActionListenerSubscription, ErasedActionScope,
    ErasedShortcutScope, FocusKeyEventHandler, StatefulShortcutHandler,
};

use crate::pointer_converter::PointerEventProjection;
use crate::scale::{ScaleEndDetails, ScaleStartDetails, ScaleUpdateDetails};

/// Matching offsets in a target's local space and the global logical space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OffsetPair {
    pub local: Offset,
    pub global: Offset,
}

impl OffsetPair {
    pub const ZERO: Self = Self::new(Offset::ZERO, Offset::ZERO);

    #[must_use]
    pub const fn new(local: Offset, global: Offset) -> Self {
        Self { local, global }
    }

    #[must_use]
    pub const fn from_event_position(event: &RawPointerEvent) -> Self {
        Self::new(event.local_position, event.position)
    }

    #[must_use]
    pub const fn from_event_delta(event: &RawPointerEvent) -> Self {
        Self::new(event.local_delta, event.delta)
    }
}

impl Add for OffsetPair {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.local + other.local, self.global + other.global)
    }
}

impl Sub for OffsetPair {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.local - other.local, self.global - other.global)
    }
}

/// A platform-neutral pointer event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub pointer: u64,
    pub position: Offset,
    pub phase: PointerPhase,
    pub time: Instant,
}

/// A pointer event carrying the metadata needed by raw input widgets.
///
/// [`PointerEvent`] predates device-aware routing and remains intentionally
/// small for source compatibility. Raw widgets use this value so a retained
/// route can preserve the pointer stream, physical device, pressed buttons,
/// phase, and event timestamp without coupling the widget crate to a native
/// event type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawPointerEvent {
    /// Stable for the duration of one pointer sequence.
    pub pointer: u64,
    /// Platform device identifier. Zero means that the source did not supply
    /// a distinct device identifier.
    pub device: u64,
    /// Physical category of the device producing the event.
    pub kind: PointerDeviceKind,
    /// Flutter-compatible button bit mask. Zero is used for hover and for
    /// adapters that do not expose button state.
    pub buttons: u32,
    /// Button whose state changed for a Down/Up event, when the source can
    /// identify it independently from the complete pressed-button chord.
    pub button: Option<u32>,
    pub sample: PointerSampleMetadata,
    pub position: Offset,
    pub phase: PointerPhase,
    pub time: Instant,
    /// Flutter `delta`: movement since this pointer's previous event. The
    /// retained tree fills it in; zero for a press.
    pub delta: Offset,
    /// Flutter `localPosition`: [`Self::position`] in the receiving
    /// widget's coordinates. The retained tree sets it per listener; it
    /// equals the position until then.
    pub local_position: Offset,
    /// Flutter `localDelta`: [`Self::delta`] in the receiving widget's
    /// coordinates.
    pub local_delta: Offset,
    /// Flutter `down`: whether the pointer is in contact. A physical
    /// packet can explicitly distinguish contact motion from hover even with
    /// no buttons; the coarse constructor infers motion from the mask.
    pub down: bool,
    /// Flutter `synthesized`: the platform did not report this event; the
    /// framework derived it from others.
    pub synthesized: bool,
    /// Flutter `PointerScrollEvent.scrollDelta`: the wheel movement of a
    /// pointer signal, positive when content scrolls down or right. Zero for
    /// other events.
    pub scroll_delta: Offset,
    /// Wheel movement mapped into the receiving widget's coordinates.
    pub local_scroll_delta: Offset,
    /// Multiplicative scale for a discrete pointer scale signal.
    pub scale_signal: Option<f32>,
}

impl RawPointerEvent {
    #[must_use]
    pub const fn new(
        pointer: u64,
        device: u64,
        kind: PointerDeviceKind,
        buttons: u32,
        position: Offset,
        phase: PointerPhase,
        time: Instant,
    ) -> Self {
        Self {
            pointer,
            device,
            kind,
            buttons,
            button: None,
            sample: PointerSampleMetadata::EMPTY,
            position,
            phase,
            time,
            delta: Offset::ZERO,
            local_position: position,
            local_delta: Offset::ZERO,
            down: pointer_is_down(phase, buttons),
            synthesized: false,
            scroll_delta: Offset::ZERO,
            local_scroll_delta: Offset::ZERO,
            scale_signal: None,
        }
    }

    /// Creates a synthetic contact press with Flutter's primary button
    /// by default. Supply a button mask, including zero, to override it.
    /// Native adapters with explicit phase and button data can use [Self::new].
    #[must_use]
    pub const fn contact_down(
        pointer: u64,
        device: u64,
        kind: PointerDeviceKind,
        position: Offset,
        time: Instant,
        buttons: Option<u32>,
    ) -> Self {
        let buttons = match buttons {
            Some(buttons) => buttons,
            None => PRIMARY_POINTER_BUTTON,
        };
        Self::new(
            pointer,
            device,
            kind,
            buttons,
            position,
            PointerPhase::Down,
            time,
        )
    }

    /// Creates synthetic motion while a pointer is in contact. Like Flutter,
    /// this defaults to the primary button and remains down even if the
    /// caller explicitly supplies a zero button mask.
    #[must_use]
    pub const fn contact_move(
        pointer: u64,
        device: u64,
        kind: PointerDeviceKind,
        position: Offset,
        delta: Offset,
        time: Instant,
        buttons: Option<u32>,
    ) -> Self {
        let buttons = match buttons {
            Some(buttons) => buttons,
            None => PRIMARY_POINTER_BUTTON,
        };
        Self {
            delta,
            local_delta: delta,
            down: true,
            ..Self::new(
                pointer,
                device,
                kind,
                buttons,
                position,
                PointerPhase::Move,
                time,
            )
        }
    }

    /// Hover follows a physical device across contact sequences. When a
    /// host has no physical device ID, its pointer ID is the fallback.
    #[doc(hidden)]
    #[must_use]
    pub const fn hover_device_identity(&self) -> (u64, u64) {
        if self.device == 0 {
            (0, self.pointer)
        } else {
            (self.device, 0)
        }
    }

    /// Creates a hover annotation's enter event from a pointer sample.
    /// Contact state and motion are preserved, but a boundary is not a
    /// hardware pressure reading or a change to any individual button.
    #[must_use]
    pub fn as_mouse_enter(self) -> Self {
        self.as_mouse_boundary(PointerPhase::Enter)
    }

    /// Creates the corresponding exit event, with the same field policy.
    #[must_use]
    pub fn as_mouse_exit(self) -> Self {
        self.as_mouse_boundary(PointerPhase::Exit)
    }

    fn as_mouse_boundary(self, phase: PointerPhase) -> Self {
        Self {
            phase,
            sample: PointerSampleMetadata {
                pressure: Some(NormalizedPressure::default()),
                ..self.sample
            },
            button: None,
            scroll_delta: Offset::ZERO,
            local_scroll_delta: Offset::ZERO,
            scale_signal: None,
            ..self
        }
    }

    /// Flutter pressure for this phase. The device measurement is not
    /// normalized for gestures: those continue to read `sample.pressure`.
    #[must_use]
    pub fn pressure(&self) -> f32 {
        let fallback = match self.phase {
            PointerPhase::Down => 1.0,
            PointerPhase::Move if self.down => 1.0,
            PointerPhase::Up => 0.0,
            _ => return 0.0,
        };
        self.sample
            .contact
            .raw_pressure
            .filter(|value| value.is_finite())
            .unwrap_or(fallback)
    }

    /// Records the button whose state changed for a Down/Up transition.
    #[must_use]
    pub const fn with_button(mut self, button: Option<u32>) -> Self {
        self.button = button;
        self
    }

    /// A pointer signal (Flutter `PointerScrollEvent`) at `position`.
    #[must_use]
    pub fn scroll(
        pointer: u64,
        device: u64,
        kind: PointerDeviceKind,
        position: Offset,
        scroll_delta: Offset,
        time: Instant,
    ) -> Self {
        Self {
            scroll_delta,
            local_scroll_delta: scroll_delta,
            ..Self::new(pointer, device, kind, 0, position, PointerPhase::Move, time)
        }
    }

    #[must_use]
    pub const fn with_sample(mut self, sample: PointerSampleMetadata) -> Self {
        self.sample = sample;
        self
    }

    /// Maps global position into an optional local affine transform.
    #[must_use]
    pub fn transform_position(transform: Option<Transform>, position: Offset) -> Offset {
        transform.map_or(position, |value| value.transform_point(position))
    }

    /// Maps a displacement between two positions. If the transformed end
    /// position is already known, use it. Otherwise the affine linear part
    /// avoids subtracting two rounded positions at a large translation.
    #[must_use]
    pub fn transform_delta_via_positions(
        transform: Option<Transform>,
        untransformed_end_position: Offset,
        transformed_end_position: Option<Offset>,
        untransformed_delta: Offset,
    ) -> Offset {
        let Some(transform) = transform else {
            return untransformed_delta;
        };
        let Some(end) = transformed_end_position else {
            return transform.transform_vector(untransformed_delta);
        };
        let [a, b, c, d, tx, ty] = transform.coefficients();
        let start_x = f64::from(untransformed_end_position.x) - f64::from(untransformed_delta.x);
        let start_y = f64::from(untransformed_end_position.y) - f64::from(untransformed_delta.y);
        Offset::new(
            (f64::from(end.x) - (a * start_x + c * start_y + tx)) as f32,
            (f64::from(end.y) - (b * start_x + d * start_y + ty)) as f32,
        )
    }

    /// Reduces a 4×4 paint matrix to its pointer-event plane. The z row
    /// and column become the identity axis, while planar perspective in w
    /// is preserved. Input coordinates and the original matrix are unchanged.
    #[must_use]
    pub const fn remove_perspective_transform(matrix: Matrix4) -> Matrix4 {
        let mut entries = *matrix.columns();
        entries[2] = 0.;
        entries[6] = 0.;
        entries[8] = 0.;
        entries[9] = 0.;
        entries[10] = 1.;
        entries[11] = 0.;
        entries[14] = 0.;
        Matrix4::from_columns(entries)
    }

    /// Borrows this raw event in the receiving target's coordinate space.
    /// Unlike a value copy, the projection retains its source identity,
    /// including on enter and exit events that never came from a converter.
    #[must_use]
    pub const fn projection<'a>(
        &'a self,
        transform: Option<&'a Matrix4>,
    ) -> PointerEventProjection<'a, Self> {
        PointerEventProjection::new(self, transform)
    }

    /// Projects local position and delta through a pointer matrix.
    /// A projective displacement is the difference of two mapped points,
    /// not the transform of the delta as an independent vector.
    #[must_use]
    pub fn transformed_matrix4(mut self, transform: Option<Matrix4>) -> Self {
        let Some(transform) = transform else {
            return self;
        };
        let coordinates = self.projection(Some(&transform)).coordinates();
        self.local_position = coordinates.position.local;
        self.local_delta = coordinates.delta.local;
        if self.scroll_delta != Offset::ZERO {
            self.local_scroll_delta = project_motion(
                transform,
                self.position + self.scroll_delta,
                self.scroll_delta,
            ).1;
        }
        self
    }

    /// Copies this sample at a new time and global position and delta.
    /// The returned event starts in global space; route it to a target
    /// again to compute that target's local coordinates. The contact,
    /// device, phase and measurements are unchanged.
    #[must_use]
    pub const fn with_global_motion(self, time: Instant, position: Offset, delta: Offset) -> Self {
        Self {
            time,
            position,
            delta,
            ..self
        }
        .in_global_coordinates()
    }

    /// Uses the original global values for the local fields as well.
    #[must_use]
    pub const fn in_global_coordinates(mut self) -> Self {
        self.local_position = self.position;
        self.local_delta = self.delta;
        self.local_scroll_delta = self.scroll_delta;
        self
    }

    /// Projects only the local fields. Metadata, position and delta remain
    /// in their original global coordinate space. Without a transform,
    /// the event is unchanged.
    #[must_use]
    pub fn transformed(mut self, transform: Option<Transform>) -> Self {
        if let Some(transform) = transform {
            self.local_position = transform.transform_point(self.position);
            self.local_delta = transform.transform_vector(self.delta);
            if self.scroll_delta != Offset::ZERO {
                self.local_scroll_delta = transform.transform_vector(self.scroll_delta);
            }
        }
        self
    }

    /// Converts to the legacy gesture event while retaining the raw event as
    /// the source of truth for listeners and tap regions.
    #[must_use]
    pub const fn legacy(self) -> PointerEvent {
        PointerEvent {
            pointer: self.pointer,
            position: self.position,
            phase: self.phase,
            time: self.time,
        }
    }
}

// Used by raw routes and borrowed projections of other pointer packet types.
pub(crate) fn project_motion(
    transform: Matrix4,
    position: Offset,
    delta: Offset,
) -> (Offset, Offset) {
    if let Some(affine) = transform.affine_xy_plane() {
        return (
            affine.transform_point(position),
            affine.transform_vector(delta),
        );
    }
    let end = Point64::new(f64::from(position.x), f64::from(position.y));
    let start = Point64::new(end.x - f64::from(delta.x), end.y - f64::from(delta.y));
    let end = transform.transform_point(end);
    let start = transform.transform_point(start);
    (
        Offset::new(end.x as f32, end.y as f32),
        Offset::new((end.x - start.x) as f32, (end.y - start.y) as f32),
    )
}

impl From<PointerEvent> for RawPointerEvent {
    fn from(event: PointerEvent) -> Self {
        Self::new(
            event.pointer,
            0,
            PointerDeviceKind::Mouse,
            0,
            event.position,
            event.phase,
            event.time,
        )
    }
}

/// Flutter's `down` for an event: presses, and moves with a button held.
const fn pointer_is_down(phase: PointerPhase, buttons: u32) -> bool {
    match phase {
        PointerPhase::Down => true,
        PointerPhase::Move => buttons != 0,
        PointerPhase::Up | PointerPhase::Cancel | PointerPhase::Enter | PointerPhase::Exit => false,
    }
}

/// A renderer-independent cursor preference consumed by higher-level mouse-region widgets.
///
/// `Defer` asks the retained hit-test route to continue behind the current
/// region. Concrete platform adapters map the remaining values to native
/// cursors; keeping the vocabulary here avoids putting platform handles in
/// the widget or painting crates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MouseCursor {
    #[default]
    Defer,
    Basic,
    Click,
    Text,
    Crosshair,
    Grab,
    Grabbing,
    NotAllowed,
    ResizeHorizontal,
    ResizeVertical,
    /// North-west / south-east diagonal resize.
    ResizeUpLeftDownRight,
    /// North-east / south-west diagonal resize.
    ResizeUpRightDownLeft,
}

pub use incular_core::PointerDeviceKind;

/// Physical pointer velocity in logical pixels per second.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct Velocity {
    pub pixels_per_second: Offset,
}

impl Velocity {
    pub const ZERO: Self = Self {
        pixels_per_second: Offset::ZERO,
    };

    #[must_use]
    pub const fn new(pixels_per_second: Offset) -> Self {
        Self { pixels_per_second }
    }

    /// Scales the magnitude into `min..=max`, preserving direction.
    ///
    /// A zero vector has no direction, so a positive minimum yields NaN
    /// coordinates, as in Flutter. Debug builds check that the bounds are
    /// nonnegative and ordered.
    #[must_use]
    pub fn clamp_magnitude(self, min: f32, max: f32) -> Self {
        debug_assert!(min >= 0. && max >= min);
        let x = f64::from(self.pixels_per_second.x);
        let y = f64::from(self.pixels_per_second.y);
        let magnitude_squared = x * x + y * y;
        let min = f64::from(min);
        let max = f64::from(max);
        let target = if magnitude_squared > max * max {
            max
        } else if magnitude_squared < min * min {
            min
        } else {
            return self;
        };
        let magnitude = magnitude_squared.sqrt();
        Self::new(Offset::new(
            ((x / magnitude) * target) as f32,
            ((y / magnitude) * target) as f32,
        ))
    }
}

impl std::ops::Neg for Velocity {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(Offset::new(
            -self.pixels_per_second.x,
            -self.pixels_per_second.y,
        ))
    }
}

impl std::ops::Add for Velocity {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.pixels_per_second + rhs.pixels_per_second)
    }
}

impl std::ops::Sub for Velocity {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.pixels_per_second - rhs.pixels_per_second)
    }
}

impl std::hash::Hash for Velocity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for value in [self.pixels_per_second.x, self.pixels_per_second.y] {
            // Equality equates positive and negative zero.
            (if value == 0. { 0 } else { value.to_bits() }).hash(state);
        }
    }
}

impl std::fmt::Display for Velocity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use incular_core::debug_double::format_double_fixed;

        write!(
            f,
            "Velocity({}, {})",
            format_double_fixed(f64::from(self.pixels_per_second.x), 1),
            format_double_fixed(f64::from(self.pixels_per_second.y), 1),
        )
    }
}

impl std::fmt::Debug for Velocity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// Position and timing metadata for pointer tap-down events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapDownDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub kind: PointerDeviceKind,
}

/// Position and timing metadata for pointer tap-up events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapUpDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub kind: PointerDeviceKind,
}

/// Metadata for drag start events (Flutter `DragStartDetails`).
///
/// With Flutter's default `DragStartBehavior.start` the position is where the
/// drag was accepted, not where the pointer first went down.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct DragStartDetails {
    /// Timestamp of the event that started the drag (`sourceTimeStamp`).
    pub source_time: Option<Instant>,
    pub global_position: Offset,
    pub local_position: Offset,
    /// The initiating device, if the drag came from a physical pointer.
    pub kind: Option<PointerDeviceKind>,
}

/// Metadata for drag down events.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct DragDownDetails {
    pub global_position: Offset,
    pub local_position: Offset,
}

/// Metadata for long-press start events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LongPressStartDetails {
    pub global_position: Offset,
    pub local_position: Offset,
}

/// Metadata for long-press movement update events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LongPressMoveUpdateDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub offset_from_origin: Offset,
    pub local_offset_from_origin: Offset,
}

/// Metadata for long-press end events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LongPressEndDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub velocity: Velocity,
}

/// Details of `onLongPressDown` (Flutter `LongPressDownDetails`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LongPressDownDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    /// The initiating device kind, or no metadata for legacy pointer input.
    pub kind: Option<PointerDeviceKind>,
}

/// Details of `onTapMove` (Flutter `TapMoveDetails`): one movement of a
/// pointer whose tap is still pending.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapMoveDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    /// The movement since the previous event, in the receiver's space.
    pub delta: Offset,
    pub kind: PointerDeviceKind,
}

/// The button a press was made with, which selects the callback family it
/// responds to (Flutter `kPrimaryButton`, `kSecondaryButton`,
/// `kTertiaryButton`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButtonClass {
    Primary,
    Secondary,
    Tertiary,
}

impl PointerButtonClass {
    /// Classifies a press by its button mask. Zero is primary, because legacy
    /// adapters do not publish button metadata; a press whose mask holds no
    /// primary, secondary or tertiary button has no class and no recognizer.
    #[must_use]
    pub const fn of_press(buttons: u32) -> Option<Self> {
        if buttons == 0 || buttons & PRIMARY_POINTER_BUTTON != 0 {
            Some(Self::Primary)
        } else if buttons & SECONDARY_POINTER_BUTTON != 0 {
            Some(Self::Secondary)
        } else if buttons & TERTIARY_POINTER_BUTTON != 0 {
            Some(Self::Tertiary)
        } else {
            None
        }
    }
}

/// Tap and long-press callbacks for one non-primary button (Flutter
/// `onSecondaryTap*`, `onTertiaryTap*`, `onSecondaryLongPress*` and
/// `onTertiaryLongPress*`). A press made with the button is recognized with
/// these callbacks in place of the primary tap and long-press callbacks.
#[derive(Clone, Default)]
pub struct ButtonGestureCallbacks {
    pub on_tap: Option<Rc<dyn Fn()>>,
    pub on_tap_down: Option<Rc<dyn Fn(TapDownDetails)>>,
    pub on_tap_up: Option<Rc<dyn Fn(TapUpDetails)>>,
    pub on_tap_cancel: Option<Rc<dyn Fn()>>,
    pub on_long_press_down: Option<Rc<dyn Fn(LongPressDownDetails)>>,
    pub on_long_press_cancel: Option<Rc<dyn Fn()>>,
    pub on_long_press: Option<Rc<dyn Fn()>>,
    pub on_long_press_start: Option<Rc<dyn Fn(LongPressStartDetails)>>,
    pub on_long_press_move_update: Option<Rc<dyn Fn(LongPressMoveUpdateDetails)>>,
    pub on_long_press_up: Option<Rc<dyn Fn()>>,
    pub on_long_press_end: Option<Rc<dyn Fn(LongPressEndDetails)>>,
}

impl ButtonGestureCallbacks {
    /// Whether any callback of this button is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.on_tap.is_none()
            && self.on_tap_down.is_none()
            && self.on_tap_up.is_none()
            && self.on_tap_cancel.is_none()
            && self.on_long_press_down.is_none()
            && self.on_long_press_cancel.is_none()
            && self.on_long_press.is_none()
            && self.on_long_press_start.is_none()
            && self.on_long_press_move_update.is_none()
            && self.on_long_press_up.is_none()
            && self.on_long_press_end.is_none()
    }
}

/// Selects who owns the raw and typed callbacks on a mounted focus node.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusKeyHandlerOwner {
    /// A lower-level callback container, not a mounted Focus widget.
    #[default]
    CallbackContainer,
    FocusWidget,
    FocusNode,
}

#[doc(hidden)]
pub type ActionListenerRegistrations = (
    Option<ActionListenerSubscription>,
    Option<ActionInvocationSubscription>,
);

#[doc(hidden)]
pub type MountActionListener = dyn Fn() -> ActionListenerRegistrations;

/// Callbacks understood by Incular's retained pointer gesture recognizer.
#[derive(Clone, Default)]
pub struct GestureCallbacks {
    pub on_tap: Option<Rc<dyn Fn()>>,
    /// A pointer-only tap with any stable contact mask (not a semantic tap).
    pub on_any_tap: Option<Rc<dyn Fn()>>,
    pub on_tap_down: Option<Rc<dyn Fn(TapDownDetails)>>,
    pub on_tap_up: Option<Rc<dyn Fn(TapUpDetails)>>,
    pub on_tap_cancel: Option<Rc<dyn Fn()>>,
    /// Flutter `onTapMove`: each movement of a pending primary-button tap.
    pub on_tap_move: Option<Rc<dyn Fn(TapMoveDetails)>>,
    pub on_double_tap: Option<Rc<dyn Fn()>>,
    pub on_double_tap_down: Option<Rc<dyn Fn(TapDownDetails)>>,
    pub on_double_tap_cancel: Option<Rc<dyn Fn()>>,
    /// Flutter `onLongPressDown`: the press went down, before any deadline.
    pub on_long_press_down: Option<Rc<dyn Fn(LongPressDownDetails)>>,
    /// Flutter `onLongPressCancel`: a press that reported `onLongPressDown`
    /// lost before its long press started.
    pub on_long_press_cancel: Option<Rc<dyn Fn()>>,
    pub on_long_press: Option<Rc<dyn Fn()>>,
    pub on_long_press_start: Option<Rc<dyn Fn(LongPressStartDetails)>>,
    pub on_long_press_move_update: Option<Rc<dyn Fn(LongPressMoveUpdateDetails)>>,
    pub on_long_press_up: Option<Rc<dyn Fn()>>,
    pub on_long_press_end: Option<Rc<dyn Fn(LongPressEndDetails)>>,
    /// Secondary-button (right-click) tap and long press (Flutter
    /// `onSecondaryTap*` and `onSecondaryLongPress*`).
    pub secondary: ButtonGestureCallbacks,
    /// Tertiary-button (middle-click) tap and long press (Flutter
    /// `onTertiaryTap*` and `onTertiaryLongPress*`).
    pub tertiary: ButtonGestureCallbacks,
    pub on_pan_down: Option<Rc<dyn Fn(DragDownDetails)>>,
    pub on_pan_start: Option<Rc<dyn Fn(DragStartDetails)>>,
    pub on_pan_update: Option<Rc<dyn Fn(DragUpdateDetails)>>,
    /// Called once when a pan ends normally with the release velocity.
    pub on_pan_end: Option<Rc<dyn Fn(DragEndDetails)>>,
    pub on_pan_cancel: Option<Rc<dyn Fn()>>,
    pub on_horizontal_drag_down: Option<Rc<dyn Fn(DragDownDetails)>>,
    pub on_horizontal_drag_start: Option<Rc<dyn Fn(DragStartDetails)>>,
    /// Receives a pan whose first slop-exceeding movement was horizontal.
    /// It competes with vertical drags in a retained `GestureDetector`.
    pub on_horizontal_drag_update: Option<Rc<dyn Fn(DragUpdateDetails)>>,
    /// Called once when a horizontal drag ends normally.
    pub on_horizontal_drag_end: Option<Rc<dyn Fn(DragEndDetails)>>,
    pub on_horizontal_drag_cancel: Option<Rc<dyn Fn()>>,
    pub on_vertical_drag_down: Option<Rc<dyn Fn(DragDownDetails)>>,
    pub on_vertical_drag_start: Option<Rc<dyn Fn(DragStartDetails)>>,
    /// Receives a pan whose first slop-exceeding movement was vertical.
    /// It competes with horizontal drags in a retained `GestureDetector`.
    pub on_vertical_drag_update: Option<Rc<dyn Fn(DragUpdateDetails)>>,
    /// Called once when a vertical drag ends normally.
    pub on_vertical_drag_end: Option<Rc<dyn Fn(DragEndDetails)>>,
    pub on_vertical_drag_cancel: Option<Rc<dyn Fn()>>,
    pub on_scale_start: Option<Rc<dyn Fn(ScaleStartDetails)>>,
    /// Receives the active focal point and relative distance for a retained
    /// multi-pointer region. Single-pointer recognizers ignore this callback.
    pub on_scale_update: Option<Rc<dyn Fn(ScaleUpdateDetails)>>,
    pub on_scale_end: Option<Rc<dyn Fn(ScaleEndDetails)>>,
    /// Flutter `onForcePressStart`: the press pressure passed the start
    /// threshold (0.4 of the device's range). Pressure-sensing devices only.
    pub on_force_press_start: Option<Rc<dyn Fn(crate::ForcePressDetails)>>,
    /// Flutter `onForcePressPeak`: the pressure passed the peak threshold
    /// (0.85).
    pub on_force_press_peak: Option<Rc<dyn Fn(crate::ForcePressDetails)>>,
    /// Flutter `onForcePressUpdate`: each pressure or position change once
    /// the force press started.
    pub on_force_press_update: Option<Rc<dyn Fn(crate::ForcePressDetails)>>,
    /// Flutter `onForcePressEnd`: the pointer lifted after the force press
    /// started.
    pub on_force_press_end: Option<Rc<dyn Fn(crate::ForcePressDetails)>>,
    /// Aggregate native trackpad gestures are delivered without manufacturing
    /// touch contacts. They use their own gesture-arena stream.
    pub on_trackpad_gesture: Option<Rc<dyn Fn(TrackpadGesture)>>,
    /// Called once for a retained pointer sequence when this region loses its
    /// arena claim or the platform cancels the sequence.
    pub on_cancel: Option<Rc<dyn Fn()>>,
    /// Receives a keyboard event after the retained runtime resolves the
    /// focused element. Returning `true` consumes the event and prevents
    /// outer keyboard scopes and text editing from seeing it.
    pub on_key: Option<Rc<dyn Fn(KeyboardEvent) -> bool>>,
    pub on_raw_key_observer: Option<Rc<dyn Fn(crate::RawKeyEvent)>>,
    /// A mounted Focus widget's typed keyboard handler.
    pub on_focus_key_event: Option<FocusKeyEventHandler>,
    /// A mounted Focus widget's deprecated raw keyboard callback.
    pub on_focus_raw_key: Option<crate::FocusRawKeyEventHandler>,
    /// Selects the callback provider and controls mounted slot installation.
    #[doc(hidden)]
    pub focus_key_handler_owner: FocusKeyHandlerOwner,
    /// Property overrides are separate from callback ownership.
    #[doc(hidden)]
    pub focus_node_widget_properties: Option<crate::FocusNodeWidgetProperties>,
    /// The widget's optional mounted focus-tree parent.
    #[doc(hidden)]
    pub focus_parent_node: Option<crate::FocusNode>,
    /// Receives key-down events that were not consumed by [`Self::on_key`].
    pub on_key_down: Option<Rc<dyn Fn(KeyboardEvent)>>,
    /// Receives repeated key-down events in addition to [`Self::on_key_down`].
    pub on_key_repeat: Option<Rc<dyn Fn(KeyboardEvent)>>,
    /// Receives key-up events that were not consumed by [`Self::on_key`].
    pub on_key_up: Option<Rc<dyn Fn(KeyboardEvent)>>,
    /// An optional typed shortcut dispatcher. Widgets owns the closure so
    /// this crate remains independent from the retained runtime.
    pub on_shortcut: Option<Rc<dyn Fn(KeyboardEvent) -> bool>>,
    /// Optional shortcut dispatcher with the separate observed lock state.
    pub on_shortcut_with_lock_modes:
        Option<Rc<dyn Fn(KeyboardEvent, incular_core::Modifiers) -> bool>>,
    /// Optional shortcut dispatcher that can inspect held logical keys.
    #[doc(hidden)]
    pub on_shortcut_with_keyboard_state: Option<Rc<StatefulShortcutHandler>>,
    /// A typed shortcut scope retained separately from the callback above so
    /// the widget tree can resolve an intent against action ancestors.
    pub shortcut_scope: Option<Rc<ErasedShortcutScope>>,
    /// A typed action scope used by descendant shortcut scopes.
    pub action_scope: Option<Rc<ErasedActionScope>>,
    /// Creates the listener registrations when the descriptor is mounted.
    #[doc(hidden)]
    pub mount_action_listener: Option<Rc<MountActionListener>>,
    /// Lifetime token for a higher-level action-listener registration.
    pub action_listener: Option<Rc<ActionListenerSubscription>>,
    /// Lifetime token for an action invocation listener installed by a
    /// retained `ActionListener` widget.
    pub action_invocation_listener: Option<Rc<ActionInvocationSubscription>>,
    /// Focus metadata associated with a keyboard listener. Keeping this on the
    /// platform-neutral callback value lets the retained tree route keyboard
    /// input without introducing a runtime dependency into gestures.
    pub focus_node: Option<FocusNode>,
    /// Keeps a mounted external scope alive alongside its node.
    #[doc(hidden)]
    pub focus_scope: Option<crate::FocusScopeNode>,
    /// A retained Focus widget without a supplied node. The tree creates
    /// one node for the mounted element and applies this focus permission.
    #[doc(hidden)]
    pub owned_focus_permission: Option<bool>,
    /// Diagnostic name set by the retained focus widget.
    #[doc(hidden)]
    pub focus_debug_label: Option<Rc<str>>,
    #[doc(hidden)]
    pub owns_focus_scope: bool,
    /// Combined retained focus, hover, and focus-highlight behavior.
    pub focus_behavior: Option<Rc<FocusBehavior>>,
    pub autofocus: bool,
    pub include_semantics: bool,
    /// The retained wrapper is Flutter's explicit Focus annotation.
    #[doc(hidden)]
    pub annotates_focus: bool,
    /// Renderer-independent cursor preference for hover-capable widgets.
    pub mouse_cursor: MouseCursor,
    /// Flutter `GestureDetector.dragStartBehavior` for the drag and scale
    /// recognizers: [`DragStartBehavior::Start`] (the default) starts at the
    /// accepted position; [`DragStartBehavior::Down`] starts at the press and
    /// reports the movement that crossed the slop as the first update.
    pub drag_start_behavior: DragStartBehavior,
    /// Convert absolute trackpad panning to scale without moving the focal point.
    pub trackpad_scroll_causes_scale: bool,
    /// Optional override of Flutter's default vertical scroll-to-scale gain.
    pub trackpad_scroll_to_scale_factor: Option<Offset>,
    /// Wait for drag slop even when the arena has already been won
    /// (Flutter `DragGestureRecognizer.onlyAcceptDragOnThreshold`).
    pub only_accept_drag_on_threshold: bool,
    /// Pointer categories admitted at the start of a press. None accepts
    /// all devices; an empty accepted set accepts none.
    pub supported_devices: Option<Rc<[PointerDeviceKind]>>,
    /// Called with `true` when keyboard focus enters this region's subtree
    /// (itself or a descendant) and `false` when it leaves. Purely
    /// observational: it never makes the region focusable.
    pub on_focus_within: Option<Rc<dyn Fn(bool)>>,
    /// Focus widget observer, notified only after the node's focus changes.
    pub on_focus_change: Option<Rc<dyn Fn(bool)>>,
    /// Observes every key event dispatched to a focused descendant, before
    /// any listener handles it and whether or not one consumes it. Never
    /// consumes. Not delivered while a text field has focus.
    pub on_key_observe: Option<Rc<dyn Fn(KeyboardEvent)>>,
}

impl GestureCallbacks {
    /// Replaces the device allowlist. The order of the input is immaterial.
    pub fn set_supported_devices(&mut self, kinds: impl IntoIterator<Item = PointerDeviceKind>) {
        let mut kinds: Vec<_> = kinds.into_iter().collect();
        kinds.sort_unstable_by_key(|kind| *kind as u8);
        kinds.dedup();
        self.supported_devices = Some(kinds.into());
    }

    #[must_use]
    pub fn allows_pointer_device(&self, kind: PointerDeviceKind) -> bool {
        self.supported_devices
            .as_ref()
            .is_none_or(|kinds| kinds.contains(&kind))
    }

    /// Includes pointer-only any-button taps even when the mask has no
    /// corresponding primary, secondary, or tertiary callback set.
    #[doc(hidden)]
    pub fn for_button_mask(&self, mask: u32) -> Option<Self> {
        let class = PointerButtonClass::of_press(mask);
        let mut callbacks = match class {
            Some(class) => self.for_button_class(class),
            None if self.on_any_tap.is_some() => Self::default(),
            None => return None,
        };
        if let Some(any) = &self.on_any_tap {
            let specific = callbacks.on_tap.take();
            callbacks.on_tap = Some(if let Some(specific) = specific {
                let any = any.clone();
                Rc::new(move || {
                    specific();
                    any();
                })
            } else {
                any.clone()
            });
            callbacks.on_any_tap = None;
        }
        Some(callbacks)
    }

    /// The callbacks a press made with `class` competes with, as Flutter
    /// routes a press to the recognizers whose `allowedButtonsFilter` accepts
    /// its buttons. A primary press keeps every callback. A secondary or
    /// tertiary press keeps only that button's tap and long-press callbacks,
    /// which the tap and long-press recognizers then report; drags, scales and
    /// the primary taps stay out of its arena.
    #[must_use]
    pub fn for_button_class(&self, class: PointerButtonClass) -> Self {
        let buttons = match class {
            PointerButtonClass::Primary => return self.clone(),
            PointerButtonClass::Secondary => &self.secondary,
            PointerButtonClass::Tertiary => &self.tertiary,
        };
        Self {
            on_tap: buttons.on_tap.clone(),
            on_tap_down: buttons.on_tap_down.clone(),
            on_tap_up: buttons.on_tap_up.clone(),
            on_tap_cancel: buttons.on_tap_cancel.clone(),
            on_long_press_down: buttons.on_long_press_down.clone(),
            on_long_press_cancel: buttons.on_long_press_cancel.clone(),
            on_long_press: buttons.on_long_press.clone(),
            on_long_press_start: buttons.on_long_press_start.clone(),
            on_long_press_move_update: buttons.on_long_press_move_update.clone(),
            on_long_press_up: buttons.on_long_press_up.clone(),
            on_long_press_end: buttons.on_long_press_end.clone(),
            on_cancel: self.on_cancel.clone(),
            drag_start_behavior: self.drag_start_behavior,
            supported_devices: self.supported_devices.clone(),
            ..Self::default()
        }
    }
}

/// The callback action selected after a recognizer has claimed its arena.
/// This is public so retained adapters can defer callbacks until arbitration
/// has completed while the higher-level gesture detector remains source-compatible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GestureAction {
    Tap,
    DoubleTap,
    /// A release arrived after the deadline before the host polled it.
    /// Dispatch the start and end together after the recognizer wins.
    LongPress {
        start: LongPressStartDetails,
        end: LongPressEndDetails,
    },
    /// The long-press deadline (`kLongPressTimeout`) elapsed with the pointer
    /// still down: the recognizer won the arena and `onLongPressStart` fires.
    LongPressStart(LongPressStartDetails),
    /// The pan recognizer won the arena (Flutter `onStart`).
    PanStart(DragStartDetails),
    Pan(DragUpdateDetails),
    PanEnd(DragEndDetails),
    /// The horizontal drag recognizer won the arena.
    HorizontalDragStart(DragStartDetails),
    HorizontalDrag(DragUpdateDetails),
    HorizontalDragEnd(DragEndDetails),
    /// The vertical drag recognizer won the arena.
    VerticalDragStart(DragStartDetails),
    VerticalDrag(DragUpdateDetails),
    VerticalDragEnd(DragEndDetails),
    /// The arena accepted an [`crate::ArenaGestureRecognizer`] for
    /// `pointer`; its recognizer's `acceptGesture` follows.
    ArenaAccepted {
        pointer: u64,
    },
}

/// A recognizer's claim for one pointer event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GestureDecision {
    Pending,
    Accept(GestureAction),
    Reject,
    Cancelled,
}

/// Details of one drag movement (Flutter `DragUpdateDetails`).
///
/// `delta` is the movement since the previous event, not the displacement
/// from the pointer-down position. Accepting a drag does not itself produce an
/// update: the first update reports the movement after acceptance.
#[derive(Clone, Copy, PartialEq)]
pub struct DragUpdateDetails {
    /// Timestamp of the pointer event (`sourceTimeStamp`).
    pub source_time: Option<Instant>,
    /// Movement since the previous event. Horizontal and vertical drags only
    /// report their own axis; the other component is zero.
    pub delta: Offset,
    /// The movement along the drag axis. `None` for a pan, which has no axis.
    pub primary_delta: Option<f32>,
    pub global_position: Offset,
    pub local_position: Offset,
    /// The kind of the device that is dragging.
    pub kind: PointerDeviceKind,
}

/// Details of the end of a drag (Flutter `DragEndDetails`).
#[derive(Clone, Copy, Default, PartialEq)]
pub struct DragEndDetails {
    /// Velocity of the pointer when it stopped, zero for a slow release.
    pub velocity: Velocity,
    /// The velocity along the drag axis. `None` for a pan, which has no axis.
    pub primary_velocity: Option<f32>,
    pub global_position: Offset,
    pub local_position: Offset,
}

/// Callbacks for [`DragGestureDetector`](crate::DragGestureDetector).
#[derive(Clone, Default)]
pub struct DragCallbacks {
    pub on_start: Option<Rc<dyn Fn(DragStartDetails)>>,
    pub on_update: Option<Rc<dyn Fn(DragUpdateDetails)>>,
    pub on_end: Option<Rc<dyn Fn(DragEndDetails)>>,
}

impl GestureCallbacks {
    /// Whether any force-press callback is set, so the press joins the
    /// arena with a [`crate::ForcePressGestureRecognizer`].
    #[must_use]
    pub fn has_force_press(&self) -> bool {
        self.on_force_press_start.is_some()
            || self.on_force_press_peak.is_some()
            || self.on_force_press_update.is_some()
            || self.on_force_press_end.is_some()
    }

    /// A [`crate::ForcePressGestureRecognizer`] reporting to these
    /// callbacks, with Flutter's default thresholds (start 0.4, peak 0.85).
    #[must_use]
    pub fn force_press_recognizer(&self) -> crate::ForcePressGestureRecognizer {
        let mut recognizer = crate::ForcePressGestureRecognizer::new();
        if let Some(callback) = self.on_force_press_start.clone() {
            recognizer = recognizer.on_start(move |details| callback(details));
        }
        if let Some(callback) = self.on_force_press_peak.clone() {
            recognizer = recognizer.on_peak(move |details| callback(details));
        }
        if let Some(callback) = self.on_force_press_update.clone() {
            recognizer = recognizer.on_update(move |details| callback(details));
        }
        if let Some(callback) = self.on_force_press_end.clone() {
            recognizer = recognizer.on_end(move |details| callback(details));
        }
        recognizer
    }
}

pub(crate) mod drag_diagnostics;
mod pointer_diagnostics;
