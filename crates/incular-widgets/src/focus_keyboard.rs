//! Focus, focus traversal, and raw keyboard listener widgets.

use std::rc::Rc;

use incular_core::KeyboardEvent;
pub use incular_gestures::{
    Action, ActionContext, ActionDispatchInvocation, ActionDispatcher, ActionInvocationPhase, ActionOutput, ActionResult, Actions,
    CallbackAction, Command, CommandId,
    FocusBehavior,
    FocusManager, FocusNode, FocusOrder, FocusScopeNode, FocusScopeSubscription, UnfocusDisposition,
    FocusTraversalPolicy, FocusTraversalPolicyKind, Intent, KeyEventResult, LockState,
    LogicalKeySet, LogicalShortcutKey, OrderedTraversalPolicy, ReadingOrderTraversalPolicy,
    ScopedActionDispatcher, ShortcutActivator, ShortcutKey, ShortcutTrigger, Shortcuts, SingleActivator,
    VoidCallbackAction, VoidCallbackIntent, WidgetOrderTraversalPolicy,
};

use crate::{Widget, WidgetKind};

/// A required action lookup found no mapping in any enclosing scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("no enclosing action maps command type {command_type}")]
pub struct ActionLookupError {
    pub command_type: &'static str,
}

/// Supplies an inherited command-to-action mapping without claiming focus.
///
/// Commands resolved from its descendants use the nearest mapping. An
/// absent mapping falls through to an ancestor, while a disabled action
/// still shadows the same command in outer scopes.
#[derive(Clone)]
pub struct ActionScope {
    actions: Rc<incular_gestures::ErasedActionScope>,
    child: Widget,
}

impl ActionScope {
    #[must_use]
    pub fn new<C: CommandId>(actions: Rc<Actions<C>>, child: impl Into<Widget>) -> Self {
        Self::from_erased(incular_gestures::ErasedActionScope::from_actions(actions), child.into())
    }

    pub(crate) fn from_erased(
        actions: Rc<incular_gestures::ErasedActionScope>,
        child: Widget,
    ) -> Self {
        Self { actions, child }
    }

    /// Installs an explicit dispatcher for this scope and descendants with
    /// mappings but no nearer dispatcher.
    #[must_use]
    pub fn dispatcher(mut self, dispatcher: Rc<ScopedActionDispatcher>) -> Self {
        self.actions = self.actions.with_dispatcher(dispatcher);
        self
    }
}

impl From<ActionScope> for Widget {
    fn from(scope: ActionScope) -> Self {
        Widget::from_kind(WidgetKind::Gesture {
            behavior: crate::gestures::HitTestBehavior::DeferToChild,
            callbacks: Box::new(incular_gestures::GestureCallbacks {
                action_scope: Some(scope.actions),
                ..Default::default()
            }),
            child: scope.child,
        })
    }
}

/// Installs key-to-command bindings without supplying actions or claiming focus.
/// The actions and dispatcher are looked up from the focused descendant.
#[derive(Clone)]
pub struct ShortcutScope {
    shortcuts: Rc<incular_gestures::ErasedShortcutScope>,
    include_semantics: bool,
    debug_label: Option<Rc<str>>,
    child: Widget,
}

impl ShortcutScope {
    #[must_use]
    pub fn new<C: CommandId>(shortcuts: Rc<Shortcuts<C>>, child: impl Into<Widget>) -> Self {
        Self::from_erased(shortcuts.erased_scope(), child.into())
    }

    pub(crate) fn from_erased(
        shortcuts: Rc<incular_gestures::ErasedShortcutScope>,
        child: Widget,
    ) -> Self {
        Self {
            shortcuts,
            include_semantics: true,
            debug_label: None,
            child,
        }
    }

    /// Sets the diagnostic suffix for the passive focus boundary.
    #[must_use]
    pub fn debug_label(mut self, label: impl AsRef<str>) -> Self {
        self.debug_label = Some(Rc::from(format!("Shortcuts: {}", label.as_ref())));
        self
    }

    /// Controls the passive accessibility annotation, not keyboard routing.
    #[must_use]
    pub fn include_semantics(mut self, include: bool) -> Self {
        self.include_semantics = include;
        self
    }
}

impl From<ShortcutScope> for Widget {
    fn from(scope: ShortcutScope) -> Self {
        passive_shortcut_boundary(
            scope.child,
            incular_gestures::GestureCallbacks {
                shortcut_scope: Some(scope.shortcuts),
                ..Default::default()
            },
            Some(scope.debug_label.unwrap_or_else(|| Rc::from("Shortcuts"))),
            false,
            scope.include_semantics,
        )
    }
}

/// All shortcut wrappers are focus ancestors, but never take primary focus.
pub(crate) fn passive_shortcut_boundary(
    child: Widget,
    mut callbacks: incular_gestures::GestureCallbacks,
    label: Option<Rc<str>>,
    skip_traversal: bool,
    include_semantics: bool,
) -> Widget {
    callbacks.owned_focus_permission = Some(false);
    callbacks.focus_debug_label = label;
    callbacks.annotates_focus = true;
    callbacks.include_semantics = false;
    callbacks.focus_node_widget_properties = Some(incular_gestures::FocusNodeWidgetProperties {
        skip_traversal: Some(skip_traversal),
        ..Default::default()
    });
    let widget = Widget::from_kind(WidgetKind::Gesture {
        behavior: crate::gestures::HitTestBehavior::DeferToChild,
        callbacks: Box::new(callbacks),
        child,
    });
    if include_semantics {
        crate::Semantics::new(widget).focusable(false).into()
    } else {
        widget
    }
}

/// A widget that manages keyboard focus for a subtree.
#[derive(Clone)]
pub struct Focus {
    node: Option<FocusNode>,
    parent_node: Option<FocusNode>,
    autofocus: bool,
    can_request_focus: Option<bool>,
    skip_traversal: Option<bool>,
    descendants_are_focusable: Option<bool>,
    descendants_are_traversable: Option<bool>,
    use_external_properties: bool,
    include_semantics: bool,
    on_key_event: Option<incular_gestures::FocusKeyEventHandler>,
    on_raw_key: Option<incular_gestures::FocusRawKeyEventHandler>,
    on_focus_change: Option<Rc<dyn Fn(bool)>>,
    child: Option<Widget>,
}

impl Default for Focus {
    fn default() -> Self {
        Self {
            node: None,
            parent_node: None,
            autofocus: false,
            can_request_focus: None,
            skip_traversal: None,
            descendants_are_focusable: None,
            descendants_are_traversable: None,
            use_external_properties: false,
            include_semantics: true,
            on_key_event: None,
            on_raw_key: None,
            on_focus_change: None,
            child: None,
        }
    }
}

