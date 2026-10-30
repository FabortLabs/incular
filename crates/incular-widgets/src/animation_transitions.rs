//! Flutter's explicit transitions (`widgets/transitions.dart`) and
//! `DualTransitionBuilder` (`widgets/dual_transition_builder.dart`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_animation::{AnimationController, AnimationStatus, Curve};
use incular_config::{Alignment, Axis};
use incular_core::{Lerp, Transform as CoreTransform};
use incular_text::TextStyle;
use typed_builder::TypedBuilder;

use crate::implicit_animations::PropertyTween;
use crate::tree::RetainedState;
use crate::{
    Align, BoxDecoration, BuildContext, DecoratedBox, DecorationPosition, DefaultTextStyle,
    Positioned, Transform, Widget,
};

/// An explicit animation widget that transitions child alignment.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct AlignTransition {
    alignment: Alignment,
    #[builder(default, setter(strip_option))]
    width_factor: Option<f32>,
    #[builder(default, setter(strip_option))]
    height_factor: Option<f32>,
    #[builder(setter(into))]
    child: Widget,
}

impl AlignTransition {
    #[must_use]
    pub fn new(alignment: Alignment, child: impl Into<Widget>) -> Self {
        Self {
            alignment,
            width_factor: None,
            height_factor: None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn width_factor(mut self, factor: f32) -> Self {
        self.width_factor = Some(factor);
        self
    }

    #[must_use]
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.height_factor = Some(factor);
        self
    }
}

impl From<AlignTransition> for Widget {
    fn from(value: AlignTransition) -> Self {
        let mut a = Align::new(value.alignment, value.child);
        if let Some(wf) = value.width_factor {
            a = a.width_factor(wf);
        }
        if let Some(hf) = value.height_factor {
            a = a.height_factor(hf);
        }
        a.into()
    }
}

/// An explicit animation widget that animates its own size and clips its child.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct SizeTransition {
    #[builder(default = Axis::Vertical)]
    axis: Axis,
    #[builder(setter(transform = |size_factor: f32| size_factor.max(0.0)))]
    size_factor: f32,
    #[builder(default)]
    axis_alignment: f32,
    #[builder(setter(into))]
    child: Widget,
}

impl SizeTransition {
    #[must_use]
    pub fn new(size_factor: f32, child: impl Into<Widget>) -> Self {
        Self {
            axis: Axis::Vertical,
            size_factor: size_factor.max(0.0),
            axis_alignment: 0.0,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    #[must_use]
    pub fn axis_alignment(mut self, alignment: f32) -> Self {
        self.axis_alignment = alignment;
        self
    }
}

impl From<SizeTransition> for Widget {
    fn from(value: SizeTransition) -> Self {
        match value.axis {
            Axis::Horizontal => crate::layout::FractionallySizedBox::new(value.child)
                .width_factor(value.size_factor)
                .into(),
            Axis::Vertical => crate::layout::FractionallySizedBox::new(value.child)
                .height_factor(value.size_factor)
                .into(),
        }
    }
}

/// An explicit animation widget that animates a [`Positioned`] child in a [`Stack`](crate::Stack).
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct PositionedTransition {
    #[builder(default, setter(strip_option))]
    left: Option<f32>,
    #[builder(default, setter(strip_option))]
    top: Option<f32>,
    #[builder(default, setter(strip_option))]
    right: Option<f32>,
    #[builder(default, setter(strip_option))]
    bottom: Option<f32>,
    #[builder(default, setter(strip_option))]
    width: Option<f32>,
    #[builder(default, setter(strip_option))]
    height: Option<f32>,
    #[builder(setter(into))]
    child: Widget,
}

impl PositionedTransition {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            left: None,
            top: None,
            right: None,
            bottom: None,
            width: None,
            height: None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn left(mut self, left: f32) -> Self {
        self.left = Some(left);
        self
    }

    #[must_use]
    pub fn top(mut self, top: f32) -> Self {
        self.top = Some(top);
        self
    }

    #[must_use]
    pub fn right(mut self, right: f32) -> Self {
        self.right = Some(right);
        self
    }

    #[must_use]
    pub fn bottom(mut self, bottom: f32) -> Self {
        self.bottom = Some(bottom);
        self
    }
}

impl From<PositionedTransition> for Widget {
    fn from(value: PositionedTransition) -> Self {
        let mut p = Positioned::new(value.child);
        if let Some(v) = value.left {
            p = p.left(v);
        }
        if let Some(v) = value.top {
            p = p.top(v);
        }
        if let Some(v) = value.right {
            p = p.right(v);
        }
        if let Some(v) = value.bottom {
            p = p.bottom(v);
        }
        p.into()
    }
}

/// An explicit animation widget that animates a relative positioned rect.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct RelativePositionedTransition {
    #[builder(setter(into))]
    child: Widget,
}

impl RelativePositionedTransition {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }
}

impl From<RelativePositionedTransition> for Widget {
    fn from(value: RelativePositionedTransition) -> Self {
        Positioned::new(value.child).into()
    }
}

