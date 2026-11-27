use std::cell::{Cell, OnceCell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

use incular_core::{Offset, Rect, Size, Transform};

use crate::details::PointerDeviceKind;

struct ScopeAutofocusRequest {
    scope: Weak<FocusScopeState>,
    target: Weak<FocusState>,
}

thread_local! {
    static FOCUS_EPOCH: Cell<u64> = const { Cell::new(0) };
    static FOCUS_ATTACHMENT_SEQUENCE: Cell<u64> = const { Cell::new(0) };
    static SCOPED_AUTOFOCUS: RefCell<Vec<ScopeAutofocusRequest>> = const { RefCell::new(Vec::new()) };
}

/// Drains requests selected by a retained host, preserving queue order.
/// Other windows and requests waiting for attachment keep their entries.
#[doc(hidden)]
pub fn take_hosted_scope_autofocus(
    mut take: impl FnMut(&FocusScopeNode, &FocusNode) -> bool,
) -> Vec<(FocusScopeNode, FocusNode)> {
    SCOPED_AUTOFOCUS.with(|pending| {
        let mut pending = pending.borrow_mut();
        let mut owned = Vec::new();
        pending.retain(|request| {
            let (Some(scope), Some(target)) = (request.scope.upgrade(), request.target.upgrade())
            else {
                return false;
            };
            let scope = FocusScopeNode { state: scope };
            if scope.mounted_anchor().is_none() {
                return false;
            }
            let target = FocusNode { state: target };
            if !take(&scope, &target) {
                return true;
            }
            owned.push((scope, target));
            false
        });
        owned
    })
}

/// Counter bumped whenever any [`FocusNode`] on this thread gains or loses
/// focus.
///
/// Hosts compare it between frames to notice focus requested directly on a
/// node without scanning every focusable element.
#[must_use]
pub fn focus_epoch() -> u64 {
    FOCUS_EPOCH.get()
}

/// Determines where a mounted focus request lands after unfocusing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnfocusDisposition {
    /// Clears the enclosing scope's history and focuses the scope directly.
    #[default]
    Scope,
    /// Restores the most recent eligible child other than the one unfocused.
    PreviouslyFocusedChild,
}

/// Overrides owned by a regular mounted Focus widget. `None` preserves the
/// value already on its node.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FocusNodeWidgetProperties {
    pub can_request_focus: Option<bool>,
    pub skip_traversal: Option<bool>,
    pub descendants_are_focusable: Option<bool>,
    pub descendants_are_traversable: Option<bool>,
}

impl FocusNodeWidgetProperties {
    pub fn apply(self, node: &FocusNode) {
        if let Some(value) = self.can_request_focus {
            node.set_can_request_focus(value);
        }
        if let Some(value) = self.skip_traversal {
            node.set_skip_traversal(value);
        }
        if let Some(value) = self.descendants_are_focusable {
            node.set_descendants_are_focusable(value);
        }
        if let Some(value) = self.descendants_are_traversable {
            node.set_descendants_are_traversable(value);
        }
    }
}

/// Shared focus handle usable by controls that are rebuilt often.
///
/// For exclusive traversal use a [`FocusManager`]; calling
/// [`Self::request_focus`] directly is retained for standalone controls.
#[derive(Clone)]
pub struct FocusNode {
    state: Rc<FocusState>,
}
impl PartialEq for FocusNode {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}
impl Eq for FocusNode {}
impl std::fmt::Debug for FocusNode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FocusNode")
            .field("has_focus", &self.has_focus())
            .field("can_request_focus", &self.can_request_focus())
            .finish()
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusAuthority {
    Unattached,
    Standalone,
    Hosted,
}

/// Signed size of a focus rectangle. Unlike a layout [Size], the projected
/// corner-to-corner extents can be negative after a rotation or reflection.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FocusExtent {
    pub width: f32,
    pub height: f32,
}

/// The transformed top-left corner and signed extent of a focus node.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FocusRect {
    pub origin: Offset,
    pub size: FocusExtent,
}

impl From<Rect> for FocusRect {
    fn from(rect: Rect) -> Self {
        Self {
            origin: rect.origin,
            size: FocusExtent {
                width: rect.size.width,
                height: rect.size.height,
            },
        }
    }
}

#[derive(Clone, Copy, Default)]
struct FocusGeometry {
    oriented: FocusRect,
    traversal: Rect,
}

type FocusHandler<E> = Rc<dyn Fn(&FocusNode, E) -> crate::KeyEventResult>;
type WeakFocusHandler<E> = Weak<dyn Fn(&FocusNode, E) -> crate::KeyEventResult>;

enum NodeKeyEventHandler<E> {
    External(FocusHandler<E>),
    Mounted(WeakFocusHandler<E>),
}

impl<E> NodeKeyEventHandler<E> {
    fn get(&self) -> Option<FocusHandler<E>> {
        match self {
            Self::External(handler) => Some(handler.clone()),
            Self::Mounted(handler) => handler.upgrade(),
        }
    }
}

struct FocusState {
    scope_owner: Option<Weak<FocusScopeState>>,
    host_enclosing_scope: RefCell<Option<Weak<FocusScopeState>>>,
    host_parent: RefCell<Option<Weak<FocusState>>>,
    host_children: RefCell<Vec<Weak<FocusState>>>,
    focused: Cell<bool>,
    descendant_focused: Cell<bool>,
    requested: Cell<Option<u64>>,
    authority: Cell<FocusAuthority>,
    host_attachment_count: Cell<usize>,
    schedule_host_frame: Cell<Option<fn()>>,
    enabled: Cell<bool>,
    can_request_focus: Cell<bool>,
    skip_traversal: Cell<bool>,
    descendants_are_focusable: Cell<bool>,
    descendants_are_traversable: Cell<bool>,
    key_event_handler: RefCell<Option<NodeKeyEventHandler<incular_core::KeyboardEvent>>>,
    raw_key_handler: RefCell<Option<NodeKeyEventHandler<crate::RawKeyEvent>>>,
    geometry: Cell<FocusGeometry>,
    traversal_order: std::cell::RefCell<Option<FocusOrder>>,
    changes: incular_core::reactivity::DependencySource,
    revision: Cell<u64>,
}

/// Owns one live subscription to a [`FocusNode`] change stream.
///
/// The retained focus tree keeps subscriptions weakly in the node, so a
/// widget can be replaced or unmounted without leaving a callback attached to
/// an application-owned node.
pub struct FocusNodeSubscription {
    _subscription: incular_core::reactivity::Subscription,
}

/// Owns a mounted runtime's connection to an external focus node.
#[doc(hidden)]
pub struct FocusNodeAttachment {
    node: FocusNode,
    mount_sequence: u64,
}

impl FocusNodeAttachment {
    #[must_use]
    pub const fn node(&self) -> &FocusNode {
        &self.node
    }

    #[must_use]
    pub const fn mount_sequence(&self) -> u64 {
        self.mount_sequence
    }
}

impl Drop for FocusNodeAttachment {
    fn drop(&mut self) {
        let state = &self.node.state;
        let remaining = state.host_attachment_count.get() - 1;
        state.host_attachment_count.set(remaining);
        if remaining == 0 {
            if let Some(scope) = self.node.scope_owner() {
                let retired = Rc::downgrade(&scope.state);
                let _ = SCOPED_AUTOFOCUS.try_with(|pending| {
                    pending
                        .borrow_mut()
                        .retain(|request| !request.scope.ptr_eq(&retired));
                });
            }
            state.schedule_host_frame.set(None);
            self.node.set_host_enclosing_scope(None);
            self.node.set_host_parent(None);
            let children = std::mem::take(&mut *state.host_children.borrow_mut());
            for child in children.into_iter().filter_map(|state| state.upgrade()) {
                let child = FocusNode { state: child };
                if child.parent().as_ref() == Some(&self.node) {
                    child.set_host_parent(None);
                }
            }
            self.node.apply_host_focus(false);
        }
    }
}

impl FocusState {
    fn schedule_host_frame(&self) {
        if let Some(schedule) = self.schedule_host_frame.get() {
            schedule();
        }
    }

    fn notify(&self) {
        self.revision.set(
            self.revision
                .get()
                .checked_add(1)
                .expect("revision exhausted"),
        );
        self.changes.notify();
    }

    fn set_focused_silent(&self, focused: bool) -> bool {
        let changed = self.focused.replace(focused) != focused;
        if changed {
            FOCUS_EPOCH.set(FOCUS_EPOCH.get().wrapping_add(1));
        }
        changed
    }

    fn set_focused(&self, focused: bool) {
        if self.set_focused_silent(focused) {
            self.notify();
        }
    }
}

impl Default for FocusNode {
    fn default() -> Self {
        Self::with_scope_owner(None)
    }
}

impl FocusNode {
    fn with_scope_owner(scope_owner: Option<Weak<FocusScopeState>>) -> Self {
        Self {
            state: Rc::new(FocusState {
                scope_owner,
                host_enclosing_scope: RefCell::new(None),
                host_parent: RefCell::new(None),
                host_children: RefCell::new(Vec::new()),
                focused: Cell::new(false),
                descendant_focused: Cell::new(false),
                requested: Cell::new(None),
                authority: Cell::new(FocusAuthority::Unattached),
                host_attachment_count: Cell::new(0),
                schedule_host_frame: Cell::new(None),
                enabled: Cell::new(true),
                can_request_focus: Cell::new(true),
                skip_traversal: Cell::new(false),
                descendants_are_focusable: Cell::new(true),
                descendants_are_traversable: Cell::new(true),
                key_event_handler: RefCell::new(None),
                raw_key_handler: RefCell::new(None),
                geometry: Cell::new(FocusGeometry::default()),
                traversal_order: std::cell::RefCell::new(None),
                changes: incular_core::reactivity::DependencySource::default(),
                revision: Cell::new(0),
            }),
        }
    }
}
impl FocusNode {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The currently installed typed keyboard callback.
    #[must_use]
    pub fn on_key_event(&self) -> Option<crate::FocusKeyEventHandler> {
        self.state.key_event_handler.borrow().as_ref()?.get()
    }

    /// The currently installed deprecated raw-key callback.
    #[must_use]
    pub fn on_key(&self) -> Option<crate::FocusRawKeyEventHandler> {
        self.state.raw_key_handler.borrow().as_ref()?.get()
    }

