//! Renderer- and platform-independent primitives shared by Incular.

mod arena;
mod caching_iterable;
mod context;
mod fast_hash;
mod geometry;
mod hardware_keyboard;
pub mod hot_reload;
mod id;
/// Platform-neutral input event values and the standardized keyboard
/// vocabulary. `Key` remains the logical keyboard key inside this module;
/// the crate root re-exports it as [`KeyboardKey`] to avoid colliding with the
/// retained widget-key trait.
pub mod input;
mod key;
mod math;
pub mod memory;
mod merged_listenable;
mod observer_list;
pub mod reactivity;
mod restoration;
pub mod time;
mod value_notifier;
mod widget;

pub use arena::{Arena, ArenaId};
pub use caching_iterable::{CachingCursor, CachingIndexError, CachingIterable};
pub use context::{BuildContext, ConsumerId, ContextGuard, DependencySnapshot};
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
    NamedKey, NormalizedPressure, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, SECONDARY_POINTER_BUTTON, StylusMetadata, StylusOrientation,
    TERTIARY_POINTER_BUTTON, TrackpadGesture, TrackpadGestureKind, TrackpadGesturePhase,
    WindowResizeDirection, additional_pointer_button_mask,
};
pub use key::{Key, KeyHandle, KeyId, KeyObject, LocalKey, StringKey, UniqueKey, ValueKey};
pub use math::{InvalidClampBounds, clamp_double, try_clamp_double};
pub use merged_listenable::{MergedListenable, MergedSubscription};
pub use observer_list::{HashedObserverList, ObserverList};
pub use restoration::{RestorationBackend, RestorationKey, RestorationKeyError, RestorationScope};
pub use value_notifier::ValueNotifier;
pub use widget::{BuildableWidget, Widget};
