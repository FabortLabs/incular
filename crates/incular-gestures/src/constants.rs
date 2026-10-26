//! Gesture timing and distance constants from Flutter's
//! `gestures/constants.dart`. Distances are logical pixels, velocities
//! logical pixels per second.

use std::time::Duration;

/// `kPressTimeout`: how long a pointer must stay down before a tap is
/// reported as pressed (`onTapDown`), unless the gesture is accepted first.
pub const PRESS_TIMEOUT: Duration = Duration::from_millis(100);

/// `kHoverTapTimeout`: the time within which a mouse hover followed by a tap
/// counts as one interaction.
///
/// Flutter 3.41 declares this for parity with Android's `ViewConfiguration`
/// but no framework recognizer or widget reads it, so no Incular recognizer
/// does either.
pub const HOVER_TAP_TIMEOUT: Duration = Duration::from_millis(150);

/// `kHoverTapSlop`: the distance a pointer may move between hovering and
/// tapping.
///
/// Flutter 3.41 declares this for parity with Android's `ViewConfiguration`
/// but no framework recognizer or widget reads it, so no Incular recognizer
/// does either.
pub const HOVER_TAP_SLOP: f32 = 20.0;

/// `kLongPressTimeout`: how long a pointer must stay down to be a long press.
pub const LONG_PRESS_TIMEOUT: Duration = Duration::from_millis(500);

/// `kDoubleTapTimeout`: the longest gap between the first tap's release and
/// the second tap's press.
pub const DOUBLE_TAP_TIMEOUT: Duration = Duration::from_millis(300);

/// `kDoubleTapMinTime`: the shortest time between the two presses of a
/// double tap; a quicker second press starts a new sequence.
pub const DOUBLE_TAP_MIN_TIME: Duration = Duration::from_millis(40);

/// `kDoubleTapTouchSlop`: how far a pointer may move during either tap of a
/// double tap.
pub const DOUBLE_TAP_TOUCH_SLOP: f32 = TOUCH_SLOP;

/// `kDoubleTapSlop`: the largest distance between the two taps of a double
/// tap.
pub const DOUBLE_TAP_SLOP: f32 = 100.0;

/// `kZoomControlsTimeout`: how long Android's zoom controls stay visible.
///
/// Flutter 3.41 declares this for parity with Android's `ViewConfiguration`
/// but no framework recognizer or widget reads it, so no Incular recognizer
/// does either.
pub const ZOOM_CONTROLS_TIMEOUT: Duration = Duration::from_millis(3000);

/// `kTouchSlop`: the hit tolerance for touch, stylus and trackpad pointers.
pub const TOUCH_SLOP: f32 = 18.0;

/// `kPagingTouchSlop`: the distance a pointer must travel to start paging.
///
/// Flutter 3.41 declares this for parity with Android's `ViewConfiguration`
/// but no framework recognizer or widget reads it, so no Incular recognizer
/// does either.
pub const PAGING_TOUCH_SLOP: f32 = TOUCH_SLOP * 2.0;

/// `kPanSlop`: the distance a pointer must travel to start a pan.
pub const PAN_SLOP: f32 = TOUCH_SLOP * 2.0;

/// `kScaleSlop`: how far the span must change to start a scale
/// ([`compute_scale_slop`](crate::compute_scale_slop)).
pub const SCALE_SLOP: f32 = TOUCH_SLOP;

/// `kWindowTouchSlop`: the distance a pointer may move before leaving its
/// window.
///
/// Flutter 3.41 declares this for parity with Android's `ViewConfiguration`
/// but no framework recognizer or widget reads it, so no Incular recognizer
/// does either.
pub const WINDOW_TOUCH_SLOP: f32 = 16.0;

/// `kMinFlingVelocity`: releases slower than this are not flings.
pub const MIN_FLING_VELOCITY: f32 = 50.0;

/// `kMaxFlingVelocity`: fling velocities are clamped to this magnitude.
pub const MAX_FLING_VELOCITY: f32 = 8000.0;

/// `kJumpTapTimeout`: the longest a pointer may stay down for a jump tap.
///
/// Flutter 3.41 declares this for parity with Android's `ViewConfiguration`
/// but no framework recognizer or widget reads it, so no Incular recognizer
/// does either.
pub const JUMP_TAP_TIMEOUT: Duration = Duration::from_millis(500);

/// `kPrecisePointerHitSlop`: the hit tolerance for mouse pointers.
pub const PRECISE_POINTER_HIT_SLOP: f32 = 1.0;

/// `kPrecisePointerPanSlop`: the pan distance for mouse pointers.
pub const PRECISE_POINTER_PAN_SLOP: f32 = PRECISE_POINTER_HIT_SLOP * 2.0;

/// `kPrecisePointerScaleSlop`: the scale slop for mouse pointers
/// ([`compute_scale_slop`](crate::compute_scale_slop)).
pub const PRECISE_POINTER_SCALE_SLOP: f32 = PRECISE_POINTER_HIT_SLOP;
