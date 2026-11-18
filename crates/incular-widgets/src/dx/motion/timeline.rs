//! `.animate()` timelines on the motion engine: a playhead (time or a
//! progress source) samples the effect chain into the node's compositor
//! values, so a tick writes layer values instead of rebuilding.

use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll, Waker},
    time::Duration,
};

use incular_animation::{MotionValue, Transition};
use incular_core::{
    Offset, Rect,
    reactivity::{DependencySource, Subscription},
    time::Instant,
};
use incular_rendering::{Brush, GradientStop, GradientStops, LinearGradient};

use super::{
    InView,
    cond::{Cond, Interaction},
    geometry::GeometryListener,
    node::{ChannelSource, MotionNode},
    pose::{Combine, PROP_COUNT, Pose, Prop},
};
use crate::dx::animate::{AnimateRepeat, Effect, EffectKind, RevealFrom};
use crate::tree::FrameGeometry;
use crate::{ShaderMask, Widget};

// ---------------------------------------------------------------------------
// Playhead and handle

#[derive(Default)]
struct Playhead {
    /// Position along the timeline, 0 to 1.
    position: Cell<f32>,
    /// +1 forward, -1 backward, 0 paused.
    direction: Cell<f32>,
    /// The frame time of the last advance; `None` restarts the clock.
    last: Cell<Option<Instant>>,
    /// Bumps each time a run finishes.
    finishes: Cell<u64>,
    wakers: RefCell<Vec<Waker>>,
}

/// What one advance did.
#[derive(Clone, Copy, Default)]
struct Advance {
    changed: bool,
    finished_forward: bool,
}

impl Playhead {
    fn set_direction(&self, direction: f32) {
        self.direction.set(direction);
        self.last.set(None);
        incular_animation::schedule_frame();
    }

    fn finish(&self, forward: bool) -> Advance {
        self.direction.set(0.);
        self.last.set(None);
        self.finishes.set(self.finishes.get().wrapping_add(1));
        for waker in self.wakers.borrow_mut().drain(..) {
            waker.wake();
        }
        Advance {
            changed: true,
            finished_forward: forward,
        }
    }

    fn advance(&self, now: Instant, total: Duration, repeat: AnimateRepeat) -> Advance {
        let direction = self.direction.get();
        if direction == 0. {
            self.last.set(None);
            return Advance::default();
        }
        let Some(last) = self.last.replace(Some(now)) else {
            // The first tick of a run fixes its clock.
            return Advance::default();
        };
        let dt = now.saturating_duration_since(last).as_secs_f32();
        if dt <= 0. && !total.is_zero() {
            return Advance::default();
        }
        let mut position = if total.is_zero() {
            if direction > 0. { 1. } else { 0. }
        } else {
            self.position.get() + direction * dt / total.as_secs_f32()
        };
        let mut outcome = Advance {
            changed: true,
            finished_forward: false,
        };
        if position >= 1. && direction > 0. {
            match repeat {
                AnimateRepeat::Once => {
                    position = 1.;
                    outcome = self.finish(true);
                }
                AnimateRepeat::Loop => position = position.fract(),
                AnimateRepeat::PingPong => {
                    position = (2. - position).clamp(0., 1.);
                    self.direction.set(-1.);
                }
            }
        } else if position <= 0. && direction < 0. {
            match repeat {
                AnimateRepeat::PingPong => {
                    position = (-position).clamp(0., 1.);
                    self.direction.set(1.);
                }
                AnimateRepeat::Loop => position = 1. + position.max(-1.),
                AnimateRepeat::Once => {
                    position = 0.;
                    outcome = self.finish(false);
                }
            }
        }
        self.position.set(position.clamp(0., 1.));
        outcome
    }
}

/// Plays, reverses, seeks and awaits `.animate()` timelines from outside
/// (`Play::handle(&handle)`). One handle can drive several timelines in
/// lockstep.
#[derive(Clone, Default)]
pub struct AnimationHandle(Rc<Playhead>);

