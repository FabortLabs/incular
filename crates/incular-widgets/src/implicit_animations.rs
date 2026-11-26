//! Flutter's implicitly animated widgets (`widgets/implicit_animations.dart`
//! and `widgets/tween_animation_builder.dart`): when a target property
//! changes, the widget animates from its current value to the new one.

use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::{
    AnimationController, AnimationStatus, Channel, Curve, MotionValue, Transition,
};
use incular_config::{
    Alignment, AlignmentDirectional, Clip, Constraints, EdgeInsets, TextDirection,
};
use incular_core::{Color, Lerp, Offset, Transform as CoreTransform};
use incular_text::{TextAlign, TextOverflow, TextStyle};

use crate::dx::motion::config::{MotionConfig, reduce};
use crate::environment::DefaultTextParagraph;
use crate::painting_effects::{BoxShape, PhysicalModel};
use crate::tree::{FrameDelegate, FrameTick, RetainedState};
use crate::{
    Align, BorderRadius, BoxDecoration, BuildContext, Container, DefaultTextStyle,
    FractionalTranslation, FractionallySizedBox, Opacity, Padding, Positioned, Transform, Widget,
};

/// One animated property: Flutter's `Tween` as the implicit animations use it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PropertyTween<T> {
    pub(crate) begin: T,
    pub(crate) end: T,
}

impl<T: Lerp + Clone> PropertyTween<T> {
    /// `Tween.transform`: exactly `begin` at 0 and `end` at 1.
    pub(crate) fn evaluate(&self, t: f32) -> T {
        if t == 0.0 {
            self.begin.clone()
        } else if t == 1.0 {
            self.end.clone()
        } else {
            self.begin.lerp(&self.end, t)
        }
    }
}

/// Flutter `AlignmentGeometry`: a physical alignment, or one whose horizontal
/// value is relative to the text direction. Mixed geometry keeps both
/// components until it is resolved against the ambient direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlignmentGeometry {
    Physical(Alignment),
    Directional(AlignmentDirectional),
    Mixed { x: f32, start: f32, y: f32 },
}

impl AlignmentGeometry {
    /// The physical alignment under `direction`.
    #[must_use]
    pub fn resolve(self, direction: TextDirection) -> Alignment {
        match self {
            Self::Physical(alignment) => alignment,
            Self::Directional(alignment) => alignment.resolve(direction),
            Self::Mixed { x, start, y } => Alignment::new(
                x + if direction == TextDirection::Rtl {
                    -start
                } else {
                    start
                },
                y,
            ),
        }
    }
}

impl Lerp for AlignmentGeometry {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let mix = |a, b| a + (b - a) * t;
        match (*self, *other) {
            (Self::Physical(a), Self::Physical(b)) => {
                Self::Physical(Alignment::new(mix(a.x, b.x), mix(a.y, b.y)))
            }
            (Self::Directional(a), Self::Directional(b)) => Self::Directional(
                AlignmentDirectional::new(mix(a.start, b.start), mix(a.y, b.y)),
            ),
            (a, b) => {
                let sum = |geometry| match geometry {
                    Self::Physical(a) => (a.x, 0., a.y),
                    Self::Directional(a) => (0., a.start, a.y),
                    Self::Mixed { x, start, y } => (x, start, y),
                };
                let (a, b) = (sum(a), sum(b));
                Self::Mixed {
                    x: mix(a.0, b.0),
                    start: mix(a.1, b.1),
                    y: mix(a.2, b.2),
                }
            }
        }
    }
}

impl From<Alignment> for AlignmentGeometry {
    fn from(value: Alignment) -> Self {
        Self::Physical(value)
    }
}

impl From<AlignmentDirectional> for AlignmentGeometry {
    fn from(value: AlignmentDirectional) -> Self {
        Self::Directional(value)
    }
}

pub use incular_config::EdgeInsetsGeometry;

/// The two walks `ImplicitlyAnimatedWidgetState` makes over its tweens. Each
/// carries the ambient text direction, which resolves directional targets.
pub(crate) enum TweenPass {
    /// `_constructTweens`: creates and drops tweens as targets appear and
    /// disappear, recording whether any target moved.
    Construct {
        start: bool,
        direction: TextDirection,
    },
    /// `didUpdateWidget`'s restart: every tween runs from its value at the
    /// curved animation value `t` to its target.
    Retarget { t: f32, direction: TextDirection },
}

impl TweenPass {
    /// The ambient text direction the targets resolve with.
    pub(crate) fn direction(&self) -> TextDirection {
        match self {
            Self::Construct { direction, .. } | Self::Retarget { direction, .. } => *direction,
        }
    }
}

/// Flutter's `TweenVisitor`, called once per property with its target.
pub(crate) fn visit<T: Lerp + Clone + PartialEq>(
    pass: &mut TweenPass,
    tween: &mut Option<PropertyTween<T>>,
    target: Option<&T>,
) {
    match pass {
        TweenPass::Construct { start, .. } => match target {
            Some(target) => match tween {
                Some(tween) => *start |= *target != tween.end,
                None => {
                    *tween = Some(PropertyTween {
                        begin: target.clone(),
                        end: target.clone(),
                    });
                }
            },
            None => *tween = None,
        },
        TweenPass::Retarget { t, .. } => {
            if let (Some(tween), Some(target)) = (tween.as_mut(), target) {
                tween.begin = tween.evaluate(*t);
                tween.end = target.clone();
            }
        }
    }
}

fn evaluate<T: Lerp + Clone>(tween: &Option<PropertyTween<T>>, t: f32) -> Option<T> {
    tween.as_ref().map(|tween| tween.evaluate(t))
}

/// The `ImplicitlyAnimatedWidget` fields every implicit animation shares.
#[derive(Clone)]
pub(crate) struct ImplicitTiming {
    duration: Duration,
    curve: Curve,
    on_end: Option<Rc<dyn Fn()>>,
    /// Set by `.transition()`: progress runs on a [`MotionValue`] (springs,
    /// delays, reduced motion) instead of the duration controller.
    transition: Option<Transition>,
    channel: Channel,
}

impl ImplicitTiming {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            curve: Curve::Linear,
            on_end: None,
            transition: None,
            channel: Channel::Geometry,
        }
    }

    fn channel(mut self, channel: Channel) -> Self {
        self.channel = channel;
        self
    }

    /// Timing driven by `transition` for a value of kind `channel`.
    pub(crate) fn from_transition(transition: Transition, channel: Channel) -> Self {
        Self {
            transition: Some(transition),
            channel,
            ..Self::new(Duration::ZERO)
        }
    }
}

