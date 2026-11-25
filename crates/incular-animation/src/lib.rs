//! Renderer- and platform-independent animation primitives.
//!
//! Controllers are driven by runtime-supplied monotonic timestamps. Curves,
//! typed tweens, composable values, and implicit retargeting live in separate
//! modules while the historical root exports remain available.

mod animations;
mod controller;
mod curve2d;
mod curves;
pub mod motion;
pub mod physics;
pub mod scheduler;
mod scheduler_debug;
mod style;
mod tween;
mod ui_tasks;
mod value;

pub use animations::{
    AlwaysStoppedAnimation, AnimatableExt, AnimationSource, ChainedTween, CompoundAnimation,
    ConstantTween, CurveTween, CurvedAnimation, DoubleAnimation, DrivenAnimation, LinearTween,
    ProxyAnimation, ReverseAnimation, ReverseTween, StepTween, TrainHoppingAnimation,
    always_complete, always_dismissed, always_stopped, animation_max, animation_mean,
    animation_min,
};
pub use controller::{
    AnimationBehavior, AnimationController, AnimationStatus, RepeatOptions, disable_animations,
    set_disable_animations,
};
pub use curve2d::{
    CatmullRomCurve, CatmullRomSpline, CatmullRomSplineError, Curve2D, Curve2DSample,
};
pub use curves::{Curve, CurveChain, Curves};
pub use motion::{
    Channel, Keyframes, MotionValue, Spring, Transition, TransitionKind, end_transaction, ms, secs,
    transaction, with_transition, without_transition,
};
pub use physics::{
    BoundedFrictionSimulation, ClampedSimulation, FrictionSimulation, GravitySimulation,
    ScrollSpringSimulation, Simulation, SpringDescription, SpringSimulation, SpringType, Tolerance,
};
pub use scheduler::{
    ActiveTimeDilation, FrameCallbackId, InvalidTimeDilation, PerformanceMode,
    PerformanceModeRequestHandle, Priority, Ticker, TickerCanceled, TickerFuture, TickerOrCancel,
    add_persistent_frame_callback, add_post_frame_callback, cancel_frame_callback,
    current_frame_time, current_system_frame_time,
    debug_assert_no_pending_performance_mode_requests, debug_assert_no_time_dilation,
    debug_requested_performance_mode, request_performance_mode, schedule_forced_frame,
    schedule_frame, schedule_frame_callback, schedule_frame_callback_with_request,
    schedule_frame_microtask, set_time_dilation, time_dilation, try_assert_no_time_dilation,
    try_set_time_dilation,
};
pub use style::AnimationStyle;
pub use tween::{
    FlippedTweenSequence, NullableTween, OptionalColorTween, OptionalRectTween, OptionalSizeTween,
    OptionalTween, Tween, TweenSegment, TweenSequence, TweenValue,
};
pub use value::{
    Animatable, AnimatedValue, Animation, AnimationValue, ColorTween, ImplicitAnimatedValue,
    ImplicitAnimation, IntTween, LerpTween, MappedAnimation, Matrix4Tween, RectTween, SizeTween,
};
