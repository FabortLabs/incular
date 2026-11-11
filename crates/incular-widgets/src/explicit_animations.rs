//! Flutter's explicit animation builders (`AnimatedBuilder`,
//! `ListenableBuilder`) and the transitions that follow an animation graph
//! rather than a retained controller.
//!
//! A builder reruns when any controller under its animation ticks; the
//! compositor transitions (`FadeTransition`, `ScaleTransition`,
//! `RotationTransition` from an animation) only update their layer, so the
//! child is neither rebuilt nor repainted, as in Flutter.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use incular_animation::{AnimationController, DoubleAnimation, DrivenAnimation};
use incular_core::Offset;

use crate::tree::RetainedState;
use crate::{
    BuildContext, FadeTransition, FractionalTranslation, OpacityController, RotationController,
    RotationTransition, ScaleController, ScaleTransition, SlideTransition, Widget,
};

/// Listeners on a set of controllers, dropped with the state that owns them.
#[derive(Default)]
struct ControllerListeners {
    listening: RefCell<Vec<(AnimationController, usize)>>,
}

impl ControllerListeners {
    /// Listens to `controllers`, replacing the listeners of a different set
    /// (`didUpdateWidget`).
    fn listen(&self, controllers: &[AnimationController], on_change: impl Fn() + 'static) {
        {
            let listening = self.listening.borrow();
            if listening.len() == controllers.len()
                && listening
                    .iter()
                    .zip(controllers)
                    .all(|((listened, _), controller)| listened == controller)
            {
                return;
            }
        }
        self.clear();
        let on_change: Rc<dyn Fn()> = Rc::new(on_change);
        let mut listening = self.listening.borrow_mut();
        for controller in controllers {
            let on_change = on_change.clone();
            let id = controller.add_listener(move |_| on_change());
            listening.push((controller.clone(), id));
        }
    }

    fn clear(&self) {
        for (controller, id) in self.listening.borrow_mut().drain(..) {
            controller.remove_listener(id);
        }
    }
}

impl Drop for ControllerListeners {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Wraps `child` so every controller in `controllers` ticks with the tree's
/// frames.
fn ticking(
    controllers: Vec<AnimationController>,
    revision: &Rc<Cell<u64>>,
    child: Widget,
) -> Widget {
    controllers.into_iter().fold(child, |child, controller| {
        Widget::animation_ticker_with_revision(controller, false, revision.clone(), child)
    })
}

struct RebuildState {
    revision: RefCell<Rc<Cell<u64>>>,
    listeners: ControllerListeners,
}

impl RetainedState for RebuildState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
        // The listeners bump the old revision; listen again on the next build.
        self.listeners.clear();
    }
}

/// Flutter's `AnimatedWidget`: `build` reruns whenever a controller under
/// `animation` notifies, and those controllers tick with the tree's frames.
pub(crate) fn animated_widget_for(
    animation: DoubleAnimation,
    build: impl for<'a> Fn(&BuildContext<'a>) -> Widget + 'static,
) -> Widget {
    Widget::retained_builder(
        |revision| RebuildState {
            revision: RefCell::new(revision),
            listeners: ControllerListeners::default(),
        },
        move |state, context, _| {
            let controllers = animation.controllers();
            let revision = state.revision.borrow().clone();
            let weak: Weak<Cell<u64>> = Rc::downgrade(&revision);
            state.listeners.listen(&controllers, move || {
                if let Some(revision) = weak.upgrade() {
                    revision.set(revision.get().wrapping_add(1));
                }
            });
            ticking(controllers, &revision, build(context))
        },
        false,
    )
}

/// Builds a subtree from a [`BuildContext`] and the optional pass-through
/// child.
pub type TransitionBuilder = Rc<dyn for<'a> Fn(&BuildContext<'a>, Option<Widget>) -> Widget>;

/// Flutter's `AnimatedBuilder`: reruns `builder` whenever `animation`
/// changes. `child` is built once and handed to every run, for the part of
/// the subtree that does not depend on the animation.
#[derive(Clone)]
pub struct AnimatedBuilder {
    animation: DoubleAnimation,
    builder: TransitionBuilder,
    child: Option<Widget>,
}

impl AnimatedBuilder {
    #[must_use]
    pub fn new(
        animation: impl Into<DoubleAnimation>,
        builder: impl for<'a> Fn(&BuildContext<'a>, Option<Widget>) -> Widget + 'static,
    ) -> Self {
        Self {
            animation: animation.into(),
            builder: Rc::new(builder),
            child: None,
        }
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }
}

impl From<AnimatedBuilder> for Widget {
    fn from(value: AnimatedBuilder) -> Self {
        let AnimatedBuilder {
            animation,
            builder,
            child,
        } = value;
        animated_widget_for(animation, move |context| builder(context, child.clone()))
    }
}

/// Something a [`ListenableBuilder`] can follow: Flutter's `Listenable`.
///
/// Reading through [`track`](Self::track) during a build makes the build
/// rerun when the value changes; [`controllers`](Self::controllers) names
/// the animation clocks the builder must keep ticking.
pub trait Listenable {
    /// Reads the value so the current build depends on it.
    fn track(&self);

    /// The animation controllers behind the value, if any.
    fn controllers(&self) -> Vec<AnimationController> {
        Vec::new()
    }
}

impl Listenable for AnimationController {
    fn track(&self) {
        let _ = self.value();
    }

    fn controllers(&self) -> Vec<AnimationController> {
        vec![self.clone()]
    }
}