    /// Replaces the node-owned callback for the deprecated raw-key route.
    pub fn set_on_key(&self, handler: Option<crate::FocusRawKeyEventHandler>) {
        *self.state.raw_key_handler.borrow_mut() = handler.map(NodeKeyEventHandler::External);
    }

    /// Replaces the node-owned callback. A regular mounted Focus widget
    /// installs its own value when it next updates.
    pub fn set_on_key_event(&self, handler: Option<crate::FocusKeyEventHandler>) {
        *self.state.key_event_handler.borrow_mut() = handler.map(NodeKeyEventHandler::External);
    }

    /// Mirrors a mounted widget's callback without creating a cycle when
    /// that callback itself captures this node.
    #[doc(hidden)]
    pub fn install_mounted_key_handlers(
        &self,
        typed: Option<&crate::FocusKeyEventHandler>,
        raw: Option<&crate::FocusRawKeyEventHandler>,
    ) {
        *self.state.key_event_handler.borrow_mut() =
            typed.map(|handler| NodeKeyEventHandler::Mounted(Rc::downgrade(handler)));
        *self.state.raw_key_handler.borrow_mut() =
            raw.map(|handler| NodeKeyEventHandler::Mounted(Rc::downgrade(handler)));
    }

    /// Subscribes to focus and focus-property changes.
    #[must_use]
    pub fn observe(&self, callback: impl Fn() + 'static) -> FocusNodeSubscription {
        FocusNodeSubscription {
            _subscription: self.state.changes.subscribe((), callback),
        }
    }

    /// Monotonic state revision for retained consumers that poll focus state.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.state.revision.get()
    }

    /// Requests focus without granting it to a node that has not been attached.
    pub fn request_focus(&self) {
        if self.can_request_focus() {
            let request = FOCUS_EPOCH.get().wrapping_add(1);
            self.state.requested.set(Some(request));
            FOCUS_EPOCH.set(request);
            self.state.schedule_host_frame();
            if self.state.authority.get() == FocusAuthority::Standalone {
                self.state.set_focused(true);
            }
        }
    }
    pub fn unfocus(&self) {
        self.unfocus_with_disposition(UnfocusDisposition::Scope);
    }

    /// Clears a pending request or leaves the active focus path. In a
    /// mounted scope, the fallback is queued for the next focus update.
    pub fn unfocus_with_disposition(&self, disposition: UnfocusDisposition) {
        let request_was_pending = self.state.requested.take().is_some();
        let focus_was_held = self.state.focused.get();
        if !request_was_pending && !focus_was_held && !self.state.descendant_focused.get() {
            return;
        }

        if let Some(mut scope) = self.host_enclosing_scope() {
            if scope.can_request_focus() {
                match disposition {
                    UnfocusDisposition::Scope => scope.state.hosted_history.borrow_mut().clear(),
                    UnfocusDisposition::PreviouslyFocusedChild => scope.forget_hosted_child(self),
                }
            }
            let mut visited = Vec::new();
            loop {
                if visited.contains(&scope) {
                    break;
                }
                if scope.can_request_focus() {
                    match disposition {
                        UnfocusDisposition::Scope => scope.request_scope_focus(),
                        UnfocusDisposition::PreviouslyFocusedChild => {
                            let _ = scope.request_focus_in_scope();
                        }
                    }
                    return;
                }
                let anchor = scope.scope_focus_node();
                let Some(enclosing) = anchor.host_enclosing_scope() else {
                    break;
                };
                if disposition == UnfocusDisposition::PreviouslyFocusedChild {
                    enclosing.forget_hosted_child(&anchor);
                }
                visited.push(scope);
                scope = enclosing;
            }
        }

        self.state.set_focused(false);
        if request_was_pending || focus_was_held {
            self.state.schedule_host_frame();
        }
    }

    /// Whether application code asked this node for focus through
    /// [`Self::request_focus`] and the host has not yet applied it with
    /// [`Self::apply_host_focus`].
    #[must_use]
    pub fn has_pending_focus_request(&self) -> bool {
        self.state.requested.get().is_some()
    }

    /// Registers this node with a runtime that settles requests at its next
    /// input or frame boundary. A detached node cannot grant itself focus.
    #[doc(hidden)]
    #[must_use]
    pub fn attach_to_focus_host(&self, schedule_frame: fn()) -> FocusNodeAttachment {
        let count = self.state.host_attachment_count.get();
        self.state.host_attachment_count.set(count + 1);
        if count == 0 {
            self.state.schedule_host_frame.set(Some(schedule_frame));
            self.state.authority.set(FocusAuthority::Hosted);
            self.state.set_focused(false);
            if self.has_pending_focus_request() {
                self.state.schedule_host_frame();
                FOCUS_EPOCH.set(FOCUS_EPOCH.get().wrapping_add(1));
            }
        }
        let mount_sequence = FOCUS_ATTACHMENT_SEQUENCE.with(|sequence| {
            let next = sequence.get().wrapping_add(1);
            sequence.set(next);
            next
        });
        FocusNodeAttachment { node: self.clone(), mount_sequence }
    }

    /// The parent of this node in its mounted focus tree, which may differ
    /// from its widget ancestry. Detached nodes have no mounted parent.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        self.state
            .host_parent
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .map(|state| Self { state })
    }

    /// Mounted focus ancestors, nearest first. The node itself is excluded.
    #[must_use]
    pub fn ancestors(&self) -> Vec<Self> {
        self.ancestor_iter().collect()
    }

    fn ancestor_iter(&self) -> impl Iterator<Item = Self> {
        std::iter::successors(self.parent(), Self::parent)
    }

    /// Immediate mounted focus children in attachment order, irrespective
    /// of eligibility or traversal settings.
    #[must_use]
    pub fn children(&self) -> Vec<Self> {
        self.state
            .host_children
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .map(|state| Self { state })
            .collect()
    }

    /// All mounted focus descendants in depth-first postorder. A child's
    /// own descendants precede that child, and this node is not included.
    #[must_use]
    pub fn descendants(&self) -> Vec<Self> {
        let mut result = Vec::new();
        let mut pending = Vec::new();
        self.push_children(&mut pending);
        while let Some((node, visited)) = pending.pop() {
            if visited {
                result.push(node);
            } else {
                pending.push((node.clone(), true));
                node.push_children(&mut pending);
            }
        }
        result
    }

    fn push_children(&self, pending: &mut Vec<(Self, bool)>) {
        pending.extend(
            self.state
                .host_children
                .borrow()
                .iter()
                .rev()
                .filter_map(Weak::upgrade)
                .map(|state| (Self { state }, false)),
        );
    }

    #[doc(hidden)]
    pub fn set_host_parent(&self, parent: Option<&Self>) {
        debug_assert_ne!(parent, Some(self), "a focus node cannot parent itself");
        let previous = self.parent();
        if previous.as_ref() == parent {
            return;
        }
        // The host resolves element cycles first. Also reject a cycle
        // involving separate elements that alias the same FocusNode.
        if parent.is_some_and(|parent| parent == self || parent.ancestor_iter().any(|ancestor| ancestor == *self)) {
            return;
        }
        let weak_self = Rc::downgrade(&self.state);
        if let Some(previous) = previous {
            previous
                .state
                .host_children
                .borrow_mut()
                .retain(|node| !node.ptr_eq(&weak_self));
        }
        *self.state.host_parent.borrow_mut() = parent.map(|node| Rc::downgrade(&node.state));
        if let Some(parent) = parent {
            parent.state.host_children.borrow_mut().push(weak_self);
        }
    }

    #[doc(hidden)]
    pub fn set_host_enclosing_scope(&self, scope: Option<&FocusScopeNode>) {
        let previous = self.host_enclosing_scope();
        if previous.as_ref() == scope {
            return;
        }
        *self.state.host_enclosing_scope.borrow_mut() =
            scope.map(|scope| Rc::downgrade(&scope.state));
        if let Some(previous) = previous {
            previous.forget_hosted_child(self);
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn host_enclosing_scope(&self) -> Option<FocusScopeNode> {
        self.state
            .host_enclosing_scope
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .map(|state| FocusScopeNode { state })
    }

    #[doc(hidden)]
    #[must_use]
    pub fn scope_owner(&self) -> Option<FocusScopeNode> {
        self.state
            .scope_owner
            .as_ref()
            .and_then(Weak::upgrade)
            .map(|state| FocusScopeNode { state })
    }

    /// Enables the synchronous focus behavior of a standalone widget tree.
    /// A runtime with a live attachment remains authoritative.
    #[doc(hidden)]
    pub fn bind_standalone_tree(&self) {
        if self.state.host_attachment_count.get() == 0
            && self.state.authority.replace(FocusAuthority::Standalone)
                != FocusAuthority::Standalone
            && self.has_pending_focus_request()
            && self.can_request_focus()
        {
            self.state.set_focused(true);
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn focus_request_order(&self) -> Option<u64> {
        self.state.requested.get()
    }

    #[doc(hidden)]
    pub fn clear_pending_focus_request(&self) {
        self.state.requested.set(None);
    }

    /// Records the focus a host (the runtime) granted or removed. Unlike
    /// [`Self::request_focus`] it settles any pending request instead of
    /// creating one, so another host sharing the node does not adopt it.
    pub fn apply_host_focus(&self, focused: bool) {
        self.clear_pending_focus_request();
        if self.set_host_focus_state_silent(focused, false) {
            self.notify_host_focus_state();
        }
    }

    /// Updates both focus roles without notifying while the host updates
    /// the other nodes on the path to primary focus.
    #[doc(hidden)]
    pub fn set_host_focus_state_silent(&self, primary: bool, descendant: bool) -> bool {
        if primary {
            self.clear_pending_focus_request();
        }
        let primary_changed = self.state.set_focused_silent(primary);
        let descendant_changed = self.state.descendant_focused.replace(descendant) != descendant;
        primary_changed || descendant_changed
    }

    #[doc(hidden)]
    pub fn notify_host_focus_state(&self) {
        self.state.notify();
    }
    #[must_use]
    pub fn has_focus(&self) -> bool {
        self.state.changes.track();
        self.state.focused.get() || self.state.descendant_focused.get()
    }
    #[must_use]
    pub fn has_primary_focus(&self) -> bool {
        self.state.changes.track();
        self.state.focused.get()
    }
    /// Excludes or includes this node in manager-driven traversal.
    pub fn set_can_request_focus(&self, can_request_focus: bool) {
        if self.state.can_request_focus.replace(can_request_focus) != can_request_focus {
            if !can_request_focus {
                self.unfocus_with_disposition(UnfocusDisposition::PreviouslyFocusedChild);
            }
            self.state.notify();
        }
    }
    fn locally_can_request_focus(&self) -> bool {
        self.state.changes.track();
        self.state.enabled.get() && self.state.can_request_focus.get()
    }

    #[must_use]
    pub fn can_request_focus(&self) -> bool {
        self.locally_can_request_focus()
            && self.ancestor_iter().all(|ancestor| ancestor.descendants_are_focusable())
    }

    /// Enables or disables this hosted focus node without overwriting its
    /// application-controlled `can_request_focus` value.
    pub fn set_enabled(&self, enabled: bool) {
        if self.state.enabled.replace(enabled) != enabled {
            if !enabled {
                self.unfocus();
            }
            self.state.notify();
        }
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.state.enabled.get()
    }
    pub fn set_skip_traversal(&self, skip: bool) {
        if self.state.skip_traversal.replace(skip) != skip {
            self.state.notify();
        }
    }
    #[must_use]
    pub fn skip_traversal(&self) -> bool {
        self.state.skip_traversal.get()
            || self.ancestor_iter().any(|ancestor| !ancestor.descendants_are_traversable())
    }
    pub fn set_descendants_are_focusable(&self, focusable: bool) {
        if self.state.descendants_are_focusable.replace(focusable) != focusable {
            if !focusable && self.state.authority.get() == FocusAuthority::Hosted {
                // Publish the restriction before restoring scope history,
                // otherwise the previously focused leaf is still eligible.
                self.unfocus_with_disposition(UnfocusDisposition::PreviouslyFocusedChild);
            }
            self.state.schedule_host_frame();
            self.state.notify();
        }
    }
    #[must_use]
    pub fn descendants_are_focusable(&self) -> bool {
        self.state.descendants_are_focusable.get()
            && (self.state.scope_owner.is_none() || self.locally_can_request_focus())
    }
    pub fn set_descendants_are_traversable(&self, traversable: bool) {
        if self.state.descendants_are_traversable.replace(traversable) != traversable {
            self.state.notify();
        }
    }
    #[must_use]
    pub fn descendants_are_traversable(&self) -> bool {
        self.state.descendants_are_traversable.get()
    }

    /// Supplies the node's current layout bounds to geometry-aware traversal.
    /// Bounds are logical pixels in the same coordinate space as the widget
    /// tree. A node that never receives bounds remains in registration order.
    pub fn set_rect(&self, rect: Rect) {
        self.state.geometry.set(FocusGeometry {
            oriented: rect.into(),
            traversal: rect,
        });
    }

    /// Publishes the mounted semantic rectangle in its current paint space.
    #[doc(hidden)]
    pub fn set_mounted_geometry(&self, size: Size, local_to_world: Transform) {
        let corner = local_to_world.transform_point(Offset::ZERO);
        let size_in_world = local_to_world.transform_vector(Offset::new(size.width, size.height));
        self.state.geometry.set(FocusGeometry {
            oriented: FocusRect {
                origin: corner,
                size: FocusExtent {
                    width: size_in_world.x,
                    height: size_in_world.y,
                },
            },
            traversal: local_to_world.transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, size)),
        });
    }

    /// Opposite transformed corners, in their original order. Rotation or
    /// reflection can give this rectangle a negative width or height.
    #[must_use]
    pub fn rect(&self) -> FocusRect {
        self.state.geometry.get().oriented
    }

    #[must_use]
    pub fn offset(&self) -> Offset {
        self.rect().origin
    }

    #[must_use]
    pub fn size(&self) -> FocusExtent {
        self.rect().size
    }

    /// Axis-aligned paint bounds for layout-independent hit and traversal
    /// helpers. This retains the previous bounded geometry of `rect()`.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        self.state.geometry.get().traversal
    }

    /// Sets the explicit numeric order used by [`OrderedTraversalPolicy`].
    /// Equal or absent orders preserve widget registration order.
    pub fn set_traversal_order(&self, order: Option<f64>) {
        self.set_focus_order(order.map(FocusOrder::Numeric));
    }

    /// The numeric order, when the node has a [`FocusOrder::Numeric`] one.
    #[must_use]
    pub fn traversal_order(&self) -> Option<f64> {
        match *self.state.traversal_order.borrow() {
            Some(FocusOrder::Numeric(order)) => Some(order),
            _ => None,
        }
    }

    /// Sets the order [`OrderedTraversalPolicy`] sorts by, numeric or
    /// lexical. A non-finite numeric order clears it.
    pub fn set_focus_order(&self, order: Option<FocusOrder>) {
        *self.state.traversal_order.borrow_mut() = order.filter(FocusOrder::is_valid);
    }

    #[must_use]
    pub fn focus_order(&self) -> Option<FocusOrder> {
        self.state.traversal_order.borrow().clone()
    }
}

