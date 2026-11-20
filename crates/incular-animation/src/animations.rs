//! Flutter's `Animation<double>` graph (`animation/animations.dart`): values
//! derived from one or more controllers without a clock of their own.
//!
//! [`DoubleAnimation`] is the cloneable handle every node converts into. Its
//! value reads track the controllers underneath, so a build that reads it
//! rebuilds when they tick, and [`DoubleAnimation::controllers`] names the
//! clocks a ticking widget must drive.
//!
//! Derived nodes are eager, as Flutter's are: each one subscribes to the
//! changes of its parents when it is built and reacts to them when they
//! happen, not when the node is next read. A train hop or a curve direction
//! change therefore takes effect, and fires its callbacks, on the change itself.

use crate::{Animatable, AnimationController, AnimationStatus, Curve, Tween};
use incular_core::reactivity::{DependencySource, Subscription};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

/// A node of an animation graph: Flutter's `Animation<double>`.
pub trait AnimationSource {
    /// The current value.
    fn value(&self) -> f32;
    /// The current status.
    fn status(&self) -> AnimationStatus;
    /// The controllers whose ticks change this value.
    fn controllers(&self) -> Vec<AnimationController>;
    /// The source notified whenever this node's value or status changes.
    /// Derived nodes forward their parents' notifications through it.
    fn changes(&self) -> DependencySource;
}

impl AnimationSource for AnimationController {
    fn value(&self) -> f32 {
        AnimationController::value(self)
    }

    fn status(&self) -> AnimationStatus {
        AnimationController::status(self)
    }

    fn controllers(&self) -> Vec<AnimationController> {
        vec![self.clone()]
    }

    fn changes(&self) -> DependencySource {
        self.dependency_source()
    }
}

/// A shared handle to any [`AnimationSource`]; what transitions and
/// builders take where Flutter takes an `Animation<double>`.
#[derive(Clone)]
pub struct DoubleAnimation(Rc<dyn AnimationSource>);

impl std::fmt::Debug for DoubleAnimation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DoubleAnimation")
            .field("value", &self.0.value())
            .field("status", &self.0.status())
            .finish()
    }
}

impl PartialEq for DoubleAnimation {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl DoubleAnimation {
    #[must_use]
    pub fn new(source: impl AnimationSource + 'static) -> Self {
        Self(Rc::new(source))
    }

    #[must_use]
    pub fn value(&self) -> f32 {
        self.0.value()
    }

    #[must_use]
    pub fn status(&self) -> AnimationStatus {
        self.0.status()
    }

    #[must_use]
    pub fn controllers(&self) -> Vec<AnimationController> {
        self.0.controllers()
    }

    /// Flutter `Animation.isAnimating`.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.status().is_animating()
    }

    /// Flutter `Animation.isCompleted`.
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.status().is_completed()
    }

    /// Flutter `Animation.isDismissed`.
    #[must_use]
    pub fn is_dismissed(&self) -> bool {
        self.status().is_dismissed()
    }

    /// Flutter `Animation.addListener` for a derived node: `listener` receives
    /// the value whenever it changes, including changes that reach it through
    /// its parents. Dropping the token unsubscribes.
    ///
    /// Unlike a controller's [`AnimationController::add_listener`], which runs
    /// on every `notifyListeners`, this follows the reactive engine and runs
    /// only when the value actually changes.
    #[must_use = "keep the subscription alive to observe changes"]
    pub fn observe(&self, listener: impl Fn(f32) + 'static) -> Subscription {
        let node = self.clone();
        self.0
            .changes()
            .subscribe((), move || listener(node.value()))
    }

    /// Flutter `Animation.addStatusListener` for a derived node. As with
    /// [`Self::observe`], it runs when the status changes.
    #[must_use = "keep the subscription alive to observe changes"]
    pub fn observe_status(&self, listener: impl Fn(AnimationStatus) + 'static) -> Subscription {
        let node = self.clone();
        self.0
            .changes()
            .subscribe((), move || listener(node.status()))
    }

    /// `CurvedAnimation(parent: self, curve: curve)`.
    #[must_use]
    pub fn curved(self, curve: Curve) -> Self {
        CurvedAnimation::new(self, curve).into()
    }

    /// `ReverseAnimation(self)`.
    #[must_use]
    pub fn reversed(self) -> Self {
        ReverseAnimation::new(self).into()
    }

    /// Flutter `Animation.drive`: this animation through `animatable`.
    #[must_use]
    pub fn drive<T, A: Animatable<T> + 'static>(self, animatable: A) -> DrivenAnimation<T> {
        DrivenAnimation {
            parent: self,
            animatable: Rc::new(animatable),
        }
    }
}