impl Focus {
    /// Creates a Focus boundary wrapping a child widget.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: Some(child.into()),
            ..Self::default()
        }
    }

    /// Attaches an external [`FocusNode`].
    #[must_use]
    pub fn node(mut self, node: FocusNode) -> Self {
        self.node = Some(node);
        self.use_external_properties = false;
        self
    }

    /// Wraps a node without replacing its callbacks or focus permissions.
    #[must_use]
    pub fn with_external_focus_node(child: impl Into<Widget>, node: FocusNode) -> Self {
        Self {
            node: Some(node),
            use_external_properties: true,
            ..Self::new(child)
        }
    }

    /// Uses an explicit parent in the mounted focus tree rather than the
    /// nearest ancestor supplied by the widget tree.
    #[must_use]
    pub fn parent_node(mut self, parent: impl Into<Option<FocusNode>>) -> Self {
        self.parent_node = parent.into();
        self
    }

    /// Requests focus once if enabled on mount or on a retained update.
    #[must_use]
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    /// Returns the nearest enclosing Focus node, not including this element.
    /// Searches stop at the nearest FocusScope; set `scope_ok` to allow that
    /// scope's own node. This retained-tree query does not subscribe to changes.
    #[must_use]
    pub fn maybe_of(
        tree: &crate::tree::WidgetTree,
        element: crate::tree::ElementId,
        scope_ok: bool,
    ) -> Option<FocusNode> {
        let mut ancestor = tree.parent(element);
        while let Some(id) = ancestor {
            if let Some(node) = tree.focus_boundary_at(id) {
                return (scope_ok || node.scope_owner().is_none()).then(|| node.clone());
            }
            ancestor = tree.parent(id);
        }
        None
    }

    /// Resolves the nearest enclosing node with the same scope rules as
    /// [`Self::maybe_of`].
    ///
    /// # Panics
    ///
    /// Panics when there is no eligible enclosing Focus boundary.
    #[must_use]
    pub fn of(
        tree: &crate::tree::WidgetTree,
        element: crate::tree::ElementId,
        scope_ok: bool,
    ) -> FocusNode {
        Self::maybe_of(tree, element, scope_ok).expect("no enclosing Focus boundary")
    }

    /// Controls whether focus can be requested by user interaction or programmatic focus.
    #[must_use]
    pub fn can_request_focus(mut self, can_request: bool) -> Self {
        self.can_request_focus = Some(can_request);
        self
    }

    /// Controls explicit or implicit focus requests by descendants.
    #[must_use]
    pub fn descendants_are_focusable(mut self, focusable: bool) -> Self {
        self.descendants_are_focusable = Some(focusable);
        self
    }

    /// Controls traversal to descendants without denying explicit requests.
    #[must_use]
    pub fn descendants_are_traversable(mut self, traversable: bool) -> Self {
        self.descendants_are_traversable = Some(traversable);
        self
    }

    /// Excludes only this focus node from traversal, not its descendants.
    #[must_use]
    pub fn skip_traversal(mut self, skip: bool) -> Self {
        self.skip_traversal = Some(skip);
        self
    }

    #[must_use]
    pub fn skips_traversal(&self) -> bool {
        (!self.use_external_properties)
            .then_some(self.skip_traversal)
            .flatten()
            .unwrap_or_else(|| self.node.as_ref().is_some_and(FocusNode::skip_traversal))
    }

    #[must_use]
    pub fn allows_focus_requests(&self) -> bool {
        (!self.use_external_properties)
            .then_some(self.can_request_focus)
            .flatten()
            .unwrap_or_else(|| self.node.as_ref().is_none_or(FocusNode::can_request_focus))
    }

    #[must_use]
    pub fn allows_descendant_focus(&self) -> bool {
        (!self.use_external_properties)
            .then_some(self.descendants_are_focusable)
            .flatten()
            .unwrap_or_else(|| {
                self.node
                    .as_ref()
                    .is_none_or(FocusNode::descendants_are_focusable)
            })
    }

    #[must_use]
    pub fn allows_descendant_traversal(&self) -> bool {
        (!self.use_external_properties)
            .then_some(self.descendants_are_traversable)
            .flatten()
            .unwrap_or_else(|| {
                self.node
                    .as_ref()
                    .is_none_or(FocusNode::descendants_are_traversable)
            })
    }

    /// Includes this widget's focus annotation in the accessibility tree.
    /// This defaults to true and does not affect ordinary keyboard focus.
    #[must_use]
    pub fn include_semantics(mut self, include: bool) -> Self {
        self.include_semantics = include;
        self
    }

    /// Handles keys delivered to this node or bubbled from a focused child.
    #[must_use]
    pub fn on_key_event(
        self,
        handler: impl Fn(&FocusNode, KeyboardEvent) -> KeyEventResult + 'static,
    ) -> Self {
        self.with_key_event_handler(Some(Rc::new(handler)))
    }

    /// Installs a shared callback or clears the typed handler.
    #[must_use]
    pub fn with_key_event_handler(
        mut self,
        handler: Option<incular_gestures::FocusKeyEventHandler>,
    ) -> Self {
        self.on_key_event = handler;
        self
    }

    /// Returns the callback that this widget supplies, or the external
    /// node's current callback when the node owns its own properties.
    #[must_use]
    pub fn key_event_handler(&self) -> Option<incular_gestures::FocusKeyEventHandler> {
        if self.use_external_properties {
            self.node.as_ref().and_then(FocusNode::on_key_event)
        } else {
            self.on_key_event.clone()
        }
    }

    /// Handles the deprecated Flutter raw-key route. Prefer `on_key_event`
    /// for new code. Both callbacks may observe the same portable key input.
    #[must_use]
    pub fn on_key(
        self,
        handler: impl Fn(&FocusNode, incular_gestures::RawKeyEvent) -> KeyEventResult + 'static,
    ) -> Self {
        self.with_raw_key_handler(Some(Rc::new(handler)))
    }

    /// Sets or clears the legacy callback for a regular Focus widget.
    #[must_use]
    pub fn with_raw_key_handler(mut self, handler: Option<incular_gestures::FocusRawKeyEventHandler>) -> Self {
        self.on_raw_key = handler;
        self
    }

    /// Resolves the widget's raw callback, or reads the externally owned one.
    #[must_use]
    pub fn raw_key_handler(&self) -> Option<incular_gestures::FocusRawKeyEventHandler> {
        if self.use_external_properties {
            self.node.as_ref().and_then(FocusNode::on_key)
        } else {
            self.on_raw_key.clone()
        }
    }

    /// Reports a change in whether the node or one of its descendants has focus.
    #[must_use]
    pub fn on_focus_change(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_focus_change = Some(Rc::new(callback));
        self
    }
}

