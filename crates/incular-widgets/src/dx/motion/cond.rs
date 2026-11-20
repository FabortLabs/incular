//! Conditions that select poses: interaction states, application data and
//! named group states, combined with `&`, `|` and `!`.

use std::{
    cell::Cell,
    fmt,
    ops::{BitAnd, BitOr, Not},
    rc::Rc,
};

use incular_core::reactivity::{DependencySource, Subscription};

/// The interaction states a motion wrapper observes by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Interaction {
    /// A mouse or pen is over the element's layout box (never from touch).
    Hovered,
    /// A pointer is held down inside, or Enter/Space is held while focus is
    /// inside.
    Pressed,
    /// Keyboard focus is on the element or inside it.
    Focused,
    /// Focus is inside and focus highlights are showing (keyboard use).
    FocusVisible,
    /// The element is being dragged.
    Dragging,
    /// The element intersects its viewport.
    InView,
    /// A compatible drag payload is over this drop target.
    DropOver,
    /// A drag compatible with this drop target is in progress anywhere.
    DropActive,
}

impl Interaction {
    pub(crate) const fn bit(self) -> u16 {
        1 << self as u16
    }
}

/// A set of active [`Interaction`]s.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Interactions(u16);

impl Interactions {
    pub const NONE: Self = Self(0);

    #[must_use]
    pub const fn contains(self, state: Interaction) -> bool {
        self.0 & state.bit() != 0
    }

    #[must_use]
    pub const fn with(self, state: Interaction, on: bool) -> Self {
        if on {
            Self(self.0 | state.bit())
        } else {
            Self(self.0 & !state.bit())
        }
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(crate) const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub(crate) const fn only(state: Interaction) -> Self {
        Self(state.bit())
    }
}

/// A named scope whose interaction state descendants can react to.
///
/// A `GROUP.hovered()` condition resolves to the **nearest ancestor with that
/// name**; an inner group of the same name shadows an outer one, and
/// different names nest freely.
///
/// ```ignore
/// const CARD: Group = Group::new("card");
/// card.motion(|m| m.group(CARD));
/// play.motion(|m| m.rest(|p| p.opacity(0.)).when(CARD.hovered(), |p| p.opacity(1.)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Group(&'static str);

impl Group {
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.0
    }

    /// A pointer is over the group's box.
    #[must_use]
    pub const fn hovered(self) -> Cond {
        Cond::Group(self, GroupFlag::Hovered)
    }

    /// A pointer is held down inside the group.
    #[must_use]
    pub const fn pressed(self) -> Cond {
        Cond::Group(self, GroupFlag::Pressed)
    }

    /// Keyboard focus is inside the group.
    #[must_use]
    pub const fn focus_within(self) -> Cond {
        Cond::Group(self, GroupFlag::FocusWithin)
    }

    /// Focus is inside the group and focus highlights are showing.
    #[must_use]
    pub const fn focus_visible(self) -> Cond {
        Cond::Group(self, GroupFlag::FocusVisible)
    }

    /// At least one nested group inside this one is hovered.
    #[must_use]
    pub const fn any_hovered(self) -> Cond {
        Cond::Group(self, GroupFlag::AnyHovered)
    }
}

/// The state a [`Group`] publishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GroupFlag {
    Hovered,
    Pressed,
    FocusWithin,
    FocusVisible,
    AnyHovered,
}

/// A group's live state, shared with descendant motion nodes.
pub(crate) struct GroupState {
    pub(crate) group: Group,
    pub(crate) hovered: Cell<bool>,
    pub(crate) pressed: Cell<bool>,
    pub(crate) focus_within: Cell<bool>,
    pub(crate) focus_visible: Cell<bool>,
    pub(crate) hovered_descendants: Cell<u32>,
    pub(crate) changes: DependencySource,
}

impl GroupState {
    pub(crate) fn new(group: Group) -> Rc<Self> {
        Rc::new(Self {
            group,
            hovered: Cell::new(false),
            pressed: Cell::new(false),
            focus_within: Cell::new(false),
            focus_visible: Cell::new(false),
            hovered_descendants: Cell::new(0),
            changes: DependencySource::default(),
        })
    }

    pub(crate) fn flag(&self, flag: GroupFlag) -> bool {
        match flag {
            GroupFlag::Hovered => self.hovered.get(),
            GroupFlag::Pressed => self.pressed.get(),
            GroupFlag::FocusWithin => self.focus_within.get(),
            GroupFlag::FocusVisible => self.focus_visible.get(),
            GroupFlag::AnyHovered => self.hovered_descendants.get() > 0,
        }
    }
}

/// The groups in scope at an element, nearest last.
#[derive(Clone, Default)]
pub(crate) struct GroupScopes(pub(crate) Rc<[Rc<GroupState>]>);

impl GroupScopes {
    pub(crate) fn nearest(&self, group: Group) -> Option<Rc<GroupState>> {
        self.0
            .iter()
            .rev()
            .find(|state| state.group == group)
            .cloned()
    }

    pub(crate) fn pushed(&self, state: Rc<GroupState>) -> Self {
        Self(
            self.0
                .iter()
                .cloned()
                .chain(std::iter::once(state))
                .collect(),
        )
    }
}

/// Application data observed without rebuilding: a dependency source and an
/// untracked reader of its current value.
#[derive(Clone)]
pub struct LiveCond {
    pub(crate) source: DependencySource,
    pub(crate) read: Rc<dyn Fn() -> bool>,
}

