//! Platform-neutral gesture recognition and interaction primitives.
//!
//! This crate deliberately contains no widget-tree, runtime, or native-window
//! types. Retained widget regions and platform adapters share it as an input
//! boundary.

mod arena;
mod arena_team;
pub(crate) mod arena_member;
mod buttons;
pub mod constants;
mod details;
mod double_tap;
mod focus;
mod force_press;
mod hit_test;
mod keyboard;
mod least_squares;
mod long_press;
mod managed_arena;
mod mouse;
mod multidrag;
mod multitap;
mod pointer;
mod pointer_converter;
mod pointer_resampler;
mod pointer_router;
mod primary_pointer;
mod raw;
mod raw_key_data;
mod scale;
mod signal;
mod slop;
mod tap_and_drag;
mod tap_callback_error;
mod velocity;

pub use arena_member::{ArenaGestureRecognizer, ArenaRecognizer, ArenaResolver};
pub use double_tap::DoubleTapGestureRecognizer;
pub use arena_team::{GestureArenaTeam, GestureTeamRegistration};
pub use buttons::{is_single_button, nth_mouse_button, nth_stylus_button, smallest_button};
pub use velocity::{VelocityTracker, VelocityTrackerError, VelocityTrackerKind};

pub use arena::{
    GestureArena, GestureArenaEntry, GestureArenaKey, GestureArenaMember, GestureArenaSource,
    GestureDisposition,
};
#[doc(hidden)]
pub use details::{ActionListenerRegistrations, MountActionListener};
pub use details::GestureDetailDiagnostics;
pub use details::{
    ButtonGestureCallbacks, DragCallbacks, DragDownDetails, DragEndDetails, DragStartDetails, FocusKeyHandlerOwner,
    DragUpdateDetails, GestureAction, GestureCallbacks, GestureDecision, LongPressDownDetails,
    LongPressEndDetails, LongPressMoveUpdateDetails, LongPressStartDetails, MouseCursor, OffsetPair,
    PointerButtonClass, PointerDeviceKind, PointerEvent, RawPointerEvent, TapDownDetails,
    TapMoveDetails, TapUpDetails, Velocity,
};
pub use hit_test::{
    BoxHitTestResult, HitTestEntry, HitTestResult, HitTestTransform, SharedHitTestResult,
};
pub use focus::{
    DirectionalFocusCandidate, FocusBehavior, FocusExtent, FocusHighlightManager, FocusHighlightMode,
    FocusHighlightStrategy, FocusHighlightSubscription, FocusManager, FocusNode,
    FocusNodeAttachment, FocusNodeSubscription, FocusNodeWidgetProperties, FocusOrder, FocusRect, FocusScopeNode, FocusScopeSubscription,
    UnfocusDisposition,
    FocusTraversalPolicy, FocusTraversalPolicyKind, OrderedTraversalPolicy,
    ReadingOrderTraversalPolicy, TraversalDirection, TraversalEdgeBehavior, WidgetOrderTraversalPolicy,
    directional_focus_index, directional_focus_index_in_phase, focus_epoch, reading_order_by, take_hosted_scope_autofocus,
};
pub use force_press::{
    ForceInterpolation, ForcePressDetails, ForcePressGestureRecognizer, ForcePressureSample,
};
pub use incular_core::DeviceGestureSettings;
pub use keyboard::{
    ShortcutManager,
    Action, ActionContext, ActionDispatchInvocation, ActionDispatcher, ActionInvocationPhase, ActionInvocationSubscription,
    ActionListenerSubscription,
    ActionOutput, ActionResult, Actions, CallbackAction, CharacterActivator, Command, CommandId, ErasedActionScope, ErasedIntent,
    ErasedShortcutScope, FocusKeyEventHandler, FocusRawKeyEventHandler, Intent, KeyEventResult,
    CustomShortcutActivator, LockState, LogicalKeySet, LogicalShortcutKey, RawKeyDelivery, RawKeyEvent, ShortcutActivator, ShortcutKey,
    ScopedActionDispatcher, ShortcutMatch, ShortcutTrigger, Shortcuts, SingleActivator,
    VoidCallbackAction, VoidCallbackIntent, combine_key_event_results,
};
pub use least_squares::{LeastSquaresSolver, PolynomialFit};
pub use long_press::LongPressGestureRecognizer;
pub use managed_arena::{
    GestureArenaClient, GestureArenaManager, GestureArenaRegistration, GestureArenaTrace,
    GestureArenaTraceHandler, GestureArenaTracePhase, GestureArenaWinCause,
};
pub use mouse::MouseRegion;
pub use multidrag::{Drag, MultiDragGestureRecognizer, MultiDragPointerState, MultiDragStartCallback};
pub use multitap::{
    MultiTapGestureRecognizer, SerialTapCancelDetails, SerialTapDownDetails,
    SerialTapGestureRecognizer, SerialTapUpDetails,
};
pub use tap_callback_error::{
    TapCallbackError, TapCallbackErrorHandler, TapCallbackKind, TapCancelReason,
    invoke_retained_tap, set_tap_callback_error_handler,
    TapCallbackTraceHandler, set_tap_callback_trace_handler,
};
#[doc(hidden)]
pub use pointer::DragBoundaryFrame;
pub use pointer::{
    DragAxis, DragGestureDetector, FlingLimits, GestureDetector, MonodragGestureRecognizer,
    MultitouchDragStrategy,
    PendingTapRelease, PointerGestureRecognizer, TapMemory, TapRecognizerDiagnostics,
    TapSlopTolerance, TapTrackingState, TrackpadDragAxis,
    TrackpadDragRecognizer,
};
pub use signal::{PointerSignalError, PointerSignalEvent, PointerSignalResolver};
pub use pointer_converter::{
    ConvertedPointerChange, ConvertedPointerEvent, ConvertedPointerKind, PointerData,
    PointerDataChange, PointerDataSignal, PointerEventConverter, PointerEventProjection,
    PointerMotionCoordinates,
};
pub use pointer_resampler::PointerEventResampler;
pub use pointer_router::{
    PointerRouteError, PointerRouteErrorHandler, PointerRouteId, PointerRouter, RoutedPointerEvent,
    WeakPointerRouter,
};
pub use raw_key_data::{
    AndroidRawKeyData, AppleKeyPlatform, AppleRawKeyData, FuchsiaRawKeyData, LinuxKeyToolkit, LinuxRawKeyData, RawKeyPlatformData,
    RawKeyboardModifier, RawKeyboardSide, WebRawKeyData, WindowsRawKeyData,
};
pub use raw::{
    ErasedGestureRecognizerFactory, GestureRecognizer, GestureRecognizerFactory,
    GestureRecognizerFactoryError, GestureRecognizerFactoryWithHandlers,
};
pub use incular_core::{PointerPanZoomEvent, PointerPanZoomPhase};
pub use scale::{
    DEFAULT_TRACKPAD_SCROLL_TO_SCALE_FACTOR,
    ScaleEndDetails, ScaleGestureDetector, ScaleStartDetails, ScaleUpdateDetails,
};
pub use primary_pointer::{
    GestureRecognizerState, GestureSlopTolerance, PrimaryPointerGestureHandler,
    PrimaryPointerGestureOptions, PrimaryPointerGestureRecognizer,
};
pub use slop::{compute_hit_slop, compute_pan_slop, compute_scale_slop};
pub use tap_and_drag::{
    TapAndDragGestureRecognizer, TapDragDownDetails, TapDragEndDetails, TapDragStartDetails,
    TapDragUpDetails, TapDragUpdateDetails,
};
