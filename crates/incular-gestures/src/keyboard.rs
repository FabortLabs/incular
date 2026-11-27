use std::any::Any;
use std::cell::{Cell, RefCell};

use std::collections::{HashMap, HashSet};

use std::rc::{Rc, Weak};

use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers};

use crate::details::{FocusKeyHandlerOwner, GestureCallbacks};

/// Flutter's result for routing an event through focus handlers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyEventResult {
    /// Run the next ancestor's handlers.
    #[default]
    Ignored,
    /// Stop routing and keep the platform from handling the key.
    Handled,
    /// Stop framework handlers but allow the platform to handle the key.
    SkipRemainingHandlers,
}

/// Input handler for a mounted, typed focus node.
pub type FocusKeyEventHandler = Rc<dyn Fn(&crate::FocusNode, KeyboardEvent) -> KeyEventResult>;

/// The portable fields of Flutter's deprecated `RawKeyEvent`. Incular
/// does not manufacture OS-specific key-table data from a normalized event.
#[derive(Clone, Debug)]
pub struct RawKeyEvent(KeyboardEvent);

impl RawKeyEvent {
    #[must_use]
    pub const fn portable(&self) -> &KeyboardEvent {
        &self.0
    }

    #[must_use]
    pub const fn logical_key(&self) -> &KeyboardKey {
        &self.0.key
    }

    #[must_use]
    pub const fn physical_key(&self) -> Code {
        self.0.code
    }

    #[must_use]
    pub fn is_key_down(&self) -> bool {
        self.0.state.is_down()
    }
}

impl From<KeyboardEvent> for RawKeyEvent {
    fn from(event: KeyboardEvent) -> Self {
        Self(event)
    }
}

/// Deprecated Focus raw-key callback, after the typed callback on that node.
pub type FocusRawKeyEventHandler = Rc<dyn Fn(&crate::FocusNode, RawKeyEvent) -> KeyEventResult>;

/// Combines outcomes from handlers on the same focus node. Handled takes
/// precedence over a request to skip the remaining framework handlers.
pub fn combine_key_event_results(results: impl IntoIterator<Item = KeyEventResult>) -> KeyEventResult {
    let mut combined = KeyEventResult::Ignored;
    for result in results {
        match result {
            KeyEventResult::Handled => return KeyEventResult::Handled,
            KeyEventResult::SkipRemainingHandlers => combined = KeyEventResult::SkipRemainingHandlers,
            KeyEventResult::Ignored => {}
        }
    }
    combined
}

/// Selects the raw counterpart of one normalized key event. Mirroring
/// delays the event clone until a raw callback actually needs it.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum RawKeyDelivery<'a> {
    Absent,
    Mirror,
    Supplied(&'a RawKeyEvent),
}

impl RawKeyDelivery<'_> {
    fn payload(self, event: &KeyboardEvent) -> Option<RawKeyEvent> {
        match self {
            Self::Absent => None,
            Self::Mirror => Some(RawKeyEvent::from(event.clone())),
            Self::Supplied(raw) => Some(raw.clone()),
        }
    }
}

impl GestureCallbacks {
    /// Whether this callback set carries retained keyboard/focus behavior.
    #[must_use]
    pub fn has_keyboard_listener(&self) -> bool {
        self.on_key.is_some()
            || self.on_focus_key_event.is_some()
            || self.on_key_down.is_some()
            || self.on_key_repeat.is_some()
            || self.on_key_up.is_some()
            || self.on_shortcut.is_some()
            || self.on_shortcut_with_lock_modes.is_some()
            || self.shortcut_scope.is_some()
            || self.focus_node.is_some()
    }

    /// Whether this listener services keyboard-derived semantic
    /// increment/decrement. Explicit semantic callbacks execute
    /// independently of this flag; it governs only the keyboard
    /// fallback, and never keyboard dispatch itself. Advertisement
    /// and execution share this predicate so they cannot disagree.
    #[must_use]
    pub fn services_semantic_keys(&self) -> bool {
        self.has_keyboard_listener() && self.include_semantics
    }

    /// Dispatches one keyboard event through the listener callbacks. The
    /// shortcut dispatcher is nearest-scope behavior and gets first refusal;
    /// the generic callback then gets the next opportunity before the
    /// phase-specific callbacks run.
    #[must_use]
    pub fn handle_keyboard(&self, event: KeyboardEvent) -> bool {
        let lock_modes = event.modifiers;
        self.handle_keyboard_with_lock_modes(event, lock_modes)
    }

    /// Routes keyboard shortcuts with a host-provided lock snapshot.
    #[must_use]
    pub fn handle_keyboard_with_lock_modes(
        &self,
        event: KeyboardEvent,
        lock_modes: Modifiers,
    ) -> bool {
        self.route_keyboard_with_lock_modes(event, lock_modes) == KeyEventResult::Handled
    }

    /// Routes a key, distinguishing a consumed event from a request to
    /// stop framework routing while leaving the key to the platform.
    #[must_use]
    pub fn route_keyboard_with_lock_modes(
        &self,
        event: KeyboardEvent,
        lock_modes: Modifiers,
    ) -> KeyEventResult {
        self.route_keyboard_with_raw_payload(event, lock_modes, RawKeyDelivery::Mirror)
    }

    /// Routes one normalized key. A framework-created semantic key has no
    /// raw counterpart and must not call the deprecated raw handler.
    #[doc(hidden)]
    pub fn route_keyboard_with_raw_payload(
        &self,
        event: KeyboardEvent,
        lock_modes: Modifiers,
        raw_delivery: RawKeyDelivery<'_>,
    ) -> KeyEventResult {
        if self
            .on_shortcut_with_lock_modes
            .as_ref()
            .is_some_and(|callback| callback(event.clone(), lock_modes))
        {
            return KeyEventResult::Handled;
        }
        if self
            .on_shortcut
            .as_ref()
            .is_some_and(|callback| callback(event.clone()))
        {
            return KeyEventResult::Handled;
        }
        if let Some(node) = &self.focus_node {
            let result = if self.focus_key_handler_owner != FocusKeyHandlerOwner::CallbackContainer {
                let typed = node.on_key_event().map(|handler| handler(node, event.clone()));
                let raw = node.on_key().and_then(|handler| {
                    raw_delivery.payload(&event).map(|raw| handler(node, raw))
                });
                combine_key_event_results(typed.into_iter().chain(raw))
            } else {
                self.on_focus_key_event.as_ref().map_or(KeyEventResult::Ignored, |handler| {
                    handler(node, event.clone())
                })
            };
            if result != KeyEventResult::Ignored {
                return result;
            }
        }
        if self
            .on_key
            .as_ref()
            .is_some_and(|callback| callback(event.clone()))
        {
            return KeyEventResult::Handled;
        }
        // Phase observers never consume the event: Flutter's KeyboardListener
        // reports KeyEventResult.ignored, so outer scopes still see the key.
        if event.state.is_down() {
            if let Some(callback) = &self.on_key_down {
                callback(event.clone());
            }
            if event.repeat
                && let Some(callback) = &self.on_key_repeat
            {
                callback(event);
            }
        } else if let Some(callback) = &self.on_key_up {
            callback(event);
        }
        KeyEventResult::Ignored
    }
}

/// A keyboard shortcut key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShortcutKey {
    pub code: Code,
    pub modifiers: Modifiers,
}