impl AnimationHandle {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Plays forward from the current position (from the start when it
    /// already finished).
    pub fn play(&self) {
        if self.0.position.get() >= 1. {
            self.0.position.set(0.);
        }
        self.0.set_direction(1.);
    }

    /// Plays backward from the current position.
    pub fn reverse(&self) {
        if self.0.position.get() <= 0. {
            self.0.position.set(1.);
        }
        self.0.set_direction(-1.);
    }

    /// Holds the current position.
    pub fn pause(&self) {
        self.0.set_direction(0.);
    }

    /// Jumps to `progress` (0 to 1) without changing direction.
    pub fn seek(&self, progress: f32) {
        let progress = if progress.is_finite() {
            progress.clamp(0., 1.)
        } else {
            0.
        };
        self.0.position.set(progress);
        self.0.last.set(None);
        incular_animation::schedule_frame();
    }

    /// Plays forward from the start.
    pub fn restart(&self) {
        self.0.position.set(0.);
        self.0.set_direction(1.);
    }

    /// The playhead position, 0 to 1.
    #[must_use]
    pub fn progress(&self) -> f32 {
        self.0.position.get()
    }

    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.0.direction.get() != 0.
    }

    /// Resolves when the current run reaches either end (immediately when
    /// nothing is playing).
    #[must_use]
    pub fn finished(&self) -> Finished {
        Finished {
            head: self.0.clone(),
            finishes: self.0.finishes.get(),
            idle: !self.is_playing(),
        }
    }
}

/// The future returned by [`AnimationHandle::finished`].
pub struct Finished {
    head: Rc<Playhead>,
    finishes: u64,
    idle: bool,
}

impl Future for Finished {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.idle || self.head.finishes.get() != self.finishes {
            return Poll::Ready(());
        }
        self.head.wakers.borrow_mut().push(context.waker().clone());
        Poll::Pending
    }
}

// ---------------------------------------------------------------------------
// Triggers and progress sources

/// What starts and steers an `.animate()` timeline.
#[derive(Clone, Default)]
pub enum Play {
    /// Plays once the element mounts (the default).
    #[default]
    OnMount,
    /// Plays when the element scrolls into view; replays on each entry
    /// unless `once`.
    InView(InView),
    /// Plays forward while the condition holds and backward when it stops.
    /// The first build jumps to the matching end without animating.
    When(Cond),
    /// Driven by an [`AnimationHandle`].
    Handle(AnimationHandle),
}

impl Play {
    /// [`Play::InView`] with any visible pixel counting.
    #[must_use]
    pub fn in_view() -> Self {
        Self::InView(InView::new())
    }

    /// For [`Play::InView`]: play the first entry only.
    #[must_use]
    pub fn once(self) -> Self {
        match self {
            Self::InView(options) => Self::InView(options.once()),
            other => other,
        }
    }

    /// For [`Play::InView`]: the visible fraction that counts.
    #[must_use]
    pub fn amount(self, amount: f32) -> Self {
        match self {
            Self::InView(options) => Self::InView(options.amount(amount)),
            other => other,
        }
    }

    #[must_use]
    pub fn when(cond: impl Into<Cond>) -> Self {
        Self::When(cond.into())
    }

    #[must_use]
    pub fn handle(handle: &AnimationHandle) -> Self {
        Self::Handle(handle.clone())
    }
}

/// The part of a view timeline (CSS `animation-range`) mapped to 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewRange {
    /// From the first pixel entering to the last pixel leaving.
    #[default]
    Cover,
    /// While the element is fully inside the viewport.
    Contain,
    /// From the first pixel entering until fully inside.
    Entry,
    /// From starting to leave until fully gone.
    Exit,
}

/// The scroll offsets a [`Progress::scroll`] maps to 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollRange {
    start: f32,
    end: Option<f32>,
    horizontal: bool,
}