/// The modality used to decide whether focus and hover highlights are
/// visible. Keyboard navigation is traditional; touch interaction is touch
/// mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FocusHighlightMode {
    Touch,
    #[default]
    Traditional,
}

/// Controls how the retained focus system selects its highlight mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FocusHighlightStrategy {
    #[default]
    Automatic,
    AlwaysTouch,
    AlwaysTraditional,
}

struct HighlightModeState {
    strategy: FocusHighlightStrategy,
    last_input_mode: Option<FocusHighlightMode>,
    platform_default: FocusHighlightMode,
    initial_mouse_connected: bool,
    mode: Option<FocusHighlightMode>,
    observers: Vec<Weak<HighlightModeObserverEntry>>,
    pending: VecDeque<HighlightNotification>,
    dispatching: bool,
}

struct HighlightNotification {
    mode: FocusHighlightMode,
    observers: Vec<Weak<HighlightModeObserverEntry>>,
}

impl HighlightModeState {
    fn mode(&self) -> FocusHighlightMode {
        self.mode.unwrap_or(if self.initial_mouse_connected {
            FocusHighlightMode::Traditional
        } else {
            self.platform_default
        })
    }

    fn commit_mode(&mut self, mode: FocusHighlightMode) {
        let previous = self.mode();
        self.mode = Some(mode);
        if previous == mode {
            return;
        }
        self.observers
            .retain(|observer| observer.strong_count() != 0);
        self.pending.push_back(HighlightNotification {
            mode,
            observers: self.observers.clone(),
        });
    }
}

struct HighlightModeObserverEntry {
    active: Cell<bool>,
    callback: Rc<dyn Fn(FocusHighlightMode)>,
}

/// Owns the process-local focus-highlight modality used by retained widgets.
///
/// The object is a cheap handle to the current UI thread's modality state.
/// This mirrors Flutter's process-local `FocusManager` behavior while keeping
/// the native platform event loop outside the gestures crate.
///
/// Changes commit before callbacks run, with no state borrow held during
/// delivery. A callback may query or change this manager: nested changes commit
/// immediately, but their notifications follow the current event in commit
/// order. An event describes its transition; [`Self::mode`] always returns the
/// latest committed state, which another callback may already have changed.
/// Application-triggered sequences must converge.
///
/// A callback panic propagates without reverting committed state. Unfinished
/// notifications are discarded, and later changes can dispatch normally.
#[derive(Clone, Copy, Debug, Default)]
pub struct FocusHighlightManager;

pub struct FocusHighlightSubscription {
    entry: Rc<HighlightModeObserverEntry>,
}

impl Drop for FocusHighlightSubscription {
    fn drop(&mut self) {
        self.entry.active.set(false);
    }
}

thread_local! {
    static HIGHLIGHT_MODE: RefCell<HighlightModeState> = const { RefCell::new(HighlightModeState {
            strategy: FocusHighlightStrategy::Automatic,
            last_input_mode: None,
            platform_default: if cfg!(any(target_os = "android", target_os = "ios", target_os = "fuchsia")) {
                FocusHighlightMode::Touch
            } else {
                FocusHighlightMode::Traditional
            },
            initial_mouse_connected: false,
            mode: None,
            observers: Vec::new(),
            pending: VecDeque::new(),
            dispatching: false,
        }) };
}

/// Releases dispatch ownership on return or unwind. Pending snapshots contain
/// weak registrations, so aborting delivery cannot extend a callback's lifetime.
struct HighlightDispatchGuard;

impl Drop for HighlightDispatchGuard {
    fn drop(&mut self) {
        let abandoned = HIGHLIGHT_MODE.with(|state| {
            let mut state = state.borrow_mut();
            state.dispatching = false;
            std::mem::take(&mut state.pending)
        });
        drop(abandoned);
    }
}

impl FocusHighlightManager {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn mode(self) -> FocusHighlightMode {
        HIGHLIGHT_MODE.with(|state| state.borrow().mode())
    }

    #[must_use]
    pub fn strategy(self) -> FocusHighlightStrategy {
        HIGHLIGHT_MODE.with(|state| state.borrow().strategy)
    }

    pub fn set_strategy(self, strategy: FocusHighlightStrategy) {
        Self::update(|state| {
            if state.strategy == strategy {
                return;
            }
            state.strategy = strategy;
            let mode = match strategy {
                FocusHighlightStrategy::Automatic => {
                    let Some(mode) = state.last_input_mode else {
                        return;
                    };
                    mode
                }
                FocusHighlightStrategy::AlwaysTouch => FocusHighlightMode::Touch,
                FocusHighlightStrategy::AlwaysTraditional => FocusHighlightMode::Traditional,
            };
            state.commit_mode(mode);
        });
    }

    /// Forces a mode, primarily for deterministic embedders and tests. Future
    /// automatic input updates may change it again.
    pub fn set_mode(self, mode: FocusHighlightMode) {
        Self::update(|state| state.commit_mode(mode));
    }

    /// Sets the guess used before a concrete interaction on the UI thread.
    /// The fallback is queried, not emitted as an input transition.
    pub fn set_platform_default_mode(self, mode: FocusHighlightMode) {
        HIGHLIGHT_MODE.with(|state| state.borrow_mut().platform_default = mode);
    }