/// A command identifier that can be bound independently from the shortcut
/// that invokes it.
///
/// `Command` is the convenient string-backed default. Applications with a
/// closed command vocabulary should use the generic `Actions<C>` and
/// `Shortcuts<C>` forms with their own `C` enum instead of converting every
/// command to a string.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Command(String);
impl Command {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }
}
impl From<&str> for Command {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Bounds required by typed command registries.
///
/// This is intentionally a value bound, not an inheritance hook. A command
/// can therefore be a small enum, an interned identifier, or the compatibility
/// [`Command`] string value.
pub trait CommandId: Clone + Eq + std::hash::Hash + 'static {}

impl<T> CommandId for T where T: Clone + Eq + std::hash::Hash + 'static {}

/// A command intent passed through an action registry.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Intent<C: CommandId = Command> {
    command: C,
}

impl<C: CommandId> Intent<C> {
    /// Creates an intent for a command identifier.
    #[must_use]
    pub fn new(command: impl Into<C>) -> Self {
        Self {
            command: command.into(),
        }
    }

    /// Returns the command identifier carried by this intent.
    #[must_use]
    pub fn command(&self) -> &C {
        &self.command
    }
}

impl Intent<Command> {
    /// Returns the stable command name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.command.name()
    }
}

impl<C: CommandId> From<C> for Intent<C> {
    fn from(value: C) -> Self {
        Self::new(value)
    }
}

impl From<&str> for Intent {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// A type-erased command intent used only while a retained keyboard event
/// walks its element ancestors. The command remains strongly typed inside
/// each [`Actions`] registry.
#[derive(Clone)]
pub struct ErasedIntent {
    command: Rc<dyn Any>,
}

impl ErasedIntent {
    #[must_use]
    pub fn from_intent<C: CommandId>(intent: &Intent<C>) -> Self {
        Self {
            command: Rc::new(intent.command().clone()),
        }
    }

    #[must_use]
    pub fn downcast_command<C: CommandId>(&self) -> Option<&C> {
        self.command.as_ref().downcast_ref::<C>()
    }
}

/// Result of invoking an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionResult {
    /// The action consumed the intent.
    Handled,
    /// The action declined it, allowing an outer scope to try.
    Ignored,
}

impl From<bool> for ActionResult {
    fn from(handled: bool) -> Self {
        if handled {
            Self::Handled
        } else {
            Self::Ignored
        }
    }
}

/// A live invocation's origin and inherited values. It is borrowed for
/// just the duration of dispatch, not retained by the action.
pub struct ActionContext<'a> {
    origin: &'a dyn Any,
    inherited: &'a incular_core::BuildContext,
}

impl<'a> ActionContext<'a> {
    #[doc(hidden)]
    #[must_use]
    pub fn new(origin: &'a dyn Any, inherited: &'a incular_core::BuildContext) -> Self {
        Self { origin, inherited }
    }

    /// The caller identity supplied by the host, such as a widget element address.
    #[must_use]
    pub fn origin<T: Any>(&self) -> Option<&T> {
        self.origin.downcast_ref::<T>()
    }

    /// Reads an inherited value from the caller, without subscribing.
    #[must_use]
    pub fn find<T: Any + Clone>(&self) -> Option<T> {
        self.inherited.read::<T>()
    }
}

/// The handling status and optional typed result of an action invocation.
/// A handled action can return no value; a missing mapping remains distinct
/// from both a disabled action and a handled action with no value.
pub struct ActionOutput {
    result: ActionResult,
    value: Option<Box<dyn Any>>,
}

impl ActionOutput {
    #[must_use]
    pub const fn result(&self) -> ActionResult {
        self.result
    }

    #[must_use]
    pub fn value<T: Any>(&self) -> Option<&T> {
        self.value.as_ref()?.downcast_ref::<T>()
    }

    #[must_use]
    pub fn into_value<T: Any>(self) -> Option<T> {
        self.value?.downcast::<T>().ok().map(|value| *value)
    }

    /// Returns a handled result with an application-defined payload.
    #[must_use]
    pub fn handled_with_value<T: Any>(value: T) -> Self {
        Self { result: ActionResult::Handled, value: Some(Box::new(value)) }
    }
}

impl From<ActionResult> for ActionOutput {
    fn from(result: ActionResult) -> Self {
        Self { result, value: None }
    }
}

struct ActionListenerState {
    listeners: RefCell<Vec<Weak<ActionListenerEntry>>>,
}

struct ActionListenerEntry {
    active: Cell<bool>,
    callback: Rc<dyn Fn()>,
}

/// Owns one registration made with [`Action::add_action_listener`]. Dropping
/// it removes the listener from future notifications.
pub struct ActionListenerSubscription {
    state: Weak<ActionListenerState>,
    entry: Rc<ActionListenerEntry>,
}

/// Phase emitted around an action invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionInvocationPhase {
    Start,
    End,
}

struct ActionInvocationListenerState {
    listeners: RefCell<Vec<Weak<ActionInvocationListenerEntry>>>,
}

struct ActionInvocationListenerEntry {
    active: Cell<bool>,
    callback: Rc<dyn Fn(ActionInvocationPhase)>,
}

/// Owns one registration made with [`Action::add_invocation_listener`].
pub struct ActionInvocationSubscription {
    state: Weak<ActionInvocationListenerState>,
    entry: Rc<ActionInvocationListenerEntry>,
}

impl ActionInvocationSubscription {
    fn remove(&self) -> bool {
        let was_active = self.entry.active.replace(false);
        if let Some(state) = self.state.upgrade() {
            state.listeners.borrow_mut().retain(|listener| {
                listener
                    .upgrade()
                    .is_some_and(|listener| !Rc::ptr_eq(&listener, &self.entry))
            });
        }
        was_active
    }
}

impl Drop for ActionInvocationSubscription {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

impl ActionListenerSubscription {
    fn remove(&self) -> bool {
        let was_active = self.entry.active.replace(false);
        if let Some(state) = self.state.upgrade() {
            state.listeners.borrow_mut().retain(|listener| {
                listener
                    .upgrade()
                    .is_some_and(|listener| !Rc::ptr_eq(&listener, &self.entry))
            });
        }
        was_active
    }
}

impl Drop for ActionListenerSubscription {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

/// A Rust-native command action.
///
/// Actions are values with an enabled state and a callback. They do not form
/// a generic inheritance tree like Flutter's `Action<T>` classes; a command
/// registry composes them by scope instead.
type ActionCallback<C> = Rc<dyn Fn(&Intent<C>, Option<&ActionContext<'_>>) -> ActionOutput>;

#[derive(Clone)]
pub struct Action<C: CommandId = Command> {
    intent: Intent<C>,
    enabled: Rc<Cell<bool>>,
    enabled_when: Option<Rc<dyn Fn() -> bool>>,
    consumes_key: bool,
    callback: ActionCallback<C>,
    listeners: Rc<ActionListenerState>,
    invocation_listeners: Rc<ActionInvocationListenerState>,
}

impl<C: CommandId + std::fmt::Debug> std::fmt::Debug for Action<C> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Action")
            .field("intent", &self.intent)
            .field("enabled", &self.is_enabled())
            .finish_non_exhaustive()
    }
}

impl<C: CommandId> Action<C> {
    /// Creates an enabled action whose callback consumes the intent.
    #[must_use]
    pub fn new(command: impl Into<Intent<C>>, callback: impl Fn() + 'static) -> Self {
        Self::with_handler(command, move |_| {
            callback();
            ActionResult::Handled
        })
    }