impl From<Focus> for Widget {
    fn from(value: Focus) -> Self {
        let child = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        // The node owns these permissions. Marking the child widget as
        // excluded as well would block nodes reparented out of this branch.
        let owned_focus_permission = value
            .node
            .is_none()
            .then_some(value.can_request_focus.unwrap_or(true));
        let callbacks = incular_gestures::GestureCallbacks {
            focus_node: value.node,
            focus_parent_node: value.parent_node,
            owned_focus_permission,
            focus_key_handler_owner: if value.use_external_properties {
                incular_gestures::FocusKeyHandlerOwner::FocusNode
            } else {
                incular_gestures::FocusKeyHandlerOwner::FocusWidget
            },
            focus_node_widget_properties: (!value.use_external_properties).then_some(
                incular_gestures::FocusNodeWidgetProperties {
                    can_request_focus: value.can_request_focus,
                    skip_traversal: value.skip_traversal,
                    descendants_are_focusable: value.descendants_are_focusable,
                    descendants_are_traversable: value.descendants_are_traversable,
                },
            ),
            autofocus: value.autofocus,
            on_focus_change: value.on_focus_change,
            on_focus_key_event: value.on_key_event,
            on_focus_raw_key: value.on_raw_key,
            include_semantics: value.include_semantics,
            annotates_focus: true,
            ..incular_gestures::GestureCallbacks::default()
        };
        Widget::from_kind(WidgetKind::Gesture {
            behavior: crate::gestures::HitTestBehavior::DeferToChild,
            callbacks: Box::new(callbacks),
            child,
        })
    }
}

/// A widget that establishes a scoped focus tree traversal domain.
///
/// Inherits the surrounding traversal policy, falling back to reading
/// order. Use [`Self::widget_order`] to explicitly select registration order.
#[derive(Clone)]
pub struct FocusScope {
    node: Option<FocusScopeNode>,
    include_semantics: bool,
    debug_label: Option<Rc<str>>,
    parent_node: Option<FocusNode>,
    can_request_focus: Option<bool>,
    skip_traversal: Option<bool>,
    descendants_are_focusable: Option<bool>,
    descendants_are_traversable: Option<bool>,
    use_external_properties: bool,
    autofocus: bool,
    policy: Option<FocusTraversalPolicyKind>,
    on_focus_change: Option<Rc<dyn Fn(bool)>>,
    child: Option<Widget>,
}

impl Default for FocusScope {
    fn default() -> Self {
        Self {
            node: None,
            parent_node: None,
            can_request_focus: None,
            skip_traversal: None,
            descendants_are_focusable: None,
            descendants_are_traversable: None,
            use_external_properties: false,
            autofocus: false,
            include_semantics: true,
            debug_label: None,
            policy: None,
            on_focus_change: None,
            child: None,
        }
    }
}

impl FocusScope {
    /// Creates a new FocusScope.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: Some(child.into()),
            ..Self::default()
        }
    }

    /// Keeps the scope's children explicit in the semantics tree. This
    /// defaults to true and does not change actual keyboard focus.
    #[must_use]
    pub fn include_semantics(mut self, include: bool) -> Self {
        self.include_semantics = include;
        self
    }

    /// Sets a diagnostic name on the scope's retained focus node.
    #[must_use]
    pub fn debug_label(mut self, label: impl Into<Rc<str>>) -> Self {
        self.debug_label = Some(label.into());
        self
    }

    /// Resolves the nearest focus boundary above this retained element and
    /// returns that boundary's mounted focus scope, if any. A scope boundary
    /// belongs to itself; another focus boundary follows its focus parents.
    #[must_use]
    pub fn maybe_of(
        tree: &crate::tree::WidgetTree,
        element: crate::tree::ElementId,
    ) -> Option<FocusScopeNode> {
        let mut ancestor = tree.parent(element);
        while let Some(id) = ancestor {
            if tree.focus_boundary_at(id).is_some() {
                return tree.scope_of_focus_boundary(id);
            }
            ancestor = tree.parent(id);
        }
        None
    }

    /// Binds a scope node and applies any explicitly supplied widget settings.
    #[must_use]
    pub fn node(mut self, node: FocusScopeNode) -> Self {
        self.node = Some(node);
        self.use_external_properties = false;
        self
    }

    /// A scope whose node owns both its callbacks and its permissions.
    #[must_use]
    pub fn with_external_focus_node(child: impl Into<Widget>, node: FocusScopeNode) -> Self {
        Self {
            node: Some(node),
            use_external_properties: true,
            ..Self::new(child)
        }
    }

    #[must_use]
    pub fn key_event_handler(&self) -> Option<incular_gestures::FocusKeyEventHandler> {
        self.node.as_ref()?.scope_focus_node().on_key_event()
    }

    #[must_use]
    pub fn raw_key_handler(&self) -> Option<incular_gestures::FocusRawKeyEventHandler> {
        self.node.as_ref()?.scope_focus_node().on_key()
    }

    #[must_use]
    pub fn allows_focus_requests(&self) -> bool {
        self.scope_widget_properties()
            .and_then(|p| p.can_request_focus)
            .unwrap_or_else(|| {
                self.node
                    .as_ref()
                    .is_none_or(FocusScopeNode::can_request_focus)
            })
    }

    #[must_use]
    pub fn allows_descendant_focus(&self) -> bool {
        self.scope_widget_properties()
            .and_then(|p| p.descendants_are_focusable)
            .unwrap_or_else(|| {
                self.node
                    .as_ref()
                    .is_none_or(FocusScopeNode::descendants_are_focusable)
            })
    }

    #[must_use]
    pub fn allows_descendant_traversal(&self) -> bool {
        self.scope_widget_properties()
            .and_then(|p| p.descendants_are_traversable)
            .unwrap_or_else(|| {
                self.node
                    .as_ref()
                    .is_none_or(FocusScopeNode::descendants_are_traversable)
            })
    }

    #[must_use]
    pub fn skips_traversal(&self) -> bool {
        self.scope_widget_properties()
            .and_then(|p| p.skip_traversal)
            .unwrap_or_else(|| {
                self.node
                    .as_ref()
                    .is_some_and(FocusScopeNode::skip_traversal)
            })
    }

    /// Controls focus eligibility inside this scope. A disabled scope blocks
    /// descendants without changing their own focus settings.
    #[must_use]
    pub fn can_request_focus(mut self, allowed: bool) -> Self {
        self.can_request_focus = Some(allowed);
        self
    }

    /// Excludes this scope's own node from traversal, not its descendants.
    #[must_use]
    pub fn skip_traversal(mut self, skip: bool) -> Self {
        self.skip_traversal = Some(skip);
        self
    }

    /// Controls whether descendants may request focus without disabling the scope.
    #[must_use]
    pub fn descendants_are_focusable(mut self, allowed: bool) -> Self {
        self.descendants_are_focusable = Some(allowed);
        self
    }

    /// Controls traversal to descendants, not their ability to request focus.
    #[must_use]
    pub fn descendants_are_traversable(mut self, allowed: bool) -> Self {
        self.descendants_are_traversable = Some(allowed);
        self
    }

    fn scope_widget_properties(&self) -> Option<incular_gestures::FocusNodeWidgetProperties> {
        let owned = self.node.is_none();
        (!self.use_external_properties).then_some(incular_gestures::FocusNodeWidgetProperties {
            can_request_focus: self.can_request_focus.or(owned.then_some(true)),
            skip_traversal: self.skip_traversal.or(owned.then_some(false)),
            descendants_are_focusable: self.descendants_are_focusable.or(owned.then_some(true)),
            descendants_are_traversable: self.descendants_are_traversable.or(owned.then_some(true)),
        })
    }

    /// Uses an explicit parent in the mounted focus tree. Pass
    /// `scope.scope_focus_node()` when the parent is another scope.
    #[must_use]
    pub fn parent_node(mut self, parent: impl Into<Option<FocusNode>>) -> Self {
        self.parent_node = parent.into();
        self
    }

    /// Automatically requests focus for its primary descendant on mount.
    #[must_use]
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    /// Reports when focus enters or leaves this scope, including its
    /// descendants. Moving between children of an already focused scope
    /// does not report a second focus transition.
    #[must_use]
    pub fn on_focus_change(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_focus_change = Some(Rc::new(callback));
        self
    }

    /// Selects the traversal policy for this scope's descendants.
    #[must_use]
    pub fn policy(mut self, policy: FocusTraversalPolicyKind) -> Self {
        self.policy = Some(policy);
        self
    }

    #[must_use]
    pub fn reading_order(self) -> Self {
        self.policy(FocusTraversalPolicyKind::ReadingOrder)
    }

    #[must_use]
    pub fn widget_order(self) -> Self {
        self.policy(FocusTraversalPolicyKind::WidgetOrder)
    }

    #[must_use]
    pub fn ordered(self) -> Self {
        self.policy(FocusTraversalPolicyKind::Ordered)
    }

    /// Uses widget order as the secondary policy for equal or absent
    /// explicit focus orders. [Self::ordered] defaults to reading order.
    #[must_use]
    pub fn ordered_with_widget_order(self) -> Self {
        self.policy(FocusTraversalPolicyKind::OrderedWithWidgetOrder)
    }
}

