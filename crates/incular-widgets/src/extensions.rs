//! Incular-specific widget extensions that intentionally sit outside the
//! pinned Flutter Widgets vocabulary exported from the crate root.

pub use crate::external_drop::ExternalDropTarget;

pub use crate::dx::animate::{
    Animate, AnimateExt, AnimateRepeat, DEFAULT_EFFECT_CURVE, DEFAULT_EFFECT_DURATION, Effect,
    EffectKind,
};
pub use crate::dx::responsive::{
    Breakpoint, Condition, DESKTOP, MOBILE, ResponsiveBreakpoints, ResponsiveBreakpointsData,
    ResponsiveRowColumn, ResponsiveScaledBox, ResponsiveValue, ResponsiveVisibility, TABLET, UHD,
};
pub use crate::dx::styled::WidgetExt;