    /// Creates an action with an explicit handled/ignored result.
    #[must_use]
    pub fn with_handler<R>(
        command: impl Into<Intent<C>>,
        callback: impl Fn(&Intent<C>) -> R + 'static,
    ) -> Self
    where
        R: Into<ActionResult> + 'static,
    {
        Self::with_context(command, move |intent, _| ActionOutput::from(callback(intent).into()))
    }

    /// Creates a handled action that returns an application-defined value.
    #[must_use]
    pub fn with_value<T: Any>(
        command: impl Into<Intent<C>>,
        callback: impl Fn(&Intent<C>) -> T + 'static,
    ) -> Self {
        Self::with_context(command, move |intent, _| ActionOutput::handled_with_value(callback(intent)))
    }

    /// Creates an action that can inspect the calling element's context.
    /// Direct invocation without a host supplies no context.
    #[must_use]
    pub fn with_context(
        command: impl Into<Intent<C>>,
        callback: impl Fn(&Intent<C>, Option<&ActionContext<'_>>) -> ActionOutput + 'static,
    ) -> Self {
        Self {
            intent: command.into(),
            enabled: Rc::new(Cell::new(true)),
            enabled_when: None,
            consumes_key: true,
            callback: Rc::new(callback),
            listeners: Rc::new(ActionListenerState {
                listeners: RefCell::new(Vec::new()),
            }),
            invocation_listeners: Rc::new(ActionInvocationListenerState {
                listeners: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Returns the intent this action handles.
    #[must_use]
    pub fn intent(&self) -> &Intent<C> {
        &self.intent
    }

    /// Returns the stable command identifier.
    #[must_use]
    pub fn command(&self) -> &C {
        self.intent.command()
    }

    /// Enables or disables this action without removing it from its scope.
    pub fn set_enabled(&self, enabled: bool) {
        if self.enabled.replace(enabled) != enabled {
            self.notify_action_listeners();
        }
    }

    /// Adds a live condition to the action's manual enabled flag. The
    /// condition is queried at lookup and invocation. Its external state
    /// does not itself send action-listener notifications.
    #[must_use]
    pub fn enabled_when(mut self, predicate: impl Fn() -> bool + 'static) -> Self {
        self.enabled_when = Some(Rc::new(predicate));
        self
    }

    /// Returns whether the action can currently run.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.get()
            && self
                .enabled_when
                .as_ref()
                .is_none_or(|predicate| predicate())
    }

    /// Returns whether this action is enabled for the supplied intent.
    #[must_use]
    pub fn is_action_enabled(&self, intent: &Intent<C>) -> bool {
        self.is_enabled() && self.intent.command() == intent.command()
    }

    /// Registers a callback that observes state changes published by this
    /// action. The returned subscription is safe to hold in a retained widget.
    #[must_use]
    pub fn add_action_listener(&self, listener: impl Fn() + 'static) -> ActionListenerSubscription {
        let entry = Rc::new(ActionListenerEntry {
            active: Cell::new(true),
            callback: Rc::new(listener),
        });
        self.listeners
            .listeners
            .borrow_mut()
            .push(Rc::downgrade(&entry));
        ActionListenerSubscription {
            state: Rc::downgrade(&self.listeners),
            entry,
        }
    }

    /// Short alias for [`Self::add_action_listener`].
    #[must_use]
    pub fn add_listener(&self, listener: impl Fn() + 'static) -> ActionListenerSubscription {
        self.add_action_listener(listener)
    }

    /// Removes a registration returned by [`Self::add_action_listener`].
    pub fn remove_action_listener(&self, subscription: &ActionListenerSubscription) -> bool {
        subscription.remove()
    }

    /// Notifies listeners that action state or another action-owned property
    /// changed. Listeners added during delivery wait for the next notification;
    /// removed listeners are skipped immediately.
    pub fn notify_action_listeners(&self) {
        let listeners = {
            let mut listeners = self.listeners.listeners.borrow_mut();
            listeners.retain(|listener| listener.strong_count() != 0);
            listeners
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>()
        };
        for listener in listeners {
            if listener.active.get() {
                (listener.callback)();
            }
        }
    }

    /// Registers a listener that observes the start and end of every
    /// successful invocation of this action.
    #[must_use]
    pub fn add_invocation_listener(
        &self,
        listener: impl Fn(ActionInvocationPhase) + 'static,
    ) -> ActionInvocationSubscription {
        let entry = Rc::new(ActionInvocationListenerEntry {
            active: Cell::new(true),
            callback: Rc::new(listener),
        });
        self.invocation_listeners
            .listeners
            .borrow_mut()
            .push(Rc::downgrade(&entry));
        ActionInvocationSubscription {
            state: Rc::downgrade(&self.invocation_listeners),
            entry,
        }
    }

    fn notify_invocation(&self, phase: ActionInvocationPhase) {
        let listeners = {
            let mut listeners = self.invocation_listeners.listeners.borrow_mut();
            listeners.retain(|listener| listener.strong_count() != 0);
            listeners
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>()
        };
        for listener in listeners {
            if listener.active.get() {
                (listener.callback)(phase);
            }
        }
    }

    #[must_use]
    pub fn listener_count(&self) -> usize {
        let mut listeners = self.listeners.listeners.borrow_mut();
        listeners.retain(|listener| listener.strong_count() != 0);
        listeners
            .iter()
            .filter_map(Weak::upgrade)
            .filter(|listener| listener.active.get())
            .count()
    }

    /// Controls whether a shortcut invocation consumes the native key.
    /// When false, Incular stops its key handlers but lets the OS see it.
    #[must_use]
    pub const fn with_key_consumption(mut self, consumes: bool) -> Self {
        self.consumes_key = consumes;
        self
    }

    #[must_use]
    pub const fn consumes_key(&self, _intent: &Intent<C>) -> bool {
        self.consumes_key
    }

    /// Converts a successfully invoked shortcut action to its key result.
    #[must_use]
    pub fn to_key_event_result(&self, intent: &Intent<C>) -> KeyEventResult {
        if self.consumes_key(intent) {
            KeyEventResult::Handled
        } else {
            KeyEventResult::SkipRemainingHandlers
        }
    }

    /// Invokes this action for an intent.
    #[must_use]
    pub fn invoke(&self, intent: &Intent<C>) -> ActionResult {
        self.invoke_output(intent).result()
    }

    /// Invokes this action and preserves any application-defined output.
    #[must_use]
    pub fn invoke_output(&self, intent: &Intent<C>) -> ActionOutput {
        if !self.is_action_enabled(intent) {
            return ActionResult::Ignored.into();
        }
        self.invoke_enabled_output(intent, None)
    }

    fn invoke_enabled_output(
        &self,
        intent: &Intent<C>,
        context: Option<&ActionContext<'_>>,
    ) -> ActionOutput {
        self.notify_invocation(ActionInvocationPhase::Start);
        let result = (self.callback)(intent, context);
        self.notify_invocation(ActionInvocationPhase::End);
        result
    }
}

/// An action whose implementation is a callback receiving the typed intent.
/// Direct invocation passes the supplied payload through unchanged.
pub struct CallbackAction<I, R = ActionResult> {
    callback: Rc<dyn Fn(&I) -> R>,
}

impl<I, R> CallbackAction<I, R> {
    #[must_use]
    pub fn new(callback: impl Fn(&I) -> R + 'static) -> Self {
        Self { callback: Rc::new(callback) }
    }

    pub fn invoke(&self, intent: &I) -> R {
        (self.callback)(intent)
    }
}

impl<C: CommandId, R: Into<ActionResult> + 'static> CallbackAction<Intent<C>, R> {
    /// Registers this callback for a concrete command key.
    #[must_use]
    pub fn bind(self, command: impl Into<Intent<C>>) -> Action<C> {
        Action::with_handler(command, move |intent| self.invoke(intent))
    }
}

/// Carries a callback to the stateless [VoidCallbackAction].
#[derive(Clone)]
pub struct VoidCallbackIntent {
    callback: Rc<dyn Fn()>,
}

impl VoidCallbackIntent {
    #[must_use]
    pub fn new(callback: impl Fn() + 'static) -> Self {
        Self { callback: Rc::new(callback) }
    }
}

/// Invokes the callback supplied by the intent without returning a value.
#[derive(Clone, Copy, Debug, Default)]
pub struct VoidCallbackAction;

impl VoidCallbackAction {
    pub fn invoke(&self, intent: &VoidCallbackIntent) {
        (intent.callback)();
    }
}

/// Invokes an action with its intent and returns the handler's result.
///
/// The default dispatcher uses the action's enabled check and emits its
/// normal invocation notifications.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct ActionDispatcher {}

impl ActionDispatcher {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    #[must_use]
    pub fn invoke_action<C: CommandId>(&self, action: &Action<C>, intent: &Intent<C>) -> ActionResult {
        action.invoke(intent)
    }
}

/// Result of looking up one local shortcut binding during ancestor dispatch.
#[derive(Clone)]
pub enum ShortcutMatch {
    Callback(Rc<dyn Fn()>),
    Intent(ErasedIntent),
}

/// Type-erased retained shortcut scope used by the widget tree to resolve
/// local shortcut precedence before walking to an ancestor scope.
pub struct ErasedShortcutScope {
    finder: Rc<dyn Fn(KeyboardEvent) -> Option<ShortcutMatch>>,
}

impl ErasedShortcutScope {
    #[must_use]
    pub fn from_shortcuts<C: CommandId>(shortcuts: Rc<Shortcuts<C>>) -> Rc<Self> {
        Rc::new(Self {
            finder: Rc::new(move |event| shortcuts.find_match(&event)),
        })
    }

    #[must_use]
    pub fn find(&self, event: KeyboardEvent) -> Option<ShortcutMatch> {
        (self.finder)(event)
    }
}

/// A custom dispatcher's view of one enabled action and its intent.
pub struct ActionDispatchInvocation<'a> {
    intent: &'a ErasedIntent,
    action: &'a dyn std::any::Any,
    context: Option<&'a ActionContext<'a>>,
    invoke: &'a dyn Fn() -> ActionResult,
}

impl ActionDispatchInvocation<'_> {
    #[must_use]
    pub fn command<C: CommandId>(&self) -> Option<&C> {
        self.intent.downcast_command::<C>()
    }

    #[must_use]
    pub fn action<C: CommandId>(&self) -> Option<&Action<C>> {
        self.action.downcast_ref()
    }

    /// The caller's context, if a host routed this invocation.
    #[must_use]
    pub fn context(&self) -> Option<&ActionContext<'_>> {
        self.context
    }

