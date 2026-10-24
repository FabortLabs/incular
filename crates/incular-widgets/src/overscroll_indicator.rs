//! Flutter's `StretchEffect` (`widgets/stretch_effect.dart`) and
//! `StretchingOverscrollIndicator` (`widgets/overscroll_indicator.dart`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_animation::{
    AnimationController, AnimationStatus,
    physics::{SpringDescription, SpringSimulation, Tolerance},
};
use incular_config::{Axis, AxisDirection, Clip, TextDirection};
use incular_core::{Offset, Transform as CoreTransform};
use incular_scroll::{ScrollNotification, ScrollNotificationType};

use crate::tree::RetainedState;
use crate::{Builder, ClipRect, NotificationListener, Transform, Widget};

/// Flutter's `StretchEffect`: stretches its child along `axis` by
/// `stretch_strength` (from -1 to 1), anchored at the edge it is pulled
/// from. This is Flutter's transform fallback for renderers without its
/// stretch shader: the child scales by `1 + |strength|` along the axis.
#[derive(Clone)]
pub struct StretchEffect {
    stretch_strength: f32,
    axis: Axis,
    child: Widget,
}

impl StretchEffect {
    #[must_use]
    pub fn new(stretch_strength: f32, axis: Axis, child: impl Into<Widget>) -> Self {
        Self {
            stretch_strength: if stretch_strength.is_finite() {
                stretch_strength.clamp(-1., 1.)
            } else {
                0.
            },
            axis,
            child: child.into(),
        }
    }
}

impl From<StretchEffect> for Widget {
    fn from(value: StretchEffect) -> Self {
        if value.stretch_strength == 0. {
            return value.child;
        }
        Builder::new(move |context| {
            let strength = value.stretch_strength;
            let forward = strength > 0.;
            let scale = 1. + strength.abs();
            // `_getAlignment`: the pull edge stays put. Vertically that is
            // the top for a forward stretch; horizontally the start, which
            // is the right edge in right-to-left text.
            let (transform, far_edge) = match value.axis {
                Axis::Vertical => (CoreTransform::scale_non_uniform(1., scale), !forward),
                Axis::Horizontal => {
                    let rtl = context.text_direction() == Some(TextDirection::Rtl);
                    (CoreTransform::scale_non_uniform(scale, 1.), forward == rtl)
                }
            };
            let scaled = Transform::new(transform, value.child.clone()).origin(Offset::ZERO);
            if !far_edge {
                return scaled.into();
            }
            // Anchoring at the far (bottom or right) edge: move that edge to
            // the origin, scale, and move it back.
            let fraction = match value.axis {
                Axis::Vertical => Offset::new(0., 1.),
                Axis::Horizontal => Offset::new(1., 0.),
            };
            let inner = Widget::fractional_translation(
                Offset::new(-fraction.x, -fraction.y),
                true,
                value.child.clone(),
            );
            Widget::fractional_translation(
                fraction,
                true,
                Transform::new(transform, inner).origin(Offset::ZERO).into(),
            )
        })
        .into()
    }
}

/// Flutter's `StretchingOverscrollIndicator`: overscrolling its scroll view
/// stretches the content (the Material 3 Android behavior).
#[derive(Clone)]
pub struct StretchingOverscrollIndicator {
    axis_direction: AxisDirection,
    clip_behavior: Clip,
    notification_predicate: Rc<dyn Fn(&ScrollNotification) -> bool>,
    child: Widget,
}

impl StretchingOverscrollIndicator {
    #[must_use]
    pub fn new(axis_direction: AxisDirection, child: impl Into<Widget>) -> Self {
        Self {
            axis_direction,
            clip_behavior: Clip::HardEdge,
            // `defaultScrollNotificationPredicate`: the nearest scroll view.
            notification_predicate: Rc::new(|notification| notification.depth == 0),
            child: child.into(),
        }
    }

    /// Defaults to [`Clip::HardEdge`].
    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    #[must_use]
    pub fn notification_predicate(
        mut self,
        predicate: impl Fn(&ScrollNotification) -> bool + 'static,
    ) -> Self {
        self.notification_predicate = Rc::new(predicate);
        self
    }

    fn axis(&self) -> Axis {
        match self.axis_direction {
            AxisDirection::Up | AxisDirection::Down => Axis::Vertical,
            AxisDirection::Left | AxisDirection::Right => Axis::Horizontal,
        }
    }
}

// Flutter's `_StretchController` constants, ported from Android's
// `EdgeEffect.java`.
const EXPONENTIAL_SCALAR: f32 = std::f32::consts::E / 0.33;
const STRETCH_INTENSITY: f32 = 0.016;
const FLING_VELOCITY_FRICTION: f32 = 1. / 6000.;
const ABSORB_IMPACT_VELOCITY_FRICTION: f32 = 1. / 3000.;
const MAX_FLING_VELOCITY: f32 = 0.5;
const MAX_ABSORB_IMPACT_VELOCITY: f32 = 1.25;
const NATURAL_FREQUENCY: f32 = 24.657;
const DAMPING_RATIO: f32 = 0.98;
const TIME_CORRECTION_FACTOR: f32 = 0.8;

