//! Platform-neutral gesture recognition and interaction primitives.
//!
//! This crate deliberately contains no widget-tree, runtime, or native-window
//! types. Retained widget regions and platform adapters share it as an input
//! boundary.

mod arena;
pub(crate) mod arena_member;
pub mod constants;
mod details;
mod focus;
mod force_press;
mod keyboard;
mod mouse;
mod multidrag;
mod multitap;
mod pointer;
mod raw;
mod scale;
mod tap_and_drag;
mod velocity;

pub use arena_member::ArenaResolver;
pub use velocity::VelocityTrackerKind;

pub use arena::{
    GestureArena, GestureArenaEntry, GestureArenaKey, GestureArenaMember, GestureArenaSource,
    GestureDisposition,
};
pub use details::{
    DragCallbacks, DragDownDetails, DragEndDetails, DragStartDetails, DragUpdateDetails,
    GestureAction, GestureCallbacks, GestureDecision, LongPressEndDetails,
    LongPressMoveUpdateDetails, LongPressStartDetails, MouseCursor, PointerDeviceKind,
    PointerEvent, RawPointerEvent, TapDownDetails, TapUpDetails, Velocity,
};
pub use focus::{
    FocusBehavior, FocusHighlightManager, FocusHighlightMode, FocusHighlightStrategy,
    FocusHighlightSubscription, FocusManager, FocusNode, FocusNodeSubscription, FocusScopeNode,
    FocusScopeSubscription, FocusTraversalPolicy, FocusTraversalPolicyKind, OrderedTraversalPolicy,
    ReadingOrderTraversalPolicy, WidgetOrderTraversalPolicy,
};
pub use force_press::{ForceInterpolation, ForcePressDetails, ForcePressGestureRecognizer};
pub use keyboard::{
    Action, ActionInvocationPhase, ActionInvocationSubscription, ActionListenerSubscription,
    ActionResult, Actions, Command, CommandId, ErasedActionScope, ErasedIntent,
    ErasedShortcutScope, Intent, LogicalShortcutKey, ShortcutActivator, ShortcutKey, ShortcutMatch,
    ShortcutTrigger, Shortcuts, SingleActivator,
};
pub use mouse::MouseRegion;
pub use multidrag::{Drag, MultiDragGestureRecognizer, MultiDragStartCallback};
pub use multitap::{
    MultiTapGestureRecognizer, SerialTapCancelDetails, SerialTapDownDetails,
    SerialTapGestureRecognizer, SerialTapUpDetails,
};
pub use pointer::{DragGestureDetector, FlingLimits, GestureDetector, PointerGestureRecognizer};
pub use raw::{
    ErasedGestureRecognizerFactory, GestureRecognizer, GestureRecognizerFactory,
    GestureRecognizerFactoryError, GestureRecognizerFactoryWithHandlers,
};
pub use scale::{
    ScaleEndDetails, ScaleGestureDetector, ScaleStartDetails, ScaleUpdateDetails,
    compute_scale_slop,
};
pub use tap_and_drag::{
    TapAndDragGestureRecognizer, TapDragDownDetails, TapDragEndDetails, TapDragStartDetails,
    TapDragUpDetails, TapDragUpdateDetails,
};
