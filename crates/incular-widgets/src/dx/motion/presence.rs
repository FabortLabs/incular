//! Mount-time orchestration: staggered enters, and the presence parent that
//! sequences keyed replacements.

use std::{
    cell::Cell,
    rc::{Rc, Weak},
    time::Duration,
};

use incular_core::time::Instant;

use super::{Motion, Presence, Stagger, node::MotionNode};
use crate::{BuildContext, Widget};

/// The stagger a motion publishes to its nearest motion descendants. A
/// motion that consumes a slot publishes an empty scope, so only the nearest
/// motion descendants (normally the direct children) are staggered.
#[derive(Clone)]
pub(crate) struct StaggerScope {
    stagger: Option<Stagger>,
    next: Rc<Cell<usize>>,
    epoch: Rc<Cell<Option<Instant>>>,
    count: usize,
}

impl StaggerScope {
    fn empty() -> Self {
        Self {
            stagger: None,
            next: Rc::new(Cell::new(0)),
            epoch: Rc::new(Cell::new(None)),
            count: 0,
        }
    }
}

/// Computes the scope this node publishes to its subtree.
pub(crate) fn before_build(
    node: &Rc<MotionNode>,
    spec: &Rc<Motion>,
    _context: &BuildContext<'_>,
    child: &Widget,
) {
    let scope = if let Some(stagger) = spec.stagger {
        let count = child.kind().structure().children.into_iter().count();
        let previous = node.child_scope.borrow().clone();
        Some(match previous {
            Some(mut scope) if scope.stagger.is_some() => {
                scope.stagger = Some(stagger);
                scope.count = count;
                scope
            }
            _ => StaggerScope {
                stagger: Some(stagger),
                next: Rc::new(Cell::new(0)),
                epoch: Rc::new(Cell::new(None)),
                count,
            },
        })
    } else if spec.enter.is_some() {
        Some(StaggerScope::empty())
    } else {
        None
    };
    *node.child_scope.borrow_mut() = scope;
}

/// The enter delay of a mounting motion inside a staggering parent. Mounts
/// in the same frame take consecutive slots; a later frame starts again at
/// the first slot, so items added later are staggered among themselves.
pub(crate) fn stagger_delay(_node: &MotionNode, context: &BuildContext<'_>) -> Duration {
    let Some(scope) = context.find::<StaggerScope>() else {
        return Duration::ZERO;
    };
    let Some(stagger) = scope.stagger else {
        return Duration::ZERO;
    };
    let now = incular_animation::current_frame_time();
    if scope.epoch.get() != now {
        scope.epoch.set(now);
        scope.next.set(0);
    }
    let index = scope.next.get();
    scope.next.set(index + 1);
    stagger.delay(index, scope.count.max(index + 1))
}

/// Wraps the motion's child in the scope it publishes.
pub(crate) fn scope_child(node: &MotionNode, widget: Widget) -> Widget {
    match node.child_scope.borrow().clone() {
        Some(scope) => Widget::environment_scope(scope, widget),
        None => widget,
    }
}

/// The outer wrapper of a motion site.
pub(crate) fn wrap(_spec: &Rc<Motion>, widget: Widget) -> Widget {
    widget
}

/// Published by a motion site with [`Motion::presence`] to the motion sites
/// it swaps, so `Presence::Wait` knows when the old child has left.
#[derive(Clone)]
pub(crate) struct PresenceScope(Option<Rc<PresenceShared>>);

pub(crate) struct PresenceShared {
    departing: Cell<usize>,
    parent: Weak<MotionNode>,
}

impl PresenceScope {
    /// The scope a presence child republishes, so deeper sites do not count.
    pub(crate) fn none() -> Self {
        Self(None)
    }

    pub(crate) fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// A child began its exit.
    pub(crate) fn departing(&self) {
        if let Some(shared) = &self.0 {
            shared.departing.set(shared.departing.get() + 1);
        }
    }

    /// A child finished or cancelled its exit.
    pub(crate) fn departed(&self) {
        if let Some(shared) = &self.0 {
            shared
                .departing
                .set(shared.departing.get().saturating_sub(1));
            if shared.departing.get() == 0
                && let Some(parent) = shared.parent.upgrade()
            {
                parent.request_rebuild();
            }
        }
    }
}

/// What a presence parent shows while a keyed child is replaced.
#[derive(Default)]
pub(crate) struct PresenceState {
    shared: Option<Rc<PresenceShared>>,
    /// The child currently shown (its key decides replacements).
    shown: Option<Widget>,
    /// A replacement is waiting for the old child to leave.
    waiting: bool,
}

/// Lays out the presence parent's child in a stack so a replaced keyed
/// child can exit beside (`Sync`) or before (`Wait`) its successor.
pub(crate) fn presence_child(
    node: &Rc<MotionNode>,
    state: &mut PresenceState,
    mode: Presence,
    child: Widget,
) -> Widget {
    let shared = state
        .shared
        .get_or_insert_with(|| {
            Rc::new(PresenceShared {
                departing: Cell::new(0),
                parent: Rc::downgrade(node),
            })
        })
        .clone();
    let children = match mode {
        Presence::Sync => vec![child],
        Presence::Wait => {
            let same = state
                .shown
                .as_ref()
                .is_none_or(|shown| shown.key() == child.key());
            if same {
                state.waiting = false;
                state.shown = Some(child.clone());
                vec![child]
            } else if !state.waiting {
                // The old child leaves this build; look again next frame,
                // once its exit (if any) has started.
                state.waiting = true;
                let weak = Rc::downgrade(node);
                incular_animation::schedule_frame_callback(move |_| {
                    if let Some(node) = weak.upgrade() {
                        node.request_rebuild();
                    }
                });
                Vec::new()
            } else if shared.departing.get() == 0 {
                state.waiting = false;
                state.shown = Some(child.clone());
                vec![child]
            } else {
                Vec::new()
            }
        }
    };
    Widget::environment_scope(
        PresenceScope(Some(shared)),
        crate::Stack::new(children).into(),
    )
}
