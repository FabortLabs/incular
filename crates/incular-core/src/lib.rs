//! Renderer- and platform-independent primitives shared by Incular.

mod app_build_version;
mod arena;
mod bit_field;
mod caching_iterable;
mod change_notifier;
mod color_swatch;
pub mod collections;
pub mod debug_double;
pub mod debug_instrumentation;
pub mod matrix4;
pub mod debug_print;
pub mod debug_timeline;
pub mod diagnostic_property;
mod context;
mod fast_hash;
mod float_color;
mod geometry;
mod hardware_keyboard;
pub mod hot_reload;
mod id;
/// Platform-neutral input event values and the standardized keyboard
/// vocabulary. `Key` remains the logical keyboard key inside this module;
/// the crate root re-exports it as [`KeyboardKey`] to avoid colliding with the
/// retained widget-key trait.
pub mod input;
pub mod licenses;
mod key;
mod math;
pub mod memory;
mod merged_listenable;
mod observer_list;
mod persistent_hash_map;
pub mod reactivity;
mod restoration;
pub mod serialization;
pub mod standard_message_codec;
pub mod time;
mod value_notifier;
mod widget;

pub use app_build_version::AppBuildVersion;
pub use arena::{Arena, ArenaId};
pub use bit_field::{BitField, K_MAX_UNSIGNED_SMI};
pub use caching_iterable::{CachingCursor, CachingIndexError, CachingIterable};
pub use change_notifier::{ChangeNotifier, ChangeNotifierSubscription};
pub use color_swatch::ColorSwatch;
pub use debug_double::{debug_double_precision, debug_format_double, set_debug_double_precision};
pub use debug_instrumentation::{
    debug_instrument_action, debug_instrumentation_enabled, set_debug_instrumentation_enabled,
};
pub use debug_print::{DebugPrintBuffer, debug_print_synchronously, debug_word_wrap};
pub use diagnostic_property::{
    DiagnosticFailure, DiagnosticLevel, DiagnosticValue, DiagnosticsProperty, DiagnosticsTreeStyle,
};
pub use debug_timeline::{AggregatedTimedBlock, AggregatedTimings, DebugTimeline, DebugTimelineError, TimedBlock};
pub use context::{BuildContext, ConsumerId, ContextGuard, DependencySnapshot};
pub use float_color::{ColorSpace, FloatColor};
pub use geometry::{
    ChangeImpact, Color, DirtyFlags, HslColor, HsvColor, Invalidation, Lerp, Offset, Rect, Size,
    Transform, finite_non_negative, finite_or_zero,
};
pub use hardware_keyboard::{
    HardwareKeyDispatch, HardwareKeyHandlers, HardwareKeySubscription, HardwareKeyboardState,
    IrregularKeyEvent,
};
pub use id::IdGenerator;
pub use input::{
    BACK_POINTER_BUTTON, Code, DeviceGestureSettings, DragStartBehavior, FORWARD_POINTER_BUTTON,
    ImeEvent, InputEvent, Key as KeyboardKey, KeyState, KeyboardEvent, Location, Modifiers,
    NamedKey, NormalizedPressure, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPanZoomEvent, PointerPanZoomPhase, PointerPhase,
    PointerSampleMetadata, SECONDARY_POINTER_BUTTON, StylusMetadata, StylusOrientation,
    TERTIARY_POINTER_BUTTON, TrackpadGesture, TrackpadGestureKind, TrackpadGesturePhase,
    WindowResizeDirection, additional_pointer_button_mask,
};
pub use key::{Key, KeyHandle, KeyId, KeyObject, LocalKey, StringKey, UniqueKey, ValueKey};
pub use licenses::{LicenseEntry, LicenseParagraph, LicenseRegistry, LicenseStream};
pub use math::{InvalidClampBounds, clamp_double, try_clamp_double};
pub use merged_listenable::{MergedListenable, MergedSubscription};
pub use observer_list::{HashedObserverList, ObserverList};
pub use persistent_hash_map::PersistentHashMap;
pub use restoration::{RestorationBackend, RestorationKey, RestorationKeyError, RestorationScope};
pub use value_notifier::ValueNotifier;
pub use widget::{BuildableWidget, Widget};

/// The horizontal reading direction for layout and accessibility text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextDirection {
    /// Left to right.
    #[default]
    Ltr,
    /// Right to left.
    Rtl,
}

pub use matrix4::{CylindricalOrientation, Matrix4, Point64, Rect64};
