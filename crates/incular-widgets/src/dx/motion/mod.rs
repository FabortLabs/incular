//! Automatic motion: poses for interaction states, application conditions,
//! named groups, presence and layout changes, after motion.dev's
//! `whileHover` / `layout` / `AnimatePresence` and CSS transitions.
//!
//! ```ignore
//! Container::new().color(surface).child(Text::new("Gallery"))
//!     .motion(|m| m
//!         .hovered(|p| p.y(-4.0).lift(16.0))
//!         .pressed(|p| p.scale(0.97).transition(ms(80))))
//! ```
//!
//! The widget as written is the resting state. A [`Pose`] holds only
//! compositor properties, so motion never relayouts or repaints the
//! element's content: values tick on the tree's animation clock and reach
//! retained compositor layers directly.

mod behavior;
mod cond;
pub(crate) mod config;
mod dnd;
mod dragging;
mod geometry;
mod layout;
mod lint;
pub(crate) mod node;
mod pose;
mod presence;
mod reorder;
mod shared;
pub(crate) mod timeline;

use std::rc::Rc;

pub use crate::dx::drag::{Bounds, Drag, DragAxis, Release};
pub use crate::dx::view_transition::{
    Reveal, ViewScope, ViewTransition, ViewTransitionExt, view_transition,
};
pub use cond::{
    Cond, Group, GroupFlag, Interaction, Interactions, LiveCond, dragging, drop_active, drop_over,
    focus_visible, focused, hovered, in_view, pressed,
};
pub use config::{MotionConfig, Reduced, ReducedMotion};
pub use dnd::{DndExt, DragDrop, DragSite, DropRules, DropSite};
pub use incular_animation::{
    Channel, Curve, Keyframes, MotionValue, Spring, Transition, TransitionKind, ms, secs,
    with_transition, without_transition,
};
pub use incular_rendering::Shadow;
pub use lint::MotionLint;
pub use pose::{Combine, PROP_COUNT, Pose, PoseValue, Prop, Stage};
pub use reorder::{
    Collision, Move, Placeholder, ReorderExt, ReorderGroup, ReorderItem, Reorderable,
};
pub use timeline::{AnimationHandle, Finished, Play, Progress, ViewRange};

use crate::Widget;

/// How much of an element must be visible to count as in view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InView {
    pub(crate) amount: f32,
    pub(crate) once: bool,
}

impl Default for InView {
    fn default() -> Self {
        Self {
            amount: 0.,
            once: false,
        }
    }
}

impl InView {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The visible fraction (0 to 1) that counts as in view; 0 means any
    /// pixel.
    #[must_use]
    pub fn amount(mut self, amount: f32) -> Self {
        self.amount = if amount.is_finite() {
            amount.clamp(0., 1.)
        } else {
            0.
        };
        self
    }

    /// Stays in view once it has entered.
    #[must_use]
    pub fn once(mut self) -> Self {
        self.once = true;
        self
    }
}

/// Where a departing element sits while its exit plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExitLayout {
    /// Keeps its slot until the exit settles.
    #[default]
    InPlace,
    /// Leaves layout at once; paints at its last position while siblings
    /// move into the gap.
    Pop,
}

/// How a parent sequences a keyed child being replaced.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Presence {
    /// The new child enters while the old one exits.
    #[default]
    Sync,
    /// The new child enters after the old one has exited.
    Wait,
}

/// Which geometry changes [`Motion::layout`] animates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutMotion {
    /// Position and size.
    Bounds,
    /// Position only.
    Position,
}

/// Identity matched across two places by [`Motion::shared`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SharedId(pub(crate) Rc<str>, pub(crate) u64);

impl SharedId {
    /// An id from any hashable value, for example `("album", album.id)`.
    #[must_use]
    pub fn of(value: impl std::hash::Hash) -> Self {
        use std::hash::{BuildHasher, BuildHasherDefault};
        let hash = BuildHasherDefault::<std::collections::hash_map::DefaultHasher>::default()
            .hash_one(&value);
        Self(Rc::from(""), hash)
    }
}

impl From<&'static str> for SharedId {
    fn from(value: &'static str) -> Self {
        Self(Rc::from(value), 0)
    }
}

impl<A: std::hash::Hash, B: std::hash::Hash> From<(A, B)> for SharedId {
    fn from(value: (A, B)) -> Self {
        Self::of(value)
    }
}