impl From<FocusScope> for Widget {
    fn from(value: FocusScope) -> Self {
        let properties = value.scope_widget_properties();
        let child = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        let scope = if let Some(node) = &value.node {
            let mut focus = Focus::with_external_focus_node(child, node.scope_focus_node())
                .parent_node(value.parent_node)
                .include_semantics(false);
            focus.on_focus_change = value.on_focus_change;
            let mut widget = Widget::from(focus);
            if let WidgetKind::Gesture { callbacks, .. } = widget.kind_mut() {
                callbacks.focus_scope = Some(node.clone());
                callbacks.focus_node_widget_properties = properties;
                callbacks.focus_debug_label = value.debug_label;
            }
            widget
        } else {
            // Flutter always mounts a scope node, even without a supplied
            // node or explicit focus permission. Mount-owned descriptors
            // retain that identity and its focused-child history on rebuild.
            let mut focus = Focus::new(child)
                .parent_node(value.parent_node)
                .can_request_focus(value.can_request_focus.unwrap_or(true))
                .include_semantics(false);
            focus.on_focus_change = value.on_focus_change;
            let mut widget = Widget::from(focus);
            if let WidgetKind::Gesture { callbacks, .. } = widget.kind_mut() {
                callbacks.owns_focus_scope = true;
                callbacks.focus_node_widget_properties = properties;
                callbacks.focus_debug_label = value.debug_label;
            }
            widget
        };
        let scope = if value.include_semantics {
            crate::Semantics::new(scope)
                .explicit_child_nodes(true)
                .focusable(false)
                .into()
        } else {
            scope
        };
        let scope = scope.with_focus_scope_autofocus(value.autofocus);
        match value.policy {
            Some(policy) => scope.with_focus_traversal_policy(policy),
            None => scope,
        }
    }
}

/// The standard activation intent for focused controls. WidgetsApp binds
/// the platform's supported logical activation keys to this command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ActivateIntent;

type RequestFocusCallback = Rc<dyn Fn(&FocusNode)>;

/// Requests focus for a specific node. The node identifies the command for
/// an [`Actions`] mapping; the optional callback is invocation data.
#[derive(Clone)]
pub struct RequestFocusIntent {
    focus_node: FocusNode,
    callback: Option<RequestFocusCallback>,
}

impl RequestFocusIntent {
    #[must_use]
    pub fn new(focus_node: FocusNode) -> Self {
        Self {
            focus_node,
            callback: None,
        }
    }

    #[must_use]
    pub fn focus_node(&self) -> &FocusNode {
        &self.focus_node
    }

    /// Replaces the default focus-and-reveal operation, rather than running
    /// in addition to it. The callback receives the target node.
    #[must_use]
    pub fn request_focus_callback(mut self, callback: impl Fn(&FocusNode) + 'static) -> Self {
        self.callback = Some(Rc::new(callback));
        self
    }
}

impl PartialEq for RequestFocusIntent {
    fn eq(&self, other: &Self) -> bool {
        self.focus_node == other.focus_node
    }
}

impl Eq for RequestFocusIntent {}

impl std::hash::Hash for RequestFocusIntent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.focus_node, state);
    }
}

impl std::fmt::Debug for RequestFocusIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestFocusIntent")
            .field("focus_node", &self.focus_node)
            .field("has_callback", &self.callback.is_some())
            .finish()
    }
}

/// The default mounted [`RequestFocusIntent`] action supplied by WidgetsApp.
#[derive(Clone, Copy, Debug, Default)]
pub struct RequestFocusAction;

impl RequestFocusAction {
    /// Invokes the callback or requests focus and immediately aligns the
    /// target's trailing edge with each enclosing scroll viewport, limited
    /// by the scroll range. Call after layout to reveal the target.
    pub fn invoke(self, tree: &crate::internal::WidgetTree, intent: &RequestFocusIntent) {
        if let Some(callback) = &intent.callback {
            callback(&intent.focus_node);
        } else {
            tree.request_and_reveal_focus_node(&intent.focus_node);
        }
    }
}

/// Requests spatial keyboard focus traversal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DirectionalFocusIntent {
    pub direction: incular_gestures::TraversalDirection,
    pub ignore_text_fields: bool,
}

impl DirectionalFocusIntent {
    #[must_use]
    pub const fn new(direction: incular_gestures::TraversalDirection) -> Self {
        Self { direction, ignore_text_fields: true }
    }

