//! Developer-experience layers over the Flutter vocabulary: chainable
//! modifiers (`styled_widget`), declarative effect chains
//! (`flutter_animate`) and breakpoints with autoscaling
//! (`responsive_framework`).

pub(crate) mod animate;
pub(crate) mod drag;
pub mod motion;
pub(crate) mod responsive;
pub(crate) mod styled;
pub mod transition;
pub mod view_transition;