/// A playhead that follows a value instead of time.
#[derive(Clone)]
pub enum Progress {
    /// A fixed position; changes animate to it (like `Animate.target`).
    Value(f32),
    /// Follows a [`MotionValue`] exactly.
    Motion(MotionValue),
    /// Follows a live source (for example a `&Signal<f32>`).
    Live {
        source: DependencySource,
        read: Rc<dyn Fn() -> f32>,
    },
    /// The nearest scrollable's offset over a pixel range.
    Scroll(ScrollRange),
    /// The element's own position in its viewport.
    View(ViewRange),
}

impl Progress {
    #[must_use]
    pub fn value(progress: f32) -> Self {
        Self::Value(clamp01(progress))
    }

    #[must_use]
    pub fn motion(value: &MotionValue) -> Self {
        Self::Motion(value.clone())
    }

    #[must_use]
    pub fn live(source: DependencySource, read: impl Fn() -> f32 + 'static) -> Self {
        Self::Live {
            source,
            read: Rc::new(read),
        }
    }

    /// The nearest scrollable's offset from 0 to one viewport extent;
    /// narrow it with [`Progress::range`].
    #[must_use]
    pub fn scroll() -> Self {
        Self::Scroll(ScrollRange {
            start: 0.,
            end: None,
            horizontal: false,
        })
    }

    /// For [`Progress::scroll`]: the scroll offsets mapped to 0 and 1.
    #[must_use]
    pub fn range(self, start: f32, end: f32) -> Self {
        match self {
            Self::Scroll(range) => Self::Scroll(ScrollRange {
                start,
                end: Some(end),
                ..range
            }),
            other => other,
        }
    }

    /// For [`Progress::scroll`]: follow the horizontal offset.
    #[must_use]
    pub fn horizontal(self) -> Self {
        match self {
            Self::Scroll(range) => Self::Scroll(ScrollRange {
                horizontal: true,
                ..range
            }),
            other => other,
        }
    }

    #[must_use]
    pub fn view(range: ViewRange) -> Self {
        Self::View(range)
    }

    fn uses_geometry(&self) -> bool {
        matches!(self, Self::Scroll(_) | Self::View(_))
    }
}

fn clamp01(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0., 1.)
    } else {
        0.
    }
}

fn scroll_progress(range: ScrollRange, geometry: FrameGeometry) -> f32 {
    // The element sits at `content` inside the scroll content and at
    // `layout` on screen, below a viewport starting at `viewport.origin`.
    let (content, layout, viewport, extent) = if range.horizontal {
        (
            geometry.content_rect.origin.x,
            geometry.layout_rect.origin.x,
            geometry.viewport.origin.x,
            geometry.viewport.size.width,
        )
    } else {
        (
            geometry.content_rect.origin.y,
            geometry.layout_rect.origin.y,
            geometry.viewport.origin.y,
            geometry.viewport.size.height,
        )
    };
    let offset = viewport + content - layout;
    let end = range.end.unwrap_or(range.start + extent);
    if (end - range.start).abs() < f32::EPSILON {
        return if offset >= end { 1. } else { 0. };
    }
    clamp01((offset - range.start) / (end - range.start))
}

fn view_progress(range: ViewRange, geometry: FrameGeometry) -> f32 {
    let rect = geometry.layout_rect;
    let viewport = geometry.viewport;
    let top = rect.origin.y;
    let height = rect.size.height;
    let view_top = viewport.origin.y;
    let view_bottom = view_top + viewport.size.height;
    // Positions of the element's top where each phase starts and ends.
    let (start, end) = match range {
        ViewRange::Cover => (view_bottom, view_top - height),
        ViewRange::Contain => (view_bottom - height, view_top),
        ViewRange::Entry => (view_bottom, view_bottom - height),
        ViewRange::Exit => (view_top, view_top - height),
    };
    if (start - end).abs() < f32::EPSILON {
        return if top <= end { 1. } else { 0. };
    }
    clamp01((start - top) / (start - end))
}