    /// Calls the action, preserving invocation observers.
    #[must_use]
    pub fn invoke(&self) -> ActionResult {
        (self.invoke)()
    }
}

/// Optional interceptor for an action scope. A handler can delegate to the
/// bound action, observe the result, or return its own result.
pub type ScopedActionDispatcher = dyn for<'a> Fn(ActionDispatchInvocation<'a>) -> ActionResult;

type DispatcherLookup<'a> = dyn Fn() -> Option<Rc<ScopedActionDispatcher>> + 'a;

fn dispatch_bound_action_output<C: CommandId>(
    action: &Action<C>,
    intent: &Intent<C>,
    erased: &ErasedIntent,
    dispatcher: &DispatcherLookup<'_>,
    context: Option<&ActionContext<'_>>,
) -> ActionOutput {
    if !action.is_action_enabled(intent) {
        return ActionResult::Ignored.into();
    }
    let Some(dispatcher) = dispatcher() else {
        return action.invoke_enabled_output(intent, context);
    };
    let invocation_output = RefCell::new(None);
    let invoke = || {
        let output = action.invoke_enabled_output(intent, context);
        let result = output.result();
        *invocation_output.borrow_mut() = Some(output);
        result
    };
    let dispatched = dispatcher(ActionDispatchInvocation { intent: erased, action, context, invoke: &invoke });
    invocation_output
        .into_inner()
        .filter(|output| output.result() == dispatched)
        .unwrap_or_else(|| dispatched.into())
}

impl ActionDispatcher {
    /// Invokes an enabled action through a supplied scope dispatcher.
    #[must_use]
    pub fn invoke_action_with<C: CommandId>(
        &self,
        action: &Action<C>,
        intent: &Intent<C>,
        dispatcher: Option<Rc<ScopedActionDispatcher>>,
    ) -> ActionResult {
        if dispatcher.is_none() {
            return self.invoke_action(action, intent);
        }
        let erased = ErasedIntent::from_intent(intent);
        dispatch_bound_action_output(action, intent, &erased, &|| dispatcher.clone(), None).result()
    }
}

trait ActionRegistry {
    fn observe_changes(
        &self,
        changed: &incular_core::reactivity::DependencySource,
    ) -> Vec<ActionListenerSubscription>;
    fn action_for(&self, command: &dyn std::any::Any) -> Option<&dyn std::any::Any>;
    fn invoke(
        &self,
        erased: &ErasedIntent,
        dispatcher: &DispatcherLookup<'_>,
        context: Option<&ActionContext<'_>>,
    ) -> Option<ActionOutput>;
    fn invoke_shortcut(
        &self,
        erased: &ErasedIntent,
        dispatcher: &DispatcherLookup<'_>,
        context: Option<&ActionContext<'_>>,
    ) -> Option<KeyEventResult>;
}

impl<C: CommandId> ActionRegistry for Actions<C> {
    fn observe_changes(
        &self,
        changed: &incular_core::reactivity::DependencySource,
    ) -> Vec<ActionListenerSubscription> {
        let mut commands = HashSet::new();
        let mut identities = HashSet::new();
        self.scopes
            .iter()
            .rev()
            .flat_map(|scope| &scope.actions)
            .filter(|(command, action)| {
                commands.insert(*command) && identities.insert(Rc::as_ptr(&action.listeners))
            })
            .map(|(_, action)| {
                let changed = changed.clone();
                action.add_action_listener(move || changed.notify())
            })
            .collect()
    }

    fn action_for(&self, command: &dyn std::any::Any) -> Option<&dyn std::any::Any> {
        self.action_ref(command.downcast_ref::<C>()?)
            .map(|action| action as &dyn std::any::Any)
    }

    fn invoke(
        &self,
        erased: &ErasedIntent,
        dispatcher: &DispatcherLookup<'_>,
        context: Option<&ActionContext<'_>>,
    ) -> Option<ActionOutput> {
        let command = erased.downcast_command::<C>()?;
        let action = self.action_ref(command)?;
        let intent: Intent<C> = Intent::new(command.clone());
        Some(dispatch_bound_action_output(action, &intent, erased, dispatcher, context))
    }