/// When a pose applies.
///
/// A `bool` is read at build time; a live condition (for example a
/// `&Signal<bool>`) subscribes the motion node directly, so flipping it
/// retargets the pose without rebuilding the subtree.
#[derive(Clone)]
pub enum Cond {
    Const(bool),
    Live(LiveCond),
    State(Interaction),
    Group(Group, GroupFlag),
    Not(Rc<Cond>),
    And(Rc<Cond>, Rc<Cond>),
    Or(Rc<Cond>, Rc<Cond>),
}

impl fmt::Debug for Cond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(value) => write!(f, "{value}"),
            Self::Live(live) => write!(f, "live({})", (live.read)()),
            Self::State(state) => write!(f, "{state:?}"),
            Self::Group(group, flag) => write!(f, "{}.{flag:?}", group.name()),
            Self::Not(inner) => write!(f, "!{inner:?}"),
            Self::And(a, b) => write!(f, "({a:?} & {b:?})"),
            Self::Or(a, b) => write!(f, "({a:?} | {b:?})"),
        }
    }
}

impl From<bool> for Cond {
    fn from(value: bool) -> Self {
        Self::Const(value)
    }
}

impl From<Interaction> for Cond {
    fn from(value: Interaction) -> Self {
        Self::State(value)
    }
}

impl Cond {
    /// A condition read from application state. `source` notifies when it
    /// may have changed; `read` returns the current value without tracking.
    #[must_use]
    pub fn live(source: DependencySource, read: impl Fn() -> bool + 'static) -> Self {
        Self::Live(LiveCond {
            source,
            read: Rc::new(read),
        })
    }

    /// The interaction states this condition reads.
    pub(crate) fn interactions(&self) -> Interactions {
        match self {
            Self::State(state) => Interactions::only(*state),
            Self::Not(inner) => inner.interactions(),
            Self::And(a, b) | Self::Or(a, b) => a.interactions().union(b.interactions()),
            _ => Interactions::NONE,
        }
    }

    pub(crate) fn visit(&self, visit: &mut impl FnMut(&Cond)) {
        visit(self);
        match self {
            Self::Not(inner) => inner.visit(visit),
            Self::And(a, b) | Self::Or(a, b) => {
                a.visit(visit);
                b.visit(visit);
            }
            _ => {}
        }
    }

    pub(crate) fn eval(&self, own: Interactions, groups: &GroupScopes) -> bool {
        match self {
            Self::Const(value) => *value,
            Self::Live(live) => (live.read)(),
            Self::State(state) => own.contains(*state),
            Self::Group(group, flag) => groups
                .nearest(*group)
                .is_some_and(|state| state.flag(*flag)),
            Self::Not(inner) => !inner.eval(own, groups),
            Self::And(a, b) => a.eval(own, groups) && b.eval(own, groups),
            Self::Or(a, b) => a.eval(own, groups) || b.eval(own, groups),
        }
    }

    /// Subscribes `invalidate` to every live source and group this reads.
    pub(crate) fn subscribe(
        &self,
        groups: &GroupScopes,
        invalidate: &Rc<dyn Fn()>,
        out: &mut Vec<Subscription>,
    ) {
        self.visit(&mut |cond| {
            let source = match cond {
                Self::Live(live) => Some(live.source.clone()),
                Self::Group(group, _) => groups.nearest(*group).map(|s| s.changes.clone()),
                _ => None,
            };
            if let Some(source) = source {
                let invalidate = invalidate.clone();
                out.push(source.subscribe("motion", move || invalidate()));
            }
        });
    }
}

impl<T: Into<Cond>> BitAnd<T> for Cond {
    type Output = Cond;
    fn bitand(self, rhs: T) -> Cond {
        Cond::And(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl<T: Into<Cond>> BitOr<T> for Cond {
    type Output = Cond;
    fn bitor(self, rhs: T) -> Cond {
        Cond::Or(Rc::new(self), Rc::new(rhs.into()))
    }
}

impl Not for Cond {
    type Output = Cond;
    fn not(self) -> Cond {
        Cond::Not(Rc::new(self))
    }
}

/// This element is hovered.
#[must_use]
pub const fn hovered() -> Cond {
    Cond::State(Interaction::Hovered)
}

/// This element is pressed.
#[must_use]
pub const fn pressed() -> Cond {
    Cond::State(Interaction::Pressed)
}

/// Focus is on or inside this element.
#[must_use]
pub const fn focused() -> Cond {
    Cond::State(Interaction::Focused)
}

/// Focus is inside and highlights are showing.
#[must_use]
pub const fn focus_visible() -> Cond {
    Cond::State(Interaction::FocusVisible)
}

/// This element is being dragged.
#[must_use]
pub const fn dragging() -> Cond {
    Cond::State(Interaction::Dragging)
}

/// This element intersects its viewport.
#[must_use]
pub const fn in_view() -> Cond {
    Cond::State(Interaction::InView)
}

/// A compatible payload is over this drop target.
#[must_use]
pub const fn drop_over() -> Cond {
    Cond::State(Interaction::DropOver)
}

/// A drag this drop target accepts is in progress.
#[must_use]
pub const fn drop_active() -> Cond {
    Cond::State(Interaction::DropActive)
}