    /// Set false to traverse away from an active editable control.
    #[must_use]
    pub const fn ignore_text_fields(mut self, ignore: bool) -> Self {
        self.ignore_text_fields = ignore;
        self
    }
}

/// A widget that listens for keyboard events routed from platform input.
#[derive(Clone)]
pub struct KeyboardListener {
    on_key: Option<Rc<dyn Fn(KeyboardEvent) -> bool>>,
    on_key_down: Option<Rc<dyn Fn(KeyboardEvent)>>,
    on_key_repeat: Option<Rc<dyn Fn(KeyboardEvent)>>,
    on_key_up: Option<Rc<dyn Fn(KeyboardEvent)>>,
    shortcut_dispatch: Option<Rc<dyn Fn(KeyboardEvent) -> bool>>,
    shortcut_scope: Option<Rc<incular_gestures::ErasedShortcutScope>>,
    focus_node: Option<FocusNode>,
    autofocus: bool,
    include_semantics: bool,
    child: Option<Widget>,
}

impl Default for KeyboardListener {
    fn default() -> Self {
        Self::new(crate::SizedBox::shrink())
    }
}

impl KeyboardListener {
    /// Creates a KeyboardListener wrapping a child widget.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            on_key: None,
            on_key_down: None,
            on_key_repeat: None,
            on_key_up: None,
            shortcut_dispatch: None,
            shortcut_scope: None,
            focus_node: None,
            autofocus: false,
            include_semantics: true,
            child: Some(child.into()),
        }
    }

    /// Installs an event handler for the complete key stream. Returning
    /// `true` consumes the event; returning `false` lets a parent shortcut or
    /// text editor continue handling it.
    #[must_use]
    pub fn on_key(mut self, callback: impl Fn(KeyboardEvent) -> bool + 'static) -> Self {
        self.on_key = Some(Rc::new(callback));
        self
    }

    /// Alias for [`Self::on_key`] using the platform-neutral event naming.
    #[must_use]
    pub fn on_key_event(self, callback: impl Fn(KeyboardEvent) -> bool + 'static) -> Self {
        self.on_key(callback)
    }

    /// Sets the key-down event handler.
    #[must_use]
    pub fn on_key_down(mut self, callback: impl Fn(KeyboardEvent) + 'static) -> Self {
        self.on_key_down = Some(Rc::new(callback));
        self
    }

    /// Sets the auto-repeat key-down handler. This is called only when
    /// [`KeyboardEvent::repeat`] is true.
    #[must_use]
    pub fn on_key_repeat(mut self, callback: impl Fn(KeyboardEvent) + 'static) -> Self {
        self.on_key_repeat = Some(Rc::new(callback));
        self
    }

    /// Sets the key-up event handler.
    #[must_use]
    pub fn on_key_up(mut self, callback: impl Fn(KeyboardEvent) + 'static) -> Self {
        self.on_key_up = Some(Rc::new(callback));
        self
    }

    /// Connects this listener to a typed shortcut/action scope.
    ///
    /// The registries remain ordinary Rust values owned by the application;
    /// the retained widget stores only a type-erased event dispatcher. This
    /// keeps the runtime independent from every command enum while allowing
    /// real platform key events to reach [`Shortcuts::handle_actions`].
    #[must_use]
    pub fn with_shortcuts<C: CommandId>(
        mut self,
        shortcuts: Rc<Shortcuts<C>>,
        actions: Rc<Actions<C>>,
    ) -> Self {
        self.shortcut_scope = Some(shortcuts.erased_scope());
        self.shortcut_dispatch = Some(Rc::new(move |event| {
            shortcuts.handle_actions(event, &actions)
        }));
        self
    }

    /// Uses an existing focus node for this listener.
    #[must_use]
    pub fn focus_node(mut self, node: FocusNode) -> Self {
        self.focus_node = Some(node);
        self
    }

    /// Alias matching the focus-node builder used by [`Focus`].
    #[must_use]
    pub fn node(self, node: FocusNode) -> Self {
        self.focus_node(node)
    }

    /// Requests the configured focus node when this listener is mounted.
    #[must_use]
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    /// Controls whether this listener contributes its keyboard affordance to
    /// the semantic tree.
    #[must_use]
    pub fn include_semantics(mut self, include: bool) -> Self {
        self.include_semantics = include;
        self
    }

    /// Returns the configured focus node, if any.
    #[must_use]
    pub fn configured_focus_node(&self) -> Option<FocusNode> {
        self.focus_node.clone()
    }

    /// Returns whether autofocus is enabled.
    #[must_use]
    pub const fn is_autofocus(&self) -> bool {
        self.autofocus
    }

    /// Returns whether semantics are included.
    #[must_use]
    pub const fn includes_semantics(&self) -> bool {
        self.include_semantics
    }

    /// Dispatches one keyboard event through the listener callbacks.
    ///
    /// The generic `on_key` callback gets first refusal. The phase-specific
    /// callback then runs for its matching event, with repeat events exposed
    /// separately in addition to the normal key-down callback.
    #[must_use]
    pub fn handle(&self, event: KeyboardEvent) -> bool {
        self.to_callbacks().handle_keyboard(event)
    }
}

/// Observes a mounted action's state changes, with optional Incular
/// invocation callbacks. Replacing the action or unmounting the
/// listener immediately removes the previous registration.
#[derive(Clone)]
pub struct ActionListener<C: CommandId = Command> {
    action: Action<C>,
    listener: Option<Rc<ActionStateCallback<C>>>,
    on_action: Option<Rc<dyn Fn()>>,
    on_action_start: Option<Rc<dyn Fn()>>,
    on_action_end: Option<Rc<dyn Fn()>>,
    child: Widget,
}

type ActionStateCallback<C> = dyn Fn(&Action<C>);

impl<C: CommandId> ActionListener<C> {
    #[must_use]
    pub fn new(action: Action<C>, child: impl Into<Widget>) -> Self {
        Self {
            action,
            listener: None,
            on_action: None,
            on_action_start: None,
            on_action_end: None,
            child: child.into(),
        }
    }