    /// Records whether an external mouse was detected before the first
    /// interaction. Mouse presence affects only the platform's initial guess.
    pub fn set_initial_mouse_connected(self, connected: bool) {
        HIGHLIGHT_MODE.with(|state| {
            let mut state = state.borrow_mut();
            if state.mode.is_none() {
                state.initial_mouse_connected = connected;
            }
        });
    }

    /// Records keyboard input, which selects traditional highlights in
    /// automatic mode.
    pub fn note_keyboard_input(self) {
        Self::update(|state| {
            if state.last_input_mode != Some(FocusHighlightMode::Traditional) {
                state.last_input_mode = Some(FocusHighlightMode::Traditional);
                if state.strategy == FocusHighlightStrategy::Automatic {
                    state.commit_mode(FocusHighlightMode::Traditional);
                }
            }
        });
    }

    /// Records pointer input using Flutter's touch/stylus versus traditional
    /// modality split.
    pub fn note_pointer_input(self, kind: PointerDeviceKind) {
        let requires_touch_highlights = matches!(
            kind,
            PointerDeviceKind::Touch
                | PointerDeviceKind::Stylus
                | PointerDeviceKind::InvertedStylus
        );
        if !requires_touch_highlights {
            return;
        }
        Self::update(|state| {
            if state.last_input_mode != Some(FocusHighlightMode::Touch) {
                state.last_input_mode = Some(FocusHighlightMode::Touch);
                if state.strategy == FocusHighlightStrategy::Automatic {
                    state.commit_mode(FocusHighlightMode::Touch);
                }
            }
        });
    }

    /// Observes committed mode transitions in registration order. Dropping the
    /// token cancels delivery, including an event already queued. Registration
    /// during a callback starts with the next subsequently committed change.
    #[must_use]
    pub fn observe(
        self,
        callback: impl Fn(FocusHighlightMode) + 'static,
    ) -> FocusHighlightSubscription {
        let entry = Rc::new(HighlightModeObserverEntry {
            active: Cell::new(true),
            callback: Rc::new(callback),
        });
        HIGHLIGHT_MODE.with(|state| {
            let mut state = state.borrow_mut();
            state
                .observers
                .retain(|observer| observer.strong_count() != 0);
            state.observers.push(Rc::downgrade(&entry));
        });
        FocusHighlightSubscription { entry }
    }

    fn update(update: impl FnOnce(&mut HighlightModeState)) {
        HIGHLIGHT_MODE.with(|state| update(&mut state.borrow_mut()));
        let owns_dispatch = HIGHLIGHT_MODE.with(|state| {
            let mut state = state.borrow_mut();
            if state.dispatching || state.pending.is_empty() {
                return false;
            }
            state.dispatching = true;
            true
        });
        if !owns_dispatch {
            return;
        }

        let _dispatch = HighlightDispatchGuard;
        loop {
            let next = HIGHLIGHT_MODE.with(|state| state.borrow_mut().pending.pop_front());
            let Some(notification) = next else {
                break;
            };
            for observer in notification.observers {
                if let Some(observer) = observer.upgrade()
                    && observer.active.get()
                {
                    (observer.callback)(notification.mode);
                }
            }
        }
    }
}

/// Retained focus/hover state used by `FocusableActionDetector`.
pub struct FocusBehavior {
    node: FocusNode,
    enabled: Cell<bool>,
    focus_permitted: Cell<bool>,
    hovering: Cell<bool>,
    focused: Cell<bool>,
    can_show_highlight: Cell<bool>,
    focus_highlight_visible: Cell<bool>,
    hover_highlight_visible: Cell<bool>,
    on_focus_change: Option<Rc<dyn Fn(bool)>>,
    on_show_focus_highlight: Option<Rc<dyn Fn(bool)>>,
    on_show_hover_highlight: Option<Rc<dyn Fn(bool)>>,
    focus_subscription: RefCell<Option<FocusNodeSubscription>>,
    highlight_subscription: RefCell<Option<FocusHighlightSubscription>>,
}

impl FocusBehavior {
    #[must_use]
    pub fn new(
        node: FocusNode,
        enabled: bool,
        on_focus_change: Option<Rc<dyn Fn(bool)>>,
        on_show_focus_highlight: Option<Rc<dyn Fn(bool)>>,
        on_show_hover_highlight: Option<Rc<dyn Fn(bool)>>,
    ) -> Rc<Self> {
        node.set_enabled(enabled);
        Self::for_detector(
            node, enabled, on_focus_change, on_show_focus_highlight,
            on_show_hover_highlight,
        )
    }

    /// Leaves the node's permission untouched until the detector mounts
    /// and knows the active navigation mode.
    #[doc(hidden)]
    #[must_use]
    pub fn for_detector(
        node: FocusNode,
        enabled: bool,
        on_focus_change: Option<Rc<dyn Fn(bool)>>,
        on_show_focus_highlight: Option<Rc<dyn Fn(bool)>>,
        on_show_hover_highlight: Option<Rc<dyn Fn(bool)>>,
    ) -> Rc<Self> {
        let can_show_highlight =
            FocusHighlightManager::new().mode() == FocusHighlightMode::Traditional;
        let focused = node.has_focus();
        let behavior = Rc::new(Self {
            focused: Cell::new(focused),
            node,
            enabled: Cell::new(enabled),
            focus_permitted: Cell::new(enabled),
            hovering: Cell::new(false),
            can_show_highlight: Cell::new(can_show_highlight),
            focus_highlight_visible: Cell::new(focused && enabled && can_show_highlight),
            hover_highlight_visible: Cell::new(false),
            on_focus_change,
            on_show_focus_highlight,
            on_show_hover_highlight,
            focus_subscription: RefCell::new(None),
            highlight_subscription: RefCell::new(None),
        });
        let weak = Rc::downgrade(&behavior);
        let focus_subscription = behavior.node.observe(move || {
            if let Some(behavior) = weak.upgrade() {
                behavior.handle_focus_change();
            }
        });
        *behavior.focus_subscription.borrow_mut() = Some(focus_subscription);
        let weak = Rc::downgrade(&behavior);
        let highlight_subscription = FocusHighlightManager::new().observe(move |mode| {
            if let Some(behavior) = weak.upgrade() {
                behavior.update_highlight_mode(mode);
            }
        });
        *behavior.highlight_subscription.borrow_mut() = Some(highlight_subscription);
        behavior
    }

    /// Applies the owning detector's navigation mode without altering
    /// whether its hover affordance and shortcut bindings are enabled.
    #[doc(hidden)]
    pub fn apply_detector_navigation(&self, directional: bool) {
        let permitted = self.enabled.get() || directional;
        self.focus_permitted.set(permitted);
        self.node.set_enabled(permitted);
    }

    #[must_use]
    pub fn node(&self) -> FocusNode {
        self.node.clone()
    }

    #[must_use]
    pub fn is_hovering(&self) -> bool {
        self.hovering.get()
    }

    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.focused.get()
    }

    pub fn mouse_enter(&self) {
        if !self.hovering.replace(true) {
            self.trigger_highlight_callbacks();
        }
    }

    pub fn mouse_exit(&self) {
        if self.hovering.replace(false) {
            self.trigger_highlight_callbacks();
        }
    }

    /// Carries hover presence over a retained widget update without
    /// replaying an enter callback for an already-hovered region.
    /// Derived visibility syncs silently so the next exit still flips;
    /// replacement callbacks keep learning only subsequent transitions.
    pub fn restore_hovering(&self, hovering: bool) {
        self.hovering.set(hovering);
        self.hover_highlight_visible
            .set(self.hover_highlight_visible());
    }

    /// Carries pointer presence into a compatible replacement and emits
    /// only changed highlight visibility, not a duplicate enter event.
    #[doc(hidden)]
    pub fn reconcile_highlights_from(&self, previous: &Self) {
        self.hovering.set(previous.hovering.get());
        self.hover_highlight_visible
            .set(previous.hover_highlight_visible.get());
        self.focus_highlight_visible
            .set(previous.focus_highlight_visible.get());
        self.trigger_highlight_callbacks();
    }

    fn handle_focus_change(&self) {
        let focused = self.node.has_focus();
        if self.focused.replace(focused) != focused {
            self.trigger_highlight_callbacks();
            if let Some(callback) = &self.on_focus_change {
                callback(focused);
            }
        }
    }

    fn update_highlight_mode(&self, mode: FocusHighlightMode) {
        self.can_show_highlight
            .set(mode == FocusHighlightMode::Traditional);
        self.trigger_highlight_callbacks();
    }

    fn focus_highlight_visible(&self) -> bool {
        self.focused.get() && self.focus_permitted.get() && self.can_show_highlight.get()
    }

    fn hover_highlight_visible(&self) -> bool {
        self.hovering.get() && self.enabled.get() && self.can_show_highlight.get()
    }

    fn trigger_highlight_callbacks(&self) {
        let focus_after = self.focus_highlight_visible();
        let hover_after = self.hover_highlight_visible();
        if self.focus_highlight_visible.replace(focus_after) != focus_after
            && let Some(callback) = &self.on_show_focus_highlight
        {
            callback(focus_after);
        }
        if self.hover_highlight_visible.replace(hover_after) != hover_after
            && let Some(callback) = &self.on_show_hover_highlight
        {
            callback(hover_after);
        }
    }
}

/// Strategy for navigating keyboard focus through a collection of focus nodes.
pub trait FocusTraversalPolicy {
    fn find_first_focus(&self, nodes: &[FocusNode]) -> Option<FocusNode>;
    fn find_last_focus(&self, nodes: &[FocusNode]) -> Option<FocusNode>;
    fn next(&self, current: &FocusNode, nodes: &[FocusNode]) -> Option<FocusNode>;
    fn previous(&self, current: &FocusNode, nodes: &[FocusNode]) -> Option<FocusNode>;
}

/// Built-in traversal policies understood by retained widget groups.
///
/// The policy is intentionally a small value rather than a trait object so it
/// can cross rebuild boundaries and be stored in a widget descriptor. Custom
/// consumers can still implement [`FocusTraversalPolicy`] directly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusTraversalPolicyKind {
    /// Follow the retained widget/registration order.
    #[default]
    WidgetOrder,
    /// Sort by the top edge and then the leading edge of each node.
    ReadingOrder,
    /// Sort explicit orders first; equal or absent orders use reading order.
    Ordered,
    /// Sort explicit orders first; equal or absent orders use widget order.
    OrderedWithWidgetOrder,
}

