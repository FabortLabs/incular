//! Flutter's `StretchEffect` (`widgets/stretch_effect.dart`) and its
//! `StretchingOverscrollIndicator` and `GlowingOverscrollIndicator`
//! (`widgets/overscroll_indicator.dart`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use incular_animation::{
    AnimationController, AnimationStatus,
    physics::{SpringDescription, SpringSimulation, Tolerance},
};
use incular_config::{Axis, AxisDirection, Clip, TextDirection};
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_scroll::{ScrollNotification, ScrollNotificationType};

use crate::tree::RetainedState;
use crate::{Builder, ClipRect, NotificationListener, Stack, Transform, Widget};

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

/// Flutter `_GlowState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GlowState {
    Idle,
    Absorb,
    Pull,
    Recede,
}

// Flutter's `_GlowController` constants.
const RECEDE_TIME: Duration = Duration::from_millis(600);
const PULL_TIME: Duration = Duration::from_millis(167);
const PULL_HOLD_TIME: Duration = Duration::from_millis(167);
const PULL_DECAY_TIME: Duration = Duration::from_millis(2000);
/// `_crossAxisHalfTime`: a sixtieth of a second, in microseconds.
const CROSS_AXIS_HALF_TIME_MICROS: f32 = 16_667.0;
const MAX_OPACITY: f32 = 0.5;
const PULL_OPACITY_GLOW_FACTOR: f32 = 0.8;
const VELOCITY_GLOW_FACTOR: f32 = 0.00006;
const SQRT3: f32 = 1.732_050_8;
const WIDTH_TO_HEIGHT_FACTOR: f32 = (3.0 / 4.0) * (2.0 - SQRT3);
const MIN_VELOCITY: f32 = 100.0;
const MAX_VELOCITY: f32 = 10_000.0;
/// `Tolerance.defaultTolerance.distance`.
const DISPLACEMENT_TOLERANCE: f32 = 1e-3;
/// The displacement ticker's clock runs on a long controller.
const DISPLACEMENT_CLOCK: Duration = Duration::from_secs(3_600);

/// Flutter's `Curves.decelerate`.
fn decelerate(t: f32) -> f32 {
    let t = 1.0 - t;
    1.0 - t * t
}

/// Flutter `_GlowController`: one edge's glow.
struct GlowController {
    state: Cell<GlowState>,
    glow: AnimationController,
    /// The glow animation's completion was handled (`_changePhase`).
    glow_settled: Cell<bool>,
    opacity: Cell<(f32, f32)>,
    size: Cell<(f32, f32)>,
    /// `_pullRecedeTimer`, as a controller whose completion fires it.
    pull_timer: AnimationController,
    pull_timer_armed: Cell<bool>,
    displacement_ticker: AnimationController,
    displacement_last_elapsed: Cell<Option<f32>>,
    displacement_target: Cell<f32>,
    displacement: Cell<f32>,
    pull_distance: Cell<f32>,
    paint_offset_scroll_pixels: Cell<f32>,
}

impl GlowController {
    fn new() -> Self {
        Self {
            state: Cell::new(GlowState::Idle),
            glow: AnimationController::new(Duration::ZERO),
            glow_settled: Cell::new(true),
            opacity: Cell::new((0.0, 0.0)),
            size: Cell::new((0.0, 0.0)),
            pull_timer: AnimationController::new(PULL_HOLD_TIME),
            pull_timer_armed: Cell::new(false),
            displacement_ticker: AnimationController::new(DISPLACEMENT_CLOCK),
            displacement_last_elapsed: Cell::new(None),
            displacement_target: Cell::new(0.5),
            displacement: Cell::new(0.5),
            pull_distance: Cell::new(0.0),
            paint_offset_scroll_pixels: Cell::new(0.0),
        }
    }

    fn tweened(&self, tween: (f32, f32)) -> f32 {
        let t = decelerate(self.glow.value());
        tween.0 + (tween.1 - tween.0) * t
    }