    fn invoke_shortcut(
        &self,
        erased: &ErasedIntent,
        dispatcher: &DispatcherLookup<'_>,
        context: Option<&ActionContext<'_>>,
    ) -> Option<KeyEventResult> {
        let command = erased.downcast_command::<C>()?;
        let action = self.action_ref(command)?;
        let intent: Intent<C> = Intent::new(command.clone());
        let result = dispatch_bound_action_output(action, &intent, erased, dispatcher, context).result();
        Some(match result {
            ActionResult::Handled => action.to_key_event_result(&intent),
            ActionResult::Ignored => KeyEventResult::Ignored,
        })
    }
}

/// Type-erased mapping used by the retained widget tree.
pub struct ErasedActionScope {
    registry: Rc<dyn ActionRegistry>,
    dispatcher: Option<Rc<ScopedActionDispatcher>>,
}

impl ErasedActionScope {
    #[must_use]
    pub fn from_actions<C: CommandId>(actions: Rc<Actions<C>>) -> Rc<Self> {
        Rc::new(Self { registry: actions, dispatcher: None })
    }

    #[must_use]
    pub fn with_dispatcher(&self, dispatcher: Rc<ScopedActionDispatcher>) -> Rc<Self> {
        Rc::new(Self {
            dispatcher: Some(dispatcher),
            registry: self.registry.clone(),
        })
    }

    #[must_use]
    pub fn dispatcher(&self) -> Option<Rc<ScopedActionDispatcher>> {
        self.dispatcher.clone()
    }

    /// Observes every effective action, once per action identity, for one
    /// mounted scope. The retained scope owns the returned subscriptions.
    #[doc(hidden)]
    pub fn observe_changes(
        &self,
        changed: &incular_core::reactivity::DependencySource,
    ) -> Vec<ActionListenerSubscription> {
        self.registry.observe_changes(changed)
    }

    /// Whether these descriptors share their immutable map and dispatcher.
    #[doc(hidden)]
    pub fn same_mapping(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.registry, &other.registry)
            && match (&self.dispatcher, &other.dispatcher) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }

    /// Reads a typed mapping without invoking the action or dispatcher.
    #[must_use]
    pub fn action<C: CommandId>(&self, intent: &Intent<C>) -> Option<Action<C>> {
        self.registry.action_for(intent.command())?.downcast_ref::<Action<C>>().cloned()
    }

    #[must_use]
    pub fn invoke(&self, intent: &ErasedIntent) -> Option<ActionResult> {
        self.invoke_resolving_dispatcher(intent, || self.dispatcher())
    }

    /// Only resolves a dispatcher if this scope owns an enabled mapping.
    #[must_use]
    pub fn invoke_resolving_dispatcher(
        &self,
        intent: &ErasedIntent,
        find: impl Fn() -> Option<Rc<ScopedActionDispatcher>>,
    ) -> Option<ActionResult> {
        self.invoke_output_resolving_dispatcher(intent, find)
            .map(|output| output.result())
    }

    /// Resolves the owning dispatcher and preserves the returned output.
    #[must_use]
    pub fn invoke_output_resolving_dispatcher(
        &self,
        intent: &ErasedIntent,
        find: impl Fn() -> Option<Rc<ScopedActionDispatcher>>,
    ) -> Option<ActionOutput> {
        self.invoke_in_context(intent, find, None)
    }

    /// Invokes an inherited action with the original calling element.
    #[doc(hidden)]
    #[must_use]
    pub fn invoke_in_context(
        &self,
        intent: &ErasedIntent,
        find: impl Fn() -> Option<Rc<ScopedActionDispatcher>>,
        context: Option<&ActionContext<'_>>,
    ) -> Option<ActionOutput> {
        self.registry.invoke(intent, &find, context)
    }

    /// Routes a shortcut through the action mapping and its key consumption policy.
    #[must_use]
    pub fn invoke_shortcut(
        &self,
        intent: &ErasedIntent,
        find: impl Fn() -> Option<Rc<ScopedActionDispatcher>>,
    ) -> Option<KeyEventResult> {
        self.invoke_shortcut_in_context(intent, find, None)
    }

    /// Routes a shortcut with the focused element as the action's caller.
    #[doc(hidden)]
    #[must_use]
    pub fn invoke_shortcut_in_context(
        &self,
        intent: &ErasedIntent,
        find: impl Fn() -> Option<Rc<ScopedActionDispatcher>>,
        context: Option<&ActionContext<'_>>,
    ) -> Option<KeyEventResult> {
        self.registry.invoke_shortcut(intent, &find, context)
    }
}

struct ActionScope<C: CommandId> {
    actions: HashMap<C, Action<C>>,
}

/// Command-to-action registry with nearest-scope dispatch.
///
/// The last pushed scope is searched first. A missing, disabled, or
/// `Ignored` action falls through to the next outer scope, which makes nested
/// command palettes and temporary modal bindings predictable.
pub struct Actions<C: CommandId = Command> {
    scopes: Vec<ActionScope<C>>,
}

impl<C: CommandId> Default for Actions<C> {
    fn default() -> Self {
        Self {
            scopes: vec![ActionScope::<C> {
                actions: HashMap::new(),
            }],
        }
    }
}

impl<C: CommandId> Actions<C> {
    /// Creates an empty typed registry. Use `Actions::new()` for the
    /// string-backed default registry when no generic type is needed.
    #[must_use]
    pub fn typed() -> Self {
        Self::default()
    }

    /// Registers a simple callback in the current (nearest) scope.
    pub fn register(&mut self, command: impl Into<C>, action: impl Fn() + 'static) {
        let command = command.into();
        self.register_action(Action::new(Intent::new(command), action));
    }

    /// Registers an explicit action in the current scope.
    pub fn register_action(&mut self, action: Action<C>) {
        self.current_scope_mut()
            .actions
            .insert(action.command().clone(), action);
    }

    /// Registers a callback that can explicitly allow fallthrough.
    pub fn register_handler<R>(
        &mut self,
        command: impl Into<Intent<C>>,
        action: impl Fn(&Intent<C>) -> R + 'static,
    ) where
        R: Into<ActionResult> + 'static,
    {
        self.register_action(Action::with_handler(command, action));
    }

    /// Pushes a nested action scope. Call [`Self::pop_scope`] when the modal
    /// or temporary scope is no longer active.
    pub fn push_scope(&mut self) {
        self.scopes.push(ActionScope::<C> {
            actions: HashMap::new(),
        });
    }

    /// Pops the nearest scope, preserving the root scope.
    #[must_use]
    pub fn pop_scope(&mut self) -> bool {
        (self.scopes.len() > 1).then(|| self.scopes.pop()).is_some()
    }

