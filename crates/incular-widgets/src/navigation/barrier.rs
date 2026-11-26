//! Animated modal barrier composition and semantics.

use crate::{
    Animation, Color, Container, ExplicitSemantics, GestureDetector, HitTestBehavior, Widget,
};
use incular_animation::{AnimationController, Tween, TweenValue};
use incular_config::EdgeInsets;
use incular_core::ValueNotifier;
use incular_semantics::{SemanticActionKind, SemanticRole, SemanticState};
use std::rc::Rc;

/// A barrier color animation whose sampled value may be absent, as Flutter's
/// `Animation<Color?>`. An absent color paints no fill while the barrier keeps
/// its pointer interception and semantics.
#[derive(Clone)]
pub struct OptionalColorAnimation {
    controller: AnimationController,
    sample: Rc<dyn Fn() -> Option<Color>>,
}

impl OptionalColorAnimation {
    /// Maps a typed animation to optional colors. `map` runs only when the
    /// barrier samples its color.
    #[must_use]
    pub fn from_map<T: Tween + 'static>(
        animation: Animation<T>,
        map: impl Fn(T) -> Option<Color> + 'static,
    ) -> Self {
        let controller = animation.controller();
        Self {
            controller,
            sample: Rc::new(move || map(animation.value())),
        }
    }

    /// A color animation whose every sample is present.
    #[must_use]
    pub fn from_color(animation: Animation<Color>) -> Self {
        Self::from_map(animation, Some)
    }

    /// The controller that drives this animation.
    #[must_use]
    pub fn controller(&self) -> AnimationController {
        self.controller.clone()
    }

    /// The sampled color, or `None` when the animation is at its absent end.
    #[must_use]
    pub fn value(&self) -> Option<Color> {
        (self.sample)()
    }
}

/// The nearest route outlet's default dismissal handler. Published by
/// the runtime without giving Widgets a dependency on the navigator.
#[doc(hidden)]
pub struct ModalBarrierDismissScope(Rc<dyn Fn()>);

impl ModalBarrierDismissScope {
    #[must_use]
    pub fn new(dismiss: impl Fn() + 'static) -> Rc<Self> {
        Rc::new(Self(Rc::new(dismiss)))
    }

    #[must_use]
    pub fn wrap(self: &Rc<Self>, child: Widget) -> Widget {
        Widget::shared_environment_scope(self.clone(), child)
    }
}

/// A non-animated modal veil. By default it covers its constraints, paints
/// no fill, and a stable tap asks the nearest mounted route outlet to pop.
/// It always intercepts background input and blocks preceding semantics.
#[derive(Clone)]
pub struct ModalBarrier {
    color: Option<Color>,
    dismissible: bool,
    semantics_label: Option<String>,
    barrier_semantics_dismissible: bool,
    semantics_on_tap_hint: Option<String>,
    clip_details_notifier: Option<Rc<ValueNotifier<EdgeInsets>>>,
    on_dismiss: Option<Rc<dyn Fn()>>,
    dismissal_handler: Option<Rc<dyn Fn() -> bool>>,
    child: Option<Widget>,
}

impl Default for ModalBarrier {
    fn default() -> Self {
        Self::new()
    }
}

impl ModalBarrier {
    #[must_use]
    pub fn new() -> Self {
        Self {
            color: None,
            dismissible: true,
            semantics_label: None,
            barrier_semantics_dismissible: true,
            semantics_on_tap_hint: None,
            clip_details_notifier: None,
            on_dismiss: None,
            dismissal_handler: None,
            child: None,
        }
    }

    /// A missing color intercepts hits without painting a fill.
    #[must_use]
    pub fn color(mut self, color: impl Into<Option<Color>>) -> Self {
        self.color = color.into();
        self
    }
}

/// A modal barrier with a retained color animation. All hit-testing and
/// accessibility settings are shared with the non-animated ModalBarrier.
#[derive(Clone)]
pub struct AnimatedModalBarrier {
    color: OptionalColorAnimation,
    barrier: ModalBarrier,
}

impl Default for AnimatedModalBarrier {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimatedModalBarrier {
    /// Creates a black, half-opacity barrier with a constant color animation.
    ///
    /// `with_color`/`color_animation` are the animated constructor forms;
    /// keeping this zero-argument constructor preserves the original Incular
    /// widget spelling while still lowering to the same animated path.
    #[must_use]
    pub fn new() -> Self {
        Self::with_color(constant_color_animation(Color::rgba(0, 0, 0, 128)))
    }

    /// Creates a barrier with a constant color (no animated transition).
    ///
    /// Route outlets use this to veil modal content without owning an
    /// animation controller per route.
    #[must_use]
    pub fn with_flat_color(color: Color) -> Self {
        Self::with_color(constant_color_animation(color))
    }

    /// Creates a barrier driven by a typed color animation.
    #[must_use]
    pub fn with_color(color: Animation<Color>) -> Self {
        Self::with_optional_color(OptionalColorAnimation::from_color(color))
    }