    /// Registers for action-state notifications after mounting. The
    /// callback can read the action's new enabled state and is not invoked
    /// simply because the widget was mounted or an action was invoked.
    #[must_use]
    pub fn listener(mut self, callback: impl Fn(&Action<C>) + 'static) -> Self {
        self.listener = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_action(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_action = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_action_start(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_action_start = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_action_end(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_action_end = Some(Rc::new(callback));
        self
    }
}

impl<C: CommandId> From<ActionListener<C>> for Widget {
    fn from(value: ActionListener<C>) -> Self {
        let ActionListener {
            action,
            listener,
            on_action,
            on_action_start,
            on_action_end,
            child,
        } = value;
        let mount_action_listener = Rc::new(move || {
            let state = listener.clone().map(|listener| {
                let observed = action.clone();
                action.add_action_listener(move || listener(&observed))
            });
            let invocation = (on_action.is_some() || on_action_start.is_some() || on_action_end.is_some())
                .then(|| {
                    let on_action = on_action.clone();
                    let on_action_start = on_action_start.clone();
                    let on_action_end = on_action_end.clone();
                    action.add_invocation_listener(move |phase| match phase {
                        ActionInvocationPhase::Start => {
                            if let Some(callback) = &on_action_start {
                                callback();
                            }
                        }
                        ActionInvocationPhase::End => {
                            if let Some(callback) = &on_action {
                                callback();
                            }
                            if let Some(callback) = &on_action_end {
                                callback();
                            }
                        }
                    })
                });
            (state, invocation)
        });
        Widget::from_kind(WidgetKind::Gesture {
            behavior: crate::gestures::HitTestBehavior::DeferToChild,
            callbacks: Box::new(incular_gestures::GestureCallbacks {
                mount_action_listener: Some(mount_action_listener),
                include_semantics: false,
                ..Default::default()
            }),
            child,
        })
    }
}

/// A local keyboard shortcut map whose values are callbacks rather than
/// application actions.
#[derive(Clone)]
pub struct CallbackShortcuts {
    bindings: Vec<(ShortcutActivator, Rc<dyn Fn()>)>,
    child: Widget,
}

impl CallbackShortcuts {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            bindings: Vec::new(),
            child: child.into(),
        }
    }

    #[must_use]
    pub fn shortcut(
        mut self,
        activator: impl Into<ShortcutActivator>,
        callback: impl Fn() + 'static,
    ) -> Self {
        let activator = activator.into();
        if let Some((_, existing)) = self.bindings.iter_mut().find(|(key, _)| key == &activator) {
            *existing = Rc::new(callback);
        } else {
            self.bindings.push((activator, Rc::new(callback)));
        }
        self
    }

    #[must_use]
    pub fn shortcuts<I, A, F>(mut self, bindings: I) -> Self
    where
        I: IntoIterator<Item = (A, F)>,
        A: Into<ShortcutActivator>,
        F: Fn() + 'static,
    {
        for (activator, callback) in bindings {
            self = self.shortcut(activator, callback);
        }
        self
    }
}

impl From<CallbackShortcuts> for Widget {
    fn from(value: CallbackShortcuts) -> Self {
        let bindings = value.bindings;
        let callbacks = incular_gestures::GestureCallbacks {
            on_shortcut_with_keyboard_state: Some(Rc::new(move |event, lock_modes, keyboard| {
                let mut handled = false;
                for (activator, callback) in &bindings {
                    if activator.accepts_with_keyboard_state(&event, lock_modes, keyboard) {
                        callback();
                        handled = true;
                    }
                }
                handled
            })),
            ..incular_gestures::GestureCallbacks::default()
        };
        passive_shortcut_boundary(value.child, callbacks, None, true, true)
    }
}

/// Combines focus, focus-highlight visibility, shortcuts, and actions around a
/// child. It is the retained equivalent of Flutter's
/// `FocusableActionDetector`.
#[derive(Clone)]
pub struct FocusableActionDetector {
    focus_node: FocusNode,
    enabled: bool,
    autofocus: bool,
    skip_traversal: Option<bool>,
    descendants_are_focusable: Option<bool>,
    descendants_are_traversable: Option<bool>,
    include_focus_semantics: bool,
    mouse_cursor: crate::MouseCursor,
    shortcuts: Option<Rc<incular_gestures::ErasedShortcutScope>>,
    actions: Option<Rc<incular_gestures::ErasedActionScope>>,
    on_focus_change: Option<Rc<dyn Fn(bool)>>,
    on_show_focus_highlight: Option<Rc<dyn Fn(bool)>>,
    on_show_hover_highlight: Option<Rc<dyn Fn(bool)>>,
    child: Widget,
}

impl FocusableActionDetector {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            focus_node: FocusNode::new(),
            enabled: true,
            autofocus: false,
            skip_traversal: None,
            descendants_are_focusable: None,
            descendants_are_traversable: None,
            include_focus_semantics: true,
            mouse_cursor: crate::MouseCursor::default(),
            shortcuts: None,
            actions: None,
            on_focus_change: None,
            on_show_focus_highlight: None,
            on_show_hover_highlight: None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn focus_node(mut self, node: FocusNode) -> Self {
        self.focus_node = node;
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    /// Skips this detector in traversal while keeping explicit focus allowed.
    #[must_use]
    pub fn skip_traversal(mut self, skip: bool) -> Self {
        self.skip_traversal = Some(skip);
        self
    }

    /// Whether this detector is skipped in traversal.
    #[must_use]
    pub fn skips_traversal(&self) -> bool {
        self.skip_traversal.unwrap_or_else(|| self.focus_node.skip_traversal())
    }

    /// Controls whether descendants can receive either explicit or traversal focus.
    #[must_use]
    pub fn descendants_are_focusable(mut self, focusable: bool) -> Self {
        self.descendants_are_focusable = Some(focusable);
        self
    }

    /// Controls descendant traversal without blocking explicit focus requests.
    #[must_use]
    pub fn descendants_are_traversable(mut self, traversable: bool) -> Self {
        self.descendants_are_traversable = Some(traversable);
        self
    }

    /// Controls the focus contribution without removing child semantics.
    #[must_use]
    pub fn include_focus_semantics(mut self, include: bool) -> Self {
        self.include_focus_semantics = include;
        self
    }

    #[must_use]
    pub const fn includes_focus_semantics(&self) -> bool {
        self.include_focus_semantics
    }

    /// Cursor to use over the detector. By default it defers to ancestors.
    #[must_use]
    pub fn mouse_cursor(mut self, cursor: crate::MouseCursor) -> Self {
        self.mouse_cursor = cursor;
        self
    }

    #[must_use]
    pub fn shortcuts<C: CommandId>(mut self, shortcuts: Rc<Shortcuts<C>>) -> Self {
        self.shortcuts = Some(shortcuts.erased_scope());
        self
    }

    #[must_use]
    pub fn actions<C: CommandId>(mut self, actions: Rc<Actions<C>>) -> Self {
        self.actions = Some(incular_gestures::ErasedActionScope::from_actions(actions));
        self
    }

    #[must_use]
    pub fn on_focus_change(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_focus_change = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_show_focus_highlight(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_show_focus_highlight = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_show_hover_highlight(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_show_hover_highlight = Some(Rc::new(callback));
        self
    }
}

impl From<FocusableActionDetector> for Widget {
    fn from(value: FocusableActionDetector) -> Self {
        let behavior = FocusBehavior::for_detector(
            value.focus_node.clone(),
            value.enabled,
            value.on_focus_change.clone(),
            value.on_show_focus_highlight.clone(),
            value.on_show_hover_highlight.clone(),
        );
        let callbacks = incular_gestures::GestureCallbacks {
            shortcut_scope: value.shortcuts.filter(|_| value.enabled),
            action_scope: value.actions.filter(|_| value.enabled),
            focus_node: Some(value.focus_node),
            focus_behavior: Some(behavior),
            autofocus: value.autofocus,
            include_semantics: value.include_focus_semantics,
            annotates_focus: true,
            focus_node_widget_properties: Some(incular_gestures::FocusNodeWidgetProperties {
                skip_traversal: value.skip_traversal,
                descendants_are_focusable: value.descendants_are_focusable,
                descendants_are_traversable: value.descendants_are_traversable,
                ..Default::default()
            }),
            ..incular_gestures::GestureCallbacks::default()
        };
        Widget::from_kind(WidgetKind::Gesture {
            behavior: crate::gestures::HitTestBehavior::DeferToChild,
            callbacks: Box::new(callbacks),
            child: crate::MouseRegion::new(value.child)
                .cursor(value.mouse_cursor)
                .into(),
        })
    }
}

impl From<KeyboardListener> for Widget {
    fn from(value: KeyboardListener) -> Self {
        let callbacks = value.to_callbacks();
        let child = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        Widget::from_kind(WidgetKind::Gesture {
            behavior: crate::gestures::HitTestBehavior::DeferToChild,
            callbacks: Box::new(callbacks),
            child,
        })
    }
}

impl KeyboardListener {
    fn to_callbacks(&self) -> incular_gestures::GestureCallbacks {
        incular_gestures::GestureCallbacks {
            on_key: self.on_key.clone(),
            on_key_down: self.on_key_down.clone(),
            on_key_repeat: self.on_key_repeat.clone(),
            on_key_up: self.on_key_up.clone(),
            on_shortcut: self.shortcut_dispatch.clone(),
            shortcut_scope: self.shortcut_scope.clone(),
            // The Flutter form has an explicit focus node. Incular also
            // supports node-less listeners that only route descendant keys.
            annotates_focus: self.focus_node.is_some(),
            focus_node: self.focus_node.clone(),
            autofocus: self.autofocus,
            include_semantics: self.include_semantics,
            ..incular_gestures::GestureCallbacks::default()
        }
    }
}

/// A focus-bound keyboard observer (Flutter's legacy RawKeyboardListener).
///
/// Rust receives the normalized platform event here; observing it never
/// consumes it, and semantics can be disabled without disabling focus.
#[derive(Clone)]
pub struct RawKeyboardListener(KeyboardListener);

impl RawKeyboardListener {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self(KeyboardListener::new(child))
    }

    #[must_use]
    pub fn focus_node(self, node: FocusNode) -> Self {
        Self(self.0.focus_node(node))
    }

    #[must_use]
    pub fn autofocus(self, autofocus: bool) -> Self {
        Self(self.0.autofocus(autofocus))
    }

    #[must_use]
    pub fn include_semantics(self, include: bool) -> Self {
        Self(self.0.include_semantics(include))
    }

    #[must_use]
    pub fn on_key(self, callback: impl Fn(KeyboardEvent) + 'static) -> Self {
        Self(self.0.on_key_event(move |event| {
            callback(event);
            false
        }))
    }
}

impl From<RawKeyboardListener> for Widget {
    fn from(value: RawKeyboardListener) -> Self {
        let mut callbacks = value.0.to_callbacks();
        callbacks.annotates_focus = true;
        callbacks.owned_focus_permission = callbacks.focus_node.is_none().then_some(true);
        let child = value.0.child.unwrap_or_else(|| crate::SizedBox::shrink().into());
        Widget::from_kind(WidgetKind::Gesture {
            behavior: crate::gestures::HitTestBehavior::DeferToChild,
            callbacks: Box::new(callbacks),
            child,
        })
    }
}

/// A visible, eligible member or nested group supplied to a custom sorter.
#[derive(Clone, Debug, PartialEq)]
pub struct FocusTraversalCandidate {
    /// An explicit or widget-owned node, if this member exposes one.
    pub node: Option<FocusNode>,
    /// Logical bounds in the mounted view.
    pub bounds: incular_core::Rect,
    /// A nested group is kept together and sorts its own descendants.
    pub is_group: bool,
}

type FocusSort = dyn Fn(&[FocusTraversalCandidate], Option<&FocusNode>) -> Vec<usize>;

/// A custom sequential policy. Return candidate indices in traversal order,
/// omitting any stops to skip. Returning an index twice or outside the supplied
/// candidates does not add a stop. The current node is supplied even if a
/// caller wants to exclude it from future traversal.
#[derive(Clone)]
pub struct CustomFocusTraversalPolicy(Rc<FocusSort>);

impl CustomFocusTraversalPolicy {
    #[must_use]
    pub fn new(
        sort: impl Fn(&[FocusTraversalCandidate], Option<&FocusNode>) -> Vec<usize> + 'static,
    ) -> Self {
        Self(Rc::new(sort))
    }

    pub(crate) fn sort(
        &self,
        candidates: &[FocusTraversalCandidate],
        current: Option<&FocusNode>,
    ) -> Vec<usize> {
        (self.0)(candidates, current)
    }
}

impl std::fmt::Debug for CustomFocusTraversalPolicy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CustomFocusTraversalPolicy(..)")
    }
}

impl PartialEq for CustomFocusTraversalPolicy {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// A focus request picked by traversal. The node is absent only for an
/// implicit native control that does not expose a FocusNode.
#[derive(Clone, Debug, PartialEq)]
pub struct FocusTraversalRequest {
    pub node: Option<FocusNode>,
    pub alignment_policy: crate::ScrollPositionAlignmentPolicy,
}

#[derive(Clone)]
pub(crate) struct CustomFocusRequest(pub(crate) Rc<dyn Fn(FocusTraversalRequest)>);

impl std::fmt::Debug for CustomFocusRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CustomFocusRequest(..)")
    }
}