impl AnimationSource for DoubleAnimation {
    fn value(&self) -> f32 {
        self.0.value()
    }

    fn status(&self) -> AnimationStatus {
        self.0.status()
    }

    fn controllers(&self) -> Vec<AnimationController> {
        self.0.controllers()
    }

    fn changes(&self) -> DependencySource {
        self.0.changes()
    }
}

impl From<AnimationController> for DoubleAnimation {
    fn from(controller: AnimationController) -> Self {
        Self::new(controller)
    }
}

impl From<&AnimationController> for DoubleAnimation {
    fn from(controller: &AnimationController) -> Self {
        Self::new(controller.clone())
    }
}

/// Forwards every change of `parent` to `changes` until the token drops.
fn forward(parent: &DoubleAnimation, changes: DependencySource) -> Subscription {
    parent.changes().subscribe((), move || changes.notify())
}

/// Flutter `Animation<T>` from `animatable.animate(parent)` /
/// `parent.drive(animatable)`: a value of any type sampled from a double
/// animation.
pub struct DrivenAnimation<T> {
    parent: DoubleAnimation,
    animatable: Rc<dyn Animatable<T>>,
}

impl<T> Clone for DrivenAnimation<T> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            animatable: self.animatable.clone(),
        }
    }
}

impl<T> DrivenAnimation<T> {
    #[must_use]
    pub fn new(
        parent: impl Into<DoubleAnimation>,
        animatable: impl Animatable<T> + 'static,
    ) -> Self {
        Self {
            parent: parent.into(),
            animatable: Rc::new(animatable),
        }
    }

    #[must_use]
    pub fn value(&self) -> T {
        self.animatable.value_at(self.parent.value())
    }

    #[must_use]
    pub fn status(&self) -> AnimationStatus {
        self.parent.status()
    }

    #[must_use]
    pub fn parent(&self) -> &DoubleAnimation {
        &self.parent
    }

    #[must_use]
    pub fn controllers(&self) -> Vec<AnimationController> {
        self.parent.controllers()
    }
}

impl<T: 'static> DrivenAnimation<T> {
    /// Flutter `Animation.addListener` for a driven value: `listener` receives
    /// the value whenever the parent changes. Dropping the token unsubscribes.
    #[must_use = "keep the subscription alive to observe changes"]
    pub fn observe(&self, listener: impl Fn(T) + 'static) -> Subscription {
        let node = self.clone();
        self.parent
            .changes()
            .subscribe((), move || listener(node.value()))
    }
}

/// Flutter `CurvedAnimation`: `curve` applied to the parent's value, or
/// `reverse_curve` while the parent runs in reverse.
///
/// As in Flutter, the direction is latched when a run starts, so a run that
/// turns around midway keeps its curve until it rests. The latch is updated
/// when the parent changes, so a reversal that no one samples still counts.
pub struct CurvedAnimation {
    parent: DoubleAnimation,
    curve: Curve,
    reverse_curve: Option<Curve>,
    /// Flutter `_curveDirection`.
    direction: Rc<Cell<Option<AnimationStatus>>>,
    changes: DependencySource,
    /// Flutter's status listener on the parent.
    _link: Subscription,
}

/// Flutter `_updateCurveDirection`: the direction latched when a run starts,
/// kept until the parent comes to rest.
fn latched_direction(
    latched: Option<AnimationStatus>,
    status: AnimationStatus,
) -> Option<AnimationStatus> {
    match status {
        AnimationStatus::Dismissed | AnimationStatus::Completed => None,
        AnimationStatus::Forward | AnimationStatus::Reverse => Some(latched.unwrap_or(status)),
    }
}

impl CurvedAnimation {
    #[must_use]
    pub fn new(parent: impl Into<DoubleAnimation>, curve: Curve) -> Self {
        let parent = parent.into();
        let direction = Rc::new(Cell::new(latched_direction(None, parent.status())));
        let changes = DependencySource::default();
        let link = parent.changes().subscribe((), {
            let parent = parent.clone();
            let direction = direction.clone();
            let changes = changes.clone();
            move || {
                direction.set(latched_direction(direction.get(), parent.status()));
                changes.notify();
            }
        });
        Self {
            parent,
            curve,
            reverse_curve: None,
            direction,
            changes,
            _link: link,
        }
    }