    /// Runs a closure in a temporary nested scope and restores the previous
    /// scope after the closure returns, including when the closure unwinds.
    pub fn with_scope<R>(&mut self, callback: impl FnOnce(&mut Self) -> R) -> R {
        self.push_scope();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(self)));
        let _ = self.pop_scope();
        match result {
            Ok(result) => result,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    /// Returns the number of active action scopes.
    #[must_use]
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    /// Returns whether no action is registered in any retained scope.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.scopes.iter().all(|scope| scope.actions.is_empty())
    }

    /// Removes a command from the nearest scope.
    pub fn unregister(&mut self, command: &C) -> bool {
        self.current_scope_mut().actions.remove(command).is_some()
    }

    #[must_use]
    pub fn invoke(&self, command: &C) -> bool {
        self.invoke_intent(&Intent::new(command.clone()))
    }

    /// Dispatches an intent from the nearest scope outward.
    #[must_use]
    pub fn invoke_intent(&self, intent: &Intent<C>) -> bool {
        self.scopes.iter().rev().any(|scope| {
            scope
                .actions
                .get(intent.command())
                .is_some_and(|action| action.invoke(intent) == ActionResult::Handled)
        })
    }

    /// Returns the result from the nearest scope that contains a mapping,
    /// including `Ignored` for a disabled or declining action. This preserves
    /// Flutter's ancestor action lookup rule: a found local mapping shadows
    /// an outer mapping even when it cannot currently handle the intent.
    #[must_use]
    pub fn invoke_intent_first(&self, intent: &Intent<C>) -> Option<ActionResult> {
        self.scopes.iter().rev().find_map(|scope| {
            scope
                .actions
                .get(intent.command())
                .map(|action| ActionDispatcher::new().invoke_action(action, intent))
        })
    }

    /// Returns the nearest registered action, if any.
    #[must_use]
    pub fn action(&self, command: &C) -> Option<Action<C>> {
        self.action_ref(command).cloned()
    }

    fn action_ref(&self, command: &C) -> Option<&Action<C>> {
        self.scopes.iter().rev().find_map(|scope| scope.actions.get(command))
    }

    fn current_scope_mut(&mut self) -> &mut ActionScope<C> {
        if self.scopes.is_empty() {
            self.scopes.push(ActionScope::<C> {
                actions: HashMap::new(),
            });
        }
        self.scopes
            .last_mut()
            .expect("action registry has a root scope")
    }
}

impl Actions<Command> {
    /// Creates an empty string-backed command registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ShortcutKey {
    #[must_use]
    pub const fn new(code: Code, modifiers: Modifiers) -> Self {
        Self { code, modifiers }
    }

    /// Builds a logical-key activator for layouts where physical codes are
    /// not stable enough (for example character shortcuts).
    #[must_use]
    pub fn logical(key: KeyboardKey, modifiers: Modifiers) -> LogicalShortcutKey {
        LogicalShortcutKey { key, modifiers }
    }
}

/// Whether a shortcut requires Num Lock to be on, off, or either.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LockState {
    #[default]
    Ignored,
    Locked,
    Unlocked,
}

/// A logical single-key shortcut with the modifier state required by the
/// shortcut. This is the retained equivalent of Flutter's
/// `SingleActivator`: the four shortcut modifiers are matched independently,
/// while unrelated non-modifier keys do not affect a match. Num Lock is
/// ignored unless the activator requests a particular state.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SingleActivator {
    pub trigger: KeyboardKey,
    pub modifiers: Modifiers,
    pub include_repeats: bool,
    pub num_lock: LockState,
}

impl SingleActivator {
    #[must_use]
    pub fn new(trigger: KeyboardKey, modifiers: Modifiers) -> Self {
        Self {
            trigger,
            modifiers: shortcut_modifiers(modifiers),
            include_repeats: true,
            num_lock: LockState::Ignored,
        }
    }

    #[must_use]
    pub fn with_repeats(mut self, include_repeats: bool) -> Self {
        self.include_repeats = include_repeats;
        self
    }

    #[must_use]
    pub fn with_num_lock(mut self, num_lock: LockState) -> Self {
        self.num_lock = num_lock;
        self
    }

    #[must_use]
    pub const fn trigger(&self) -> &KeyboardKey {
        &self.trigger
    }

    #[must_use]
    pub const fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    #[must_use]
    pub const fn includes_repeats(&self) -> bool {
        self.include_repeats
    }

    #[must_use]
    pub fn accepts(&self, event: &KeyboardEvent) -> bool {
        self.accepts_with_lock_modes(event, event.modifiers)
    }

    /// Tests the event against a separate hardware lock snapshot.
    #[must_use]
    pub fn accepts_with_lock_modes(&self, event: &KeyboardEvent, lock_modes: Modifiers) -> bool {
        let num_locked = lock_modes.contains(Modifiers::NUM_LOCK);
        let lock_matches = match self.num_lock {
            LockState::Ignored => true,
            LockState::Locked => num_locked,
            LockState::Unlocked => !num_locked,
        };
        lock_matches
            && event.state.is_down()
            && (self.include_repeats || !event.repeat)
            && event.key == self.trigger
            && shortcut_modifiers(event.modifiers) == self.modifiers
    }
}

/// Flutter's `CharacterActivator`: matches the character a key press
/// produces, whatever key produced it, with the given modifiers held.
/// Shift is not a field: it is part of the character, whose case matters.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CharacterActivator {
    pub character: String,
    pub alt: bool,
    pub control: bool,
    pub meta: bool,
    pub include_repeats: bool,
}

impl CharacterActivator {
    #[must_use]
    pub fn new(character: impl Into<String>) -> Self {
        Self {
            character: character.into(),
            alt: false,
            control: false,
            meta: false,
            include_repeats: true,
        }
    }

    #[must_use]
    pub fn alt(mut self, alt: bool) -> Self {
        self.alt = alt;
        self
    }

    #[must_use]
    pub fn control(mut self, control: bool) -> Self {
        self.control = control;
        self
    }

    #[must_use]
    pub fn meta(mut self, meta: bool) -> Self {
        self.meta = meta;
        self
    }

    #[must_use]
    pub fn include_repeats(mut self, include_repeats: bool) -> Self {
        self.include_repeats = include_repeats;
        self
    }

    /// Flutter's `CharacterActivator.accepts`.
    #[must_use]
    pub fn accepts(&self, event: &KeyboardEvent) -> bool {
        let KeyboardKey::Character(character) = &event.key else {
            return false;
        };
        event.state.is_down()
            && (self.include_repeats || !event.repeat)
            && character == &self.character
            && event.modifiers.alt() == self.alt
            && event.modifiers.ctrl() == self.control
            && event.modifiers.meta() == self.meta
    }
}

/// A key activator accepted by callback shortcut maps.
///
/// Physical and logical activators are kept distinct, and a `Single` uses the
/// logical key with Flutter-style modifier matching. `Character` instead
/// compares the produced character. The enum is intentionally
/// value-based so equivalent registrations replace one another while distinct
/// activators may both fire for one event.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutActivator {
    Physical {
        key: ShortcutKey,
        trigger: ShortcutTrigger,
    },
    Logical {
        key: LogicalShortcutKey,
        trigger: ShortcutTrigger,
    },
    Single(SingleActivator),
    Character(CharacterActivator),
}

impl ShortcutActivator {
    #[must_use]
    pub const fn physical(key: ShortcutKey) -> Self {
        Self::Physical {
            key,
            trigger: ShortcutTrigger::Down,
        }
    }

    #[must_use]
    pub const fn physical_on(key: ShortcutKey, trigger: ShortcutTrigger) -> Self {
        Self::Physical { key, trigger }
    }

    #[must_use]
    pub fn logical(key: LogicalShortcutKey) -> Self {
        Self::Logical {
            key,
            trigger: ShortcutTrigger::Down,
        }
    }

    #[must_use]
    pub fn logical_on(key: LogicalShortcutKey, trigger: ShortcutTrigger) -> Self {
        Self::Logical { key, trigger }
    }

    #[must_use]
    pub fn single(trigger: KeyboardKey, modifiers: Modifiers) -> Self {
        Self::Single(SingleActivator::new(trigger, modifiers))
    }

    #[must_use]
    pub fn accepts(&self, event: &KeyboardEvent) -> bool {
        self.accepts_with_lock_modes(event, event.modifiers)
    }

