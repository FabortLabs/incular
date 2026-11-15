//! Flutter's `TransitionDelegate` (`widgets/navigator.dart`): how the page
//! routes of one declarative update enter and leave.
//!
//! [`Navigator::set_pages`](crate::Navigator::set_pages) describes the
//! update as a [`PageTransition`]: one [`PageRouteRecord`] per page of the
//! new history and one [`ExitingPageRoute`] per route the update removed.
//! Each record starts with [`DefaultTransitionDelegate`]'s decision, so a
//! delegate only changes the records it cares about. The decisions are read
//! by the presenter that animates the stack (the runtime's route outlet).

use crate::{PageKey, RouteId};

/// How a page route that a declarative update added enters the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageEntering {
    /// Flutter `markForPush`: the route animates in.
    Push,
    /// Flutter `markForAdd`: the route appears at once, fully shown.
    Add,
}

/// How a page route that a declarative update removed leaves the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageExiting {
    /// Flutter `markForPop`: the route animates out where it was.
    Pop,
    /// Flutter `markForComplete`: the route leaves at once.
    Complete,
    /// Flutter `markForRemove`: the route leaves at once. Incular routes
    /// carry no completion result, so this differs from
    /// [`Self::Complete`] only in what it records.
    Remove,
}

impl PageExiting {
    /// Whether the route animates out instead of leaving at once.
    #[must_use]
    pub const fn animates(self) -> bool {
        matches!(self, Self::Pop)
    }
}

/// One page route of the new history (Flutter `RouteTransitionRecord`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageRouteRecord {
    route: RouteId,
    name: String,
    key: Option<PageKey>,
    entering: Option<PageEntering>,
}

impl PageRouteRecord {
    /// The route's identity.
    #[must_use]
    pub fn route(&self) -> RouteId {
        self.route
    }

    /// The route's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The key of the page that produced the route.
    #[must_use]
    pub fn key(&self) -> Option<&PageKey> {
        self.key.as_ref()
    }

    /// Flutter `isWaitingForEnteringDecision`: whether the update added this
    /// route. Routes kept from the previous history have no decision.
    #[must_use]
    pub fn is_entering(&self) -> bool {
        self.entering.is_some()
    }

    /// The entering decision, for a route the update added.
    #[must_use]
    pub fn entering(&self) -> Option<PageEntering> {
        self.entering
    }

    /// Flutter `markForPush`. Returns `false`, changing nothing, for a route
    /// the update kept.
    pub fn mark_for_push(&mut self) -> bool {
        self.decide(PageEntering::Push)
    }

    /// Flutter `markForAdd`. Returns `false`, changing nothing, for a route
    /// the update kept.
    pub fn mark_for_add(&mut self) -> bool {
        self.decide(PageEntering::Add)
    }

    fn decide(&mut self, decision: PageEntering) -> bool {
        match &mut self.entering {
            Some(entering) => {
                *entering = decision;
                true
            }
            None => false,
        }
    }
}

/// A page route the update removed (Flutter's exiting
/// `RouteTransitionRecord`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitingPageRoute {
    route: RouteId,
    name: String,
    key: Option<PageKey>,
    location: Option<RouteId>,
    exiting: PageExiting,
}

impl ExitingPageRoute {
    /// The route's identity.
    #[must_use]
    pub fn route(&self) -> RouteId {
        self.route
    }

    /// The route's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The key of the page that produced the route.
    #[must_use]
    pub fn key(&self) -> Option<&PageKey> {
        self.key.as_ref()
    }

    /// Flutter's `locationToExitingPageRoute` key: the route of the new
    /// history that this route sat directly above, or `None` when it sat
    /// below every kept route. Several exiting routes can share a location;
    /// they keep their old order, bottom first.
    #[must_use]
    pub fn location(&self) -> Option<RouteId> {
        self.location
    }

    /// The exiting decision.
    #[must_use]
    pub fn exiting(&self) -> PageExiting {
        self.exiting
    }

    /// Flutter `markForPop`.
    pub fn mark_for_pop(&mut self) {
        self.exiting = PageExiting::Pop;
    }

    /// Flutter `markForComplete`.
    pub fn mark_for_complete(&mut self) {
        self.exiting = PageExiting::Complete;
    }

    /// Flutter `markForRemove`.
    pub fn mark_for_remove(&mut self) {
        self.exiting = PageExiting::Remove;
    }
}

/// One declarative update's route changes, as a delegate sees them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageTransition {
    history: Vec<PageRouteRecord>,
    exiting: Vec<ExitingPageRoute>,
}

impl PageTransition {
    /// Flutter `newPageRouteHistory`: the new stack, bottom first.
    #[must_use]
    pub fn history(&self) -> &[PageRouteRecord] {
        &self.history
    }