// ---------------------------------------------------------------------------
// The spec and the channel

/// `build_each_frame(|t, child| ...)`.
pub(crate) type BuildEach = Rc<dyn Fn(f32, Widget) -> Widget>;

/// An `.animate()` chain as the motion node runs it.
#[derive(Clone, Default)]
pub(crate) struct TimelineSpec {
    pub(crate) effects: Vec<Effect>,
    pub(crate) delay: Duration,
    pub(crate) repeat: AnimateRepeat,
    pub(crate) on_complete: Option<Rc<dyn Fn()>>,
    pub(crate) play: Play,
    pub(crate) progress: Option<Progress>,
    pub(crate) build_each: Option<BuildEach>,
}

impl TimelineSpec {
    fn effects_end(&self) -> Duration {
        self.effects
            .iter()
            .map(|effect| effect.delay + effect.duration)
            .max()
            .unwrap_or(Duration::ZERO)
    }

    /// Whether frames rebuild the node (paint-tier effects).
    fn rebuilds(&self) -> bool {
        self.build_each.is_some()
            || self
                .effects
                .iter()
                .any(|effect| matches!(effect.kind, EffectKind::Shimmer { .. }))
    }

    fn loops(&self) -> bool {
        self.repeat != AnimateRepeat::Once
    }

    /// Whether the node must observe its visibility.
    pub(crate) fn uses_in_view(&self) -> bool {
        self.progress.is_none() && (matches!(self.play, Play::InView(_)) || self.loops())
    }

    pub(crate) fn in_view_options(&self) -> Option<InView> {
        match self.play {
            Play::InView(options) => Some(InView {
                once: false,
                ..options
            }),
            _ => None,
        }
    }

    pub(crate) fn uses_geometry(&self) -> bool {
        self.progress.as_ref().is_some_and(Progress::uses_geometry)
    }
}

fn put(out: &mut [f32; PROP_COUNT], prop: Prop, value: f32) {
    let index = prop.index();
    match prop.combine() {
        Combine::Multiply => out[index] *= value,
        Combine::Add => out[index] += value,
        Combine::Override => out[index] = value,
    }
}

fn lerp(begin: f32, end: f32, t: f32) -> f32 {
    begin + (end - begin) * t
}