/// How a shared element morphs between its two places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedMode {
    /// The same thing in two places: bounds morph.
    Element,
    /// Different content sharing bounds: bounds morph and contents
    /// crossfade.
    Bounds,
}

/// Offsets the start of children's enter and `.animate()` timelines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stagger {
    pub(crate) every: std::time::Duration,
    pub(crate) from: StaggerFrom,
}

/// Which child starts first in a [`Stagger`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StaggerFrom {
    #[default]
    Start,
    End,
    Center,
    /// Ripples outward from this index.
    Index(usize),
}

impl Stagger {
    /// Each child starts `every` after the previous one.
    #[must_use]
    pub const fn every(every: std::time::Duration) -> Self {
        Self {
            every,
            from: StaggerFrom::Start,
        }
    }

    #[must_use]
    pub const fn from(mut self, from: StaggerFrom) -> Self {
        self.from = from;
        self
    }

    /// The delay of child `index` among `count`.
    #[must_use]
    pub fn delay(&self, index: usize, count: usize) -> std::time::Duration {
        let steps = match self.from {
            StaggerFrom::Start => index,
            StaggerFrom::End => count.saturating_sub(1).saturating_sub(index),
            StaggerFrom::Center => {
                let center = count.saturating_sub(1) as f32 / 2.;
                (index as f32 - center).abs().round() as usize
            }
            StaggerFrom::Index(origin) => index.abs_diff(origin),
        };
        self.every.saturating_mul(steps as u32)
    }
}

/// A motion configuration, built in the closure passed to
/// [`MotionExt::motion`].
#[derive(Clone, Default)]
pub struct Motion {
    pub(crate) rest: Pose,
    pub(crate) states: Vec<(Cond, Pose)>,
    pub(crate) enter: Option<Pose>,
    pub(crate) exit: Option<Pose>,
    pub(crate) transition: Option<Transition>,
    pub(crate) group: Option<Group>,
    pub(crate) in_view: InView,
    pub(crate) reduced: Reduced,
    pub(crate) exit_layout: ExitLayout,
    pub(crate) presence: Option<Presence>,
    pub(crate) layout: Option<LayoutMotion>,
    pub(crate) shared: Option<(SharedId, SharedMode)>,
    pub(crate) shared_scope: bool,
    pub(crate) stagger: Option<Stagger>,
    pub(crate) drag: Option<node::DragConfig>,
    pub(crate) timeline: Option<Rc<timeline::TimelineSpec>>,
    pub(crate) behaviors: Vec<Rc<dyn behavior::SiteBehavior>>,
}

impl Motion {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The resting look for visual properties the widget itself cannot
    /// express (for example hidden until a group is hovered). Size, color
    /// and position come from the widget.
    #[must_use]
    pub fn rest(mut self, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.rest = pose(self.rest);
        self
    }

    /// A pose applied while `cond` holds. Later declarations win, property
    /// by property.
    #[must_use]
    pub fn when(mut self, cond: impl Into<Cond>, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.states.push((cond.into(), pose(Pose::new())));
        self
    }

    /// While a mouse or pen is over the element.
    #[must_use]
    pub fn hovered(self, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.when(hovered(), pose)
    }

    /// While pressed by a pointer, or by Enter/Space with focus inside.
    #[must_use]
    pub fn pressed(self, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.when(pressed(), pose)
    }

    /// While focus is on or inside the element.
    #[must_use]
    pub fn focused(self, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.when(focused(), pose)
    }

    /// While focus is inside and focus highlights are showing.
    #[must_use]
    pub fn focus_visible(self, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.when(focus_visible(), pose)
    }

    /// While the element is dragged.
    #[must_use]
    pub fn dragging(self, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.when(dragging(), pose)
    }

    /// While the element intersects its viewport.
    #[must_use]
    pub fn in_view(self, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.when(in_view(), pose)
    }

    /// How [`Self::in_view`] decides visibility.
    #[must_use]
    pub fn in_view_options(mut self, options: InView) -> Self {
        self.in_view = options;
        self
    }

    /// Plays from this pose when the element mounts ("enter from").
    #[must_use]
    pub fn enter(mut self, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.enter = Some(pose(Pose::new()));
        self
    }

    /// Plays to this pose when the element is removed ("exit to"); the
    /// element stays, non-interactive and hidden from semantics, until the
    /// exit settles.
    #[must_use]
    pub fn exit(mut self, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.exit = Some(pose(Pose::new()));
        self
    }

    /// Where the element sits while exiting.
    #[must_use]
    pub fn exit_layout(mut self, layout: ExitLayout) -> Self {
        self.exit_layout = layout;
        self
    }