    /// Mutable access to the new stack's records.
    pub fn history_mut(&mut self) -> &mut [PageRouteRecord] {
        &mut self.history
    }

    /// The removed routes in their old order, bottom first.
    #[must_use]
    pub fn exiting(&self) -> &[ExitingPageRoute] {
        &self.exiting
    }

    /// Mutable access to the removed routes' records.
    pub fn exiting_mut(&mut self) -> &mut [ExitingPageRoute] {
        &mut self.exiting
    }

    pub(crate) fn new(
        history: impl IntoIterator<Item = (RouteId, String, Option<PageKey>, bool)>,
        exiting: impl IntoIterator<Item = (RouteId, String, Option<PageKey>, Option<RouteId>)>,
    ) -> Self {
        let mut transition = Self {
            history: history
                .into_iter()
                .map(|(route, name, key, added)| PageRouteRecord {
                    route,
                    name,
                    key,
                    entering: added.then_some(PageEntering::Add),
                })
                .collect(),
            exiting: exiting
                .into_iter()
                .map(|(route, name, key, location)| ExitingPageRoute {
                    route,
                    name,
                    key,
                    location,
                    exiting: PageExiting::Complete,
                })
                .collect(),
        };
        DefaultTransitionDelegate.resolve(&mut transition);
        transition
    }

    pub(crate) fn decisions(&self) -> PageTransitionDecisions {
        PageTransitionDecisions {
            entering: self
                .history
                .iter()
                .filter_map(|record| Some((record.route, record.entering?)))
                .collect(),
            exiting: self
                .exiting
                .iter()
                .map(|record| (record.route, (record.exiting, record.location)))
                .collect(),
        }
    }
}

/// Flutter's `TransitionDelegate`: decides how the routes of one declarative
/// update enter and leave. Records arrive with the default decisions.
pub trait TransitionDelegate {
    /// Changes the decisions of the records in `transition`.
    fn resolve(&self, transition: &mut PageTransition);
}

impl<F: Fn(&mut PageTransition)> TransitionDelegate for F {
    fn resolve(&self, transition: &mut PageTransition) {
        self(transition);
    }
}

/// Flutter's `DefaultTransitionDelegate`:
///
/// - the new top route pushes when the update added it; every other added
///   route is added without animation (a removed route sits above a kept
///   route, so it never covers an added top);
/// - the removed route that sat on top of the old stack, above the new top,
///   pops; every other removed route completes without animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefaultTransitionDelegate;

impl TransitionDelegate for DefaultTransitionDelegate {
    fn resolve(&self, transition: &mut PageTransition) {
        let top = transition.history.last().map(|record| record.route);
        for record in &mut transition.exiting {
            record.exiting = PageExiting::Complete;
        }
        // Flutter `handleExitingRoute(.., isLast)`: only the last route of
        // the exiting chain at the new top's location pops.
        if let Some(last) = transition
            .exiting
            .iter_mut()
            .rev()
            .find(|record| record.location == top)
        {
            last.exiting = PageExiting::Pop;
        }
        let count = transition.history.len();
        for (index, record) in transition.history.iter_mut().enumerate() {
            if record.entering.is_some() {
                record.entering = Some(if index + 1 == count {
                    PageEntering::Push
                } else {
                    PageEntering::Add
                });
            }
        }
    }
}

/// The decisions of the declarative updates a presenter has not read yet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageTransitionDecisions {
    entering: std::collections::HashMap<RouteId, PageEntering>,
    exiting: std::collections::HashMap<RouteId, (PageExiting, Option<RouteId>)>,
}

impl PageTransitionDecisions {
    /// How `route` enters, when a declarative update added it.
    #[must_use]
    pub fn entering(&self, route: RouteId) -> Option<PageEntering> {
        self.entering.get(&route).copied()
    }

    /// How `route` leaves and the location it leaves from, when a
    /// declarative update removed it.
    #[must_use]
    pub fn exiting(&self, route: RouteId) -> Option<(PageExiting, Option<RouteId>)> {
        self.exiting.get(&route).copied()
    }

    /// Whether no decision is pending.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entering.is_empty() && self.exiting.is_empty()
    }

    /// Merges a later update's decisions over these. Entering decisions of
    /// routes no longer in `live` are dropped.
    pub(crate) fn merge(&mut self, later: Self, live: &std::collections::HashSet<RouteId>) {
        self.entering.extend(later.entering);
        self.entering.retain(|route, _| live.contains(route));
        self.exiting.extend(later.exiting);
        // A route removed by one update and re-added later is live again.
        self.exiting.retain(|route, _| !live.contains(route));
    }
}