/// Adds the `curve` and `onEnd` options of `ImplicitlyAnimatedWidget`.
macro_rules! implicit_timing_options {
    () => {
        /// The curve applied to the animation. Defaults to linear.
        #[must_use]
        pub fn curve(mut self, curve: Curve) -> Self {
            self.timing.curve = curve;
            self
        }

        /// Called each time an animation completes.
        #[must_use]
        pub fn on_end(mut self, callback: impl Fn() + 'static) -> Self {
            self.timing.on_end = Some(Rc::new(callback));
            self
        }

        /// Runs on a [`Transition`] (a spring, an ease, a delay) instead of
        /// the duration and curve: the same engine as `.transition()`.
        #[must_use]
        pub fn transition(mut self, transition: impl Into<Transition>) -> Self {
            let on_end = self.timing.on_end.take();
            self.timing = ImplicitTiming::from_transition(transition.into(), self.timing.channel);
            self.timing.on_end = on_end;
            self
        }
    };
}

/// A widget whose properties animate, `ImplicitlyAnimatedWidget`.
pub(crate) trait ImplicitlyAnimated: 'static {
    type Tweens: Default + 'static;

    fn timing(&self) -> &ImplicitTiming;

    /// `forEachTween`: visits every property with its target.
    fn for_each_tween(&self, tweens: &mut Self::Tweens, pass: &mut TweenPass);

    /// `initState`: builds the first tweens and reports whether the
    /// animation runs at once. Only `TweenAnimationBuilder` starts one.
    fn init_tweens(&self, tweens: &mut Self::Tweens, direction: TextDirection) -> bool {
        self.for_each_tween(
            tweens,
            &mut TweenPass::Construct {
                start: false,
                direction,
            },
        );
        false
    }

    /// `build`, with the tweens evaluated at the curved animation value `t`.
    fn build(&self, tweens: &Self::Tweens, t: f32, context: &BuildContext<'_>) -> Widget;
}

/// `ImplicitlyAnimatedWidgetState` with `AnimatedWidgetBaseState`'s rebuild
/// on every animation tick.
struct ImplicitState<W: ImplicitlyAnimated> {
    config: RefCell<Option<Rc<W>>>,
    tweens: RefCell<W::Tweens>,
    controller: AnimationController,
    listening: Cell<bool>,
    /// The curve in effect since the last target change. A curve-only update
    /// keeps it until the next target change (`AnimatedWidgetBaseState`).
    curve: RefCell<Curve>,
    revision: RefCell<Rc<Cell<u64>>>,
    /// `.transition()` progress from 0 to 1 (springs may overshoot).
    progress: MotionValue,
    /// Ticks [`Self::progress`]; created on the first transition build.
    delegate: RefCell<Option<FrameDelegate>>,
}

impl<W: ImplicitlyAnimated> RetainedState for ImplicitState<W> {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

/// Drives a `.transition()` state's progress on the frame clock.
struct ProgressTicker<W: ImplicitlyAnimated>(std::rc::Weak<ImplicitState<W>>);

impl<W: ImplicitlyAnimated> FrameTick for ProgressTicker<W> {
    fn tick(&self, now: incular_core::time::Instant) -> bool {
        let Some(state) = self.0.upgrade() else {
            return false;
        };
        let was = state.progress.is_animating();
        if !state.progress.tick(now) {
            return false;
        }
        let revision = state.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
        if was && !state.progress.is_animating() {
            // Take the callback out first so the config borrow ends before it runs.
            let on_end = state
                .config
                .borrow()
                .as_ref()
                .and_then(|config| config.timing().on_end.clone());
            if let Some(on_end) = on_end {
                on_end();
            }
        }
        true
    }
    fn is_active(&self) -> bool {
        self.0
            .upgrade()
            .is_some_and(|state| state.progress.is_animating())
    }
}

impl<W: ImplicitlyAnimated> ImplicitState<W> {
    /// The curved animation value the tweens are evaluated at.
    fn t(&self, config: &W) -> f32 {
        if config.timing().transition.is_some() {
            self.progress.get()
        } else {
            self.curve.borrow().apply(self.controller.value())
        }
    }

    fn update(&self, config: &Rc<W>, direction: TextDirection, context: &BuildContext<'_>) {
        let previous = self.config.replace(Some(config.clone()));
        let mut tweens = self.tweens.borrow_mut();
        let Some(previous) = previous else {
            if config.init_tweens(&mut tweens, direction) {
                self.controller.forward_on_next_tick();
            }
            return;
        };
        if Rc::ptr_eq(&previous, config) {
            // A rebuild for an animation tick: the widget is unchanged.
            return;
        }
        // `didUpdateWidget`.
        self.controller.set_duration(config.timing().duration);
        let mut construct = TweenPass::Construct {
            start: false,
            direction,
        };
        config.for_each_tween(&mut tweens, &mut construct);
        if let TweenPass::Construct { start: true, .. } = construct {
            if let Some(transition) = &config.timing().transition {
                // Retarget from the value on screen, then run 0 -> 1 again.
                let t = self.t(&previous);
                config.for_each_tween(&mut tweens, &mut TweenPass::Retarget { t, direction });
                let channel = config.timing().channel;
                let scope = MotionConfig::of(context);
                let transition =
                    &incular_animation::transaction().unwrap_or_else(|| transition.clone());
                let transition = if MotionConfig::is_reduced(context, &scope) {
                    reduce(transition, channel)
                } else {
                    transition.for_channel(channel)
                };
                self.progress.set(0.);
                self.progress.animate_to(1., &transition);
                incular_animation::schedule_frame();
                return;
            }
            // The new curve takes effect with the new target.
            self.curve.replace(config.timing().curve.clone());
            let t = self.curve.borrow().apply(self.controller.value());
            config.for_each_tween(&mut tweens, &mut TweenPass::Retarget { t, direction });
            // `controller.forward(from: 0.0)`.
            self.controller.set_value(0.0);
            self.controller.forward_on_next_tick();
        }
    }

