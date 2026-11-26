//! Flutter's `SliverAnimatedOpacity` (`widgets/implicit_animations.dart`),
//! a sliver whose opacity animates to each new target, and
//! `SliverFadeTransition` (`widgets/transitions.dart`).

use std::any::Any;

use incular_animation::{AnimationStatus, DoubleAnimation};

use super::layout::WidgetWrapRenderSliver;
use super::*;
use crate::implicit_animations::PropertyTween;
use crate::tree::OpacityController;

/// Flutter's `SliverFadeTransition`: the sliver's opacity follows
/// `opacity`, as [`FadeTransition`](crate::FadeTransition) does for a box.
pub struct SliverFadeTransition {
    source: FadeSource,
    sliver: Box<dyn Sliver>,
}

/// Where the sliver's opacity comes from: a retained controller the caller
/// sets, or any `Animation<double>` the sliver samples on each tick.
enum FadeSource {
    Controller(OpacityController),
    Animation(DoubleAnimation),
}

impl SliverFadeTransition {
    #[must_use]
    pub fn new(opacity: OpacityController, sliver: impl Sliver + 'static) -> Self {
        Self {
            source: FadeSource::Controller(opacity),
            sliver: Box::new(sliver),
        }
    }

    /// Flutter's `SliverFadeTransition(opacity: Animation<double>)`: the
    /// opacity is the animation's value, so curved, reversed or composed
    /// animations drive it directly. The value is clamped to `0.0..=1.0`.
    #[must_use]
    pub fn from_animation(
        opacity: impl Into<DoubleAnimation>,
        sliver: impl Sliver + 'static,
    ) -> Self {
        Self {
            source: FadeSource::Animation(opacity.into()),
            sliver: Box::new(sliver),
        }
    }
}

impl Sliver for SliverFadeTransition {
    fn build(&self, controller: &ScrollController) -> Widget {
        match &self.source {
            FadeSource::Controller(opacity) => {
                crate::Opacity::controlled(opacity.clone(), self.sliver.build(controller)).into()
            }
            FadeSource::Animation(animation) => crate::FadeTransition::from_animation(
                animation.clone(),
                self.sliver.build(controller),
            ),
        }
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        let inner = self.sliver.create_render_sliver(controller, axis, reverse);
        match &self.source {
            FadeSource::Controller(opacity) => {
                let opacity = opacity.clone();
                Box::new(WidgetWrapRenderSliver {
                    inner: RefCell::new(inner),
                    hidden: false,
                    wrap: Rc::new(move |widget| {
                        crate::Opacity::controlled(opacity.clone(), widget).into()
                    }),
                })
            }
            FadeSource::Animation(animation) => Box::new(AnimatedFadeRenderSliver {
                inner: RefCell::new(inner),
                value: animation.value().clamp(0., 1.),
                animation: animation.clone(),
            }),
        }
    }
}

/// The render side of a [`SliverFadeTransition`] driven by an animation.
/// It samples the animation on every tick and lays its children out at the
/// current opacity, as [`AnimatedOpacityRenderSliver`] does for its target.
struct AnimatedFadeRenderSliver {
    inner: RefCell<Box<dyn RenderSliver>>,
    animation: DoubleAnimation,
    /// The value laid out last, so a tick reports only real changes.
    value: f32,
}

impl RenderSliver for AnimatedFadeRenderSliver {
    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        self.inner.get_mut().reveal_child(child, rect, options)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.inner.borrow().scroll_layout_dependency()
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let mut layout = self.inner.borrow_mut().perform_layout(constraints);
        let opacity = self.animation.value().clamp(0., 1.);
        for child in &mut layout.children {
            child.widget = crate::Opacity::new(opacity, child.widget.clone()).into();
        }
        layout
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        self.inner.borrow_mut().invalidate_child_measurement(child);
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        self.inner.borrow_mut().set_child_extent(child, extent)
    }

    fn revision(&self) -> u64 {
        self.inner.borrow().revision()
    }

    fn tick(&mut self, now: Instant) -> bool {
        let inner = self.inner.borrow_mut().tick(now);
        // Ticking with the frame time is idempotent, so a controller that
        // another ticker already advanced this frame reports no change here.
        for controller in self.animation.controllers() {
            controller.tick(now);
        }
        let value = self.animation.value().clamp(0., 1.);
        let changed = value != self.value;
        self.value = value;
        inner || changed
    }

    fn is_animating(&self) -> bool {
        self.animation.is_animating() || self.inner.borrow().is_animating()
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        self.inner.borrow_mut().set_reduced_motion(reduced);
    }
}