/// The chain's contribution to each property at `elapsed` into the effects.
fn sample(effects: &[Effect], elapsed: Duration) -> ([f32; PROP_COUNT], [bool; PROP_COUNT]) {
    let mut out: [f32; PROP_COUNT] = std::array::from_fn(|index| Prop::ALL[index].identity());
    let mut touched = [false; PROP_COUNT];
    let mut set = |out: &mut [f32; PROP_COUNT], prop: Prop, value: f32| {
        touched[prop.index()] = true;
        put(out, prop, value);
    };
    for effect in effects {
        let t = effect.progress(elapsed);
        match &effect.kind {
            EffectKind::Fade { begin, end } => {
                set(&mut out, Prop::Opacity, lerp(*begin, *end, t).clamp(0., 1.));
            }
            EffectKind::Slide { begin, end } => {
                set(&mut out, Prop::SlideX, lerp(begin.x, end.x, t));
                set(&mut out, Prop::SlideY, lerp(begin.y, end.y, t));
            }
            EffectKind::Move { begin, end } => {
                set(&mut out, Prop::X, lerp(begin.x, end.x, t));
                set(&mut out, Prop::Y, lerp(begin.y, end.y, t));
            }
            EffectKind::Scale { begin, end } => set(&mut out, Prop::Scale, lerp(*begin, *end, t)),
            EffectKind::Rotate { begin, end } => {
                set(&mut out, Prop::Rotate, lerp(*begin, *end, t));
            }
            EffectKind::Blur { begin, end } => {
                set(&mut out, Prop::Blur, lerp(*begin, *end, t).max(0.));
            }
            EffectKind::Shake { hz, offset } => {
                // `ShakeEffect`: sin(t * 2π * hz * seconds), fading out.
                let phase = t * effect.duration.as_secs_f32() * hz * std::f32::consts::TAU;
                set(&mut out, Prop::X, phase.sin() * offset * (1. - t));
            }
            EffectKind::Tint { begin, end, .. } => {
                set(&mut out, Prop::Tint, lerp(*begin, *end, t).clamp(0., 1.));
            }
            EffectKind::Saturate { begin, end } => {
                set(&mut out, Prop::Saturate, lerp(*begin, *end, t).max(0.));
            }
            EffectKind::Shadow { shadow, begin, end } => {
                set(&mut out, Prop::ShadowX, shadow.offset.x);
                set(&mut out, Prop::ShadowY, shadow.offset.y);
                set(&mut out, Prop::ShadowBlur, shadow.blur_radius);
                set(&mut out, Prop::Shadow, lerp(*begin, *end, t).clamp(0., 1.));
            }
            EffectKind::ClipReveal { from } => {
                let hidden = 1. - t;
                match from {
                    RevealFrom::Left => set(&mut out, Prop::RevealRight, hidden),
                    RevealFrom::Right => set(&mut out, Prop::RevealLeft, hidden),
                    RevealFrom::Top => set(&mut out, Prop::RevealBottom, hidden),
                    RevealFrom::Bottom => set(&mut out, Prop::RevealTop, hidden),
                    RevealFrom::Center => {
                        set(&mut out, Prop::RevealCircle, if t >= 1. { -1. } else { t });
                    }
                }
            }
            EffectKind::Keys { prop, keys } => set(&mut out, *prop, keys.sample(t)),
            EffectKind::Custom(custom) => {
                let pose = (custom.0)(t, Pose::new());
                for prop in pose.props() {
                    if let Some(value) = pose.get(prop) {
                        set(&mut out, prop, value.get());
                    }
                }
            }
            EffectKind::Shimmer { .. } => {}
        }
    }
    (out, touched)
}

/// The props a chain writes, discovered by sampling (custom effects).
fn written_props(effects: &[Effect], total: Duration) -> [bool; PROP_COUNT] {
    let mut written = [false; PROP_COUNT];
    for at in [Duration::ZERO, total / 2, total] {
        let (_, touched) = sample(effects, at);
        for (index, touched) in touched.into_iter().enumerate() {
            written[index] |= touched;
        }
    }
    written
}

/// A running `.animate()` chain inside a motion node.
pub(crate) struct Timeline {
    node: Weak<MotionNode>,
    spec: RefCell<Rc<TimelineSpec>>,
    own: AnimationHandle,
    /// Smooths [`Progress::Value`] changes.
    smoothed: MotionValue,
    smoothed_set: Cell<bool>,
    /// Stagger offset applied on mount.
    mount_delay: Cell<Duration>,
    started: Cell<bool>,
    seen: Cell<bool>,
    /// The loop was paused because the element left the viewport.
    parked: Cell<f32>,
    /// The playhead position the values were last sampled at.
    position: Cell<f32>,
    live_dirty: Rc<Cell<bool>>,
    subscriptions: RefCell<Vec<Subscription>>,
    written: Cell<[bool; PROP_COUNT]>,
}

impl Timeline {
    pub(crate) fn new(node: Weak<MotionNode>) -> Rc<Self> {
        Rc::new(Self {
            node,
            spec: RefCell::new(Rc::new(TimelineSpec::default())),
            own: AnimationHandle::new(),
            smoothed: MotionValue::new(0.),
            smoothed_set: Cell::new(false),
            mount_delay: Cell::new(Duration::ZERO),
            started: Cell::new(false),
            seen: Cell::new(false),
            parked: Cell::new(0.),
            position: Cell::new(0.),
            live_dirty: Rc::new(Cell::new(false)),
            subscriptions: RefCell::new(Vec::new()),
            written: Cell::new([false; PROP_COUNT]),
        })
    }