    fn build(self: &Rc<Self>, context: &BuildContext<'_>) -> Widget {
        let config = self
            .config
            .borrow()
            .clone()
            .expect("the animation updates before it builds");
        let t = self.t(&config);
        let child = config.build(&self.tweens.borrow(), t, context);
        if config.timing().transition.is_some() {
            let delegate = self
                .delegate
                .borrow_mut()
                .get_or_insert_with(|| {
                    FrameDelegate::new(Rc::new(ProgressTicker(Rc::downgrade(self))))
                })
                .clone();
            return Widget::frame_ticker(delegate, child);
        }
        Widget::animation_ticker_with_revision(
            self.controller.clone(),
            false,
            self.revision.borrow().clone(),
            child,
        )
    }
}

pub(crate) fn implicitly_animated<W: ImplicitlyAnimated>(widget: W) -> Widget {
    let config = Rc::new(widget);
    Widget::retained_builder(
        {
            let config = config.clone();
            move |revision| ImplicitState::<W> {
                config: RefCell::new(None),
                tweens: RefCell::new(W::Tweens::default()),
                controller: AnimationController::new(config.timing().duration),
                listening: Cell::new(false),
                curve: RefCell::new(config.timing().curve.clone()),
                revision: RefCell::new(revision),
                progress: MotionValue::new(1.),
                delegate: RefCell::new(None),
            }
        },
        move |state, context, _| {
            if !state.listening.replace(true) {
                let weak = Rc::downgrade(state);
                state.controller.add_listener(move |_| {
                    let Some(state) = weak.upgrade() else {
                        return;
                    };
                    let revision = state.revision.borrow().clone();
                    revision.set(revision.get().wrapping_add(1));
                    if state.controller.status() == AnimationStatus::Completed {
                        // Take the callback out first so the config borrow ends before it runs.
                        let on_end = state
                            .config
                            .borrow()
                            .as_ref()
                            .and_then(|config| config.timing().on_end.clone());
                        if let Some(on_end) = on_end {
                            on_end();
                        }
                    }
                });
            }
            let direction = context.text_direction().unwrap_or(TextDirection::Ltr);
            state.update(&config, direction, context);
            state.build(context)
        },
        false,
    )
}

/// Flutter's `AnimatedContainer`: a [`Container`] whose alignment, padding,
/// decoration, foreground decoration, constraints, margin and transform
/// animate. Decorations interpolate with Flutter's `BoxDecoration.lerp`.
#[derive(Clone)]
pub struct AnimatedContainer {
    child: Option<Widget>,
    alignment: Option<AlignmentGeometry>,
    padding: Option<EdgeInsetsGeometry>,
    color: Option<Color>,
    decoration: Option<BoxDecoration>,
    foreground_decoration: Option<BoxDecoration>,
    width: Option<f32>,
    height: Option<f32>,
    constraints: Option<Constraints>,
    margin: Option<EdgeInsetsGeometry>,
    transform: Option<CoreTransform>,
    transform_alignment: Option<AlignmentGeometry>,
    clip_behavior: Clip,
    timing: ImplicitTiming,
}

impl AnimatedContainer {
    #[must_use]
    pub fn new(duration: Duration) -> Self {
        Self {
            child: None,
            alignment: None,
            padding: None,
            color: None,
            decoration: None,
            foreground_decoration: None,
            width: None,
            height: None,
            constraints: None,
            margin: None,
            transform: None,
            transform_alignment: None,
            clip_behavior: Clip::None,
            timing: ImplicitTiming::new(duration),
        }
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Accepts an [`Alignment`] or an [`AlignmentDirectional`], which follows
    /// the text direction.
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = Some(alignment.into());
        self
    }

    /// Accepts [`EdgeInsets`] or [`EdgeInsetsDirectional`](incular_config::EdgeInsetsDirectional).
    #[must_use]
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Shorthand for a decoration of only this color. Do not also set a
    /// [`AnimatedContainer::decoration`].
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    pub fn decoration(mut self, decoration: BoxDecoration) -> Self {
        self.decoration = Some(decoration);
        self
    }

    /// Flutter `foregroundDecoration`: painted in front of the child.
    #[must_use]
    pub fn foreground_decoration(mut self, decoration: BoxDecoration) -> Self {
        self.foreground_decoration = Some(decoration);
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    #[must_use]
    pub fn constraints(mut self, constraints: Constraints) -> Self {
        self.constraints = Some(constraints);
        self
    }

    #[must_use]
    pub fn margin(mut self, margin: impl Into<EdgeInsetsGeometry>) -> Self {
        self.margin = Some(margin.into());
        self
    }

    #[must_use]
    pub fn transform(mut self, transform: CoreTransform) -> Self {
        self.transform = Some(transform);
        self
    }

    /// The pivot of [`AnimatedContainer::transform`] within the box. Without
    /// one the transform applies about the top-left corner.
    #[must_use]
    pub fn transform_alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.transform_alignment = Some(alignment.into());
        self
    }

    /// Defaults to [`Clip::None`].
    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    implicit_timing_options!();

    /// The constructor's `decoration ?? BoxDecoration(color: color)`.
    fn effective_decoration(&self) -> Option<Cow<'_, BoxDecoration>> {
        match (&self.decoration, self.color) {
            (Some(decoration), _) => Some(Cow::Borrowed(decoration)),
            (None, Some(color)) => Some(Cow::Owned(BoxDecoration::new().color(color))),
            (None, None) => None,
        }
    }

    /// The constructor's width and height folded into the constraints.
    fn effective_constraints(&self) -> Option<Constraints> {
        if self.width.is_none() && self.height.is_none() {
            return self.constraints;
        }
        Some(self.constraints.map_or_else(
            || Constraints::tight_for(self.width, self.height),
            |constraints| constraints.tighten(self.width, self.height),
        ))
    }
}

#[derive(Default)]
pub(crate) struct ContainerTweens {
    alignment: Option<PropertyTween<Alignment>>,
    padding: Option<PropertyTween<EdgeInsetsGeometry>>,
    decoration: Option<PropertyTween<BoxDecoration>>,
    foreground_decoration: Option<PropertyTween<BoxDecoration>>,
    constraints: Option<PropertyTween<Constraints>>,
    margin: Option<PropertyTween<EdgeInsetsGeometry>>,
    transform: Option<PropertyTween<CoreTransform>>,
    transform_alignment: Option<PropertyTween<Alignment>>,
}

impl ImplicitlyAnimated for AnimatedContainer {
    type Tweens = ContainerTweens;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tweens: &mut ContainerTweens, pass: &mut TweenPass) {
        let direction = pass.direction();
        let alignment = self.alignment.map(|value| value.resolve(direction));
        visit(pass, &mut tweens.alignment, alignment.as_ref());
        visit(pass, &mut tweens.padding, self.padding.as_ref());
        visit(
            pass,
            &mut tweens.decoration,
            self.effective_decoration().as_deref(),
        );
        visit(
            pass,
            &mut tweens.foreground_decoration,
            self.foreground_decoration.as_ref(),
        );
        visit(
            pass,
            &mut tweens.constraints,
            self.effective_constraints().as_ref(),
        );
        visit(pass, &mut tweens.margin, self.margin.as_ref());
        visit(pass, &mut tweens.transform, self.transform.as_ref());
        let transform_alignment = self
            .transform_alignment
            .map(|value| value.resolve(direction));
        visit(
            pass,
            &mut tweens.transform_alignment,
            transform_alignment.as_ref(),
        );
    }

    fn build(&self, tweens: &ContainerTweens, t: f32, context: &BuildContext<'_>) -> Widget {
        let resolve = |value| resolve_animated_insets(value, context);
        let mut container = Container {
            child: self.child.clone(),
            alignment: evaluate(&tweens.alignment, t),
            padding: evaluate(&tweens.padding, t).map(resolve),
            margin: evaluate(&tweens.margin, t).map(resolve),
            constraints: evaluate(&tweens.constraints, t),
            clip_behavior: self.clip_behavior,
            ..Container::default()
        };
        if let Some(decoration) = evaluate(&tweens.decoration, t) {
            container = container.decoration(decoration);
        }
        container.foreground_decoration = evaluate(&tweens.foreground_decoration, t);
        // Flutter's `Container` applies the transform outermost.
        match evaluate(&tweens.transform, t) {
            Some(transform) => {
                let transformed = Transform::new(transform, container);
                match evaluate(&tweens.transform_alignment, t) {
                    Some(alignment) => transformed.alignment(alignment).into(),
                    None => transformed.into(),
                }
            }
            None => container.into(),
        }
    }
}