    fn glow_opacity(&self) -> f32 {
        self.tweened(self.opacity.get())
    }

    fn glow_size(&self) -> f32 {
        self.tweened(self.size.get())
    }

    /// `_glowController.forward(from: 0.0)`.
    fn restart(&self, duration: Duration) {
        self.glow.set_duration(duration);
        self.glow.set_value(0.0);
        self.glow_settled.set(false);
        self.glow.forward_on_next_tick();
    }

    fn cancel_pull_timer(&self) {
        self.pull_timer_armed.set(false);
        self.pull_timer.stop();
    }

    fn absorb_impact(&self, velocity: f32) {
        self.cancel_pull_timer();
        let velocity = velocity.clamp(MIN_VELOCITY, MAX_VELOCITY);
        let begin = if self.state.get() == GlowState::Idle {
            0.3
        } else {
            self.glow_opacity()
        };
        let end = (velocity * VELOCITY_GLOW_FACTOR).clamp(begin, MAX_OPACITY);
        self.opacity.set((begin, end));
        self.size.set((
            self.glow_size(),
            (0.025 + 7.5e-7 * velocity * velocity).min(1.0),
        ));
        self.restart(Duration::from_millis(
            (0.15 + velocity * 0.02).round() as u64
        ));
        self.displacement.set(0.5);
        self.state.set(GlowState::Absorb);
    }

    fn pull(&self, overscroll: f32, extent: f32, cross_axis_offset: f32, cross_extent: f32) {
        self.cancel_pull_timer();
        // Flutter's magic factor matching Android.
        self.pull_distance
            .set(self.pull_distance.get() + overscroll / 200.0);
        let opacity = self.glow_opacity();
        self.opacity.set((
            opacity,
            (opacity + overscroll / extent * PULL_OPACITY_GLOW_FACTOR).min(MAX_OPACITY),
        ));
        let height = extent.min(cross_extent * WIDTH_TO_HEIGHT_FACTOR);
        let size = self.glow_size();
        self.size.set((
            size,
            (1.0 - 1.0 / (0.7 * (self.pull_distance.get() * height).sqrt())).max(size),
        ));
        self.displacement_target
            .set(cross_axis_offset / cross_extent);
        if self.displacement_target.get() == self.displacement.get() {
            self.displacement_ticker.stop();
            self.displacement_last_elapsed.set(None);
        } else if !self.displacement_ticker.is_active() {
            self.displacement_ticker.set_value(0.0);
            self.displacement_ticker.forward_on_next_tick();
        }
        if self.state.get() == GlowState::Pull {
            // A completed pull holds its values.
            if !self.glow.is_active() {
                self.glow.set_duration(PULL_TIME);
                self.glow.set_value(1.0);
            }
        } else {
            self.restart(PULL_TIME);
            self.state.set(GlowState::Pull);
        }
        self.pull_timer.set_value(0.0);
        self.pull_timer_armed.set(true);
        self.pull_timer.forward_on_next_tick();
    }

    fn scroll_end(&self) {
        if self.state.get() == GlowState::Pull {
            self.recede(RECEDE_TIME);
        }
    }

    /// `_changePhase` when the glow animation completes.
    fn change_phase(&self) {
        match self.state.get() {
            GlowState::Absorb => self.recede(RECEDE_TIME),
            GlowState::Recede => {
                self.state.set(GlowState::Idle);
                self.pull_distance.set(0.0);
            }
            GlowState::Pull | GlowState::Idle => {}
        }
    }

    fn recede(&self, duration: Duration) {
        if matches!(self.state.get(), GlowState::Recede | GlowState::Idle) {
            return;
        }
        self.cancel_pull_timer();
        self.opacity.set((self.glow_opacity(), 0.0));
        self.size.set((self.glow_size(), 0.0));
        self.restart(duration);
        self.state.set(GlowState::Recede);
    }

