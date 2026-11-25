//! Focus, focus traversal, and raw keyboard listener widgets.

use std::rc::Rc;

use incular_core::KeyboardEvent;
pub use incular_gestures::{
    Action, ActionInvocationPhase, ActionResult, Actions, Command, CommandId, FocusBehavior,
    FocusManager, FocusNode, FocusOrder, FocusScopeNode, FocusScopeSubscription, UnfocusDisposition,
    FocusTraversalPolicy, FocusTraversalPolicyKind, Intent, KeyEventResult, LockState,
    LogicalKeySet, LogicalShortcutKey, OrderedTraversalPolicy, ReadingOrderTraversalPolicy,
    ShortcutActivator, ShortcutKey, ShortcutTrigger, Shortcuts, SingleActivator,
    WidgetOrderTraversalPolicy,
};

use crate::{Widget, WidgetKind};

/// A widget that manages keyboard focus for a subtree.
#[derive(Clone)]
pub struct Focus {
    node: Option<FocusNode>,
    autofocus: bool,
    can_request_focus: Option<bool>,
    descendants_are_focusable: bool,
    descendants_are_traversable: bool,
    include_semantics: bool,
    on_key_event: Option<incular_gestures::FocusKeyEventHandler>,
    on_focus_change: Option<Rc<dyn Fn(bool)>>,
    child: Option<Widget>,
}

impl Default for Focus {
    fn default() -> Self {
        Self {
            node: None,
            autofocus: false,
            can_request_focus: None,
            descendants_are_focusable: true,
            descendants_are_traversable: true,
            include_semantics: true,
            on_key_event: None,
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
        self
    }

    /// Automatically requests focus upon initial mount.
    #[must_use]
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
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
        self.descendants_are_focusable = focusable;
        self
    }