impl From<AnimatedContainer> for Widget {
    fn from(value: AnimatedContainer) -> Self {
        implicitly_animated(value)
    }
}

fn resolve_animated_insets(value: EdgeInsetsGeometry, context: &BuildContext<'_>) -> EdgeInsets {
    match value {
        EdgeInsetsGeometry::Physical(insets) => insets,
        _ => value.resolve(context.text_direction().unwrap_or(TextDirection::Ltr)),
    }
}

/// Flutter's `AnimatedPadding`.
#[derive(Clone)]
pub struct AnimatedPadding {
    padding: EdgeInsetsGeometry,
    child: Widget,
    timing: ImplicitTiming,
}

impl AnimatedPadding {
    /// Accepts [`EdgeInsets`] or [`EdgeInsetsDirectional`](incular_config::EdgeInsetsDirectional), which follows the
    /// text direction.
    #[must_use]
    pub fn new(
        padding: impl Into<EdgeInsetsGeometry>,
        duration: Duration,
        child: impl Into<Widget>,
    ) -> Self {
        Self {
            padding: padding.into(),
            child: child.into(),
            timing: ImplicitTiming::new(duration),
        }
    }

    implicit_timing_options!();
}

impl ImplicitlyAnimated for AnimatedPadding {
    type Tweens = Option<PropertyTween<EdgeInsetsGeometry>>;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tween: &mut Self::Tweens, pass: &mut TweenPass) {
        visit(pass, tween, Some(&self.padding));
    }

    fn build(&self, tween: &Self::Tweens, t: f32, context: &BuildContext<'_>) -> Widget {
        // Clamp each unresolved component before combining physical and
        // directional sides; the current direction is not a tween target.
        let padding = evaluate(tween, t).unwrap_or(self.padding).non_negative();
        Padding::new(resolve_animated_insets(padding, context), self.child.clone()).into()
    }
}

impl From<AnimatedPadding> for Widget {
    fn from(value: AnimatedPadding) -> Self {
        implicitly_animated(value)
    }
}

/// Flutter's `AnimatedAlign`.
#[derive(Clone)]
pub struct AnimatedAlign {
    alignment: AlignmentGeometry,
    width_factor: Option<f32>,
    height_factor: Option<f32>,
    child: Widget,
    timing: ImplicitTiming,
}

impl AnimatedAlign {
    /// Accepts an [`Alignment`] or an [`AlignmentDirectional`], which follows
    /// the text direction.
    #[must_use]
    pub fn new(
        alignment: impl Into<AlignmentGeometry>,
        duration: Duration,
        child: impl Into<Widget>,
    ) -> Self {
        Self {
            alignment: alignment.into(),
            width_factor: None,
            height_factor: None,
            child: child.into(),
            timing: ImplicitTiming::new(duration),
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

    implicit_timing_options!();
}

#[derive(Default)]
pub(crate) struct AlignTweens {
    alignment: Option<PropertyTween<AlignmentGeometry>>,
    width_factor: Option<PropertyTween<f32>>,
    height_factor: Option<PropertyTween<f32>>,
}

/// `AnimatedAlign` and `AnimatedFractionallySizedBox` visit a factor only
/// while it is set: a factor that becomes null keeps its last tween.
fn visit_factors(
    pass: &mut TweenPass,
    tweens: &mut AlignTweens,
    alignment: AlignmentGeometry,
    width_factor: Option<f32>,
    height_factor: Option<f32>,
) {
    // Interpolate physical and logical components before applying direction.
    // A direction-only rebuild must not change the tween target or its clock.
    visit(pass, &mut tweens.alignment, Some(&alignment));
    if let Some(height_factor) = height_factor {
        visit(pass, &mut tweens.height_factor, Some(&height_factor));
    }
    if let Some(width_factor) = width_factor {
        visit(pass, &mut tweens.width_factor, Some(&width_factor));
    }
}

impl ImplicitlyAnimated for AnimatedAlign {
    type Tweens = AlignTweens;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tweens: &mut AlignTweens, pass: &mut TweenPass) {
        visit_factors(
            pass,
            tweens,
            self.alignment,
            self.width_factor,
            self.height_factor,
        );
    }

    fn build(&self, tweens: &AlignTweens, t: f32, context: &BuildContext<'_>) -> Widget {
        let geometry = evaluate(&tweens.alignment, t).unwrap_or(self.alignment);
        let alignment = match geometry {
            AlignmentGeometry::Physical(alignment) => alignment,
            logical => logical.resolve(context.text_direction().unwrap_or(TextDirection::Ltr)),
        };
        let mut align = Align::new(alignment, self.child.clone());
        if let Some(factor) = evaluate(&tweens.height_factor, t) {
            align = align.height_factor(factor);
        }
        if let Some(factor) = evaluate(&tweens.width_factor, t) {
            align = align.width_factor(factor);
        }
        align.into()
    }
}

impl From<AnimatedAlign> for Widget {
    fn from(value: AnimatedAlign) -> Self {
        implicitly_animated(value)
    }
}

/// Flutter's `AnimatedFractionallySizedBox`.
#[derive(Clone)]
pub struct AnimatedFractionallySizedBox {
    alignment: AlignmentGeometry,
    width_factor: Option<f32>,
    height_factor: Option<f32>,
    child: Option<Widget>,
    timing: ImplicitTiming,
}

impl AnimatedFractionallySizedBox {
    /// Centered, with no factors.
    #[must_use]
    pub fn new(duration: Duration, child: impl Into<Widget>) -> Self {
        Self::empty(duration).child(child)
    }

    /// Creates a childless box whose factors and alignment can be animated.
    #[must_use]
    pub fn empty(duration: Duration) -> Self {
        Self {
            alignment: Alignment::CENTER.into(),
            width_factor: None,
            height_factor: None,
            child: None,
            timing: ImplicitTiming::new(duration),
        }
    }