    /// `_tickDisplacement` at `elapsed` seconds on the ticker.
    fn tick_displacement(&self, elapsed: f32) {
        if let Some(last) = self.displacement_last_elapsed.get() {
            let t = (elapsed - last) * 1_000_000.0;
            let target = self.displacement_target.get();
            self.displacement.set(
                target
                    - (target - self.displacement.get())
                        * 2f32.powf(-t / CROSS_AXIS_HALF_TIME_MICROS),
            );
        }
        if (self.displacement_target.get() - self.displacement.get()).abs()
            <= DISPLACEMENT_TOLERANCE
        {
            self.displacement_ticker.stop();
            self.displacement_last_elapsed.set(None);
        } else {
            self.displacement_last_elapsed.set(Some(elapsed));
        }
    }

    /// `_GlowController.paint` into `canvas`, already in this edge's frame
    /// (`transform`) with the edge's `size`.
    fn paint(
        &self,
        canvas: &mut incular_rendering::Canvas,
        transform: CoreTransform,
        size: Size,
        color: Color,
    ) {
        let opacity = self.glow_opacity();
        if opacity == 0.0 {
            return;
        }
        let base_glow_scale = if size.width > size.height {
            size.height / size.width
        } else {
            1.0
        };
        let radius = size.width * 3.0 / 2.0;
        let height = size.height.min(size.width * WIDTH_TO_HEIGHT_FACTOR);
        let scale_y = self.glow_size() * base_glow_scale;
        let center = Offset::new(
            (size.width / 2.0) * (0.5 + self.displacement.get()),
            height - radius,
        );
        let transform = transform
            .then(CoreTransform::translation(Offset::new(
                0.0,
                self.paint_offset_scroll_pixels.get(),
            )))
            .then(CoreTransform::scale_non_uniform(1.0, scale_y));
        canvas.save_transform(transform);
        canvas.save_clip(Rect::from_origin_size(
            Offset::ZERO,
            Size::new(size.width, height),
        ));
        let alpha = (f32::from(color.alpha) * opacity).round().clamp(0.0, 255.0) as u8;
        canvas.fill_path(
            Arc::new(circle(center, radius)),
            Color::rgba(color.red, color.green, color.blue, alpha),
            incular_rendering::FillRule::NonZero,
        );
        canvas.restore();
        canvas.restore();
    }
}

/// A circle as four conic quarter arcs.
fn circle(center: Offset, radius: f32) -> incular_rendering::Path {
    let weight = std::f32::consts::FRAC_1_SQRT_2;
    let point = |x: f32, y: f32| Offset::new(center.x + x * radius, center.y + y * radius);
    let mut builder = incular_rendering::Path::builder();
    builder
        .move_to(point(1.0, 0.0))
        .conic_to(point(1.0, 1.0), point(0.0, 1.0), weight)
        .conic_to(point(-1.0, 1.0), point(-1.0, 0.0), weight)
        .conic_to(point(-1.0, -1.0), point(0.0, -1.0), weight)
        .conic_to(point(1.0, -1.0), point(1.0, 0.0), weight)
        .close();
    builder.build()
}

/// Flutter's `GlowingOverscrollIndicator`: overscrolling its scroll view
/// glows at the edge (the Android behavior before Material 3).
#[derive(Clone)]
pub struct GlowingOverscrollIndicator {
    show_leading: bool,
    show_trailing: bool,
    axis_direction: AxisDirection,
    color: Color,
    notification_predicate: Rc<dyn Fn(&ScrollNotification) -> bool>,
    child: Widget,
}

impl GlowingOverscrollIndicator {
    #[must_use]
    pub fn new(axis_direction: AxisDirection, color: Color, child: impl Into<Widget>) -> Self {
        Self {
            show_leading: true,
            show_trailing: true,
            axis_direction,
            color,
            notification_predicate: Rc::new(|notification| notification.depth == 0),
            child: child.into(),
        }
    }