/// Flutter's `_StretchController`.
struct StretchState {
    controller: AnimationController,
    animating: Cell<bool>,
    overscroll: Cell<f32>,
    interrupted_overscroll: Cell<f32>,
    total_overscroll: Cell<f32>,
    viewport_dimension: Cell<Option<f32>>,
    listening: Cell<bool>,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl RetainedState for StretchState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl StretchState {
    fn set_overscroll(&self, value: f32) {
        let value = value.clamp(-1., 1.);
        if value != self.overscroll.replace(value) {
            let revision = self.revision.borrow().clone();
            revision.set(revision.get().wrapping_add(1));
        }
    }

    fn animate(&self, velocity: f32) {
        let stiffness = NATURAL_FREQUENCY * NATURAL_FREQUENCY;
        let spring = SpringDescription::with_damping_ratio(
            1.,
            stiffness * TIME_CORRECTION_FACTOR * TIME_CORRECTION_FACTOR,
            DAMPING_RATIO,
        );
        self.animating.set(true);
        self.controller.animate_with(SpringSimulation::new(
            spring,
            self.overscroll.get(),
            0.,
            velocity * TIME_CORRECTION_FACTOR,
            Tolerance::DEFAULT,
        ));
    }

    fn absorb_impact(&self, velocity: f32) {
        if velocity == 0. {
            return;
        }
        self.animate(
            (velocity * ABSORB_IMPACT_VELOCITY_FRICTION)
                .clamp(-MAX_ABSORB_IMPACT_VELOCITY, MAX_ABSORB_IMPACT_VELOCITY),
        );
    }

    fn scroll_end(&self, velocity: f32) {
        if velocity == 0. && self.overscroll.get() == 0. {
            return;
        }
        if !self.animating.get() {
            self.animate(
                (-(velocity * FLING_VELOCITY_FRICTION))
                    .clamp(-MAX_FLING_VELOCITY, MAX_FLING_VELOCITY),
            );
        }
    }

    fn pull(&self, normalized: f32) {
        if self.animating.replace(false) {
            self.interrupted_overscroll.set(self.controller.value());
            self.controller.stop();
        }
        let distance = normalized.abs();
        let linear = STRETCH_INTENSITY * distance;
        let exponential = STRETCH_INTENSITY * (1. - (-distance * EXPONENTIAL_SCALAR).exp());
        self.set_overscroll(
            normalized.signum() * (linear + exponential) + self.interrupted_overscroll.get(),
        );
    }

    /// `_handleScrollNotification`.
    fn handle(&self, config: &StretchingOverscrollIndicator, notification: &ScrollNotification) {
        if !(config.notification_predicate)(notification)
            || notification.metrics.axis != config.axis()
        {
            return;
        }
        match notification.kind {
            ScrollNotificationType::Overscroll => {
                self.viewport_dimension
                    .set(Some(notification.metrics.viewport_dimension));
                self.total_overscroll
                    .set(self.total_overscroll.get() + notification.overscroll);
                if notification.velocity != 0. {
                    self.absorb_impact(notification.velocity);
                } else if notification.overscroll != 0. {
                    // Clamped to the viewport, the furthest one pointer pulls.
                    let viewport = notification.metrics.viewport_dimension;
                    let distance = if viewport > 0. {
                        self.total_overscroll.get() / viewport
                    } else {
                        0.
                    };
                    self.pull(distance.clamp(-1., 1.));
                }
            }
            ScrollNotificationType::End | ScrollNotificationType::Update => {
                // Incular's end carries no drag velocity: the content
                // springs back from rest.
                self.total_overscroll.set(0.);
                self.scroll_end(0.);
            }
            _ => {}
        }
    }
}

impl From<StretchingOverscrollIndicator> for Widget {
    fn from(value: StretchingOverscrollIndicator) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            |revision| StretchState {
                controller: AnimationController::unbounded(),
                animating: Cell::new(false),
                overscroll: Cell::new(0.),
                interrupted_overscroll: Cell::new(0.),
                total_overscroll: Cell::new(0.),
                viewport_dimension: Cell::new(None),
                listening: Cell::new(false),
                revision: RefCell::new(revision),
            },
            move |state, context, _| {
                if !state.listening.replace(true) {
                    let weak = Rc::downgrade(state);
                    state.controller.add_listener(move |value| {
                        let Some(state) = weak.upgrade() else {
                            return;
                        };
                        if !state.animating.get() {
                            return;
                        }
                        if state.controller.status() == AnimationStatus::Completed {
                            state.animating.set(false);
                            state.interrupted_overscroll.set(0.);
                            state.set_overscroll(0.);
                        } else {
                            state.set_overscroll(value);
                        }
                    });
                }
                let stretch = state.overscroll.get();
                let axis = config.axis();
                let screen = context.viewport();
                let main_axis = match axis {
                    Axis::Horizontal => screen.width,
                    Axis::Vertical => screen.height,
                };
                let viewport = state.viewport_dimension.get().unwrap_or(main_axis);
                let mut strength = -stretch;
                if matches!(
                    config.axis_direction,
                    AxisDirection::Up | AxisDirection::Left
                ) {
                    strength = -strength;
                }
                // Clipping matters only when the viewport is smaller than
                // the screen along the axis.
                let clip = if stretch != 0. && viewport != main_axis {
                    config.clip_behavior
                } else {
                    Clip::None
                };
                let content =
                    ClipRect::new(StretchEffect::new(strength, axis, config.child.clone()))
                        .clip_behavior(clip);
                let ticked = Widget::animation_ticker_with_revision(
                    state.controller.clone(),
                    false,
                    state.revision.borrow().clone(),
                    content,
                );
                let handler = Rc::downgrade(state);
                let handler_config = config.clone();
                NotificationListener::new(ticked)
                    .on_notification(move |notification| {
                        if let Some(state) = handler.upgrade() {
                            state.handle(&handler_config, &notification);
                        }
                        false
                    })
                    .into()
            },
            false,
        )
    }
}