    /// The curve used while the parent runs in reverse. Flutter uses
    /// `curve` when none is given.
    #[must_use]
    pub fn reverse_curve(mut self, curve: Curve) -> Self {
        self.reverse_curve = Some(curve);
        self
    }
}

impl AnimationSource for CurvedAnimation {
    fn value(&self) -> f32 {
        let t = self.parent.value();
        let reverse = self.direction.get().unwrap_or_else(|| self.parent.status())
            == AnimationStatus::Reverse;
        let curve = match self.reverse_curve {
            Some(reverse_curve) if reverse => reverse_curve,
            _ => self.curve,
        };
        if t == 0.0 || t == 1.0 {
            t
        } else {
            curve.apply(t)
        }
    }

    fn status(&self) -> AnimationStatus {
        self.parent.status()
    }

    fn controllers(&self) -> Vec<AnimationController> {
        self.parent.controllers()
    }

    fn changes(&self) -> DependencySource {
        self.changes.clone()
    }
}

impl From<CurvedAnimation> for DoubleAnimation {
    fn from(animation: CurvedAnimation) -> Self {
        Self::new(animation)
    }
}

/// Flutter `ReverseAnimation`: `1 - parent`, with the status reversed.
pub struct ReverseAnimation {
    parent: DoubleAnimation,
}

impl ReverseAnimation {
    #[must_use]
    pub fn new(parent: impl Into<DoubleAnimation>) -> Self {
        Self {
            parent: parent.into(),
        }
    }
}

impl AnimationSource for ReverseAnimation {
    fn value(&self) -> f32 {
        1.0 - self.parent.value()
    }

    fn status(&self) -> AnimationStatus {
        match self.parent.status() {
            AnimationStatus::Forward => AnimationStatus::Reverse,
            AnimationStatus::Reverse => AnimationStatus::Forward,
            AnimationStatus::Completed => AnimationStatus::Dismissed,
            AnimationStatus::Dismissed => AnimationStatus::Completed,
        }
    }

    fn controllers(&self) -> Vec<AnimationController> {
        self.parent.controllers()
    }

    /// Stateless, so it reports exactly what the parent reports.
    fn changes(&self) -> DependencySource {
        self.parent.changes()
    }
}

impl From<ReverseAnimation> for DoubleAnimation {
    fn from(animation: ReverseAnimation) -> Self {
        Self::new(animation)
    }
}

/// Flutter `AlwaysStoppedAnimation`: a constant value whose status is
/// `forward`. `kAlwaysCompleteAnimation` and `kAlwaysDismissedAnimation` are
/// [`always_complete`] and [`always_dismissed`]. For a constant of another
/// type, see [`always_stopped`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlwaysStoppedAnimation(pub f32);

impl AnimationSource for AlwaysStoppedAnimation {
    fn value(&self) -> f32 {
        self.0
    }

    fn status(&self) -> AnimationStatus {
        AnimationStatus::Forward
    }

    fn controllers(&self) -> Vec<AnimationController> {
        Vec::new()
    }

    fn changes(&self) -> DependencySource {
        DependencySource::default()
    }
}

impl From<AlwaysStoppedAnimation> for DoubleAnimation {
    fn from(animation: AlwaysStoppedAnimation) -> Self {
        Self::new(animation)
    }
}

/// Flutter `AlwaysStoppedAnimation<T>` for any value type: `value` at every
/// progress, with status `forward` and no controller to tick.
#[must_use]
pub fn always_stopped<T: Clone + 'static>(value: T) -> DrivenAnimation<T> {
    DrivenAnimation::new(
        DoubleAnimation::new(FixedStatus(0.0, AnimationStatus::Forward)),
        ConstantTween(value),
    )
}

struct FixedStatus(f32, AnimationStatus);

impl AnimationSource for FixedStatus {
    fn value(&self) -> f32 {
        self.0
    }

    fn status(&self) -> AnimationStatus {
        self.1
    }

    fn controllers(&self) -> Vec<AnimationController> {
        Vec::new()
    }

    fn changes(&self) -> DependencySource {
        DependencySource::default()
    }
}

/// Flutter `kAlwaysCompleteAnimation`: value 1, status completed.
#[must_use]
pub fn always_complete() -> DoubleAnimation {
    DoubleAnimation::new(FixedStatus(1.0, AnimationStatus::Completed))
}