impl PartialEq for CustomFocusRequest {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Establishes a focus traversal policy group for its descendants.
#[derive(Clone)]
pub struct FocusTraversalGroup {
    policy: FocusTraversalPolicyKind,
    custom_policy: Option<CustomFocusTraversalPolicy>,
    request_focus_callback: Option<CustomFocusRequest>,
    descendants_are_focusable: bool,
    descendants_are_traversable: bool,
    parent_node: Option<FocusNode>,
    on_focus_node_created: Option<Rc<dyn Fn(FocusNode)>>,
    child: Option<Widget>,
}

impl Default for FocusTraversalGroup {
    fn default() -> Self {
        Self {
            policy: FocusTraversalPolicyKind::ReadingOrder,
            custom_policy: None,
            request_focus_callback: None,
            descendants_are_focusable: true,
            descendants_are_traversable: true,
            parent_node: None,
            on_focus_node_created: None,
            child: None,
        }
    }
}

impl FocusTraversalGroup {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            policy: FocusTraversalPolicyKind::ReadingOrder,
            custom_policy: None,
            request_focus_callback: None,
            descendants_are_focusable: true,
            descendants_are_traversable: true,
            parent_node: None,
            on_focus_node_created: None,
            child: Some(child.into()),
        }
    }

    /// Flutter `descendantsAreFocusable` (default true): when false, nothing
    /// inside the group can take focus.
    #[must_use]
    pub fn descendants_are_focusable(mut self, focusable: bool) -> Self {
        self.descendants_are_focusable = focusable;
        self
    }

    /// Flutter `descendantsAreTraversable` (default true): when false,
    /// traversal skips the group's descendants, which can still be focused
    /// directly.
    #[must_use]
    pub fn descendants_are_traversable(mut self, traversable: bool) -> Self {
        self.descendants_are_traversable = traversable;
        self
    }

    /// Installs an explicit parent for the group's retained focus node.
    #[must_use]
    pub fn parent_node(mut self, parent: impl Into<Option<FocusNode>>) -> Self {
        self.parent_node = parent.into();
        self
    }

    /// Called once per mount when the group creates its focus node.
    #[must_use]
    pub fn on_focus_node_created(mut self, callback: impl Fn(FocusNode) + 'static) -> Self {
        self.on_focus_node_created = Some(Rc::new(callback));
        self
    }

    /// Replaces traversal's default focus-and-reveal operation. Calling
    /// `request_focus` on the supplied node requests focus without
    /// accepting the default reveal behavior.
    #[must_use]
    pub fn request_focus_callback(
        mut self,
        callback: impl Fn(FocusTraversalRequest) + 'static,
    ) -> Self {
        self.request_focus_callback = Some(CustomFocusRequest(Rc::new(callback)));
        self
    }

    /// Installs a sorter for next/previous traversal. Explicit requests
    /// and directional spatial focus still use the eligible node set.
    #[must_use]
    pub fn custom_policy(mut self, policy: CustomFocusTraversalPolicy) -> Self {
        self.custom_policy = Some(policy);
        self
    }

    /// Reads the nearest mounted focus group's custom policy. An explicitly
    /// reparented focus subtree uses its focus ancestry, not visual nesting.
    #[must_use]
    pub fn maybe_custom_policy_of(
        tree: &crate::tree::WidgetTree,
        element: crate::tree::ElementId,
    ) -> Option<CustomFocusTraversalPolicy> {
        tree.custom_focus_policy_of(element)
    }

    /// Selects the policy used for descendants of this group.
    #[must_use]
    pub fn policy(mut self, policy: FocusTraversalPolicyKind) -> Self {
        self.policy = policy;
        self.custom_policy = None;
        self
    }

    #[must_use]
    pub fn reading_order(self) -> Self {
        self.policy(FocusTraversalPolicyKind::ReadingOrder)
    }

    #[must_use]
    pub fn widget_order(self) -> Self {
        self.policy(FocusTraversalPolicyKind::WidgetOrder)
    }

    #[must_use]
    pub fn ordered(self) -> Self {
        self.policy(FocusTraversalPolicyKind::Ordered)
    }

    /// Uses widget order as the secondary policy for equal or absent
    /// explicit focus orders. [Self::ordered] defaults to reading order.
    #[must_use]
    pub fn ordered_with_widget_order(self) -> Self {
        self.policy(FocusTraversalPolicyKind::OrderedWithWidgetOrder)
    }
}