/// Flutter's `DecoratedBoxTransition`: a [`DecoratedBox`] whose decoration
/// follows `decoration`, rebuilt on every change of its controller.
#[derive(Clone)]
pub struct DecoratedBoxTransition {
    decoration: TweenAnimation<BoxDecoration>,
    position: DecorationPosition,
    child: Widget,
}

impl DecoratedBoxTransition {
    #[must_use]
    pub fn new(decoration: TweenAnimation<BoxDecoration>, child: impl Into<Widget>) -> Self {
        Self {
            decoration,
            position: DecorationPosition::Background,
            child: child.into(),
        }
    }

    /// Defaults to [`DecorationPosition::Background`].
    #[must_use]
    pub fn position(mut self, position: DecorationPosition) -> Self {
        self.position = position;
        self
    }
}

impl From<DecoratedBoxTransition> for Widget {
    fn from(value: DecoratedBoxTransition) -> Self {
        let controller = value.decoration.controller().clone();
        animated_widget(controller, move |_| {
            DecoratedBox::new(value.child.clone())
                .decoration(value.decoration.value())
                .position(value.position)
                .into()
        })
    }
}

/// Flutter's `DefaultTextStyleTransition`: a [`DefaultTextStyle`] whose
/// style follows `style`, rebuilt on every change of its controller.
///
/// Incular's [`DefaultTextStyle`] carries only the style, so Flutter's
/// `textAlign`, `softWrap`, `overflow` and `maxLines` have no counterpart.
#[derive(Clone)]
pub struct DefaultTextStyleTransition {
    style: TweenAnimation<TextStyle>,
    child: Widget,
}

impl DefaultTextStyleTransition {
    #[must_use]
    pub fn new(style: TweenAnimation<TextStyle>, child: impl Into<Widget>) -> Self {
        Self {
            style,
            child: child.into(),
        }
    }
}

impl From<DefaultTextStyleTransition> for Widget {
    fn from(value: DefaultTextStyleTransition) -> Self {
        let controller = value.style.controller().clone();
        animated_widget(controller, move |_| {
            DefaultTextStyle::new(value.style.value(), value.child.clone()).into()
        })
    }
}

/// An explicit animation widget that transitions an arbitrary affine transform matrix.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct MatrixTransition {
    #[builder(setter(into))]
    transform: CoreTransform,
    #[builder(setter(into))]
    child: Widget,
}

impl MatrixTransition {
    #[must_use]
    pub fn new(transform: CoreTransform, child: impl Into<Widget>) -> Self {
        Self {
            transform,
            child: child.into(),
        }
    }
}

impl From<MatrixTransition> for Widget {
    fn from(value: MatrixTransition) -> Self {
        Transform::new(value.transform, value.child).into()
    }
}

/// Builds one side of a [`DualTransitionBuilder`] from the context, the
/// animation value it should follow and the child.
pub type AnimatedTransitionBuilder =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, f32, Option<Widget>) -> Widget>;

/// Flutter's `DualTransitionBuilder`: `forward_builder` animates while the
/// animation runs forward and `reverse_builder` while it runs in reverse.
///
/// Each builder gets the value of the animation it follows. While the
/// animation is dismissed or running forward, the forward builder follows it
/// and the reverse builder sees 0 (`kAlwaysDismissedAnimation`). Once it
/// completes or runs in reverse, the forward builder sees 1
/// (`kAlwaysCompleteAnimation`) and the reverse builder follows the reversed
/// animation, `1 - value`. An animation that turns around midway keeps
/// playing the transition it started.
#[derive(Clone)]
pub struct DualTransitionBuilder {
    animation: AnimationController,
    forward_builder: AnimatedTransitionBuilder,
    reverse_builder: AnimatedTransitionBuilder,
    child: Option<Widget>,
}

impl DualTransitionBuilder {
    #[must_use]
    pub fn new(
        animation: AnimationController,
        forward_builder: impl for<'a> Fn(&BuildContext<'a>, f32, Option<Widget>) -> Widget + 'static,
        reverse_builder: impl for<'a> Fn(&BuildContext<'a>, f32, Option<Widget>) -> Widget + 'static,
    ) -> Self {
        Self {
            animation,
            forward_builder: Rc::new(forward_builder),
            reverse_builder: Rc::new(reverse_builder),
            child: None,
        }
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// `_calculateEffectiveAnimationStatus`: a transition interrupted
    /// midway plays back the way it came instead of switching to the other
    /// transition.
    #[must_use]
    pub fn effective_status(last: AnimationStatus, current: AnimationStatus) -> AnimationStatus {
        match (current, last) {
            (AnimationStatus::Forward, AnimationStatus::Reverse)
            | (AnimationStatus::Reverse, AnimationStatus::Forward) => last,
            _ => current,
        }
    }

    /// `_updateAnimations`: the values the forward and reverse builders see
    /// for an effective status and the animation's value.
    #[must_use]
    pub fn builder_values(effective: AnimationStatus, value: f32) -> (f32, f32) {
        match effective {
            AnimationStatus::Dismissed | AnimationStatus::Forward => (value, 0.0),
            AnimationStatus::Reverse | AnimationStatus::Completed => (1.0, 1.0 - value),
        }
    }
}

