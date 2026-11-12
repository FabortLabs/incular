//! Renderer- and platform-independent animation primitives.
//!
//! Controllers are driven by runtime-supplied monotonic timestamps. Curves,
//! typed tweens, composable values, and implicit retargeting live in separate
//! modules while the historical root exports remain available.

mod animations;
mod controller;
mod curve2d;
mod curves;
pub mod physics;
pub mod scheduler;
mod style;
mod tween;
mod value;

pub use animations::{
    AlwaysStoppedAnimation, AnimatableExt, AnimationSource, ChainedTween, CompoundAnimation,
    ConstantTween, CurveTween, CurvedAnimation, DoubleAnimation, DrivenAnimation, LinearTween,
    ProxyAnimation, ReverseAnimation, ReverseTween, StepTween, TrainHoppingAnimation,
    always_complete, always_dismissed, animation_max, animation_mean, animation_min,
};
pub use controller::{
    AnimationBehavior, AnimationController, AnimationStatus, RepeatOptions, disable_animations,
    set_disable_animations,
};
pub use curve2d::{CatmullRomCurve, CatmullRomSpline, Curve2D, Curve2DSample};
pub use curves::{Curve, CurveChain, Curves};
pub use physics::{
    BoundedFrictionSimulation, ClampedSimulation, FrictionSimulation, GravitySimulation,
    ScrollSpringSimulation, Simulation, SpringDescription, SpringSimulation, SpringType, Tolerance,
};
pub use scheduler::{
    FrameCallbackId, Ticker, TickerCanceled, TickerFuture, TickerOrCancel,
    add_persistent_frame_callback, add_post_frame_callback, cancel_frame_callback,
    current_frame_time, schedule_frame, schedule_frame_callback,
};
pub use style::AnimationStyle;
pub use tween::{Tween, TweenSegment, TweenSequence, TweenValue};
pub use value::{
    Animatable, AnimatedValue, Animation, AnimationValue, ColorTween, ImplicitAnimatedValue,
    ImplicitAnimation, IntTween, LerpTween, MappedAnimation, Matrix4Tween, RectTween, SizeTween,
};