fn is_traversable(node: &FocusNode) -> bool {
    node.can_request_focus() && !node.skip_traversal()
}

fn traversable(nodes: &[FocusNode]) -> Vec<FocusNode> {
    nodes
        .iter()
        .filter(|node| is_traversable(node))
        .cloned()
        .collect()
}

fn reading_order(nodes: Vec<FocusNode>) -> Vec<FocusNode> {
    reading_order_by(nodes, FocusNode::bounds)
}

/// Flutter's `ReadingOrderTraversalPolicy.sort` for left-to-right text.
///
/// Repeatedly picks the topmost remaining item, takes every remaining item
/// whose rectangle overlaps it vertically as its band, and emits the band's
/// left-most member. Items keep their input order where the sort ties.
#[must_use]
pub fn reading_order_by<T>(mut unplaced: Vec<T>, rect: impl Fn(&T) -> Rect) -> Vec<T> {
    if unplaced.len() <= 1 {
        return unplaced;
    }
    let by_top = |left: &T, right: &T| {
        rect(left)
            .origin
            .y
            .partial_cmp(&rect(right).origin.y)
            .unwrap_or(std::cmp::Ordering::Equal)
    };
    // A stable top sort survives every removal. Keep the original repeated
    // comparison for unordered floating-point coordinates.
    let ordered_tops = unplaced.iter().all(|item| rect(item).origin.y.is_finite());
    if ordered_tops {
        unplaced.sort_by(by_top);
    }
    if ordered_tops && has_disjoint_rows(&unplaced, &rect) {
        // Every remaining member of a row overlaps its topmost member.
        // Removing any one of them can never bring the next row into band.
        for row in unplaced.chunk_by_mut(|a, b| rect(a).origin.y == rect(b).origin.y) {
            row.sort_by(|a, b| {
                rect(a)
                    .origin
                    .x
                    .partial_cmp(&rect(b).origin.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        return unplaced;
    }
    let mut sorted = Vec::with_capacity(unplaced.len());
    while !unplaced.is_empty() {
        if !ordered_tops {
            unplaced.sort_by(by_top);
        }
        let topmost = rect(&unplaced[0]);
        let band: Vec<usize> = (0..unplaced.len())
            .filter(|&index| overlaps_band(rect(&unplaced[index]), topmost))
            .collect();
        let mut pick = 0;
        if band.len() > 1 {
            let mut band = band;
            band.sort_by(|&left, &right| {
                rect(&unplaced[left])
                    .origin
                    .x
                    .partial_cmp(&rect(&unplaced[right]).origin.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            pick = band[0];
        }
        sorted.push(unplaced.remove(pick));
    }
    sorted
}

fn has_disjoint_rows<T>(items: &[T], rect: &impl Fn(&T) -> Rect) -> bool {
    let mut row_top = f32::NEG_INFINITY;
    let mut row_bottom = f32::NEG_INFINITY;
    for item in items {
        let bounds = rect(item);
        let top = bounds.origin.y;
        let bottom = top + bounds.size.height;
        if !(bounds.origin.x.is_finite() && bounds.size.width > 0.0 && bottom > top) {
            return false;
        }
        if top != row_top {
            if top < row_bottom {
                return false;
            }
            row_top = top;
        }
        row_bottom = row_bottom.max(bottom);
    }
    true
}

/// Flutter's `_pickNext` band test: the candidate has a non-empty width and
/// height and overlaps the topmost rectangle vertically.
fn overlaps_band(candidate: Rect, topmost: Rect) -> bool {
    let top = candidate.origin.y.max(topmost.origin.y);
    let bottom =
        (candidate.origin.y + candidate.size.height).min(topmost.origin.y + topmost.size.height);
    candidate.size.width > 0.0 && candidate.size.height > 0.0 && top < bottom
}

/// Direction of a directional focus move or a keyboard scroll, matching
/// Flutter's `TraversalDirection` and the arrow keys it is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TraversalDirection {
    Up,
    Down,
    Left,
    Right,
}

/// A focusable rectangle considered by [`directional_focus_index`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalFocusCandidate {
    /// The candidate's bounds in the same coordinates as the focused rectangle.
    pub rect: Rect,
    /// Whether the candidate sits in the same nearest scrollable as the
    /// focused node on the search axis. Flutter prefers these candidates when
    /// any exist.
    pub shares_scroll_owner: bool,
}

/// Picks the candidate Flutter's `DirectionalFocusTraversalPolicyMixin` would
/// focus when moving in `direction` from `current`.
///
/// Without a focused rectangle the first candidate in that direction's edge
/// order is chosen. Otherwise the search runs forward first, then wraps to the
/// far side (Flutter's default `TraversalEdgeBehavior.closedLoop`). The
/// returned value indexes `candidates`.
#[must_use]
pub fn directional_focus_index(
    current: Option<Rect>,
    candidates: &[DirectionalFocusCandidate],
    direction: TraversalDirection,
) -> Option<usize> {
    let Some(current) = current else {
        return first_in_direction(candidates, direction);
    };
    directional_search(current, candidates, direction, true)
        .or_else(|| directional_search(current, candidates, direction, false))
}

/// Selects only the forward or wrap phase of directional traversal.
#[doc(hidden)]
#[must_use]
pub fn directional_focus_index_in_phase(
    current: Option<Rect>,
    candidates: &[DirectionalFocusCandidate],
    direction: TraversalDirection,
    forward: bool,
) -> Option<usize> {
    match current {
        Some(current) => directional_search(current, candidates, direction, forward),
        None => first_in_direction(candidates, direction),
    }
}

fn first_in_direction(
    candidates: &[DirectionalFocusCandidate],
    direction: TraversalDirection,
) -> Option<usize> {
    let key = |index: usize| {
        let rect = candidates[index].rect;
        match direction {
            TraversalDirection::Down => rect.origin.y,
            TraversalDirection::Up => -(rect.origin.y + rect.size.height),
            TraversalDirection::Left => -(rect.origin.x + rect.size.width),
            TraversalDirection::Right => rect.origin.x,
        }
    };
    (0..candidates.len()).min_by(|&left, &right| {
        key(left)
            .partial_cmp(&key(right))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

fn directional_search(
    current: Rect,
    candidates: &[DirectionalFocusCandidate],
    direction: TraversalDirection,
    forward: bool,
) -> Option<usize> {
    let vertical = matches!(direction, TraversalDirection::Up | TraversalDirection::Down);
    let target = rect_center(current);
    let main_center = |index: usize| {
        let center = rect_center(candidates[index].rect);
        if vertical { center.1 } else { center.0 }
    };
    let current_top = current.origin.y;
    let current_bottom = current.origin.y + current.size.height;
    let current_left = current.origin.x;
    let current_right = current.origin.x + current.size.width;

    // Flutter `_sortAndFilterVertically` / `_sortAndFilterHorizontally`: keep
    // candidates on the requested side, nearest first on the main axis.
    let mut eligible: Vec<usize> = (0..candidates.len())
        .filter(|&index| {
            let rect = candidates[index].rect;
            if same_rect(rect, current) {
                return false;
            }
            let center = rect_center(rect);
            match (direction, forward) {
                (TraversalDirection::Down, true) => center.1 >= current_bottom,
                (TraversalDirection::Down, false) => center.1 <= current_bottom,
                (TraversalDirection::Up, true) => center.1 <= current_top,
                (TraversalDirection::Up, false) => center.1 >= current_top,
                (TraversalDirection::Right, true) => center.0 >= current_right,
                (TraversalDirection::Right, false) => center.0 <= current_right,
                (TraversalDirection::Left, true) => center.0 <= current_left,
                (TraversalDirection::Left, false) => center.0 >= current_left,
            }
        })
        .collect();
    eligible.sort_by(|&left, &right| {
        main_center(left)
            .partial_cmp(&main_center(right))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if eligible.is_empty() {
        return None;
    }
    if eligible
        .iter()
        .any(|&index| candidates[index].shares_scroll_owner)
    {
        eligible.retain(|&index| candidates[index].shares_scroll_owner);
    }
    if matches!(direction, TraversalDirection::Up | TraversalDirection::Left) {
        eligible.reverse();
    }

    let distance = |a: f32, b: f32| (a - b).abs();
    // `min_by`/`max_by` return the first/last of the stable sort the band used
    // to be sorted by, so only the extreme element is ever computed.
    let by_target = |&left: &usize, &right: &usize| {
        let (a, b) = (
            rect_center(candidates[left].rect),
            rect_center(candidates[right].rect),
        );
        let (primary_a, primary_b, secondary_a, secondary_b) = if vertical {
            (
                distance(a.1, target.1),
                distance(b.1, target.1),
                distance(a.0, target.0),
                distance(b.0, target.0),
            )
        } else {
            (
                distance(a.0, target.0),
                distance(b.0, target.0),
                distance(a.1, target.1),
                distance(b.1, target.1),
            )
        };
        primary_a
            .partial_cmp(&primary_b)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                secondary_a
                    .partial_cmp(&secondary_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    };
    let in_band = eligible
        .iter()
        .copied()
        .filter(|&index| overlaps_cross_axis(candidates[index].rect, current, vertical));
    let best = if forward {
        in_band.min_by(by_target)
    } else {
        in_band.max_by(by_target)
    };
    if best.is_some() {
        return best;
    }

    // Only out-of-band candidates remain: compare the closest cross-axis edge
    // first, then the centers along the main axis.
    let by_edge = |&left: &usize, &right: &usize| {
        let (rect_a, rect_b) = (candidates[left].rect, candidates[right].rect);
        let (a, b) = (rect_center(rect_a), rect_center(rect_b));
        let (edge_a, edge_b, center_a, center_b) = if vertical {
            (
                closest_edge_distance(
                    rect_a.origin.x,
                    rect_a.origin.x + rect_a.size.width,
                    target.0,
                ),
                closest_edge_distance(
                    rect_b.origin.x,
                    rect_b.origin.x + rect_b.size.width,
                    target.0,
                ),
                distance(a.1, target.1),
                distance(b.1, target.1),
            )
        } else {
            (
                closest_edge_distance(
                    rect_a.origin.y,
                    rect_a.origin.y + rect_a.size.height,
                    target.1,
                ),
                closest_edge_distance(
                    rect_b.origin.y,
                    rect_b.origin.y + rect_b.size.height,
                    target.1,
                ),
                distance(a.0, target.0),
                distance(b.0, target.0),
            )
        };
        edge_a
            .partial_cmp(&edge_b)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                center_a
                    .partial_cmp(&center_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    };
    if forward {
        eligible.iter().copied().min_by(by_edge)
    } else {
        eligible.iter().copied().max_by(by_edge)
    }
}

/// Flutter's `Rect.intersect(band).isEmpty` test against the band spanned by
/// `current` on the cross axis: the candidate needs a positive extent on the
/// main axis and a strictly positive overlap on the cross axis.
fn overlaps_cross_axis(candidate: Rect, current: Rect, vertical: bool) -> bool {
    let (main_extent, cross_lo, cross_hi, cur_lo, cur_hi) = if vertical {
        (
            candidate.size.height,
            candidate.origin.x,
            candidate.origin.x + candidate.size.width,
            current.origin.x,
            current.origin.x + current.size.width,
        )
    } else {
        (
            candidate.size.width,
            candidate.origin.y,
            candidate.origin.y + candidate.size.height,
            current.origin.y,
            current.origin.y + current.size.height,
        )
    };
    main_extent > 0.0 && cross_lo.max(cur_lo) < cross_hi.min(cur_hi)
}

/// Flutter's `_verticalCompareClosestEdge` and its horizontal twin: the
/// distance from the target to whichever edge of the candidate is closer.
fn closest_edge_distance(low: f32, high: f32, target: f32) -> f32 {
    let edge = if (low - target).abs() < (high - target).abs() {
        low
    } else {
        high
    };
    (edge - target).abs()
}

fn rect_center(rect: Rect) -> (f32, f32) {
    (
        rect.origin.x + rect.size.width * 0.5,
        rect.origin.y + rect.size.height * 0.5,
    )
}

fn same_rect(left: Rect, right: Rect) -> bool {
    left.origin == right.origin && left.size == right.size
}

fn explicit_order(mut nodes: Vec<FocusNode>) -> Vec<FocusNode> {
    // A stable sort, so equal and absent orders keep widget order; absent
    // orders sort last.
    nodes.sort_by(|left, right| {
        let left = left.state.traversal_order.borrow();
        let right = right.state.traversal_order.borrow();
        match (left.as_ref(), right.as_ref()) {
            (Some(left), Some(right)) => left.compare(right),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
    nodes
}

/// Flutter's `FocusOrder`: `NumericFocusOrder` or `LexicalFocusOrder`, the
/// key an [`OrderedTraversalPolicy`] sorts focus nodes by.
///
/// Flutter asserts that sibling orders share one type; Incular instead sorts
/// numeric orders before lexical ones.
#[derive(Clone, Debug, PartialEq)]
pub enum FocusOrder {
    /// Flutter's `NumericFocusOrder`: ascending numeric order.
    Numeric(f64),
    /// Flutter's `LexicalFocusOrder`: ascending `String.compareTo` order
    /// (UTF-16 code units).
    Lexical(String),
}

impl FocusOrder {
    #[must_use]
    pub fn numeric(order: f64) -> Self {
        Self::Numeric(order)
    }

    #[must_use]
    pub fn lexical(order: impl Into<String>) -> Self {
        Self::Lexical(order.into())
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Numeric(order) => order.is_finite(),
            Self::Lexical(_) => true,
        }
    }

    /// Flutter's `FocusOrder.compareTo`.
    #[must_use]
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Numeric(left), Self::Numeric(right)) => {
                left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Self::Lexical(left), Self::Lexical(right)) => {
                left.encode_utf16().cmp(right.encode_utf16())
            }
            (Self::Numeric(_), Self::Lexical(_)) => std::cmp::Ordering::Less,
            (Self::Lexical(_), Self::Numeric(_)) => std::cmp::Ordering::Greater,
        }
    }
}

fn policy_nodes(policy: FocusTraversalPolicyKind, nodes: Vec<FocusNode>) -> Vec<FocusNode> {
    match policy {
        FocusTraversalPolicyKind::WidgetOrder => nodes,
        FocusTraversalPolicyKind::ReadingOrder => reading_order(nodes),
        FocusTraversalPolicyKind::Ordered => explicit_order(reading_order(nodes)),
        FocusTraversalPolicyKind::OrderedWithWidgetOrder => explicit_order(nodes),
    }
}

/// Traversal policy following logical widget structure order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WidgetOrderTraversalPolicy;

impl FocusTraversalPolicy for WidgetOrderTraversalPolicy {
    fn find_first_focus(&self, nodes: &[FocusNode]) -> Option<FocusNode> {
        traversable(nodes).into_iter().next()
    }

    fn find_last_focus(&self, nodes: &[FocusNode]) -> Option<FocusNode> {
        traversable(nodes).into_iter().next_back()
    }

    fn next(&self, current: &FocusNode, nodes: &[FocusNode]) -> Option<FocusNode> {
        let traversable = traversable(nodes);
        if let Some(pos) = traversable.iter().position(|n| n == current) {
            traversable.get(pos + 1).cloned()
        } else {
            traversable.into_iter().next()
        }
    }

    fn previous(&self, current: &FocusNode, nodes: &[FocusNode]) -> Option<FocusNode> {
        let traversable = traversable(nodes);
        if let Some(pos) = traversable.iter().position(|n| n == current) {
            if pos > 0 {
                traversable.get(pos - 1).cloned()
            } else {
                None
            }
        } else {
            traversable.into_iter().next_back()
        }
    }
}

/// Traversal policy following natural reading order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadingOrderTraversalPolicy;

impl FocusTraversalPolicy for ReadingOrderTraversalPolicy {
    fn find_first_focus(&self, nodes: &[FocusNode]) -> Option<FocusNode> {
        reading_order(traversable(nodes)).into_iter().next()
    }

    fn find_last_focus(&self, nodes: &[FocusNode]) -> Option<FocusNode> {
        reading_order(traversable(nodes)).into_iter().next_back()
    }

    fn next(&self, current: &FocusNode, nodes: &[FocusNode]) -> Option<FocusNode> {
        WidgetOrderTraversalPolicy.next(current, &reading_order(traversable(nodes)))
    }

    fn previous(&self, current: &FocusNode, nodes: &[FocusNode]) -> Option<FocusNode> {
        WidgetOrderTraversalPolicy.previous(current, &reading_order(traversable(nodes)))
    }
}

/// Traversal policy using explicit numeric ordering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OrderedTraversalPolicy;

impl FocusTraversalPolicy for OrderedTraversalPolicy {
    // Flutter sorts with the reading-order secondary policy first, so unordered
    // nodes and equal orders follow reading order.
    fn find_first_focus(&self, nodes: &[FocusNode]) -> Option<FocusNode> {
        explicit_order(reading_order(traversable(nodes)))
            .into_iter()
            .next()
    }

    fn find_last_focus(&self, nodes: &[FocusNode]) -> Option<FocusNode> {
        explicit_order(reading_order(traversable(nodes)))
            .into_iter()
            .next_back()
    }

    fn next(&self, current: &FocusNode, nodes: &[FocusNode]) -> Option<FocusNode> {
        WidgetOrderTraversalPolicy.next(current, &explicit_order(reading_order(traversable(nodes))))
    }

    fn previous(&self, current: &FocusNode, nodes: &[FocusNode]) -> Option<FocusNode> {
        WidgetOrderTraversalPolicy
            .previous(current, &explicit_order(reading_order(traversable(nodes))))
    }
}

/// Owns a single focus scope.  Register nodes in visual traversal order, then
/// use [`Self::focus_next`] for Tab or Shift+Tab semantics.  The manager is
/// weakly registered, so unmounted/rebuilt controls cannot leak focus entries.
#[derive(Default)]
pub struct FocusManager {
    nodes: Vec<Weak<FocusState>>,
    policy: FocusTraversalPolicyKind,
}

#[derive(Default)]
struct FocusTransition {
    changed: Vec<Rc<FocusState>>,
}

impl FocusTransition {
    fn set(&mut self, state: Rc<FocusState>, focused: bool) {
        if state.set_focused_silent(focused)
            && !self.changed.iter().any(|known| Rc::ptr_eq(known, &state))
        {
            self.changed.push(state);
        }
    }

    fn notify(self) {
        for state in self.changed {
            state.notify();
        }
    }
}
impl FocusManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_policy(policy: FocusTraversalPolicyKind) -> Self {
        Self {
            nodes: Vec::new(),
            policy,
        }
    }

    pub fn set_policy(&mut self, policy: FocusTraversalPolicyKind) {
        self.policy = policy;
    }

    #[must_use]
    pub const fn policy(&self) -> FocusTraversalPolicyKind {
        self.policy
    }
    pub fn register(&mut self, node: &FocusNode) {
        self.prune();
        if !self.nodes.iter().any(|known| {
            known
                .upgrade()
                .is_some_and(|known| Rc::ptr_eq(&known, &node.state))
        }) {
            self.nodes.push(Rc::downgrade(&node.state));
        }
    }
    pub fn unregister(&mut self, node: &FocusNode) {
        self.unregister_silent(node).notify();
    }

    fn unregister_silent(&mut self, node: &FocusNode) -> FocusTransition {
        self.nodes.retain(|known| {
            known
                .upgrade()
                .is_some_and(|known| !Rc::ptr_eq(&known, &node.state))
        });
        let mut transition = FocusTransition::default();
        transition.set(node.state.clone(), false);
        transition
    }
    /// Gives this node exclusive focus within the scope.
    #[must_use]
    pub fn request_focus(&mut self, node: &FocusNode) -> bool {
        let (requested, transition) = self.request_focus_silent(node);
        transition.notify();
        requested
    }

    fn request_focus_silent(&mut self, node: &FocusNode) -> (bool, FocusTransition) {
        self.register(node);
        if !node.can_request_focus() {
            return (false, FocusTransition::default());
        }
        let mut transition = FocusTransition::default();
        for known in self.nodes.iter().filter_map(Weak::upgrade) {
            transition.set(known, false);
        }
        transition.set(node.state.clone(), true);
        (true, transition)
    }
    pub fn clear_focus(&mut self) {
        self.clear_focus_silent().notify();
    }

    fn clear_focus_silent(&mut self) -> FocusTransition {
        let mut transition = FocusTransition::default();
        for node in self.nodes.iter().filter_map(Weak::upgrade) {
            transition.set(node, false);
        }
        self.prune();
        transition
    }
    #[must_use]
    pub fn focused(&mut self) -> Option<FocusNode> {
        self.prune();
        self.nodes
            .iter()
            .filter_map(Weak::upgrade)
            .find_map(|state| state.focused.get().then_some(FocusNode { state }))
    }
    /// Traverses to the next focusable node.  `reverse` provides Shift+Tab
    /// behavior and traversal wraps inside this scope.
    #[must_use]
    pub fn focus_next(&mut self, reverse: bool) -> Option<FocusNode> {
        let (next, transition) = self.focus_next_silent(reverse);
        transition.notify();
        next
    }

    fn focus_next_silent(&mut self, reverse: bool) -> (Option<FocusNode>, FocusTransition) {
        self.prune();
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .filter_map(Weak::upgrade)
            .map(|state| FocusNode { state })
            .filter(is_traversable)
            .collect();
        let nodes = policy_nodes(self.policy, nodes);
        let Some(first) = nodes.first().cloned() else {
            return (None, FocusTransition::default());
        };
        let current = nodes.iter().position(|node| node.state.focused.get());
        let next = match current {
            Some(index) if reverse => nodes[(index + nodes.len() - 1) % nodes.len()].clone(),
            Some(index) => nodes[(index + 1) % nodes.len()].clone(),
            None if reverse => nodes.last().cloned().unwrap_or(first),
            None => first,
        };
        let mut transition = FocusTransition::default();
        for node in self.nodes.iter().filter_map(Weak::upgrade) {
            transition.set(node, false);
        }
        transition.set(next.state.clone(), true);
        (Some(next), transition)
    }
    #[must_use]
    pub fn registered_count(&mut self) -> usize {
        self.prune();
        self.nodes.len()
    }
    fn prune(&mut self) {
        self.nodes.retain(|node| node.strong_count() > 0);
    }
}

/// What focus traversal does upon reaching the edge of a focus scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TraversalEdgeBehavior {
    /// Traverse from the opposite edge of the same scope.
    ClosedLoop,
    /// Unfocus the current target and let the host handle traversal.
    LeaveFlutterView,
    /// Continue traversal in the enclosing scope when one exists.
    ParentScope,
    /// Leave focus unchanged.
    Stop,
}

/// A retained focus-scope handle.
///
/// `FocusScopeNode` is the Rust-native equivalent of a scoped focus owner. It
/// owns a [`FocusManager`] behind interior mutability so a scope can be kept
/// outside rebuildable widget descriptors and shared by controls that need to
/// request focus. When mounted as a FocusScope, its shared focus node
/// participates in the retained host's focus ancestry.
#[derive(Clone)]
pub struct FocusScopeNode {
    state: Rc<FocusScopeState>,
}

impl PartialEq for FocusScopeNode {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl Eq for FocusScopeNode {}

/// Owns one live subscription to a [`FocusScopeNode`] change stream.
///
/// The token is intentionally lightweight and single-threaded, matching the
/// retained UI model. Dropping it removes the callback from future delivery;
/// the scope prunes the corresponding weak entry on its next notification.
pub struct FocusScopeSubscription {
    _subscription: incular_core::reactivity::Subscription,
}

struct ScopeFocusAnchor {
    node: FocusNode,
    _subscription: FocusNodeSubscription,
}

struct FocusScopeState {
    traversal_edge_behavior: Cell<TraversalEdgeBehavior>,
    directional_traversal_edge_behavior: Cell<TraversalEdgeBehavior>,
    anchor: OnceCell<ScopeFocusAnchor>,
    hosted_history: RefCell<Vec<Weak<FocusState>>>,
    manager: RefCell<FocusManager>,
    last_focused: RefCell<Option<FocusNode>>,
    parent: RefCell<Option<Weak<FocusScopeState>>>,
    changes: incular_core::reactivity::DependencySource,
    revision: Cell<u64>,
}

impl FocusScopeState {
    fn bump_revision(&self) {
        self.revision.set(
            self.revision
                .get()
                .checked_add(1)
                .expect("revision exhausted"),
        );
        self.changes.notify();
    }
}

impl Default for FocusScopeNode {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for FocusScopeNode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut manager = self.state.manager.borrow_mut();
        formatter
            .debug_struct("FocusScopeNode")
            .field("registered_count", &manager.registered_count())
            .field("has_focus", &manager.focused().is_some())
            .finish()
    }
}

impl FocusScopeNode {
    /// Creates an empty retained focus scope.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Rc::new(FocusScopeState {
                traversal_edge_behavior: Cell::new(TraversalEdgeBehavior::ClosedLoop),
                directional_traversal_edge_behavior: Cell::new(TraversalEdgeBehavior::Stop),
                anchor: OnceCell::new(),
                hosted_history: RefCell::new(Vec::new()),
                manager: RefCell::new(FocusManager::new()),
                last_focused: RefCell::new(None),
                parent: RefCell::new(None),
                changes: incular_core::reactivity::DependencySource::default(),
                revision: Cell::new(0),
            }),
        }
    }

    /// The behavior at the edge of Tab and Shift+Tab. Defaults to ClosedLoop.
    #[must_use]
    pub fn traversal_edge_behavior(&self) -> TraversalEdgeBehavior {
        self.state.traversal_edge_behavior.get()
    }

    /// Changes sequential traversal without replacing the scope.
    pub fn set_traversal_edge_behavior(&self, behavior: TraversalEdgeBehavior) {
        if self.state.traversal_edge_behavior.replace(behavior) != behavior {
            self.state.bump_revision();
        }
    }

    /// Discards the focused-child memory after traversal unfocuses this scope.
    #[doc(hidden)]
    pub fn clear_hosted_focus_history(&self) {
        self.state.hosted_history.borrow_mut().clear();
    }

    /// The behavior at the edge of arrow-key traversal. Defaults to Stop.
    #[must_use]
    pub fn directional_traversal_edge_behavior(&self) -> TraversalEdgeBehavior {
        self.state.directional_traversal_edge_behavior.get()
    }

    /// Changes directional edge behavior without replacing the scope.
    pub fn set_directional_traversal_edge_behavior(&self, behavior: TraversalEdgeBehavior) {
        if self.state.directional_traversal_edge_behavior.replace(behavior) != behavior {
            self.state.bump_revision();
        }
    }

    /// The retained scope's own focus target. Unlike its descendants,
    /// the scope is not a stop in ordinary Tab traversal.
    #[doc(hidden)]
    #[must_use]
    pub fn scope_focus_node(&self) -> FocusNode {
        self.state
            .anchor
            .get_or_init(|| {
                let node = FocusNode::with_scope_owner(Some(Rc::downgrade(&self.state)));
                let state = Rc::downgrade(&self.state);
                let subscription = node.observe(move || {
                    if let Some(state) = state.upgrade() {
                        state.bump_revision();
                    }
                });
                ScopeFocusAnchor {
                    node,
                    _subscription: subscription,
                }
            })
            .node
            .clone()
    }

    /// Skips the scope itself in raw traversal without excluding its descendants.
    pub fn set_skip_traversal(&self, skip: bool) {
        self.scope_focus_node().set_skip_traversal(skip);
    }

    #[must_use]
    pub fn skip_traversal(&self) -> bool {
        self.scope_focus_node().skip_traversal()
    }

    /// Enables requests for the scope and its hosted descendants.
    /// Disabling the scope does not overwrite any descendant's own setting.
    pub fn set_can_request_focus(&self, allowed: bool) {
        self.scope_focus_node().set_can_request_focus(allowed);
    }

    #[must_use]
    pub fn can_request_focus(&self) -> bool {
        self.scope_focus_node().can_request_focus()
    }

    /// Independently restricts descendants without disabling the scope itself.
    pub fn set_descendants_are_focusable(&self, allowed: bool) {
        self.scope_focus_node().set_descendants_are_focusable(allowed);
    }

    #[must_use]
    pub fn descendants_are_focusable(&self) -> bool {
        self.scope_focus_node().descendants_are_focusable()
    }

    pub fn set_descendants_are_traversable(&self, allowed: bool) {
        self.scope_focus_node().set_descendants_are_traversable(allowed);
    }

    #[must_use]
    pub fn descendants_are_traversable(&self) -> bool {
        self.scope_focus_node().descendants_are_traversable()
    }

    /// Requests primary focus for the scope itself, without forwarding
    /// the request to a previously focused descendant.
    pub fn request_scope_focus(&self) {
        let _ = self.last_hosted_child(true);
        self.scope_focus_node().request_focus();
    }

    /// Requests the most recently focused eligible descendant, following
    /// nested scopes. When none is remembered, requests the scope itself.
    #[must_use]
    pub fn request_focus_in_scope(&self) -> bool {
        let mut current = self.clone();
        let mut visited = Vec::new();
        loop {
            if !current.can_request_focus() || visited.contains(&current) {
                return false;
            }
            let Some(child) = current.last_hosted_child(true) else {
                current.request_scope_focus();
                return true;
            };
            if let Some(child_scope) = child.scope_owner() {
                visited.push(current);
                current = child_scope;
            } else {
                child.request_focus();
                return true;
            }
        }
    }

    /// Whether the mounted scope or any of its descendants has focus.
    #[must_use]
    pub fn has_focus(&self) -> bool {
        self.state.anchor.get().map_or_else(
            || self.focused().is_some(),
            |anchor| anchor.node.has_focus(),
        )
    }

    /// Whether this mounted scope itself owns primary focus.
    #[must_use]
    pub fn has_primary_focus(&self) -> bool {
        self.state
            .anchor
            .get()
            .is_some_and(|anchor| anchor.node.has_primary_focus())
    }

    /// Creates a scope nested below `parent`.
    #[must_use]
    pub fn nested(parent: &Self) -> Self {
        let scope = Self::new();
        scope.set_parent(Some(parent));
        scope
    }

    /// Sets or clears this scope's parent relationship.
    pub fn set_parent(&self, parent: Option<&Self>) {
        let parent = parent.filter(|scope| !Rc::ptr_eq(&scope.state, &self.state));
        *self.state.parent.borrow_mut() = parent.map(|scope| Rc::downgrade(&scope.state));
        self.state.bump_revision();
    }

    /// Subscribes to focus registration and selection changes.
    ///
    /// Runtime builders use this to turn retained focus changes into ordinary
    /// element invalidation. Consumers outside a runtime may use the same
    /// stream for status indicators or accessibility adapters.
    #[must_use]
    pub fn observe(&self, callback: impl Fn() + 'static) -> FocusScopeSubscription {
        FocusScopeSubscription {
            _subscription: self.state.changes.subscribe((), callback),
        }
    }

    /// Monotonic state revision, useful for diagnostics and non-runtime
    /// consumers that need to cheaply detect a focus change.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.state.revision.get()
    }

    /// Registers a focus node in this scope.
    pub fn register(&self, node: &FocusNode) {
        self.state.manager.borrow_mut().register(node);
        self.state.bump_revision();
    }

    /// Sets the traversal policy used by this retained scope.
    pub fn set_policy(&self, policy: FocusTraversalPolicyKind) {
        self.state.manager.borrow_mut().set_policy(policy);
        self.state.bump_revision();
    }

    #[must_use]
    pub fn policy(&self) -> FocusTraversalPolicyKind {
        self.state.manager.borrow().policy()
    }

    /// Removes a focus node from this scope and clears its focus.
    pub fn unregister(&self, node: &FocusNode) {
        let transition = self.state.manager.borrow_mut().unregister_silent(node);
        let clear_last = self
            .state
            .last_focused
            .borrow()
            .as_ref()
            .is_some_and(|focused| focused == node);
        if clear_last {
            self.state.last_focused.borrow_mut().take();
        }
        self.state.bump_revision();
        transition.notify();
    }

    /// Requests autofocus for a descendant after the host's next layout.
    /// It is considered only while this scope still has no focused child
    /// and the target belongs to it. Returns false if this scope has no
    /// mounted host; a queued request may still be rejected when applied.
    #[must_use]
    pub fn autofocus(&self, node: &FocusNode) -> bool {
        let Some(anchor) = self.mounted_anchor() else {
            return false;
        };
        SCOPED_AUTOFOCUS.with(|pending| {
            pending.borrow_mut().push(ScopeAutofocusRequest {
                scope: Rc::downgrade(&self.state),
                target: Rc::downgrade(&node.state),
            });
        });
        anchor.state.schedule_host_frame();
        true
    }

    /// Gives a node exclusive focus in this scope. A mounted scope queues
    /// the request through the host and only accepts its own descendants.
    #[must_use]
    pub fn request_focus(&self, node: &FocusNode) -> bool {
        if !node.can_request_focus() {
            return false;
        }
        if self.mounted_anchor().is_some() {
            let mut scope = node.host_enclosing_scope();
            let mut visited = Vec::new();
            while let Some(parent) = scope {
                if !parent.descendants_are_focusable() || visited.contains(&parent) {
                    return false;
                }
                if &parent == self {
                    if let Some(scope) = node.scope_owner() {
                        return scope.request_focus_in_scope();
                    }
                    node.request_focus();
                    return true;
                }
                scope = parent.scope_focus_node().host_enclosing_scope();
                visited.push(parent);
            }
            return false;
        }
        let parent = self
            .state
            .parent
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .map(|state| Self { state });
        if let Some(parent) = &parent {
            let mut parent_manager = parent.state.manager.borrow_mut();
            let parent_focused = parent_manager.focused();
            let parent_transition = parent_focused
                .as_ref()
                .map(|_| parent_manager.clear_focus_silent());
            drop(parent_manager);
            if let Some(parent_focused) = parent_focused {
                *parent.state.last_focused.borrow_mut() = Some(parent_focused);
            }
            let mut manager = self.state.manager.borrow_mut();
            let previous = manager.focused();
            let (requested, transition) = manager.request_focus_silent(node);
            drop(manager);
            if !requested {
                return false;
            }
            *self.state.last_focused.borrow_mut() = previous;
            if parent_transition.is_some() {
                parent.state.bump_revision();
            }
            self.state.bump_revision();
            if let Some(parent_transition) = parent_transition {
                parent_transition.notify();
            }
            transition.notify();
            return true;
        }
        let mut manager = self.state.manager.borrow_mut();
        let previous = manager.focused();
        let (requested, transition) = manager.request_focus_silent(node);
        if requested {
            *self.state.last_focused.borrow_mut() = previous;
            drop(manager);
            self.state.bump_revision();
            transition.notify();
        }
        requested
    }

    /// Clears the current focus while retaining it for [`Self::restore_focus`].
    pub fn clear_focus(&self) {
        if let Some(anchor) = self.mounted_anchor() {
            anchor.unfocus();
            return;
        }
        let mut manager = self.state.manager.borrow_mut();
        let previous = manager.focused();
        let transition = manager.clear_focus_silent();
        drop(manager);
        *self.state.last_focused.borrow_mut() = previous;
        self.state.bump_revision();
        transition.notify();
    }

    /// Clears this scope's focus.
    pub fn unfocus(&self) {
        self.clear_focus();
    }

    /// Moves focus away from this mounted scope, leaving its own child
    /// history unchanged. For a standalone scope, clears local focus.
    pub fn unfocus_with_disposition(&self, disposition: UnfocusDisposition) {
        if let Some(anchor) = self.mounted_anchor() {
            anchor.unfocus_with_disposition(disposition);
        } else {
            self.clear_focus();
        }
    }

    fn mounted_anchor(&self) -> Option<&FocusNode> {
        self.state
            .anchor
            .get()
            .map(|anchor| &anchor.node)
            .filter(|node| node.state.host_attachment_count.get() != 0)
    }

    #[doc(hidden)]
    pub fn record_host_focused_child(&self, child: &FocusNode) {
        let child_state = Rc::downgrade(&child.state);
        let mut history = self.state.hosted_history.borrow_mut();
        if history.last().is_some_and(|last| last.ptr_eq(&child_state)) {
            return;
        }
        history.retain(|known| {
            !known.ptr_eq(&child_state)
                && known.upgrade().is_some_and(|state| {
                    FocusNode { state }.host_enclosing_scope().as_ref() == Some(self)
                })
        });
        history.push(child_state);
    }

    fn forget_hosted_child(&self, child: &FocusNode) {
        let child_state = Rc::downgrade(&child.state);
        self.state
            .hosted_history
            .borrow_mut()
            .retain(|known| !known.ptr_eq(&child_state));
    }

    /// Selects the newest still-eligible child according to the retained
    /// host, pruning stale and newly excluded history before returning it.
    #[doc(hidden)]
    pub fn restore_hosted_focus_child<T>(
        &self,
        mut resolve: impl FnMut(&FocusNode) -> Option<T>,
    ) -> Option<T> {
        let mut history = self.state.hosted_history.borrow_mut();
        while let Some(remembered) = history.last() {
            if let Some(state) = remembered.upgrade() {
                let child = FocusNode { state };
                if child.host_enclosing_scope().as_ref() == Some(self)
                    && let Some(target) = resolve(&child)
                {
                    return Some(target);
                }
            }
            history.pop();
        }
        None
    }

    fn last_hosted_child(&self, require_focusable: bool) -> Option<FocusNode> {
        let mut history = self.state.hosted_history.borrow_mut();
        if require_focusable {
            while let Some(last) = history.last() {
                if last.upgrade().is_some_and(|state| {
                    let child = FocusNode { state };
                    child.can_request_focus()
                        && child.host_enclosing_scope().as_ref() == Some(self)
                }) {
                    break;
                }
                history.pop();
            }
        }
        history
            .iter()
            .rev()
            .filter_map(Weak::upgrade)
            .map(|state| FocusNode { state })
            .find(|child| child.host_enclosing_scope().as_ref() == Some(self))
    }

    /// The most recently focused live child, even when another scope owns
    /// primary focus. Children that move to another scope no longer count.
    #[must_use]
    pub fn focused_child(&self) -> Option<FocusNode> {
        if self.mounted_anchor().is_none() {
            return self.state.manager.borrow_mut().focused();
        }
        self.last_hosted_child(false)
    }

    /// Returns the currently active child of this scope, if any.
    #[must_use]
    pub fn focused(&self) -> Option<FocusNode> {
        if let Some(anchor) = self.mounted_anchor() {
            return (anchor.has_focus() && !anchor.has_primary_focus())
                .then(|| self.focused_child())
                .flatten()
                .filter(FocusNode::has_focus);
        }
        self.state.manager.borrow_mut().focused()
    }

    /// Moves focus to the next/previous registered node.
    #[must_use]
    pub fn focus_next(&self, reverse: bool) -> Option<FocusNode> {
        let mut manager = self.state.manager.borrow_mut();
        let previous = manager.focused();
        let (next, transition) = manager.focus_next_silent(reverse);
        if next.is_some() {
            *self.state.last_focused.borrow_mut() = previous;
            drop(manager);
            self.state.bump_revision();
            transition.notify();
        }
        next
    }

    /// Restores the last focus that was active before this scope was cleared
    /// or moved. Returns `false` when the remembered node is no longer
    /// requestable.
    #[must_use]
    pub fn restore_focus(&self) -> bool {
        if self.mounted_anchor().is_some() {
            return self
                .last_hosted_child(true)
                .is_some_and(|child| self.request_focus(&child));
        }
        let Some(node) = self.state.last_focused.borrow().clone() else {
            return false;
        };
        let mut manager = self.state.manager.borrow_mut();
        let (restored, transition) = manager.request_focus_silent(&node);
        if restored {
            *self.state.last_focused.borrow_mut() = Some(node);
            drop(manager);
            self.state.bump_revision();
            transition.notify();
        }
        restored
    }

    /// Restores the focused child remembered by this scope's parent.
    #[must_use]
    pub fn restore_parent_focus(&self) -> bool {
        self.state
            .parent
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|parent| {
                let scope = Self { state: parent };
                scope.restore_focus()
            })
    }

    /// Number of live nodes registered in this scope.
    #[must_use]
    pub fn registered_count(&self) -> usize {
        self.state.manager.borrow_mut().registered_count()
    }
}