    /// How keyed children of this element are replaced.
    #[must_use]
    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = Some(presence);
        self
    }

    /// The default transition for this motion's poses.
    #[must_use]
    pub fn transition(mut self, transition: impl Into<Transition>) -> Self {
        self.transition = Some(transition.into());
        self
    }

    /// Publishes this element's interaction state to descendants as `group`.
    #[must_use]
    pub fn group(mut self, group: Group) -> Self {
        self.group = Some(group);
        self
    }

    /// How this site behaves under reduced motion.
    #[must_use]
    pub fn reduced(mut self, reduced: Reduced) -> Self {
        self.reduced = reduced;
        self
    }

    /// Animates position and size changes from layout (keyed siblings).
    #[must_use]
    pub fn layout(mut self) -> Self {
        self.layout = Some(LayoutMotion::Bounds);
        self
    }

    /// Animates position changes from layout only.
    #[must_use]
    pub fn layout_position(mut self) -> Self {
        self.layout = Some(LayoutMotion::Position);
        self
    }

    /// The same element in two places: a newly mounted element with this id
    /// starts from where the last one was.
    #[must_use]
    pub fn shared(mut self, id: impl Into<SharedId>) -> Self {
        self.shared = Some((id.into(), SharedMode::Element));
        self
    }

    /// Different content sharing bounds, with a crossfade.
    #[must_use]
    pub fn shared_bounds(mut self, id: impl Into<SharedId>) -> Self {
        self.shared = Some((id.into(), SharedMode::Bounds));
        self
    }

    /// Shared ids inside this element do not match ids outside it.
    #[must_use]
    pub fn shared_scope(mut self) -> Self {
        self.shared_scope = true;
        self
    }

    /// Staggers the enter and `.animate()` start of direct children.
    #[must_use]
    pub fn stagger(mut self, stagger: Stagger) -> Self {
        self.stagger = Some(stagger);
        self
    }

    /// Makes the element draggable on the compositor (§7.1).
    #[must_use]
    pub fn drag(mut self, drag: crate::dx::drag::Drag) -> Self {
        self.drag = Some(node::DragConfig::from(drag));
        self
    }

    /// Likely declaration mistakes, given whether the child is keyed.
    #[must_use]
    pub fn lints(&self, child_keyed: bool) -> Vec<MotionLint> {
        lint::lints(self, child_keyed)
    }

    /// Every pose this motion declares, rest first.
    pub(crate) fn poses(&self) -> impl Iterator<Item = &Pose> {
        std::iter::once(&self.rest)
            .chain(self.states.iter().map(|(_, pose)| pose))
            .chain(self.enter.iter())
            .chain(self.exit.iter())
    }

    /// The interaction states any condition reads.
    pub(crate) fn interactions(&self) -> Interactions {
        let mut used = self
            .states
            .iter()
            .fold(Interactions::NONE, |used, (cond, _)| {
                used.union(cond.interactions())
            });
        if self.group.is_some() {
            used = used
                .with(Interaction::Hovered, true)
                .with(Interaction::Pressed, true)
                .with(Interaction::Focused, true)
                .with(Interaction::FocusVisible, true);
        }
        if self.drag.is_some() {
            used = used.with(Interaction::Dragging, true);
        }
        if self
            .timeline
            .as_ref()
            .is_some_and(|timeline| timeline.uses_in_view())
        {
            used = used.with(Interaction::InView, true);
        }
        for behavior in &self.behaviors {
            used = used.union(behavior.interactions());
        }
        used
    }
}

/// `.motion()` for anything that becomes a [`Widget`].
pub trait MotionExt: Into<Widget> + Sized {
    /// Animates this widget's look for interaction states, conditions,
    /// presence and layout changes.
    #[must_use]
    fn motion(self, build: impl FnOnce(Motion) -> Motion) -> Widget {
        node::motion_widget(self.into(), build(Motion::new()))
    }
}

impl<T: Into<Widget>> MotionExt for T {}

thread_local! {
    static REASSEMBLY: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Makes the next build replay every enter animation and mount timeline,
/// as after a hot reload (the runtime calls this when it reassembles).
pub fn replay_enter_animations() {
    REASSEMBLY.with(|epoch| epoch.set(epoch.get().wrapping_add(1)));
}

pub(crate) fn reassembly() -> u64 {
    REASSEMBLY.try_with(std::cell::Cell::get).unwrap_or(0)
}