    /// Adopts the chain from a new build.
    pub(crate) fn bind(
        &self,
        spec: Rc<TimelineSpec>,
        mount_delay: Option<Duration>,
        subscribe: impl FnOnce(&Cond, &Rc<dyn Fn()>, &mut Vec<Subscription>),
    ) {
        if let Some(delay) = mount_delay {
            self.mount_delay.set(delay);
        }
        let total = spec.effects_end();
        self.written.set(written_props(&spec.effects, total));
        let mut subscriptions = Vec::new();
        let wake: Rc<dyn Fn()> = {
            let dirty = self.live_dirty.clone();
            Rc::new(move || {
                dirty.set(true);
                incular_animation::schedule_frame();
            })
        };
        if let Play::When(cond) = &spec.play {
            subscribe(cond, &wake, &mut subscriptions);
        }
        match &spec.progress {
            Some(Progress::Live { source, .. }) => {
                let wake = wake.clone();
                subscriptions.push(source.subscribe("animate", move || wake()));
                self.live_dirty.set(true);
            }
            Some(Progress::Motion(_)) => self.live_dirty.set(true),
            _ => {}
        }
        *self.subscriptions.borrow_mut() = subscriptions;
        let changed_progress = {
            let old = self.spec.borrow();
            !matches!(
                (&old.progress, &spec.progress),
                (Some(Progress::Value(a)), Some(Progress::Value(b))) if a == b
            )
        };
        *self.spec.borrow_mut() = spec;
        if changed_progress {
            self.live_dirty.set(true);
        }
    }

    fn head(&self) -> AnimationHandle {
        match &self.spec.borrow().play {
            Play::Handle(handle) => handle.clone(),
            _ => self.own.clone(),
        }
    }

    /// The whole timeline: chain delay, stagger and effects.
    fn total(&self) -> Duration {
        let spec = self.spec.borrow();
        spec.delay + self.mount_delay.get() + spec.effects_end()
    }

    fn lead(&self) -> Duration {
        self.spec.borrow().delay + self.mount_delay.get()
    }

    fn visible(node: &MotionNode) -> bool {
        node.interactions().contains(Interaction::InView)
    }

    /// Steers the playhead from the trigger; returns whether it moved.
    fn steer(&self, node: &MotionNode, now: Instant) -> bool {
        let spec = self.spec.borrow().clone();
        let head = self.head();
        let total = self.total();
        if node.is_reduced() {
            // Reduced motion: one-shot chains jump to their end, loops hold.
            let target = match &spec.play {
                Play::When(cond) => {
                    if cond.eval(node.interactions(), &node.groups()) {
                        1.
                    } else {
                        0.
                    }
                }
                Play::InView(_) if !Self::visible(node) && !self.seen.get() => 0.,
                _ => 1.,
            };
            if !matches!(spec.play, Play::Handle(_)) {
                head.0.direction.set(0.);
                head.0.position.set(target);
            }
            let moved = self.position.replace(head.progress()) != head.progress();
            return moved;
        }
        match &spec.play {
            Play::OnMount => {
                if !self.started.replace(true) {
                    head.restart();
                }
            }
            Play::InView(options) => {
                let visible = Self::visible(node);
                if visible && !self.seen.get() {
                    self.seen.set(true);
                    head.restart();
                } else if !visible && self.seen.get() && !options.once {
                    self.seen.set(false);
                    head.0.direction.set(0.);
                    head.seek(0.);
                }
            }
            Play::When(cond) => {
                let on = cond.eval(node.interactions(), &node.groups());
                if !self.started.replace(true) {
                    head.0.position.set(if on { 1. } else { 0. });
                    head.0.direction.set(0.);
                } else if on && head.0.direction.get() <= 0. && head.progress() < 1. {
                    head.0.set_direction(1.);
                } else if !on && head.0.direction.get() >= 0. && head.progress() > 0. {
                    head.0.set_direction(-1.);
                }
            }
            Play::Handle(_) => {}
        }
        // Loops pause while their element is off screen.
        if spec.loops() && node.geometry.get().is_some() && !matches!(spec.play, Play::Handle(_)) {
            let visible = Self::visible(node);
            if !visible && head.is_playing() {
                self.parked.set(head.0.direction.get());
                head.0.direction.set(0.);
                head.0.last.set(None);
            } else if visible && self.parked.get() != 0. {
                head.0.set_direction(self.parked.replace(0.));
            }
        }
        let advance = head.0.advance(now, total, spec.repeat);
        if advance.finished_forward
            && spec.repeat == AnimateRepeat::Once
            && let Some(callback) = &spec.on_complete
        {
            callback();
        }
        let position = head.progress();
        self.position.replace(position) != position || advance.changed
    }