/// Flutter `kAlwaysDismissedAnimation`: value 0, status dismissed.
#[must_use]
pub fn always_dismissed() -> DoubleAnimation {
    DoubleAnimation::new(FixedStatus(0.0, AnimationStatus::Dismissed))
}

/// Flutter `CompoundAnimation`: two animations combined; its status is the
/// second's while that runs, otherwise the first's.
pub struct CompoundAnimation {
    first: DoubleAnimation,
    next: DoubleAnimation,
    combine: fn(f32, f32) -> f32,
    changes: DependencySource,
    /// Forwards both parents' changes.
    _links: Vec<Subscription>,
}

impl CompoundAnimation {
    /// Flutter `AnimationMax`.
    #[must_use]
    pub fn max(first: impl Into<DoubleAnimation>, next: impl Into<DoubleAnimation>) -> Self {
        Self::with(first, next, f32::max)
    }

    /// Flutter `AnimationMin`.
    #[must_use]
    pub fn min(first: impl Into<DoubleAnimation>, next: impl Into<DoubleAnimation>) -> Self {
        Self::with(first, next, f32::min)
    }

    /// Flutter `AnimationMean`.
    #[must_use]
    pub fn mean(first: impl Into<DoubleAnimation>, next: impl Into<DoubleAnimation>) -> Self {
        Self::with(first, next, |a, b| (a + b) / 2.0)
    }

    fn with(
        first: impl Into<DoubleAnimation>,
        next: impl Into<DoubleAnimation>,
        combine: fn(f32, f32) -> f32,
    ) -> Self {
        let first = first.into();
        let next = next.into();
        let changes = DependencySource::default();
        let mut links = vec![forward(&first, changes.clone())];
        if next != first {
            links.push(forward(&next, changes.clone()));
        }
        Self {
            first,
            next,
            combine,
            changes,
            _links: links,
        }
    }
}

impl AnimationSource for CompoundAnimation {
    fn value(&self) -> f32 {
        (self.combine)(self.first.value(), self.next.value())
    }

    fn status(&self) -> AnimationStatus {
        let next = self.next.status();
        if next.is_animating() {
            next
        } else {
            self.first.status()
        }
    }

    fn controllers(&self) -> Vec<AnimationController> {
        let mut controllers = self.first.controllers();
        for controller in self.next.controllers() {
            if !controllers.contains(&controller) {
                controllers.push(controller);
            }
        }
        controllers
    }

    fn changes(&self) -> DependencySource {
        self.changes.clone()
    }
}

impl From<CompoundAnimation> for DoubleAnimation {
    fn from(animation: CompoundAnimation) -> Self {
        Self::new(animation)
    }
}

/// Flutter `AnimationMax`.
#[must_use]
pub fn animation_max(
    first: impl Into<DoubleAnimation>,
    next: impl Into<DoubleAnimation>,
) -> DoubleAnimation {
    CompoundAnimation::max(first, next).into()
}

/// Flutter `AnimationMin`.
#[must_use]
pub fn animation_min(
    first: impl Into<DoubleAnimation>,
    next: impl Into<DoubleAnimation>,
) -> DoubleAnimation {
    CompoundAnimation::min(first, next).into()
}

/// Flutter `AnimationMean`.
#[must_use]
pub fn animation_mean(
    first: impl Into<DoubleAnimation>,
    next: impl Into<DoubleAnimation>,
) -> DoubleAnimation {
    CompoundAnimation::mean(first, next).into()
}

/// Flutter `ProxyAnimation`: follows a parent that can be swapped, and holds
/// the last value while it has none.
#[derive(Clone)]
pub struct ProxyAnimation {
    state: Rc<RefCell<ProxyState>>,
}

struct ProxyState {
    parent: Option<DoubleAnimation>,
    /// Forwards the current parent's changes; replaced when the parent is.
    link: Option<Subscription>,
    value: f32,
    status: AnimationStatus,
    changes: DependencySource,
}

impl ProxyAnimation {
    /// A proxy with no parent, at `animation`'s value or 0 dismissed.
    #[must_use]
    pub fn new(animation: Option<DoubleAnimation>) -> Self {
        let changes = DependencySource::default();
        let link = animation
            .as_ref()
            .map(|parent| forward(parent, changes.clone()));
        Self {
            state: Rc::new(RefCell::new(ProxyState {
                parent: animation,
                link,
                value: 0.0,
                status: AnimationStatus::Dismissed,
                changes,
            })),
        }
    }