/// Flutter's `SliverAnimatedOpacity`.
pub struct SliverAnimatedOpacity {
    opacity: f32,
    sliver: Box<dyn Sliver>,
    duration: Duration,
    curve: Curve,
    on_end: Option<Rc<dyn Fn()>>,
}

impl SliverAnimatedOpacity {
    /// `opacity` is clamped to `0.0..=1.0`.
    #[must_use]
    pub fn new(opacity: f32, duration: Duration, sliver: impl Sliver + 'static) -> Self {
        Self {
            opacity: opacity.clamp(0., 1.),
            sliver: Box::new(sliver),
            duration,
            curve: Curve::Linear,
            on_end: None,
        }
    }

    /// The curve applied to the animation. Defaults to linear.
    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Called each time an animation completes.
    #[must_use]
    pub fn on_end(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_end = Some(Rc::new(callback));
        self
    }
}

impl Sliver for SliverAnimatedOpacity {
    fn build(&self, controller: &ScrollController) -> Widget {
        crate::Opacity::new(self.opacity, self.sliver.build(controller)).into()
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(AnimatedOpacityRenderSliver {
            inner: RefCell::new(self.sliver.create_render_sliver(controller, axis, reverse)),
            target: self.opacity,
            curve: self.curve.clone(),
            on_end: self.on_end.clone(),
            controller: AnimationController::new(self.duration),
            tween: PropertyTween {
                begin: self.opacity,
                end: self.opacity,
            },
        })
    }
}

/// The retained state of `_SliverAnimatedOpacityState`. A rebuilt sliver
/// creates a fresh render sliver, which [`Self::adopt`] continues from the
/// retained one, as `didUpdateWidget` does.
pub(super) struct AnimatedOpacityRenderSliver {
    inner: RefCell<Box<dyn RenderSliver>>,
    target: f32,
    curve: Curve,
    on_end: Option<Rc<dyn Fn()>>,
    controller: AnimationController,
    tween: PropertyTween<f32>,
}

impl AnimatedOpacityRenderSliver {
    fn opacity(&self) -> f32 {
        self.tween
            .evaluate(self.curve.apply(self.controller.value()))
    }

    /// Takes over the retained animation, then restarts it from the
    /// current opacity when the target moved.
    pub(super) fn adopt(&mut self, retained: &Self) {
        let duration = self.controller.duration();
        self.controller = retained.controller.clone();
        self.controller.set_duration(duration);
        self.tween = retained.tween.clone();
        if self.target != self.tween.end {
            let t = self.curve.apply(self.controller.value());
            self.tween = PropertyTween {
                begin: self.tween.evaluate(t),
                end: self.target,
            };
            self.controller.set_value(0.);
            self.controller.forward_on_next_tick();
        }
    }

    pub(super) fn inner_cell(&mut self) -> &RefCell<Box<dyn RenderSliver>> {
        &self.inner
    }
}

impl RenderSliver for AnimatedOpacityRenderSliver {
    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        self.inner.get_mut().reveal_child(child, rect, options)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.inner.borrow().scroll_layout_dependency()
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let mut layout = self.inner.borrow_mut().perform_layout(constraints);
        let opacity = self.opacity();
        for child in &mut layout.children {
            child.widget = crate::Opacity::new(opacity, child.widget.clone()).into();
        }
        layout
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        self.inner.borrow_mut().invalidate_child_measurement(child);
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        self.inner.borrow_mut().set_child_extent(child, extent)
    }

    fn revision(&self) -> u64 {
        self.inner.borrow().revision()
    }

    fn tick(&mut self, now: Instant) -> bool {
        let inner = self.inner.borrow_mut().tick(now);
        let was_completed = self.controller.status() == AnimationStatus::Completed;
        let changed = self.controller.tick(now);
        if changed
            && !was_completed
            && self.controller.status() == AnimationStatus::Completed
            && let Some(on_end) = &self.on_end
        {
            on_end();
        }
        inner || changed
    }

    fn is_animating(&self) -> bool {
        self.controller.is_active() || self.inner.borrow().is_animating()
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        self.inner.borrow_mut().set_reduced_motion(reduced);
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}