    /// Follows the progress source; returns whether it moved.
    fn follow(&self, progress: &Progress, now: Instant) -> bool {
        let position = match progress {
            Progress::Value(target) => {
                if !self.smoothed_set.replace(true) {
                    self.smoothed.set(*target);
                } else if (self.smoothed.target() - target).abs() > f32::EPSILON {
                    let span = (self.smoothed.get() - target).abs();
                    let duration = self.total().mul_f32(span.clamp(0., 1.));
                    self.smoothed
                        .animate_to(*target, &Transition::linear(duration));
                }
                self.smoothed.tick(now);
                self.smoothed.get()
            }
            Progress::Motion(value) => clamp01(value.get()),
            Progress::Live { read, .. } => clamp01(read()),
            Progress::Scroll(_) | Progress::View(_) => return false,
        };
        self.live_dirty.set(false);
        self.position.replace(position) != position
    }

    /// The node rebuilds each frame for paint-tier effects.
    pub(crate) fn rebuilds(&self) -> bool {
        self.spec.borrow().rebuilds()
    }

    /// The child with the paint-tier effects at the current position.
    pub(crate) fn decorate(&self, child: Widget) -> Widget {
        let spec = self.spec.borrow().clone();
        let elapsed = self.elapsed(&spec);
        let mut child = child;
        if let Some(build) = &spec.build_each {
            child = build(self.position.get(), child);
        }
        for effect in &spec.effects {
            if let EffectKind::Shimmer { color, angle } = effect.kind {
                let t = effect.progress(elapsed);
                child = shimmer(child, color, angle, t);
            }
        }
        child
    }

    fn elapsed(&self, _spec: &TimelineSpec) -> Duration {
        self.total()
            .mul_f32(self.position.get().clamp(0., 1.))
            .saturating_sub(self.lead())
    }

    pub(crate) fn wants_geometry(&self) -> bool {
        let spec = self.spec.borrow();
        spec.uses_geometry() || spec.uses_in_view()
    }

    /// The tint and shadow colors the chain uses.
    pub(crate) fn colors(&self) -> (Option<incular_core::Color>, Option<incular_core::Color>) {
        let spec = self.spec.borrow();
        let mut tint = None;
        let mut shadow = None;
        for effect in &spec.effects {
            match effect.kind {
                EffectKind::Tint { color, .. } => tint = Some(color),
                EffectKind::Shadow { shadow: value, .. } => shadow = Some(value.color),
                _ => {}
            }
        }
        (tint, shadow)
    }
}