    /// Flutter `ProxyAnimation.parent=`: detaching keeps the parent's last
    /// value and status. Observers hear the switch.
    pub fn set_parent(&self, parent: Option<DoubleAnimation>) {
        let changes = {
            let mut state = self.state.borrow_mut();
            if let Some(old) = state.parent.take() {
                state.value = old.value();
                state.status = old.status();
            }
            let changes = state.changes.clone();
            state.link = parent.as_ref().map(|next| forward(next, changes.clone()));
            state.parent = parent;
            changes
        };
        changes.notify();
    }

    #[must_use]
    pub fn parent(&self) -> Option<DoubleAnimation> {
        self.state.borrow().parent.clone()
    }
}

impl AnimationSource for ProxyAnimation {
    fn value(&self) -> f32 {
        let state = self.state.borrow();
        state
            .parent
            .as_ref()
            .map_or(state.value, DoubleAnimation::value)
    }

    fn status(&self) -> AnimationStatus {
        let state = self.state.borrow();
        state
            .parent
            .as_ref()
            .map_or(state.status, DoubleAnimation::status)
    }

    fn controllers(&self) -> Vec<AnimationController> {
        self.state
            .borrow()
            .parent
            .as_ref()
            .map(DoubleAnimation::controllers)
            .unwrap_or_default()
    }

    fn changes(&self) -> DependencySource {
        self.state.borrow().changes.clone()
    }
}

impl From<ProxyAnimation> for DoubleAnimation {
    fn from(animation: ProxyAnimation) -> Self {
        Self::new(animation)
    }
}

/// Flutter `TrainHoppingAnimation`: follows `current` until `next` crosses
/// its value, then follows `next` for good.
pub struct TrainHoppingAnimation {
    state: Rc<TrainState>,
}

struct TrainState {
    current: RefCell<DoubleAnimation>,
    next: RefCell<Option<DoubleAnimation>>,
    /// Flutter `_mode`: whether `current` was below `next` at the start.
    current_below: Option<bool>,
    on_switched_train: RefCell<Option<Rc<dyn Fn()>>>,
    changes: DependencySource,
    /// Flutter's value listener on the current train.
    current_link: RefCell<Option<Subscription>>,
    /// Flutter's value listener on the next train; moves to `current_link`
    /// when the hop happens.
    next_link: RefCell<Option<Subscription>>,
}

/// Re-checks the trains whenever one of them changes, as Flutter's
/// `_valueChangeHandler` does.
fn train_relay(state: Weak<TrainState>) -> impl Fn() {
    move || {
        if let Some(state) = state.upgrade() {
            state.hop();
            state.changes.notify();
        }
    }
}

impl TrainState {
    /// Flutter `_valueChangeHandler`: hops once `next` has caught up with, or
    /// crossed, `current`, then runs `onSwitchedTrain`. Called from the
    /// change of either train, never from a read.
    fn hop(&self) {
        let Some(next) = self.next.borrow().clone() else {
            return;
        };
        let current = self.current.borrow().value();
        let next_value = next.value();
        let hop = match self.current_below {
            Some(true) => next_value <= current,
            Some(false) => next_value >= current,
            None => true,
        };
        if !hop {
            return;
        }
        *self.current.borrow_mut() = next;
        *self.next.borrow_mut() = None;
        // The old current train no longer matters: its edge goes, and the
        // edge of the train that is now current takes its place.
        let link = self.next_link.borrow_mut().take();
        *self.current_link.borrow_mut() = link;
        let callback = self.on_switched_train.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}

impl TrainHoppingAnimation {
    #[must_use]
    pub fn new(current: impl Into<DoubleAnimation>, next: Option<DoubleAnimation>) -> Self {
        let current = current.into();
        let current_below = next.as_ref().and_then(|next| {
            let (a, b) = (current.value(), next.value());
            if a == b { None } else { Some(a < b) }
        });
        let switched = next.is_some() && current_below.is_none();
        let (current, next) = match (switched, next) {
            (true, Some(next)) => (next, None),
            (_, next) => (current, next),
        };
        let state = Rc::new(TrainState {
            current: RefCell::new(current),
            next: RefCell::new(next),
            current_below,
            on_switched_train: RefCell::new(None),
            changes: DependencySource::default(),
            current_link: RefCell::new(None),
            next_link: RefCell::new(None),
        });
        let weak = Rc::downgrade(&state);
        let current = state.current.borrow().clone();
        *state.current_link.borrow_mut() =
            Some(current.changes().subscribe((), train_relay(weak.clone())));
        let next = state.next.borrow().clone();
        if let Some(next) = next {
            *state.next_link.borrow_mut() = Some(next.changes().subscribe((), train_relay(weak)));
        }
        Self { state }
    }