    /// Sets the child of the animated box.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Accepts an [`Alignment`] or an [`AlignmentDirectional`].
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
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

    implicit_timing_options!();
}

impl ImplicitlyAnimated for AnimatedFractionallySizedBox {
    type Tweens = AlignTweens;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tweens: &mut AlignTweens, pass: &mut TweenPass) {
        visit_factors(
            pass,
            tweens,
            self.alignment,
            self.width_factor,
            self.height_factor,
        );
    }

    fn build(&self, tweens: &AlignTweens, t: f32, _context: &BuildContext<'_>) -> Widget {
        let mut sized = FractionallySizedBox::empty()
            .alignment(evaluate(&tweens.alignment, t).unwrap_or(self.alignment));
        if let Some(factor) = evaluate(&tweens.height_factor, t) {
            sized = sized.height_factor(factor);
        }
        if let Some(factor) = evaluate(&tweens.width_factor, t) {
            sized = sized.width_factor(factor);
        }
        if let Some(child) = &self.child {
            sized = sized.child(child.clone());
        }
        sized.into()
    }
}

impl From<AnimatedFractionallySizedBox> for Widget {
    fn from(value: AnimatedFractionallySizedBox) -> Self {
        implicitly_animated(value)
    }
}

#[derive(Clone, Copy, Default)]
struct Edges {
    leading: Option<f32>,
    top: Option<f32>,
    trailing: Option<f32>,
    bottom: Option<f32>,
    width: Option<f32>,
    height: Option<f32>,
}

#[derive(Default)]
pub(crate) struct EdgeTweens {
    leading: Option<PropertyTween<f32>>,
    top: Option<PropertyTween<f32>>,
    trailing: Option<PropertyTween<f32>>,
    bottom: Option<PropertyTween<f32>>,
    width: Option<PropertyTween<f32>>,
    height: Option<PropertyTween<f32>>,
}

impl Edges {
    fn visit(&self, tweens: &mut EdgeTweens, pass: &mut TweenPass) {
        visit(pass, &mut tweens.leading, self.leading.as_ref());
        visit(pass, &mut tweens.top, self.top.as_ref());
        visit(pass, &mut tweens.trailing, self.trailing.as_ref());
        visit(pass, &mut tweens.bottom, self.bottom.as_ref());
        visit(pass, &mut tweens.width, self.width.as_ref());
        visit(pass, &mut tweens.height, self.height.as_ref());
    }

    /// A [`Positioned`] with the left and right edges given.
    fn positioned(tweens: &EdgeTweens, t: f32, left_is_leading: bool, child: Widget) -> Widget {
        let (left, right) = if left_is_leading {
            (&tweens.leading, &tweens.trailing)
        } else {
            (&tweens.trailing, &tweens.leading)
        };
        let mut positioned = Positioned::new(child);
        if let Some(left) = evaluate(left, t) {
            positioned = positioned.left(left);
        }
        if let Some(top) = evaluate(&tweens.top, t) {
            positioned = positioned.top(top);
        }
        if let Some(right) = evaluate(right, t) {
            positioned = positioned.right(right);
        }
        if let Some(bottom) = evaluate(&tweens.bottom, t) {
            positioned = positioned.bottom(bottom);
        }
        if let Some(width) = evaluate(&tweens.width, t) {
            positioned = positioned.width(width);
        }
        if let Some(height) = evaluate(&tweens.height, t) {
            positioned = positioned.height(height);
        }
        positioned.into()
    }
}

/// Flutter's `AnimatedPositioned`. Only works as a [`Stack`](crate::Stack)
/// child.
#[derive(Clone)]
pub struct AnimatedPositioned {
    edges: Edges,
    child: Widget,
    timing: ImplicitTiming,
}

impl AnimatedPositioned {
    #[must_use]
    pub fn new(duration: Duration, child: impl Into<Widget>) -> Self {
        Self {
            edges: Edges::default(),
            child: child.into(),
            timing: ImplicitTiming::new(duration),
        }
    }

    #[must_use]
    pub fn left(mut self, left: f32) -> Self {
        self.edges.leading = Some(left);
        self
    }

    #[must_use]
    pub fn top(mut self, top: f32) -> Self {
        self.edges.top = Some(top);
        self
    }

    #[must_use]
    pub fn right(mut self, right: f32) -> Self {
        self.edges.trailing = Some(right);
        self
    }

    #[must_use]
    pub fn bottom(mut self, bottom: f32) -> Self {
        self.edges.bottom = Some(bottom);
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.edges.width = Some(width);
        self
    }

    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.edges.height = Some(height);
        self
    }

    implicit_timing_options!();
}

impl ImplicitlyAnimated for AnimatedPositioned {
    type Tweens = EdgeTweens;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tweens: &mut EdgeTweens, pass: &mut TweenPass) {
        self.edges.visit(tweens, pass);
    }

    fn build(&self, tweens: &EdgeTweens, t: f32, _: &BuildContext<'_>) -> Widget {
        Edges::positioned(tweens, t, true, self.child.clone())
    }
}

impl From<AnimatedPositioned> for Widget {
    fn from(value: AnimatedPositioned) -> Self {
        implicitly_animated(value)
    }
}

/// Flutter's `AnimatedPositionedDirectional`: start and end follow the
/// ambient [`Directionality`](crate::Directionality).
#[derive(Clone)]
pub struct AnimatedPositionedDirectional {
    edges: Edges,
    child: Widget,
    timing: ImplicitTiming,
}

impl AnimatedPositionedDirectional {
    #[must_use]
    pub fn new(duration: Duration, child: impl Into<Widget>) -> Self {
        Self {
            edges: Edges::default(),
            child: child.into(),
            timing: ImplicitTiming::new(duration),
        }
    }

    #[must_use]
    pub fn start(mut self, start: f32) -> Self {
        self.edges.leading = Some(start);
        self
    }

    #[must_use]
    pub fn top(mut self, top: f32) -> Self {
        self.edges.top = Some(top);
        self
    }

    #[must_use]
    pub fn end(mut self, end: f32) -> Self {
        self.edges.trailing = Some(end);
        self
    }

    #[must_use]
    pub fn bottom(mut self, bottom: f32) -> Self {
        self.edges.bottom = Some(bottom);
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.edges.width = Some(width);
        self
    }

    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.edges.height = Some(height);
        self
    }

    implicit_timing_options!();
}

impl ImplicitlyAnimated for AnimatedPositionedDirectional {
    type Tweens = EdgeTweens;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tweens: &mut EdgeTweens, pass: &mut TweenPass) {
        self.edges.visit(tweens, pass);
    }

    fn build(&self, tweens: &EdgeTweens, t: f32, context: &BuildContext<'_>) -> Widget {
        // `Positioned.directional(textDirection: Directionality.of(context))`.
        let left_is_leading = context.text_direction() != Some(TextDirection::Rtl);
        Edges::positioned(tweens, t, left_is_leading, self.child.clone())
    }
}

