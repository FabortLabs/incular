//! `.transition()`: a widget whose own layout or paint properties animate
//! when they change, with the same [`Transition`] vocabulary as `.motion()`.
//!
//! ```ignore
//! Container::new()
//!     .width(if expanded { 320. } else { 160. })
//!     .color(if selected { accent } else { surface })
//!     .transition(Spring::smooth())
//! ```
//!
//! Only widgets whose properties can interpolate implement
//! [`Transitionable`]: [`Container`](crate::Container),
//! [`Padding`](crate::Padding), [`Align`](crate::Align),
//! [`Positioned`](crate::Positioned), [`Opacity`](crate::Opacity),
//! [`FractionallySizedBox`](crate::FractionallySizedBox),
//! [`PhysicalModel`](crate::PhysicalModel) and
//! [`DefaultTextStyle`](crate::DefaultTextStyle). Each is the matching
//! `Animated*` widget underneath, so `AnimatedContainer::new(..).transition(..)`
//! is the same implementation. Interrupting a change retargets from the value
//! on screen. Under reduced motion sizes and positions snap and fades stay
//! short.

use incular_animation::Transition;

use crate::Widget;

/// A widget whose properties animate between builds with `.transition()`.
pub trait Transitionable: Into<Widget> + Sized {
    /// The widget, animating its properties with `transition`.
    #[doc(hidden)]
    fn into_transitioning(self, transition: Transition) -> Widget;
}

/// `.transition()` for every [`Transitionable`] widget.
pub trait TransitionExt: Transitionable {
    /// Animates this widget's own properties when they change: size,
    /// padding, alignment, position, color, opacity, elevation or text style.
    #[must_use]
    fn transition(self, transition: impl Into<Transition>) -> Widget {
        self.into_transitioning(transition.into())
    }
}

impl<T: Transitionable> TransitionExt for T {}