    /// Whether the leading edge glows. Defaults to true.
    #[must_use]
    pub fn show_leading(mut self, show: bool) -> Self {
        self.show_leading = show;
        self
    }

    /// Whether the trailing edge glows. Defaults to true.
    #[must_use]
    pub fn show_trailing(mut self, show: bool) -> Self {
        self.show_trailing = show;
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
        axis_of(self.axis_direction)
    }
}

fn axis_of(direction: AxisDirection) -> Axis {
    match direction {
        AxisDirection::Up | AxisDirection::Down => Axis::Vertical,
        AxisDirection::Left | AxisDirection::Right => Axis::Horizontal,
    }
}

fn flipped(direction: AxisDirection) -> AxisDirection {
    match direction {
        AxisDirection::Up => AxisDirection::Down,
        AxisDirection::Down => AxisDirection::Up,
        AxisDirection::Left => AxisDirection::Right,
        AxisDirection::Right => AxisDirection::Left,
    }
}

/// `_GlowingOverscrollIndicatorState`.
struct GlowIndicatorState {
    leading: GlowController,
    trailing: GlowController,
    size: Cell<Size>,
    listening: Cell<bool>,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl RetainedState for GlowIndicatorState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl GlowIndicatorState {
    fn bump(&self) {
        let revision = self.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
    }

    fn controllers(&self) -> [&GlowController; 2] {
        [&self.leading, &self.trailing]
    }

    /// `_handleScrollNotification`.
    fn handle(&self, config: &GlowingOverscrollIndicator, notification: &ScrollNotification) {
        if !(config.notification_predicate)(notification)
            || notification.metrics.axis != config.axis()
        {
            return;
        }
        let metrics = notification.metrics;
        // With no `OverscrollIndicatorNotification` listener the paint
        // offset stays zero.
        self.leading
            .paint_offset_scroll_pixels
            .set(-(metrics.pixels - metrics.min_scroll_extent).min(0.0));
        self.trailing
            .paint_offset_scroll_pixels
            .set(-(metrics.max_scroll_extent - metrics.pixels).min(0.0));
        match notification.kind {
            ScrollNotificationType::Overscroll => {
                let controller = if notification.overscroll < 0.0 {
                    &self.leading
                } else if notification.overscroll > 0.0 {
                    &self.trailing
                } else {
                    return;
                };
                if notification.velocity != 0.0 {
                    controller.absorb_impact(notification.velocity.abs());
                } else if let Some(position) = notification.drag_position {
                    let size = self.size.get();
                    match metrics.axis {
                        Axis::Horizontal => controller.pull(
                            notification.overscroll.abs(),
                            size.width,
                            position.y.clamp(0.0, size.height),
                            size.height,
                        ),
                        Axis::Vertical => controller.pull(
                            notification.overscroll.abs(),
                            size.height,
                            position.x.clamp(0.0, size.width),
                            size.width,
                        ),
                    }
                }
                self.bump();
            }
            ScrollNotificationType::End => {
                self.leading.scroll_end();
                self.trailing.scroll_end();
                self.bump();
            }
            ScrollNotificationType::Update if notification.drag_position.is_some() => {
                self.leading.scroll_end();
                self.trailing.scroll_end();
                self.bump();
            }
            _ => {}
        }
    }

    /// `_GlowingOverscrollIndicatorPainter.paint`.
    fn paint(
        &self,
        config: &GlowingOverscrollIndicator,
        size: Size,
    ) -> incular_rendering::DisplayList {
        let mut canvas = incular_rendering::Canvas::default();
        let sides = [
            (
                config.show_leading,
                &self.leading,
                flipped(config.axis_direction),
            ),
            (config.show_trailing, &self.trailing, config.axis_direction),
        ];
        for (show, controller, direction) in sides {
            if !show {
                continue;
            }
            let quarter = std::f32::consts::FRAC_PI_2;
            let (transform, edge) = match direction {
                AxisDirection::Up => (CoreTransform::IDENTITY, size),
                AxisDirection::Down => (
                    CoreTransform::translation(Offset::new(0.0, size.height))
                        .then(CoreTransform::scale_non_uniform(1.0, -1.0)),
                    size,
                ),
                AxisDirection::Left => (
                    CoreTransform::rotation(quarter)
                        .then(CoreTransform::scale_non_uniform(1.0, -1.0)),
                    Size::new(size.height, size.width),
                ),
                AxisDirection::Right => (
                    CoreTransform::translation(Offset::new(size.width, 0.0))
                        .then(CoreTransform::rotation(quarter)),
                    Size::new(size.height, size.width),
                ),
            };
            controller.paint(&mut canvas, transform, edge, config.color);
        }
        canvas.finish()
    }
}

impl From<GlowingOverscrollIndicator> for Widget {
    fn from(value: GlowingOverscrollIndicator) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            |revision| GlowIndicatorState {
                leading: GlowController::new(),
                trailing: GlowController::new(),
                size: Cell::new(Size::ZERO),
                listening: Cell::new(false),
                revision: RefCell::new(revision),
            },
            move |state, _, constraints| {
                if !state.listening.replace(true) {
                    for (index, controller) in state.controllers().into_iter().enumerate() {
                        let weak = Rc::downgrade(state);
                        controller.glow.add_listener({
                            let weak = weak.clone();
                            move |_| {
                                let Some(state) = weak.upgrade() else { return };
                                let controller = glow_side(&state, index);
                                if controller.glow.status() == AnimationStatus::Completed
                                    && !controller.glow_settled.replace(true)
                                {
                                    controller.change_phase();
                                }
                                state.bump();
                            }
                        });
                        controller.pull_timer.add_listener({
                            let weak = weak.clone();
                            move |_| {
                                let Some(state) = weak.upgrade() else { return };
                                let controller = glow_side(&state, index);
                                if controller.pull_timer.status() == AnimationStatus::Completed
                                    && controller.pull_timer_armed.replace(false)
                                {
                                    controller.recede(PULL_DECAY_TIME);
                                    state.bump();
                                }
                            }
                        });
                        controller.displacement_ticker.add_listener(move |value| {
                            let Some(state) = weak.upgrade() else { return };
                            let controller = glow_side(&state, index);
                            controller.tick_displacement(value * DISPLACEMENT_CLOCK.as_secs_f32());
                            state.bump();
                        });
                    }
                }
                let size = Size::new(
                    if constraints.max_width().is_finite() {
                        constraints.max_width()
                    } else {
                        constraints.min_width()
                    },
                    if constraints.max_height().is_finite() {
                        constraints.max_height()
                    } else {
                        constraints.min_height()
                    },
                );
                state.size.set(size);
                let glow = crate::IgnorePointer::new(crate::CustomPaint::new(
                    size,
                    state.paint(&config, size),
                ));
                let mut content: Widget = Stack::new([config.child.clone(), glow.into()])
                    .fit(crate::StackFit::Passthrough)
                    .into();
                let revision = state.revision.borrow().clone();
                for controller in state.controllers() {
                    for animation in [
                        &controller.glow,
                        &controller.pull_timer,
                        &controller.displacement_ticker,
                    ] {
                        content = Widget::animation_ticker_with_revision(
                            animation.clone(),
                            false,
                            revision.clone(),
                            content,
                        );
                    }
                }
                let handler = Rc::downgrade(state);
                let handler_config = config.clone();
                NotificationListener::new(content)
                    .on_notification(move |notification| {
                        if let Some(state) = handler.upgrade() {
                            state.handle(&handler_config, &notification);
                        }
                        false
                    })
                    .into()
            },
            true,
        )
    }
}

fn glow_side(state: &GlowIndicatorState, index: usize) -> &GlowController {
    if index == 0 {
        &state.leading
    } else {
        &state.trailing
    }
}