struct DualState {
    listener: AnimationListenerState,
    effective: Cell<Option<AnimationStatus>>,
}

impl RetainedState for DualState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        self.listener.rebind_revision(revision);
    }
}

impl DualState {
    /// `_animationListener`.
    fn update(&self, current: AnimationStatus) -> AnimationStatus {
        let effective = match self.effective.get() {
            Some(last) => DualTransitionBuilder::effective_status(last, current),
            None => current,
        };
        self.effective.set(Some(effective));
        effective
    }
}

impl From<DualTransitionBuilder> for Widget {
    fn from(value: DualTransitionBuilder) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            |revision| DualState {
                listener: AnimationListenerState::new(revision),
                effective: Cell::new(None),
            },
            move |state, context, _| {
                let animation = &config.animation;
                let weak = Rc::downgrade(state);
                let listened = animation.clone();
                state.listener.listen(animation, move || {
                    if let Some(state) = weak.upgrade() {
                        state.update(listened.status());
                    }
                });
                let effective = state.update(animation.status());
                let (forward, reverse) =
                    DualTransitionBuilder::builder_values(effective, animation.value());
                let reversed = (config.reverse_builder)(context, reverse, config.child.clone());
                let child = (config.forward_builder)(context, forward, Some(reversed));
                Widget::animation_ticker_with_revision(
                    animation.clone(),
                    false,
                    state.listener.revision(),
                    child,
                )
            },
            false,
        )
    }
}

/// Flutter's `Animation<T>` built by `Tween.animate(CurvedAnimation(...))`:
/// a controller's value through a curve, then interpolated from `begin` to
/// `end` with [`Lerp`].
#[derive(Clone)]
pub struct TweenAnimation<T> {
    controller: AnimationController,
    curve: Curve,
    tween: PropertyTween<T>,
}

impl<T: Lerp + Clone> TweenAnimation<T> {
    #[must_use]
    pub fn new(controller: AnimationController, begin: T, end: T) -> Self {
        Self {
            controller,
            curve: Curve::Linear,
            tween: PropertyTween { begin, end },
        }
    }

    /// The curve between the controller and the tween. Defaults to linear.
    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    #[must_use]
    pub fn controller(&self) -> &AnimationController {
        &self.controller
    }

    /// The tween evaluated at the curved controller value: exactly `begin`
    /// at 0 and `end` at 1.
    #[must_use]
    pub fn value(&self) -> T {
        self.tween
            .evaluate(self.curve.apply(self.controller.value()))
    }
}

/// A controller listener that bumps a retained build's revision, so the
/// build reruns whenever the controller notifies: Flutter `AnimatedWidget`'s
/// `_handleChange`.
pub(crate) struct AnimationListenerState {
    revision: RefCell<Rc<Cell<u64>>>,
    listening: RefCell<Option<(AnimationController, usize)>>,
}

impl AnimationListenerState {
    pub(crate) fn new(revision: Rc<Cell<u64>>) -> Self {
        Self {
            revision: RefCell::new(revision),
            listening: RefCell::new(None),
        }
    }

    pub(crate) fn revision(&self) -> Rc<Cell<u64>> {
        self.revision.borrow().clone()
    }

    pub(crate) fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }

    /// Listens to `controller`, dropping the listener of a replaced one
    /// (`didUpdateWidget`). `also` runs on every notification before the
    /// rebuild is scheduled.
    pub(crate) fn listen(&self, controller: &AnimationController, also: impl Fn() + 'static) {
        if matches!(&*self.listening.borrow(), Some((listened, _)) if listened == controller) {
            return;
        }
        if let Some((old, id)) = self.listening.borrow_mut().take() {
            old.remove_listener(id);
        }
        let revision = Rc::downgrade(&self.revision.borrow());
        let id = controller.add_listener(move |_| {
            also();
            if let Some(revision) = revision.upgrade() {
                revision.set(revision.get().wrapping_add(1));
            }
        });
        *self.listening.borrow_mut() = Some((controller.clone(), id));
    }
}

impl Drop for AnimationListenerState {
    fn drop(&mut self) {
        if let Some((controller, id)) = self.listening.borrow_mut().take() {
            controller.remove_listener(id);
        }
    }
}

struct AnimatedWidgetState(AnimationListenerState);

impl RetainedState for AnimatedWidgetState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        self.0.rebind_revision(revision);
    }
}

/// Flutter's `AnimatedWidget`: `build` reruns whenever `controller`
/// notifies, and the controller ticks with the tree's frames.
pub(crate) fn animated_widget(
    controller: AnimationController,
    build: impl for<'a> Fn(&BuildContext<'a>) -> Widget + 'static,
) -> Widget {
    Widget::retained_builder(
        |revision| AnimatedWidgetState(AnimationListenerState::new(revision)),
        move |state, context, _| {
            state.0.listen(&controller, || {});
            Widget::animation_ticker_with_revision(
                controller.clone(),
                false,
                state.0.revision(),
                build(context),
            )
        },
        false,
    )
}
