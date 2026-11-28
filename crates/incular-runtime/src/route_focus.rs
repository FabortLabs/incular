//! Per-route focus save/restore policy over navigation lifetimes.
//!
//! The state is deliberately pull-driven: it owns the save records and the
//! restore validation, while the mount integration drives it — save the
//! outgoing route on deactivation, restore the incoming route on
//! reactivation, and drop records through [`RouteFocusState::forget`] or
//! [`RouteFocusState::retain_mounted`] on permanent removal. There is no
//! background watcher and no second focus engine; restoration funnels
//! through the runtime's own focus slot.

use std::{collections::HashMap, rc::Rc};

use incular_navigation::{Navigator, RouteId};
use incular_widgets::internal::{ElementId, WidgetTree};

use super::frame::Runtime;

/// Maps a mounted element to the route that owns it.
///
/// The mount integration supplies this attribution from mounted structure
/// (keys, scope containment); the state never guesses ownership, so an
/// element without an owner is never restored. Route identities are
/// navigator-scoped, so use one state per [`Navigator`].
pub type RouteFocusOwner = Rc<dyn Fn(&WidgetTree, ElementId) -> Option<RouteId>>;

/// Remembers which element was focused per route and restores it on return.
///
/// Mount autofocus fires once per window, so route activation is what
/// focuses incoming autofocus targets — never a background scan. Restore
/// is otherwise fail-closed, with three record states and three restore
/// behaviors:
/// - No saved information (never saved, forgotten, or saved with nothing
///   owned): restore leaves focus untouched, except for genuinely orphaned
///   focus (see below).
/// - Previously saved focus that is no longer eligible: the deterministic
///   fallback clears focus, but only when current focus is itself orphaned —
///   an invalid saved record never steals unrelated focus.
/// - Focus currently owned by another route: saves reject it (a route
///   records only its own focus) and restores leave it alone while its
///   owner stays mounted.
/// - No usable record at all: a vacant stage (no focus, or only orphaned
///   focus) takes the route's own autofocus target when it names one;
///   valid focus is never displaced for a missing record.
///
/// Restoration uses the same eligibility as a direct request, through
/// [`WidgetTree::can_request_focus`]. Detached, disabled or excluded
/// targets cannot receive focus, but a node omitted from Tab traversal
/// can still receive it. Ownership comes from the [`RouteFocusOwner`].
///
/// Route identity and element identity stay separate throughout: records
/// are keyed by [`RouteId`], targets are [`ElementId`] values validated
/// against the live tree on every restore, never by index or name.
///
/// Dropping the state forgets all records; later restores are impossible,
/// which is the teardown policy.
pub struct RouteFocusState {
    saved: HashMap<RouteId, Option<ElementId>>,
    owner_of: RouteFocusOwner,
}

impl RouteFocusState {
    /// Creates focus state attributing elements through `owner_of`.
    pub fn new(owner_of: impl Fn(&WidgetTree, ElementId) -> Option<RouteId> + 'static) -> Self {
        Self {
            saved: HashMap::new(),
            owner_of: Rc::new(owner_of),
        }
    }

    pub(crate) fn has_saved_target(&self, route: RouteId) -> bool {
        matches!(self.saved.get(&route), Some(Some(_)))
    }

    /// Records the currently focused element as `route`'s focus. Call when
    /// `route` deactivates. Only focus owned by `route` is recorded: focus
    /// owned elsewhere (or by nothing) records no information rather than
    /// a foreign snapshot, so retained records never lie about ownership.
    /// Absent and rejected records restore identically (orphan sweep), so
    /// validation is record hygiene, not a second restore predicate.
    pub fn save_focused(&mut self, runtime: &Runtime, route: RouteId) {
        let focused = self.read_owned_focus(runtime, route);
        self.saved.insert(route, focused);
    }

    /// Reads the currently focused element when owned by `route`, without
    /// touching records. Transactional drivers capture through this and
    /// commit through [`Self::commit_saved`], so a failed attempt never
    /// disturbs committed state.
    pub(crate) fn read_owned_focus(&self, runtime: &Runtime, route: RouteId) -> Option<ElementId> {
        runtime.focused_element().filter(|id| {
            runtime.tree().element_exists(*id)
                && (self.owner_of)(runtime.tree(), *id) == Some(route)
        })
    }

    /// Reports the route that owns the actual live primary focus.
    pub(crate) fn current_owner(&self, runtime: &Runtime) -> Option<RouteId> {
        runtime
            .focused_element()
            .and_then(|id| (self.owner_of)(runtime.tree(), id))
    }