    /// Creates a barrier driven by an optional color animation. A sampled
    /// `None` paints no fill; the barrier still absorbs pointer input and
    /// keeps its dismissal and semantics, as Flutter's `ModalBarrier` does
    /// for a null `color`.
    #[must_use]
    pub fn with_optional_color(color: OptionalColorAnimation) -> Self {
        Self {
            color,
            barrier: ModalBarrier::new(),
        }
    }

    /// Alias for [`Self::with_color`].
    #[must_use]
    pub fn color_animation(color: Animation<Color>) -> Self {
        Self::with_color(color)
    }

    /// Sets the color animation.
    #[must_use]
    pub fn color(mut self, color: Animation<Color>) -> Self {
        self.color = OptionalColorAnimation::from_color(color);
        self
    }

    /// Sets an optional color animation (see [`Self::with_optional_color`]).
    #[must_use]
    pub fn optional_color(mut self, color: OptionalColorAnimation) -> Self {
        self.color = color;
        self
    }

    /// Returns the current sampled barrier color, or transparent when the
    /// animation is at its absent end.
    #[must_use]
    pub fn current_color(&self) -> Color {
        self.color.value().unwrap_or(Color::TRANSPARENT)
    }

    /// Returns the current sampled color, `None` when the animation is at its
    /// absent end.
    #[must_use]
    pub fn current_optional_color(&self) -> Option<Color> {
        self.color.value()
    }

    /// Returns the animation driving this barrier.
    #[must_use]
    pub fn animation(&self) -> OptionalColorAnimation {
        self.color.clone()
    }

    /// Enables or disables pointer and semantic dismissal.
    #[must_use]
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.barrier = self.barrier.dismissible(dismissible);
        self
    }

    /// Sets the callback invoked by pointer and semantic dismissal.
    #[must_use]
    pub fn on_dismiss(mut self, callback: impl Fn() + 'static) -> Self {
        self.barrier = self.barrier.on_dismiss(callback);
        self
    }

    /// Installs a boolean dismissal bridge.  This is the integration seam for
    /// `incular-navigation::Overlay::dismiss_top` or a route pop operation.
    #[must_use]
    pub fn dismissal_handler(mut self, callback: impl Fn() -> bool + 'static) -> Self {
        self.barrier = self.barrier.dismissal_handler(callback);
        self
    }

    /// Sets the accessibility label exposed for the modal barrier.
    #[must_use]
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.barrier = self.barrier.semantics_label(label);
        self
    }

    /// Controls whether a labeled barrier exposes an accessibility dismiss
    /// action.  Pointer dismissal remains governed by `dismissible`.
    #[must_use]
    pub fn barrier_semantics_dismissible(mut self, value: bool) -> Self {
        self.barrier = self.barrier.barrier_semantics_dismissible(value);
        self
    }

    /// Supplies the platform-specific accessibility tap hint.
    #[must_use]
    pub fn semantics_on_tap_hint(mut self, hint: impl Into<String>) -> Self {
        self.barrier = self.barrier.semantics_on_tap_hint(hint);
        self
    }

    /// Insets the barrier's accessibility bounds only. Updating the notifier
    /// does not change the veil's size, paint, or pointer hit region.
    #[must_use]
    pub fn clip_details_notifier(mut self, notifier: ValueNotifier<EdgeInsets>) -> Self {
        self.barrier = self.barrier.clip_details_notifier(notifier);
        self
    }

    /// Optionally places the barrier above a retained child in a stack.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.barrier = self.barrier.child(child);
        self
    }

    /// Invokes the configured dismissal bridge and returns whether it handled
    /// the dismissal.  A void callback is considered handled.
    #[must_use]
    pub fn dismiss(&self) -> bool {
        self.barrier.dismiss()
    }
}

impl ModalBarrier {
    /// Enables or disables pointer and semantic dismissal.
    #[must_use]
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Sets the callback invoked by pointer and semantic dismissal.
    #[must_use]
    pub fn on_dismiss(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(callback));
        self
    }

    /// Installs a boolean dismissal bridge.  This is the integration seam for
    /// `incular-navigation::Overlay::dismiss_top` or a route pop operation.
    #[must_use]
    pub fn dismissal_handler(mut self, callback: impl Fn() -> bool + 'static) -> Self {
        self.dismissal_handler = Some(Rc::new(callback));
        self
    }

    /// Sets the accessibility label exposed for the modal barrier.
    #[must_use]
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }

    /// Controls whether a labeled barrier exposes an accessibility dismiss
    /// action.  Pointer dismissal remains governed by `dismissible`.
    #[must_use]
    pub fn barrier_semantics_dismissible(mut self, value: bool) -> Self {
        self.barrier_semantics_dismissible = value;
        self
    }

    /// Supplies the platform-specific accessibility tap hint.
    #[must_use]
    pub fn semantics_on_tap_hint(mut self, hint: impl Into<String>) -> Self {
        self.semantics_on_tap_hint = Some(hint.into());
        self
    }

    /// Insets the barrier's accessibility bounds only. Updating the notifier
    /// does not change the veil's size, paint, or pointer hit region.
    #[must_use]
    pub fn clip_details_notifier(mut self, notifier: ValueNotifier<EdgeInsets>) -> Self {
        self.clip_details_notifier = Some(Rc::new(notifier));
        self
    }

    /// Optionally places the barrier above a retained child in a stack.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Invokes the configured dismissal bridge and returns whether it handled
    /// the dismissal.  A void callback is considered handled.
    #[must_use]
    pub fn dismiss(&self) -> bool {
        if !self.dismissible {
            return false;
        }
        if let Some(callback) = &self.on_dismiss {
            callback();
            return true;
        }
        self.dismissal_handler
            .as_ref()
            .is_some_and(|callback| callback())
    }
}