impl Listenable for DoubleAnimation {
    fn track(&self) {
        let _ = self.value();
    }

    fn controllers(&self) -> Vec<AnimationController> {
        DoubleAnimation::controllers(self)
    }
}

impl Listenable for incular_scroll::ScrollController {
    fn track(&self) {
        let _ = self.offset();
        let _ = self.max_offset();
    }
}

impl Listenable for incular_core::reactivity::DependencySource {
    fn track(&self) {
        let _ = incular_core::reactivity::DependencySource::track(self);
    }
}

impl<T: Listenable> Listenable for Rc<T> {
    fn track(&self) {
        (**self).track();
    }

    fn controllers(&self) -> Vec<AnimationController> {
        (**self).controllers()
    }
}

/// Flutter's `ListenableBuilder`: reruns `builder` whenever `listenable`
/// notifies.
#[derive(Clone)]
pub struct ListenableBuilder {
    listenable: Rc<dyn Listenable>,
    builder: TransitionBuilder,
    child: Option<Widget>,
}

impl ListenableBuilder {
    #[must_use]
    pub fn new(
        listenable: impl Listenable + 'static,
        builder: impl for<'a> Fn(&BuildContext<'a>, Option<Widget>) -> Widget + 'static,
    ) -> Self {
        Self {
            listenable: Rc::new(listenable),
            builder: Rc::new(builder),
            child: None,
        }
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }
}

impl From<ListenableBuilder> for Widget {
    fn from(value: ListenableBuilder) -> Self {
        let ListenableBuilder {
            listenable,
            builder,
            child,
        } = value;
        let controllers = listenable.controllers();
        if controllers.is_empty() {
            return crate::Builder::new(move |context| {
                listenable.track();
                builder(context, child.clone())
            })
            .into();
        }
        animated_widget_for(DoubleAnimation::new(Clocks(controllers)), move |context| {
            listenable.track();
            builder(context, child.clone())
        })
    }
}

/// The clocks of a listenable, as an animation source with no value of its
/// own.
struct Clocks(Vec<AnimationController>);

impl incular_animation::AnimationSource for Clocks {
    fn value(&self) -> f32 {
        0.0
    }

    fn status(&self) -> incular_animation::AnimationStatus {
        incular_animation::AnimationStatus::Dismissed
    }

    fn controllers(&self) -> Vec<AnimationController> {
        self.0.clone()
    }
}

struct CompositorState<C> {
    target: C,
    listeners: ControllerListeners,
}

impl<C: 'static> RetainedState for CompositorState<C> {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

/// A compositor-only transition: `apply` copies the animation's value into
/// the retained `target` on every tick, without rebuilding.
fn compositor_transition<C: Clone + 'static>(
    animation: DoubleAnimation,
    make: fn() -> C,
    apply: fn(&C, f32),
    wrap: impl Fn(C, Widget) -> Widget + 'static,
    child: Widget,
) -> Widget {
    Widget::retained_builder(
        move |_| CompositorState {
            target: make(),
            listeners: ControllerListeners::default(),
        },
        move |state, _, _| {
            apply(&state.target, animation.value());
            let target = state.target.clone();
            let followed = animation.clone();
            let controllers = animation.controllers();
            state
                .listeners
                .listen(&controllers, move || apply(&target, followed.value()));
            ticking(
                controllers,
                &Rc::new(Cell::new(0)),
                wrap(state.target.clone(), child.clone()),
            )
        },
        false,
    )
}

impl FadeTransition {
    /// Flutter's `FadeTransition(opacity:)`: the child's opacity follows
    /// `opacity` on the compositor, without rebuilding or repainting it.
    #[must_use]
    pub fn from_animation(opacity: impl Into<DoubleAnimation>, child: impl Into<Widget>) -> Widget {
        compositor_transition(
            opacity.into(),
            OpacityController::new,
            |controller, value| {
                controller.set_opacity(value);
            },
            |controller, child| FadeTransition::new(controller, child).into(),
            child.into(),
        )
    }
}

impl ScaleTransition {
    /// Flutter's `ScaleTransition(scale:)`.
    #[must_use]
    pub fn from_animation(scale: impl Into<DoubleAnimation>, child: impl Into<Widget>) -> Widget {
        compositor_transition(
            scale.into(),
            ScaleController::new,
            |controller, value| {
                controller.set_scale(value);
            },
            |controller, child| ScaleTransition::new(controller, child).into(),
            child.into(),
        )
    }
}

impl RotationTransition {
    /// Flutter's `RotationTransition(turns:)`: one turn is a full clockwise
    /// revolution about the child's center.
    #[must_use]
    pub fn from_animation(turns: impl Into<DoubleAnimation>, child: impl Into<Widget>) -> Widget {
        compositor_transition(
            turns.into(),
            RotationController::new,
            |controller, value| {
                controller.set_turns(value);
            },
            |controller, child| RotationTransition::new(controller, child).into(),
            child.into(),
        )
    }
}

impl SlideTransition {
    /// Flutter's `SlideTransition(position:)`: translates the child by a
    /// fraction of its own size, as `FractionalTranslation` does.
    #[must_use]
    pub fn from_animation(position: DrivenAnimation<Offset>, child: impl Into<Widget>) -> Widget {
        let child = child.into();
        let parent = position.parent().clone();
        animated_widget_for(parent, move |_| {
            FractionalTranslation::new(position.value(), child.clone()).into()
        })
    }
}
