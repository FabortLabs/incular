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
mod hit_test;
mod keyboard;
mod mouse;
mod multidrag;
mod multitap;
mod pointer;
mod pointer_router;
mod raw;
mod scale;
mod slop;
mod tap_and_drag;
mod velocity;

pub use arena_member::{ArenaGestureRecognizer, ArenaRecognizer, ArenaResolver};
pub use velocity::VelocityTrackerKind;

pub use arena::{
    GestureArena, GestureArenaEntry, GestureArenaKey, GestureArenaMember, GestureArenaSource,
    GestureDisposition,
};
pub use details::{
    ButtonGestureCallbacks, DragCallbacks, DragDownDetails, DragEndDetails, DragStartDetails,
    DragUpdateDetails, GestureAction, GestureCallbacks, GestureDecision, LongPressDownDetails,
    LongPressEndDetails, LongPressMoveUpdateDetails, LongPressStartDetails, MouseCursor,
    PointerButtonClass, PointerDeviceKind, PointerEvent, RawPointerEvent, TapDownDetails,
    TapMoveDetails, TapUpDetails, Velocity,
};
pub use hit_test::{BoxHitTestResult, HitTestEntry, HitTestResult, HitTestTransform};
pub use focus::{
    DirectionalFocusCandidate, FocusBehavior, FocusHighlightManager, FocusHighlightMode,
    FocusHighlightStrategy, FocusHighlightSubscription, FocusManager, FocusNode,
    FocusNodeAttachment, FocusNodeSubscription, FocusOrder, FocusScopeNode, FocusScopeSubscription,
    UnfocusDisposition,
    FocusTraversalPolicy, FocusTraversalPolicyKind, OrderedTraversalPolicy,
    ReadingOrderTraversalPolicy, TraversalDirection, WidgetOrderTraversalPolicy,
    directional_focus_index, focus_epoch, reading_order_by,
};
pub use force_press::{ForceInterpolation, ForcePressDetails, ForcePressGestureRecognizer};
pub use incular_core::DeviceGestureSettings;
pub use keyboard::{
    Action, ActionInvocationPhase, ActionInvocationSubscription, ActionListenerSubscription,
    ActionResult, Actions, CharacterActivator, Command, CommandId, ErasedActionScope, ErasedIntent,
    ErasedShortcutScope, FocusKeyEventHandler, Intent, KeyEventResult, LockState, LogicalKeySet,
    LogicalShortcutKey, ShortcutActivator, ShortcutKey, ShortcutMatch, ShortcutTrigger, Shortcuts,
    SingleActivator,
};
pub use mouse::MouseRegion;
pub use multidrag::{Drag, MultiDragGestureRecognizer, MultiDragStartCallback};
pub use multitap::{
    MultiTapGestureRecognizer, SerialTapCancelDetails, SerialTapDownDetails,
    SerialTapGestureRecognizer, SerialTapUpDetails,
};
pub use pointer::{
    DragGestureDetector, FlingLimits, GestureDetector, PointerGestureRecognizer, TapMemory,
};
pub use pointer_router::{
    PointerRouteError, PointerRouteErrorHandler, PointerRouteId, PointerRouter, RoutedPointerEvent,
    WeakPointerRouter,
};
pub use raw::{
    ErasedGestureRecognizerFactory, GestureRecognizer, GestureRecognizerFactory,
    GestureRecognizerFactoryError, GestureRecognizerFactoryWithHandlers,
};
pub use scale::{ScaleEndDetails, ScaleGestureDetector, ScaleStartDetails, ScaleUpdateDetails};
pub use slop::{compute_hit_slop, compute_pan_slop, compute_scale_slop};
pub use tap_and_drag::{
    TapAndDragGestureRecognizer, TapDragDownDetails, TapDragEndDetails, TapDragStartDetails,
    TapDragUpDetails, TapDragUpdateDetails,
};