    #[must_use]
    pub fn accepts_with_lock_modes(&self, event: &KeyboardEvent, lock_modes: Modifiers) -> bool {
        match self {
            Self::Physical { key, trigger } => {
                key.code == event.code && key.modifiers == event.modifiers && trigger.matches(event)
            }
            Self::Logical { key, trigger } => {
                key.key == event.key && key.modifiers == event.modifiers && trigger.matches(event)
            }
            Self::Single(activator) => activator.accepts_with_lock_modes(event, lock_modes),
            Self::Character(activator) => activator.accepts(event),
        }
    }
}

impl From<ShortcutKey> for ShortcutActivator {
    fn from(value: ShortcutKey) -> Self {
        Self::physical(value)
    }
}

impl From<LogicalShortcutKey> for ShortcutActivator {
    fn from(value: LogicalShortcutKey) -> Self {
        Self::logical(value)
    }
}

impl From<SingleActivator> for ShortcutActivator {
    fn from(value: SingleActivator) -> Self {
        Self::Single(value)
    }
}

impl From<CharacterActivator> for ShortcutActivator {
    fn from(value: CharacterActivator) -> Self {
        Self::Character(value)
    }
}

/// Flutter's `LogicalKeySet`: a set of logical keys pressed together, such
/// as Control+C. Modifier keys in the set become required modifiers; the one
/// other key (or, for a modifier-only set, the last modifier) triggers it.
///
/// Incular reports modifiers without sides, so `Control` stands for Flutter's
/// `LogicalKeyboardKey.control` synonym and matches either side.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LogicalKeySet {
    keys: Vec<KeyboardKey>,
}

impl LogicalKeySet {
    /// Flutter's `LogicalKeySet(key1, [key2, key3, key4])`. Duplicates are
    /// dropped; the order of the remaining keys does not matter.
    #[must_use]
    pub fn new(keys: impl IntoIterator<Item = KeyboardKey>) -> Self {
        let mut unique = Vec::new();
        for key in keys {
            if !unique.contains(&key) {
                unique.push(key);
            }
        }
        Self { keys: unique }
    }

    #[must_use]
    pub fn keys(&self) -> &[KeyboardKey] {
        &self.keys
    }

    fn modifier_of(key: &KeyboardKey) -> Option<Modifiers> {
        use incular_core::NamedKey;
        match key {
            KeyboardKey::Named(NamedKey::Alt) => Some(Modifiers::ALT),
            KeyboardKey::Named(NamedKey::Control) => Some(Modifiers::CONTROL),
            #[expect(
                deprecated,
                reason = "older key sources still send the legacy Super key"
            )]
            KeyboardKey::Named(NamedKey::Meta | NamedKey::Super) => Some(Modifiers::META),
            KeyboardKey::Named(NamedKey::Shift) => Some(Modifiers::SHIFT),
            _ => None,
        }
    }

    /// The equivalent single-key activator, or `None` for an empty set or a
    /// set with more than one non-modifier key (which Flutter's matching can
    /// never accept from one key event either).
    #[must_use]
    pub fn activator(&self) -> Option<SingleActivator> {
        let mut modifiers = Modifiers::empty();
        let mut trigger = None;
        for key in &self.keys {
            match Self::modifier_of(key) {
                Some(modifier) => modifiers |= modifier,
                None if trigger.is_some() => return None,
                None => trigger = Some(key.clone()),
            }
        }
        match trigger {
            Some(trigger) => Some(SingleActivator::new(trigger, modifiers)),
            None => {
                let last = self.keys.last()?.clone();
                Some(SingleActivator::new(last, modifiers))
            }
        }
    }

    /// Flutter's `LogicalKeySet.accepts`: the event's key is in the set and
    /// exactly the set's modifiers are held.
    #[must_use]
    pub fn accepts(&self, event: &KeyboardEvent) -> bool {
        self.activator().is_some_and(|activator| {
            if Self::modifier_of(&activator.trigger).is_some() {
                // A modifier-only set: the trigger's own modifier is held
                // (or, on some platforms, not yet reported) on its down event.
                event.state.is_down()
                    && self.keys.contains(&event.key)
                    && shortcut_modifiers(event.modifiers)
                        | Self::modifier_of(&event.key).unwrap_or_default()
                        == activator.modifiers
            } else {
                activator.accepts(event)
            }
        })
    }
}

impl From<LogicalKeySet> for ShortcutActivator {
    /// A set that can never match becomes a never-matching logical key.
    fn from(value: LogicalKeySet) -> Self {
        match value.activator() {
            Some(activator) => Self::Single(activator),
            None => Self::logical(LogicalShortcutKey {
                key: KeyboardKey::Named(incular_core::NamedKey::Unidentified),
                modifiers: Modifiers::all(),
            }),
        }
    }
}

fn shortcut_modifiers(modifiers: Modifiers) -> Modifiers {
    modifiers & (Modifiers::ALT | Modifiers::CONTROL | Modifiers::META | Modifiers::SHIFT)
}

/// A keyboard shortcut matched against the event's logical key value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LogicalShortcutKey {
    pub key: KeyboardKey,
    pub modifiers: Modifiers,
}

impl LogicalShortcutKey {
    #[must_use]
    pub fn new(key: KeyboardKey, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }
}

/// Controls when a shortcut binding is eligible for a keyboard event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShortcutTrigger {
    /// A key-down event, including operating-system repeat events.
    #[default]
    Down,
    /// The initial key-down event, excluding repeats.
    Press,
    /// A repeated key-down event.
    Repeat,
    /// A key-up event.
    Up,
    /// Either key-down or key-up.
    Any,
}

impl ShortcutTrigger {
    #[must_use]
    pub const fn matches(self, event: &KeyboardEvent) -> bool {
        match self {
            Self::Down => event.state.is_down(),
            Self::Press => event.state.is_down() && !event.repeat,
            Self::Repeat => event.state.is_down() && event.repeat,
            Self::Up => event.state.is_up(),
            Self::Any => true,
        }
    }
}

enum ShortcutBinding<C: CommandId> {
    Callback(Rc<dyn Fn()>),
    Intent(Intent<C>),
}

struct ShortcutRegistration<C: CommandId> {
    trigger: ShortcutTrigger,
    binding: ShortcutBinding<C>,
}

struct ShortcutScope<C: CommandId> {
    physical: HashMap<ShortcutKey, Vec<ShortcutRegistration<C>>>,
    logical: HashMap<LogicalShortcutKey, Vec<ShortcutRegistration<C>>>,
}

/// A small typed shortcut map that centralizes keyboard dispatch.
pub struct Shortcuts<C: CommandId = Command> {
    scopes: Vec<ShortcutScope<C>>,
}

impl<C: CommandId> Default for Shortcuts<C> {
    fn default() -> Self {
        Self {
            scopes: vec![ShortcutScope {
                physical: HashMap::new(),
                logical: HashMap::new(),
            }],
        }
    }
}

impl<C: CommandId> Shortcuts<C> {
    /// Creates an empty typed shortcut map. Use `Shortcuts::new()` for the
    /// string-backed default map when no generic type is needed.
    #[must_use]
    pub fn typed() -> Self {
        Self::default()
    }

    pub fn register(&mut self, key: ShortcutKey, action: impl Fn() + 'static) {
        self.register_on(key, ShortcutTrigger::Down, action);
    }

    /// Registers a callback for a specific key transition.
    pub fn register_on(
        &mut self,
        key: ShortcutKey,
        trigger: ShortcutTrigger,
        action: impl Fn() + 'static,
    ) {
        Self::replace_binding(
            &mut self.current_scope_mut().physical,
            key,
            trigger,
            ShortcutBinding::Callback(Rc::new(action)),
        );
    }