    /// Controls traversal to descendants without denying explicit requests.
    #[must_use]
    pub fn descendants_are_traversable(mut self, traversable: bool) -> Self {
        self.descendants_are_traversable = traversable;
        self
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
        mut self,
        handler: impl Fn(&FocusNode, KeyboardEvent) -> KeyEventResult + 'static,
    ) -> Self {
        self.on_key_event = Some(Rc::new(handler));
        self
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
        let mut child = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        if !value.descendants_are_focusable {
            child = child.with_excluded_focus(true);
        }
        if !value.descendants_are_traversable {
            child = child.with_excluded_focus_traversal(true);
        }
        if let Some(node) = &value.node
            && let Some(can_request_focus) = value.can_request_focus
        {
            node.set_can_request_focus(can_request_focus);
        }
        let owned_focus_permission = value
            .node
            .is_none()
            .then_some(value.can_request_focus.unwrap_or(true));
        let callbacks = incular_gestures::GestureCallbacks {
            focus_node: value.node,
            owned_focus_permission,
            autofocus: value.autofocus,
            on_focus_change: value.on_focus_change,
            on_focus_key_event: value.on_key_event,
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
/// Descendants are traversed in reading order by default, as Flutter's
/// `FocusScope` inherits `ReadingOrderTraversalPolicy`; use
/// [`Self::widget_order`] for registration order.
#[derive(Clone)]
pub struct FocusScope {
    node: Option<FocusScopeNode>,
    can_request_focus: Option<bool>,
    autofocus: bool,
    policy: FocusTraversalPolicyKind,
    child: Option<Widget>,
}

impl Default for FocusScope {
    fn default() -> Self {
        Self {
            node: None,
            can_request_focus: None,
            autofocus: false,
            policy: FocusTraversalPolicyKind::ReadingOrder,
            child: None,
        }
    }
}

impl FocusScope {
    /// Creates a new FocusScope.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            node: None,
            can_request_focus: None,
            autofocus: false,
            policy: FocusTraversalPolicyKind::ReadingOrder,
            child: Some(child.into()),
        }
    }

    /// Finds the nearest mounted scope above this retained element.
    /// Returns none when the root has no explicit scope node.
    #[must_use]
    pub fn maybe_of(
        tree: &crate::tree::WidgetTree,
        element: crate::tree::ElementId,
    ) -> Option<FocusScopeNode> {
        let mut ancestor = tree.parent(element);
        while let Some(id) = ancestor {
            if let Some(scope) = tree.focus_node_at(id).and_then(|node| node.scope_owner()) {
                return Some(scope);
            }
            ancestor = tree.parent(id);
        }
        None
    }

    /// Binds an external scope node without changing its configuration.
    #[must_use]
    pub fn node(mut self, node: FocusScopeNode) -> Self {
        self.node = Some(node);
        self
    }

    /// Controls focus eligibility inside this scope. A disabled scope blocks
    /// descendants without changing their own focus settings.
    #[must_use]
    pub fn can_request_focus(mut self, allowed: bool) -> Self {
        self.can_request_focus = Some(allowed);
        self
    }

    /// Automatically requests focus for its primary descendant on mount.
    #[must_use]
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    /// Selects the traversal policy for this scope's descendants.
    #[must_use]
    pub fn policy(mut self, policy: FocusTraversalPolicyKind) -> Self {
        self.policy = policy;
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
}

impl From<FocusScope> for Widget {
    fn from(value: FocusScope) -> Self {
        let child = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        let scope = if let Some(node) = &value.node {
            if let Some(allowed) = value.can_request_focus {
                node.set_can_request_focus(allowed);
            }
            let mut widget = Widget::from(Focus::new(child).node(node.scope_focus_node()));
            if let WidgetKind::Gesture { callbacks, .. } = widget.kind_mut() {
                callbacks.focus_scope = Some(node.clone());
            }
            widget
        } else {
            // Flutter always mounts a scope node, even without a supplied
            // node or explicit focus permission. Mount-owned descriptors
            // retain that identity and its focused-child history on rebuild.
            let focus = Focus::new(child).can_request_focus(value.can_request_focus.unwrap_or(true));
            let mut widget = Widget::from(focus);
            if let WidgetKind::Gesture { callbacks, .. } = widget.kind_mut() {
                callbacks.owns_focus_scope = true;
            }
            widget
        };
        scope
            .with_focus_traversal_policy(value.policy)
            .with_focus_scope_autofocus(value.autofocus)
    }
}

/// A widget that listens for keyboard events routed from platform input.
#[derive(Clone, Default)]
pub struct KeyboardListener {
    on_key: Option<Rc<dyn Fn(KeyboardEvent) -> bool>>,
    on_key_down: Option<Rc<dyn Fn(KeyboardEvent)>>,
    on_key_repeat: Option<Rc<dyn Fn(KeyboardEvent)>>,
    on_key_up: Option<Rc<dyn Fn(KeyboardEvent)>>,
    shortcut_dispatch: Option<Rc<dyn Fn(KeyboardEvent) -> bool>>,
    focus_node: Option<FocusNode>,
    autofocus: bool,
    include_semantics: bool,
    child: Option<Widget>,
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

/// Observes invocations of one action while retaining the child in the
/// focused widget tree.
///
/// The action itself remains the source of truth. The listener registration is
/// stored in the gesture callback record so it survives widget rebuilds and is
/// removed automatically when the retained element is unmounted.
#[derive(Clone)]
pub struct ActionListener {
    action: Action,
    on_action: Option<Rc<dyn Fn()>>,
    on_action_start: Option<Rc<dyn Fn()>>,
    on_action_end: Option<Rc<dyn Fn()>>,
    child: Widget,
}

impl ActionListener {
    #[must_use]
    pub fn new(action: Action, child: impl Into<Widget>) -> Self {
        Self {
            action,
            on_action: None,
            on_action_start: None,
            on_action_end: None,
            child: child.into(),
        }
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

impl From<ActionListener> for Widget {
    fn from(value: ActionListener) -> Self {
        let on_action = value.on_action.clone();
        let on_action_start = value.on_action_start.clone();
        let on_action_end = value.on_action_end.clone();
        let subscription = value
            .action
            .add_invocation_listener(move |phase| match phase {
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
            });
        let callbacks = incular_gestures::GestureCallbacks {
            action_invocation_listener: Some(Rc::new(subscription)),
            include_semantics: false,
            ..incular_gestures::GestureCallbacks::default()
        };
        Widget::from_kind(WidgetKind::Gesture {
            behavior: crate::gestures::HitTestBehavior::DeferToChild,
            callbacks: Box::new(callbacks),
            child: value.child,
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
            on_shortcut_with_lock_modes: Some(Rc::new(move |event, lock_modes| {
                let mut handled = false;
                for (activator, callback) in &bindings {
                    if activator.accepts_with_lock_modes(&event, lock_modes) {
                        callback();
                        handled = true;
                    }
                }
                handled
            })),
            include_semantics: false,
            ..incular_gestures::GestureCallbacks::default()
        };
        Widget::from_kind(WidgetKind::Gesture {
            behavior: crate::gestures::HitTestBehavior::DeferToChild,
            callbacks: Box::new(callbacks),
            child: value.child,
        })
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
        let behavior = FocusBehavior::new(
            value.focus_node.clone(),
            value.enabled,
            value.on_focus_change.clone(),
            value.on_show_focus_highlight.clone(),
            value.on_show_hover_highlight.clone(),
        );
        let shortcut_scope = value.shortcuts.clone();
        let action_scope = value.actions.clone();
        let action_scope_for_shortcut = action_scope.clone();
        let callbacks = incular_gestures::GestureCallbacks {
            on_shortcut: shortcut_scope.clone().map(|scope| {
                let action_scope = action_scope_for_shortcut;
                Rc::new(move |event| {
                    scope.find(event).is_some_and(|matched| match matched {
                        incular_gestures::ShortcutMatch::Callback(callback) => {
                            callback();
                            true
                        }
                        incular_gestures::ShortcutMatch::Intent(intent) => action_scope
                            .as_ref()
                            .and_then(|scope| scope.invoke(&intent))
                            .is_some_and(|result| {
                                result == incular_gestures::ActionResult::Handled
                            }),
                    })
                }) as Rc<dyn Fn(incular_core::KeyboardEvent) -> bool>
            }),
            shortcut_scope,
            action_scope,
            focus_node: Some(value.focus_node),
            focus_behavior: Some(behavior),
            autofocus: value.autofocus,
            include_semantics: true,
            ..incular_gestures::GestureCallbacks::default()
        };
        Widget::from_kind(WidgetKind::Gesture {
            behavior: crate::gestures::HitTestBehavior::DeferToChild,
            callbacks: Box::new(callbacks),
            child: value.child,
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
            focus_node: self.focus_node.clone(),
            autofocus: self.autofocus,
            include_semantics: self.include_semantics,
            ..incular_gestures::GestureCallbacks::default()
        }
    }
}

/// Establishes a focus traversal policy group for its descendants.
#[derive(Clone)]
pub struct FocusTraversalGroup {
    policy: FocusTraversalPolicyKind,
    descendants_are_focusable: bool,
    descendants_are_traversable: bool,
    child: Option<Widget>,
}

impl Default for FocusTraversalGroup {
    fn default() -> Self {
        Self {
            policy: FocusTraversalPolicyKind::ReadingOrder,
            descendants_are_focusable: true,
            descendants_are_traversable: true,
            child: None,
        }
    }
}

impl FocusTraversalGroup {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            policy: FocusTraversalPolicyKind::ReadingOrder,
            descendants_are_focusable: true,
            descendants_are_traversable: true,
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

    /// Selects the policy used for descendants of this group.
    #[must_use]
    pub fn policy(mut self, policy: FocusTraversalPolicyKind) -> Self {
        self.policy = policy;
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
}

impl From<FocusTraversalGroup> for Widget {
    fn from(value: FocusTraversalGroup) -> Self {
        let policy = value.policy;
        let mut child = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        // Only ever add an exclusion: the child may carry its own.
        if !value.descendants_are_focusable {
            child = child.with_excluded_focus(true);
        }
        if !value.descendants_are_traversable {
            child = child.with_excluded_focus_traversal(true);
        }
        child.with_focus_traversal_policy(policy)
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