impl From<AnimatedPositionedDirectional> for Widget {
    fn from(value: AnimatedPositionedDirectional) -> Self {
        implicitly_animated(value)
    }
}

/// Flutter's `AnimatedScale`.
#[derive(Clone)]
pub struct AnimatedScale {
    scale: f32,
    alignment: Alignment,
    child: Widget,
    timing: ImplicitTiming,
}

impl AnimatedScale {
    #[must_use]
    pub fn new(scale: f32, duration: Duration, child: impl Into<Widget>) -> Self {
        Self {
            scale,
            alignment: Alignment::CENTER,
            child: child.into(),
            timing: ImplicitTiming::new(duration),
        }
    }

    /// The pivot. Defaults to center.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    implicit_timing_options!();
}

impl ImplicitlyAnimated for AnimatedScale {
    type Tweens = Option<PropertyTween<f32>>;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tween: &mut Self::Tweens, pass: &mut TweenPass) {
        visit(pass, tween, Some(&self.scale));
    }

    fn build(&self, tween: &Self::Tweens, t: f32, _: &BuildContext<'_>) -> Widget {
        // `ScaleTransition`.
        let scale = evaluate(tween, t).unwrap_or(self.scale);
        Transform::new(CoreTransform::scale(scale), self.child.clone())
            .alignment(self.alignment)
            .into()
    }
}

impl From<AnimatedScale> for Widget {
    fn from(value: AnimatedScale) -> Self {
        implicitly_animated(value)
    }
}

/// Flutter's `AnimatedRotation`: `turns` is in full clockwise turns.
#[derive(Clone)]
pub struct AnimatedRotation {
    turns: f32,
    alignment: Alignment,
    child: Widget,
    timing: ImplicitTiming,
}

impl AnimatedRotation {
    #[must_use]
    pub fn new(turns: f32, duration: Duration, child: impl Into<Widget>) -> Self {
        Self {
            turns,
            alignment: Alignment::CENTER,
            child: child.into(),
            timing: ImplicitTiming::new(duration),
        }
    }

    /// The pivot. Defaults to center.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    implicit_timing_options!();
}

impl ImplicitlyAnimated for AnimatedRotation {
    type Tweens = Option<PropertyTween<f32>>;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tween: &mut Self::Tweens, pass: &mut TweenPass) {
        visit(pass, tween, Some(&self.turns));
    }

    fn build(&self, tween: &Self::Tweens, t: f32, _: &BuildContext<'_>) -> Widget {
        // `RotationTransition`: `Matrix4.rotationZ(turns * pi * 2.0)`.
        let turns = evaluate(tween, t).unwrap_or(self.turns);
        let radians = turns * std::f32::consts::TAU;
        Transform::new(CoreTransform::rotation(radians), self.child.clone())
            .alignment(self.alignment)
            .into()
    }
}

impl From<AnimatedRotation> for Widget {
    fn from(value: AnimatedRotation) -> Self {
        implicitly_animated(value)
    }
}

/// Flutter's `AnimatedSlide`: `offset` is a fraction of the child's size.
#[derive(Clone)]
pub struct AnimatedSlide {
    offset: Offset,
    child: Widget,
    timing: ImplicitTiming,
}

impl AnimatedSlide {
    #[must_use]
    pub fn new(offset: Offset, duration: Duration, child: impl Into<Widget>) -> Self {
        Self {
            offset,
            child: child.into(),
            timing: ImplicitTiming::new(duration),
        }
    }

    implicit_timing_options!();
}

impl ImplicitlyAnimated for AnimatedSlide {
    type Tweens = Option<PropertyTween<Offset>>;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tween: &mut Self::Tweens, pass: &mut TweenPass) {
        visit(pass, tween, Some(&self.offset));
    }

    fn build(&self, tween: &Self::Tweens, t: f32, _: &BuildContext<'_>) -> Widget {
        // `SlideTransition`.
        let offset = evaluate(tween, t).unwrap_or(self.offset);
        FractionalTranslation::new(offset, self.child.clone()).into()
    }
}

impl From<AnimatedSlide> for Widget {
    fn from(value: AnimatedSlide) -> Self {
        implicitly_animated(value)
    }
}

/// Flutter's `AnimatedOpacity`.
#[derive(Clone)]
pub struct AnimatedOpacity {
    opacity: f32,
    child: Widget,
    timing: ImplicitTiming,
    always_include_semantics: bool,
}

impl AnimatedOpacity {
    /// `opacity` is clamped to `0.0..=1.0`.
    #[must_use]
    pub fn new(opacity: f32, duration: Duration, child: impl Into<Widget>) -> Self {
        Self {
            opacity: opacity.clamp(0., 1.),
            child: child.into(),
            timing: ImplicitTiming::new(duration).channel(Channel::Opacity),
            always_include_semantics: false,
        }
    }

    /// Flutter's `alwaysIncludeSemantics`: keep the child's semantics while
    /// the animated opacity is zero.
    #[must_use]
    pub fn always_include_semantics(mut self, include: bool) -> Self {
        self.always_include_semantics = include;
        self
    }

    implicit_timing_options!();
}

impl ImplicitlyAnimated for AnimatedOpacity {
    type Tweens = Option<PropertyTween<f32>>;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tween: &mut Self::Tweens, pass: &mut TweenPass) {
        visit(pass, tween, Some(&self.opacity));
    }

    fn build(&self, tween: &Self::Tweens, t: f32, _: &BuildContext<'_>) -> Widget {
        // `FadeTransition`.
        let opacity = evaluate(tween, t).unwrap_or(self.opacity);
        Opacity::new(opacity, self.child.clone())
            .always_include_semantics(self.always_include_semantics)
            .into()
    }
}

impl From<AnimatedOpacity> for Widget {
    fn from(value: AnimatedOpacity) -> Self {
        implicitly_animated(value)
    }
}

/// Flutter's `AnimatedDefaultTextStyle`. The style interpolates; the paragraph
/// options (`textAlign`, `softWrap`, `overflow`, `maxLines`) are set directly, as
/// in Flutter, and apply to descendants from the first frame.
#[derive(Clone)]
pub struct AnimatedDefaultTextStyle {
    style: TextStyle,
    paragraph: DefaultTextParagraph,
    child: Widget,
    timing: ImplicitTiming,
}

impl AnimatedDefaultTextStyle {
    #[must_use]
    pub fn new(style: TextStyle, duration: Duration, child: impl Into<Widget>) -> Self {
        Self {
            style,
            paragraph: DefaultTextParagraph::default(),
            child: child.into(),
            timing: ImplicitTiming::new(duration),
        }
    }