impl From<FocusTraversalGroup> for Widget {
    fn from(value: FocusTraversalGroup) -> Self {
        let policy = value.policy;
        let child = value.child.unwrap_or_else(|| crate::SizedBox::shrink().into());
        Widget::from(
            Focus::new(child)
                .can_request_focus(false)
                .skip_traversal(true)
                .include_semantics(false)
                .descendants_are_focusable(value.descendants_are_focusable)
                .descendants_are_traversable(value.descendants_are_traversable)
                .parent_node(value.parent_node),
        )
        .with_focus_traversal_policy(policy)
        .with_owned_focus_group(
            value.on_focus_node_created,
            value.custom_policy,
            value.request_focus_callback,
        )
    }
}

/// Customizes the focus traversal ordering of a widget.
#[derive(Clone, Default)]
pub struct FocusTraversalOrder {
    order: Option<incular_gestures::FocusOrder>,
    child: Option<Widget>,
}

impl FocusTraversalOrder {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            order: None,
            child: Some(child.into()),
        }
    }

    /// Supplies the explicit numeric order used by an ordered traversal
    /// group. Equal orders retain widget order.
    #[must_use]
    pub fn order(mut self, order: f64) -> Self {
        self.order = order
            .is_finite()
            .then_some(incular_gestures::FocusOrder::Numeric(order));
        self
    }

    /// Flutter's `FocusTraversalOrder(order: NumericFocusOrder(..))` or
    /// `LexicalFocusOrder(..)`.
    #[must_use]
    pub fn focus_order(mut self, order: incular_gestures::FocusOrder) -> Self {
        self.order = order.is_valid().then_some(order);
        self
    }

    /// Flutter's `LexicalFocusOrder`.
    #[must_use]
    pub fn lexical(self, order: impl Into<String>) -> Self {
        self.focus_order(incular_gestures::FocusOrder::Lexical(order.into()))
    }
}

impl From<FocusTraversalOrder> for Widget {
    fn from(value: FocusTraversalOrder) -> Self {
        let order = value.order;
        value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into())
            .with_focus_traversal_order(order)
    }
}

/// Excludes a subtree from receiving focus.
#[derive(Clone, Debug, PartialEq)]
pub struct ExcludeFocus {
    excluding: bool,
    child: Widget,
}

impl ExcludeFocus {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            excluding: true,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn excluding(mut self, excluding: bool) -> Self {
        self.excluding = excluding;
        self
    }
}

impl From<ExcludeFocus> for Widget {
    fn from(value: ExcludeFocus) -> Self {
        value.child.with_excluded_focus(value.excluding)
    }
}

/// Excludes a subtree from tab traversal while still allowing direct programmatic focus.
#[derive(Clone, Debug, PartialEq)]
pub struct ExcludeFocusTraversal {
    excluding: bool,
    child: Widget,
}

impl ExcludeFocusTraversal {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            excluding: true,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn excluding(mut self, excluding: bool) -> Self {
        self.excluding = excluding;
        self
    }
}

impl From<ExcludeFocusTraversal> for Widget {
    fn from(value: ExcludeFocusTraversal) -> Self {
        value.child.with_excluded_focus_traversal(value.excluding)
    }
}