    /// Flutter `onSwitchedTrain`. Runs on the change that crosses the trains,
    /// not on a later read.
    #[must_use]
    pub fn on_switched_train(self, callback: impl Fn() + 'static) -> Self {
        *self.state.on_switched_train.borrow_mut() = Some(Rc::new(callback));
        self
    }

    /// Flutter `currentTrain`.
    #[must_use]
    pub fn current_train(&self) -> DoubleAnimation {
        self.state.current.borrow().clone()
    }
}

impl AnimationSource for TrainHoppingAnimation {
    fn value(&self) -> f32 {
        self.state.current.borrow().value()
    }

    fn status(&self) -> AnimationStatus {
        self.state.current.borrow().status()
    }

    fn controllers(&self) -> Vec<AnimationController> {
        let mut controllers = self.state.current.borrow().controllers();
        if let Some(next) = &*self.state.next.borrow() {
            for controller in next.controllers() {
                if !controllers.contains(&controller) {
                    controllers.push(controller);
                }
            }
        }
        controllers
    }

    fn changes(&self) -> DependencySource {
        self.state.changes.clone()
    }
}

impl From<TrainHoppingAnimation> for DoubleAnimation {
    fn from(animation: TrainHoppingAnimation) -> Self {
        Self::new(animation)
    }
}

/// Flutter `CurveTween`: transforms a double through `curve`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveTween(pub Curve);

impl Animatable<f32> for CurveTween {
    fn value_at(&self, progress: f32) -> f32 {
        self.0.apply(progress)
    }
}

/// Flutter `ConstantTween`: the same value at every progress.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstantTween<T>(pub T);

impl<T: Clone> Animatable<T> for ConstantTween<T> {
    fn value_at(&self, _progress: f32) -> T {
        self.0.clone()
    }
}

/// Flutter `StepTween`: an integer tween that floors, so it reaches `end`
/// only at the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepTween {
    pub begin: i64,
    pub end: i64,
}

impl Animatable<i64> for StepTween {
    fn value_at(&self, progress: f32) -> i64 {
        (self.begin as f64 + (self.end - self.begin) as f64 * f64::from(progress)).floor() as i64
    }
}

/// Flutter `ReverseTween`: `parent` played from end to beginning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReverseTween<A>(pub A);

impl<T, A: Animatable<T>> Animatable<T> for ReverseTween<A> {
    fn value_at(&self, progress: f32) -> T {
        self.0.value_at(1.0 - progress)
    }
}

/// Flutter `Animatable.chain`: `inner` first, then `outer`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChainedTween<Outer, Inner> {
    pub outer: Outer,
    pub inner: Inner,
}

impl<T, Outer: Animatable<T>, Inner: Animatable<f32>> Animatable<T> for ChainedTween<Outer, Inner> {
    fn value_at(&self, progress: f32) -> T {
        self.outer.value_at(self.inner.value_at(progress))
    }
}

/// A linear tween over any [`Tween`] value, as Flutter's `Tween<T>`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearTween<T> {
    pub begin: T,
    pub end: T,
}

impl<T: Tween> Animatable<T> for LinearTween<T> {
    fn value_at(&self, progress: f32) -> T {
        T::lerp(self.begin, self.end, progress)
    }
}

/// Flutter `Animatable` helpers available on every animatable.
pub trait AnimatableExt<T>: Animatable<T> + Sized {
    /// Flutter `Animatable.chain`: feeds `inner`'s output into this one.
    fn chain<Inner: Animatable<f32>>(self, inner: Inner) -> ChainedTween<Self, Inner> {
        ChainedTween { outer: self, inner }
    }

    /// Flutter `Animatable.animate`: this animatable driven by `parent`.
    fn animate(self, parent: impl Into<DoubleAnimation>) -> DrivenAnimation<T>
    where
        Self: 'static,
    {
        DrivenAnimation::new(parent, self)
    }

    /// Flutter `Animatable.transform`.
    fn transform(&self, progress: f32) -> T {
        self.value_at(progress)
    }
}

impl<T, A: Animatable<T>> AnimatableExt<T> for A {}

impl From<f32> for DoubleAnimation {
    /// A fixed value: `AlwaysStoppedAnimation(value)`.
    fn from(value: f32) -> Self {
        Self::new(AlwaysStoppedAnimation(value))
    }
}