fn shimmer(child: Widget, color: incular_core::Color, angle: f32, t: f32) -> Widget {
    // A soft band sweeping across the child, from before its start edge to
    // past its end edge.
    let band = 0.25;
    let center = -band + t * (1. + 2. * band);
    ShaderMask::new(
        move |bounds: Rect| {
            let (dx, dy) = (angle.cos(), angle.sin());
            let half = Offset::new(bounds.size.width * dx, bounds.size.height * dy);
            let start = Offset::new(
                bounds.origin.x + bounds.size.width * 0.5 - half.x * 0.5,
                bounds.origin.y + bounds.size.height * 0.5 - half.y * 0.5,
            );
            let end = Offset::new(start.x + half.x, start.y + half.y);
            let stops = GradientStops::new(vec![
                GradientStop {
                    offset: 0.,
                    color: incular_core::Color::WHITE,
                },
                GradientStop {
                    offset: (center - band).clamp(0., 1.),
                    color: incular_core::Color::WHITE,
                },
                GradientStop {
                    offset: center.clamp(0., 1.),
                    color,
                },
                GradientStop {
                    offset: (center + band).clamp(0., 1.),
                    color: incular_core::Color::WHITE,
                },
                GradientStop {
                    offset: 1.,
                    color: incular_core::Color::WHITE,
                },
            ]);
            Brush::LinearGradient(LinearGradient::new(start, end, stops))
        },
        child,
    )
    .into()
}

impl ChannelSource for Timeline {
    fn tick(&self, now: Instant) -> bool {
        let Some(node) = self.node.upgrade() else {
            return false;
        };
        let progress = self.spec.borrow().progress.clone();
        let changed = match &progress {
            Some(progress) => self.follow(progress, now),
            None => {
                // Activity probes may run repeatedly while muted. Consume a
                // wake only when ticking, before callbacks can enqueue another.
                self.live_dirty.set(false);
                self.steer(&node, now)
            }
        };
        if changed && self.rebuilds() {
            node.request_rebuild();
        }
        changed
    }

    fn is_active(&self) -> bool {
        let spec = self.spec.borrow().clone();
        match &spec.progress {
            Some(Progress::Value(_)) => self.smoothed.is_animating() || self.live_dirty.get(),
            Some(Progress::Motion(value)) => {
                value.is_animating() || (clamp01(value.get()) - self.position.get()).abs() > 0.
            }
            Some(Progress::Live { .. }) => self.live_dirty.get(),
            Some(_) => false,
            None => {
                if self.live_dirty.get() {
                    return true;
                }
                let Some(node) = self.node.upgrade() else {
                    return false;
                };
                let head = self.head();
                if head.is_playing() {
                    return true;
                }
                if self.parked.get() != 0. && Self::visible(&node) {
                    return true;
                }
                match &spec.play {
                    Play::OnMount => !self.started.get(),
                    Play::InView(_) => Self::visible(&node) != self.seen.get(),
                    Play::When(cond) => {
                        let on = cond.eval(node.interactions(), &node.groups());
                        !self.started.get()
                            || (on && head.progress() < 1.)
                            || (!on && head.progress() > 0.)
                    }
                    Play::Handle(_) => (head.progress() - self.position.get()).abs() > 0.,
                }
            }
        }
    }

    fn apply(&self, values: &mut [f32; PROP_COUNT]) {
        let spec = self.spec.borrow().clone();
        let (out, touched) = sample(&spec.effects, self.elapsed(&spec));
        for prop in Prop::ALL {
            let index = prop.index();
            if !touched[index] {
                continue;
            }
            put(values, prop, out[index]);
        }
    }

    fn stages(&self, out: &mut [bool; 6]) {
        let written = self.written.get();
        for prop in Prop::ALL {
            if written[prop.index()] {
                out[prop.stage() as usize] = true;
            }
        }
    }
}

impl GeometryListener for Timeline {
    fn geometry(&self, node: &MotionNode, _previous: Option<FrameGeometry>, now: FrameGeometry) {
        let progress = self.spec.borrow().progress.clone();
        let position = match progress {
            Some(Progress::Scroll(range)) => scroll_progress(range, now),
            Some(Progress::View(range)) => view_progress(range, now),
            _ => return,
        };
        if self.position.replace(position) != position {
            node.push();
            if self.rebuilds() {
                node.request_rebuild();
            }
        }
    }
}