impl ModalBarrier {
    fn dismissal_callback(&self, context: &crate::BuildContext<'_>) -> Option<Rc<dyn Fn()>> {
        if !self.dismissible {
            return None;
        }
        if let Some(callback) = &self.on_dismiss {
            return Some(callback.clone());
        }
        if let Some(handler) = &self.dismissal_handler {
            let handler = handler.clone();
            return Some(Rc::new(move || {
                let _ = handler();
            }));
        }
        context
            .find_shared::<ModalBarrierDismissScope>()
            .map(|scope| scope.0.clone())
    }

    fn veil(&self, color: Option<Color>, context: &crate::BuildContext<'_>) -> Widget {
        let mut veil: Widget = match color {
            Some(fill) => Container::new().color(fill).into(),
            None => Container::new().into(),
        };
        let dismiss = self.dismissal_callback(context);
        if self.dismissible {
            // Join the same arena as translucent siblings; a stable
            // contact dismisses only if it wins on release.
            let dismiss = dismiss.clone();
            veil = GestureDetector::new(veil)
                .behavior(HitTestBehavior::Opaque)
                .on_any_pointer_tap(move || {
                    if let Some(dismiss) = &dismiss {
                        dismiss();
                    }
                })
                .into();
        } else {
            veil = Widget::absorb_pointer(true, veil);
        }

        // Retain a supplied label when native semantic dismissal is
        // disabled. Pointer blocking is independent of semantic policy.
        let semantic_dismiss = self.dismissible && self.barrier_semantics_dismissible;
        if self.semantics_label.is_some() || semantic_dismiss {
            let mut explicit = ExplicitSemantics::new(if semantic_dismiss {
                SemanticRole::Button
            } else {
                SemanticRole::GenericContainer
            });
            if let Some(label) = &self.semantics_label {
                explicit = explicit.label(label.clone());
                if semantic_dismiss {
                    explicit = explicit.state(SemanticState {
                        text_direction: context.text_direction(),
                        ..Default::default()
                    });
                }
            }
            if let Some(hint) = &self.semantics_on_tap_hint {
                explicit = explicit.description(hint.clone());
            }
            if semantic_dismiss {
                let dismiss = dismiss.unwrap_or_else(|| Rc::new(|| {}));
                explicit =
                    explicit.actions([SemanticActionKind::Activate, SemanticActionKind::Dismiss]);
                veil = veil
                    .semantics(explicit)
                    .with_semantic_callback(SemanticActionKind::Activate, dismiss.clone())
                    .with_semantic_callback(SemanticActionKind::Dismiss, dismiss);
                if let Some(insets) = &self.clip_details_notifier {
                    veil = veil.with_semantic_bounds_insets(insets.clone());
                }
            } else {
                veil = veil.semantics(explicit);
            }
        }
        // Inner block hides a child behind the veil; the outer block on
        // each descriptor hides preceding siblings outside the barrier.
        let veil = veil.block_semantics();
        if let Some(child) = &self.child {
            crate::Stack::new([child.clone(), veil]).into()
        } else {
            veil
        }
    }
}

impl From<ModalBarrier> for Widget {
    fn from(barrier: ModalBarrier) -> Self {
        let widget: Widget =
            crate::Builder::new(move |context| barrier.veil(barrier.color, context)).into();
        widget.block_semantics()
    }
}

impl From<AnimatedModalBarrier> for Widget {
    fn from(value: AnimatedModalBarrier) -> Self {
        // Listen only while mounted; the static spelling never creates a ticker.
        let animation = incular_animation::DoubleAnimation::from(value.color.controller());
        let color = value.color;
        let barrier = value.barrier;
        crate::explicit_animations::animated_widget_for(animation, move |context| {
            barrier.veil(color.value(), context)
        })
        .block_semantics()
    }
}

fn constant_color_animation(color: Color) -> Animation<Color> {
    Animation::new(
        AnimationController::default(),
        TweenValue::new(color, color),
    )
}