    /// Flutter `AnimatedDefaultTextStyle.textAlign`.
    #[must_use]
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.paragraph.align = Some(align);
        self
    }

    /// Flutter `AnimatedDefaultTextStyle.softWrap`.
    #[must_use]
    pub fn soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.paragraph.soft_wrap = Some(soft_wrap);
        self
    }

    /// Flutter `AnimatedDefaultTextStyle.overflow`.
    #[must_use]
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.paragraph.overflow = Some(overflow);
        self
    }

    /// Flutter `AnimatedDefaultTextStyle.maxLines`.
    #[must_use]
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.paragraph.max_lines = Some(max_lines);
        self
    }

    implicit_timing_options!();
}

impl ImplicitlyAnimated for AnimatedDefaultTextStyle {
    type Tweens = Option<PropertyTween<TextStyle>>;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tween: &mut Self::Tweens, pass: &mut TweenPass) {
        visit(pass, tween, Some(&self.style));
    }

    fn build(&self, tween: &Self::Tweens, t: f32, _: &BuildContext<'_>) -> Widget {
        let style = evaluate(tween, t).unwrap_or_else(|| self.style.clone());
        DefaultTextStyle::new(style, self.child.clone())
            .with_paragraph(self.paragraph)
            .into()
    }
}

impl From<AnimatedDefaultTextStyle> for Widget {
    fn from(value: AnimatedDefaultTextStyle) -> Self {
        implicitly_animated(value)
    }
}

/// Flutter's `AnimatedPhysicalModel`. The shape is immediate, as in Flutter:
/// only the color, shadow color, elevation and radius animate.
#[derive(Clone)]
pub struct AnimatedPhysicalModel {
    color: Color,
    shape: BoxShape,
    shadow_color: Color,
    elevation: f32,
    border_radius: BorderRadius,
    clip_behavior: Clip,
    animate_color: bool,
    animate_shadow_color: bool,
    child: Widget,
    timing: ImplicitTiming,
}

impl AnimatedPhysicalModel {
    #[must_use]
    pub fn new(
        color: Color,
        shadow_color: Color,
        elevation: f32,
        duration: Duration,
        child: impl Into<Widget>,
    ) -> Self {
        Self {
            color,
            shape: BoxShape::Rectangle,
            shadow_color,
            elevation,
            border_radius: BorderRadius::ZERO,
            clip_behavior: Clip::None,
            animate_color: true,
            animate_shadow_color: true,
            child: child.into(),
            timing: ImplicitTiming::new(duration),
        }
    }

    #[must_use]
    pub fn border_radius(mut self, radius: BorderRadius) -> Self {
        self.border_radius = radius;
        self
    }

    /// Flutter `shape`: a rectangle with the border radius, or a circle.
    /// Immediate, not animated. Defaults to a rectangle.
    #[must_use]
    pub fn shape(mut self, shape: BoxShape) -> Self {
        self.shape = shape;
        self
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    /// Whether the color animates; otherwise it changes at once. Defaults
    /// to true.
    #[must_use]
    pub fn animate_color(mut self, animate: bool) -> Self {
        self.animate_color = animate;
        self
    }

    /// Whether the shadow color animates. Defaults to true.
    #[must_use]
    pub fn animate_shadow_color(mut self, animate: bool) -> Self {
        self.animate_shadow_color = animate;
        self
    }

    implicit_timing_options!();
}

#[derive(Default)]
pub(crate) struct PhysicalModelTweens {
    border_radius: Option<PropertyTween<BorderRadius>>,
    elevation: Option<PropertyTween<f32>>,
    color: Option<PropertyTween<Color>>,
    shadow_color: Option<PropertyTween<Color>>,
}

impl ImplicitlyAnimated for AnimatedPhysicalModel {
    type Tweens = PhysicalModelTweens;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn for_each_tween(&self, tweens: &mut PhysicalModelTweens, pass: &mut TweenPass) {
        visit(pass, &mut tweens.border_radius, Some(&self.border_radius));
        visit(pass, &mut tweens.elevation, Some(&self.elevation));
        visit(pass, &mut tweens.color, Some(&self.color));
        visit(pass, &mut tweens.shadow_color, Some(&self.shadow_color));
    }

    fn build(&self, tweens: &PhysicalModelTweens, t: f32, _: &BuildContext<'_>) -> Widget {
        let color = if self.animate_color {
            evaluate(&tweens.color, t).unwrap_or(self.color)
        } else {
            self.color
        };
        let shadow_color = if self.animate_shadow_color {
            evaluate(&tweens.shadow_color, t).unwrap_or(self.shadow_color)
        } else {
            self.shadow_color
        };
        let border_radius = evaluate(&tweens.border_radius, t).unwrap_or(self.border_radius);
        PhysicalModel::new(color, self.child.clone())
            .shape(self.shape)
            .shadow_color(shadow_color)
            .elevation(evaluate(&tweens.elevation, t).unwrap_or(self.elevation))
            .border_radius(border_radius.to_corner_radii())
            .clip_behavior(self.clip_behavior)
            .into()
    }
}

impl From<AnimatedPhysicalModel> for Widget {
    fn from(value: AnimatedPhysicalModel) -> Self {
        implicitly_animated(value)
    }
}

/// Builds the child of a [`TweenAnimationBuilder`] from the current value.
pub type TweenAnimationWidgetBuilder<T> =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, T, Option<Widget>) -> Widget>;

/// Flutter's `TweenAnimationBuilder`: animates from `begin` (or `end`) to
/// `end` on first build, then from the current value to each new `end`.
/// Later changes to `begin` have no effect, as in Flutter.
pub struct TweenAnimationBuilder<T> {
    begin: Option<T>,
    end: T,
    builder: TweenAnimationWidgetBuilder<T>,
    child: Option<Widget>,
    timing: ImplicitTiming,
}

impl<T: Lerp + Clone + PartialEq + 'static> TweenAnimationBuilder<T> {
    #[must_use]
    pub fn new<W: Into<Widget>>(
        end: T,
        duration: Duration,
        builder: impl for<'a> Fn(&BuildContext<'a>, T, Option<Widget>) -> W + 'static,
    ) -> Self {
        Self {
            begin: None,
            end,
            builder: Rc::new(move |context, value, child| builder(context, value, child).into()),
            child: None,
            timing: ImplicitTiming::new(duration),
        }
    }

    /// The value the first animation starts from. Defaults to `end`, which
    /// means no animation on first build.
    #[must_use]
    pub fn begin(mut self, begin: T) -> Self {
        self.begin = Some(begin);
        self
    }

    /// A subtree passed to the builder unchanged on every frame.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    implicit_timing_options!();
}