    /// Registers a callback against a logical key value.
    pub fn register_logical(&mut self, key: LogicalShortcutKey, action: impl Fn() + 'static) {
        self.register_logical_on(key, ShortcutTrigger::Down, action);
    }

    /// Registers a logical-key callback for a specific key transition.
    pub fn register_logical_on(
        &mut self,
        key: LogicalShortcutKey,
        trigger: ShortcutTrigger,
        action: impl Fn() + 'static,
    ) {
        Self::replace_binding(
            &mut self.current_scope_mut().logical,
            key,
            trigger,
            ShortcutBinding::Callback(Rc::new(action)),
        );
    }

    /// Binds a shortcut to a command registered in [`Actions`].
    pub fn bind(&mut self, key: ShortcutKey, command: impl Into<C>) {
        self.bind_on(key, ShortcutTrigger::Down, Intent::new(command.into()));
    }

    /// Binds a shortcut to an intent for a specific key transition.
    pub fn bind_on(
        &mut self,
        key: ShortcutKey,
        trigger: ShortcutTrigger,
        command: impl Into<Intent<C>>,
    ) {
        Self::replace_binding(
            &mut self.current_scope_mut().physical,
            key,
            trigger,
            ShortcutBinding::Intent(command.into()),
        );
    }

    /// Binds a logical key to a typed command.
    pub fn bind_logical(&mut self, key: LogicalShortcutKey, command: impl Into<C>) {
        self.bind_logical_on(key, ShortcutTrigger::Down, Intent::new(command.into()));
    }

    /// Binds a logical key to an intent for a specific transition.
    pub fn bind_logical_on(
        &mut self,
        key: LogicalShortcutKey,
        trigger: ShortcutTrigger,
        command: impl Into<Intent<C>>,
    ) {
        Self::replace_binding(
            &mut self.current_scope_mut().logical,
            key,
            trigger,
            ShortcutBinding::Intent(command.into()),
        );
    }

    /// Removes all bindings for a shortcut key.
    pub fn clear(&mut self, key: &ShortcutKey) -> bool {
        self.current_scope_mut().physical.remove(key).is_some()
    }

    /// Removes all logical-key bindings from the nearest scope.
    pub fn clear_logical(&mut self, key: &LogicalShortcutKey) -> bool {
        self.current_scope_mut().logical.remove(key).is_some()
    }

    /// Pushes a nested shortcut scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(ShortcutScope {
            physical: HashMap::new(),
            logical: HashMap::new(),
        });
    }

    /// Pops the nearest shortcut scope, preserving the root scope.
    #[must_use]
    pub fn pop_scope(&mut self) -> bool {
        (self.scopes.len() > 1).then(|| self.scopes.pop()).is_some()
    }

    /// Runs a closure in a temporary shortcut scope.
    pub fn with_scope<R>(&mut self, callback: impl FnOnce(&mut Self) -> R) -> R {
        self.push_scope();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(self)));
        let _ = self.pop_scope();
        match result {
            Ok(result) => result,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    /// Returns the number of active shortcut scopes.
    #[must_use]
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    /// Returns whether no shortcut is registered in any retained scope.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.scopes.iter().all(|scope| {
            scope.physical.values().all(Vec::is_empty) && scope.logical.values().all(Vec::is_empty)
        })
    }

    pub fn handle(&self, event: KeyboardEvent) -> bool {
        self.dispatch(event, None)
    }

    /// Dispatches a command binding. Direct callback registrations continue to
    /// use [`Self::handle`] for source compatibility.
    #[must_use]
    pub fn handle_actions(&self, event: KeyboardEvent, actions: &Actions<C>) -> bool {
        self.dispatch(event, Some(actions))
    }

    /// Alias emphasizing that this is the complete keyboard-event path.
    #[must_use]
    pub fn handle_event(&self, event: KeyboardEvent, actions: &Actions<C>) -> bool {
        self.handle_actions(event, actions)
    }

    /// Creates a type-erased scope for the retained focus-chain dispatcher.
    #[must_use]
    pub fn erased_scope(self: &Rc<Self>) -> Rc<ErasedShortcutScope> {
        ErasedShortcutScope::from_shortcuts(self.clone())
    }

    fn replace_binding<K>(
        bindings: &mut HashMap<K, Vec<ShortcutRegistration<C>>>,
        key: K,
        trigger: ShortcutTrigger,
        binding: ShortcutBinding<C>,
    ) where
        K: Eq + std::hash::Hash,
    {
        let registrations = bindings.entry(key).or_default();
        registrations.retain(|registration| registration.trigger != trigger);
        registrations.push(ShortcutRegistration { trigger, binding });
    }

    fn dispatch(&self, event: KeyboardEvent, actions: Option<&Actions<C>>) -> bool {
        let physical = ShortcutKey::new(event.code, event.modifiers);
        let logical = LogicalShortcutKey::new(event.key.clone(), event.modifiers);
        for scope in self.scopes.iter().rev() {
            if scope.physical.get(&physical).is_some_and(|registrations| {
                Self::dispatch_registrations(registrations, &event, actions)
            }) || scope.logical.get(&logical).is_some_and(|registrations| {
                Self::dispatch_registrations(registrations, &event, actions)
            }) {
                return true;
            }
        }
        false
    }

    fn find_match(&self, event: &KeyboardEvent) -> Option<ShortcutMatch> {
        let physical = ShortcutKey::new(event.code, event.modifiers);
        let logical = LogicalShortcutKey::new(event.key.clone(), event.modifiers);
        for scope in self.scopes.iter().rev() {
            if let Some(registration) = scope.physical.get(&physical).and_then(|registrations| {
                registrations
                    .iter()
                    .find(|registration| registration.trigger.matches(event))
            }) {
                return Some(Self::erase_binding(&registration.binding));
            }
            if let Some(registration) = scope.logical.get(&logical).and_then(|registrations| {
                registrations
                    .iter()
                    .find(|registration| registration.trigger.matches(event))
            }) {
                return Some(Self::erase_binding(&registration.binding));
            }
        }
        None
    }

    fn erase_binding(binding: &ShortcutBinding<C>) -> ShortcutMatch {
        match binding {
            ShortcutBinding::Callback(callback) => ShortcutMatch::Callback(callback.clone()),
            ShortcutBinding::Intent(intent) => {
                ShortcutMatch::Intent(ErasedIntent::from_intent(intent))
            }
        }
    }

    fn dispatch_registrations(
        registrations: &[ShortcutRegistration<C>],
        event: &KeyboardEvent,
        actions: Option<&Actions<C>>,
    ) -> bool {
        registrations
            .iter()
            .filter(|registration| registration.trigger.matches(event))
            .any(|registration| match &registration.binding {
                ShortcutBinding::Callback(callback) => {
                    callback();
                    true
                }
                ShortcutBinding::Intent(intent) => {
                    actions.is_some_and(|actions| actions.invoke_intent(intent))
                }
            })
    }

    fn current_scope_mut(&mut self) -> &mut ShortcutScope<C> {
        if self.scopes.is_empty() {
            self.scopes.push(ShortcutScope {
                physical: HashMap::new(),
                logical: HashMap::new(),
            });
        }
        self.scopes
            .last_mut()
            .expect("shortcut registry has a root scope")
    }
}

impl Shortcuts<Command> {
    /// Creates an empty string-backed shortcut map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