    /// Commits a previously captured save into `route`'s record.
    pub(crate) fn commit_saved(&mut self, route: RouteId, saved: Option<ElementId>) {
        self.saved.insert(route, saved);
    }

    /// Restores `route`'s saved focus into `runtime`. Call when `route`
    /// reactivates. A removed `route` restores nothing even with a leftover
    /// record, so cleanup never depends on the host having forgotten first.
    pub fn restore_saved(&mut self, runtime: &mut Runtime, navigator: &Navigator, route: RouteId) {
        if navigator.lifetime_of(route).is_none() {
            return;
        }
        if let Some(saved) = self.saved.get(&route).copied().flatten()
            && Self::eligible(runtime, &self.owner_of, route, saved)
        {
            if runtime.focused_element() != Some(saved) {
                runtime.set_focus(Some(saved));
            }
            return;
        }
        // No usable record. Fresh starts focus the route's own autofocus
        // target, but only onto a vacant stage — valid focus is never
        // displaced for a missing record.
        let current = runtime.focused_element();
        let vacant =
            current.is_none_or(|id| Self::orphaned(runtime, navigator, &self.owner_of, id));
        if vacant {
            let tree = runtime.tree();
            // During activation the previous route may still be mounted
            // and focusable until the committed frame releases its staged
            // focus. Filter ownership before choosing the first candidate.
            if let Some(auto) =
                tree.autofocus_element_where(|id| (self.owner_of)(tree, id) == Some(route))
            {
                runtime.set_focus(Some(auto));
                return;
            }
        }
        if let Some(current) = current
            && Self::orphaned(runtime, navigator, &self.owner_of, current)
        {
            runtime.set_focus(None);
        }
    }

    /// Eligibility is the tree's own rule plus attribution: live, focusable
    /// (hence enabled and in an active subtree), and owned by `route`.
    fn eligible(
        runtime: &Runtime,
        owner_of: &RouteFocusOwner,
        route: RouteId,
        id: ElementId,
    ) -> bool {
        runtime.tree().can_request_focus(id) && owner_of(runtime.tree(), id) == Some(route)
    }

    /// Orphaned focus has no live owner to keep it: a dead or unfocusable
    /// target, or a target whose owning route is no longer mounted. Focus
    /// owned by a mounted route — or by nothing attributable — is left
    /// alone.
    fn orphaned(
        runtime: &Runtime,
        navigator: &Navigator,
        owner_of: &RouteFocusOwner,
        id: ElementId,
    ) -> bool {
        !runtime.tree().can_request_focus(id)
            || owner_of(runtime.tree(), id)
                .is_some_and(|owner| navigator.lifetime_of(owner).is_none())
    }

    /// Restores `route` only when no valid focus would be displaced: the
    /// record exists and current focus is absent or itself orphaned. This
    /// is how deferred content (mounted or enabled through ordinary frame
    /// invalidation after the transition) still restores without callers
    /// retrying arbitrarily — retries never yank live focus.
    pub(crate) fn restore_if_vacant(
        &mut self,
        runtime: &mut Runtime,
        navigator: &Navigator,
        route: RouteId,
    ) {
        if !matches!(self.saved.get(&route), Some(Some(_))) {
            return;
        }
        let vacant = runtime
            .focused_element()
            .is_none_or(|current| Self::orphaned(runtime, navigator, &self.owner_of, current));
        if vacant {
            self.restore_saved(runtime, navigator, route);
        }
    }

    /// The previous route became current again before any transition
    /// committed. Its captured focus belongs to this return, not to the
    /// persistent per-route history. Do not displace valid newer focus.
    pub(crate) fn restore_capture_if_vacant(
        &self,
        runtime: &mut Runtime,
        navigator: &Navigator,
        route: RouteId,
        captured: ElementId,
    ) {
        if navigator.lifetime_of(route).is_some()
            && runtime
                .focused_element()
                .is_none_or(|current| Self::orphaned(runtime, navigator, &self.owner_of, current))
            && Self::eligible(runtime, &self.owner_of, route, captured)
        {
            runtime.set_focus(Some(captured));
        }
    }

    /// Drops `route`'s record on permanent removal. A removed route never
    /// restores: without this call a stale record would outlive its route.
    pub fn forget(&mut self, route: RouteId) {
        self.saved.remove(&route);
    }

    /// Drops records for routes no longer mounted in `navigator`. Call
    /// after declarative reconciliation, which can remove routes without
    /// an explicit per-route removal call.
    pub fn retain_mounted(&mut self, navigator: &Navigator) {
        self.saved
            .retain(|route, _| navigator.lifetime_of(*route).is_some());
    }
}