impl<T: Lerp + Clone + PartialEq + 'static> ImplicitlyAnimated for TweenAnimationBuilder<T> {
    type Tweens = Option<PropertyTween<T>>;

    fn timing(&self) -> &ImplicitTiming {
        &self.timing
    }

    fn init_tweens(&self, tween: &mut Self::Tweens, _direction: TextDirection) -> bool {
        // `_currentTween = widget.tween; begin ??= end`, then forward when
        // the two differ.
        let begin = self.begin.clone().unwrap_or_else(|| self.end.clone());
        let start = begin != self.end;
        *tween = Some(PropertyTween {
            begin,
            end: self.end.clone(),
        });
        start
    }

    fn for_each_tween(&self, tween: &mut Self::Tweens, pass: &mut TweenPass) {
        visit(pass, tween, Some(&self.end));
    }

    fn build(&self, tween: &Self::Tweens, t: f32, context: &BuildContext<'_>) -> Widget {
        let value = evaluate(tween, t).unwrap_or_else(|| self.end.clone());
        (self.builder)(context, value, self.child.clone())
    }
}

impl<T: Lerp + Clone + PartialEq + 'static> From<TweenAnimationBuilder<T>> for Widget {
    fn from(value: TweenAnimationBuilder<T>) -> Self {
        implicitly_animated(value)
    }
}

// `.transition()`: each transitionable widget becomes its `Animated*` twin.

fn border_radius(radii: incular_rendering::CornerRadii) -> BorderRadius {
    use crate::painting_effects::Radius;
    BorderRadius {
        top_left: Radius::circular(radii.top_left),
        top_right: Radius::circular(radii.top_right),
        bottom_right: Radius::circular(radii.bottom_right),
        bottom_left: Radius::circular(radii.bottom_left),
    }
}

fn decoration_border(border: incular_rendering::Border) -> crate::Border {
    crate::Border::new(border.width, border.color)
}

impl crate::dx::transition::Transitionable for Container {
    fn into_transitioning(self, transition: Transition) -> Widget {
        // The container's paint fields folded into one decoration; a set
        // decoration keeps what the fields cannot express.
        let has_paint = self.color.is_some()
            || self.background.is_some()
            || self.border.is_some()
            || !self.radius.is_zero();
        let decoration = match self.decoration {
            Some(mut decoration) => {
                if let Some(color) = self.color {
                    decoration.color = Some(color);
                }
                if decoration.border.is_none() {
                    decoration.border = self.border.map(decoration_border);
                }
                if decoration.border_radius.is_none() && !self.radius.is_zero() {
                    decoration.border_radius = Some(border_radius(self.radius));
                }
                Some(decoration)
            }
            None if has_paint => {
                let mut decoration = BoxDecoration::new();
                decoration.color = self.color;
                if self.color.is_none() {
                    decoration.gradient = self.background;
                }
                decoration.border = self.border.map(decoration_border);
                if !self.radius.is_zero() {
                    decoration.border_radius = Some(border_radius(self.radius));
                }
                Some(decoration)
            }
            None => None,
        };
        AnimatedContainer {
            child: self.child,
            alignment: self.alignment.map(Into::into),
            padding: self.padding.map(Into::into),
            color: None,
            decoration,
            foreground_decoration: self.foreground_decoration,
            width: self.width,
            height: self.height,
            constraints: self.constraints,
            margin: self.margin.map(Into::into),
            transform: self.transform,
            transform_alignment: None,
            clip_behavior: self.clip_behavior,
            timing: ImplicitTiming::from_transition(transition, Channel::Geometry),
        }
        .into()
    }
}

impl crate::dx::transition::Transitionable for Padding {
    fn into_transitioning(self, transition: Transition) -> Widget {
        AnimatedPadding {
            padding: self.padding.into(),
            child: self.child,
            timing: ImplicitTiming::from_transition(transition, Channel::Geometry),
        }
        .into()
    }
}

impl crate::dx::transition::Transitionable for Align {
    fn into_transitioning(self, transition: Transition) -> Widget {
        AnimatedAlign {
            alignment: self.alignment.into(),
            width_factor: self.width_factor,
            height_factor: self.height_factor,
            child: self.child,
            timing: ImplicitTiming::from_transition(transition, Channel::Geometry),
        }
        .into()
    }
}

impl crate::dx::transition::Transitionable for FractionallySizedBox {
    fn into_transitioning(self, transition: Transition) -> Widget {
        AnimatedFractionallySizedBox {
            alignment: self.alignment,
            width_factor: self.width_factor,
            height_factor: self.height_factor,
            child: self.child,
            timing: ImplicitTiming::from_transition(transition, Channel::Geometry),
        }
        .into()
    }
}

impl crate::dx::transition::Transitionable for Positioned {
    fn into_transitioning(self, transition: Transition) -> Widget {
        type Set = fn(AnimatedPositioned, f32) -> AnimatedPositioned;
        let mut animated = AnimatedPositioned::new(Duration::ZERO, self.child);
        let edges: [(Option<f32>, Set); 6] = [
            (self.left, AnimatedPositioned::left),
            (self.top, AnimatedPositioned::top),
            (self.right, AnimatedPositioned::right),
            (self.bottom, AnimatedPositioned::bottom),
            (self.width, AnimatedPositioned::width),
            (self.height, AnimatedPositioned::height),
        ];
        for (value, set) in edges {
            if let Some(value) = value {
                animated = set(animated, value);
            }
        }
        animated.timing = ImplicitTiming::from_transition(transition, Channel::Geometry);
        animated.into()
    }
}

impl crate::dx::transition::Transitionable for Opacity {
    fn into_transitioning(self, transition: Transition) -> Widget {
        AnimatedOpacity {
            opacity: self.alpha,
            child: self.child,
            timing: ImplicitTiming::from_transition(transition, Channel::Opacity),
            always_include_semantics: self.always_include_semantics,
        }
        .into()
    }
}

impl crate::dx::transition::Transitionable for PhysicalModel {
    fn into_transitioning(self, transition: Transition) -> Widget {
        AnimatedPhysicalModel {
            color: self.color,
            shape: self.shape,
            shadow_color: self.shadow_color,
            elevation: self.elevation,
            border_radius: border_radius(self.border_radius),
            clip_behavior: self.clip_behavior,
            animate_color: true,
            animate_shadow_color: true,
            child: self.child,
            timing: ImplicitTiming::from_transition(transition, Channel::Geometry),
        }
        .into()
    }
}

impl crate::dx::transition::Transitionable for DefaultTextStyle {
    fn into_transitioning(self, transition: Transition) -> Widget {
        AnimatedDefaultTextStyle {
            style: self.style,
            paragraph: self.paragraph,
            child: self.child,
            timing: ImplicitTiming::from_transition(transition, Channel::Geometry),
        }
        .into()
    }
}
