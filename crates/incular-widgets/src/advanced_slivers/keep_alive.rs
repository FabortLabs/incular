use super::*;
use crate::{
    BuildContext,
    notification::{ListenerState, NotificationHandler},
    tree::RetainedState,
};
use incular_core::ConsumerId;
use std::cell::{Cell, OnceCell};

/// A disposable descendant keep-alive handle.
///
/// Flutter's handle is a `Listenable`: disposing it tells the nearest
/// `AutomaticKeepAlive` that this descendant no longer needs retention.  The
/// callback list is deliberately cloneable so one client can pass a handle
/// through several widget layers without changing its identity.
#[derive(Clone)]
pub struct KeepAliveHandle {
    inner: Rc<RefCell<KeepAliveHandleState>>,
}

struct KeepAliveHandleState {
    id: u64,
    released: bool,
    next_listener: u64,
    listeners: Vec<(u64, Rc<dyn Fn()>)>,
}

impl KeepAliveHandle {
    #[must_use]
    pub fn new() -> Self {
        static NEXT_HANDLE_ID: IdGenerator = IdGenerator::new();
        Self {
            inner: Rc::new(RefCell::new(KeepAliveHandleState {
                id: NEXT_HANDLE_ID.next(),
                released: false,
                next_listener: 1,
                listeners: Vec::new(),
            })),
        }
    }

    #[must_use]
    pub fn id(&self) -> u64 {
        self.inner.borrow().id
    }

    #[must_use]
    pub fn is_released(&self) -> bool {
        self.inner.borrow().released
    }

    /// Releases this handle and synchronously notifies every automatic
    /// keep-alive ancestor that registered it. This is idempotent.
    pub fn release(&self) {
        let listeners = {
            let mut state = self.inner.borrow_mut();
            if state.released {
                return;
            }
            state.released = true;
            std::mem::take(&mut state.listeners)
        };
        for (_, listener) in listeners {
            listener();
        }
    }

    /// Whether any keep-alive ancestor is currently listening to this handle.
    #[must_use]
    pub fn has_listeners(&self) -> bool {
        !self.inner.borrow().listeners.is_empty()
    }

    fn add_listener(&self, listener: Rc<dyn Fn()>) -> Option<u64> {
        let mut state = self.inner.borrow_mut();
        if state.released {
            return None;
        }
        let id = state.next_listener;
        state.next_listener = state
            .next_listener
            .checked_add(1)
            .expect("keep-alive listener identity space exhausted");
        state.listeners.push((id, listener));
        Some(id)
    }

    fn remove_listener(&self, listener_id: u64) {
        self.inner
            .borrow_mut()
            .listeners
            .retain(|(id, _)| *id != listener_id);
    }
}

impl Default for KeepAliveHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// Descendant notification consumed by `AutomaticKeepAlive`.
#[derive(Clone)]
pub struct KeepAliveNotification {
    handle: KeepAliveHandle,
}

impl KeepAliveNotification {
    #[must_use]
    pub fn new(handle: KeepAliveHandle) -> Self {
        Self { handle }
    }

    #[must_use]
    pub fn handle(&self) -> KeepAliveHandle {
        self.handle.clone()
    }

    /// Sends this request through the mounted descendant's ancestors. A
    /// dispatcher captured from the context can also send it later.
    pub fn dispatch(&self, context: &BuildContext<'_>) -> bool {
        crate::NotificationDispatcher::of(context).dispatch(self)
    }
}

type KeepAliveChange = Rc<dyn Fn(bool)>;

struct KeepAliveRegistryState {
    clients: HashMap<u64, (KeepAliveHandle, u64)>,
    revision: u64,
    watchers: HashMap<u64, KeepAliveChange>,
}

impl KeepAliveRegistryState {
    fn active_changed(&mut self, previously_keeping_alive: bool) -> Vec<KeepAliveChange> {
        self.revision = self.revision.checked_add(1).expect("revision exhausted");
        let keeping_alive = !self.clients.is_empty();
        if previously_keeping_alive != keeping_alive {
            self.watchers.values().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

impl Drop for KeepAliveRegistryState {
    fn drop(&mut self) {
        for (_, (handle, listener_id)) in self.clients.drain() {
            handle.remove_listener(listener_id);
        }
    }
}

/// Ancestor-side registry implementing the notification semantics of
/// `AutomaticKeepAlive`.
#[derive(Clone)]
pub struct KeepAliveRegistry {
    state: Rc<RefCell<KeepAliveRegistryState>>,
}

impl KeepAliveRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(KeepAliveRegistryState {
                clients: HashMap::new(),
                revision: 0,
                watchers: HashMap::new(),
            })),
        }
    }

    /// Registers a notification handle. `false` mirrors Flutter's
    /// `NotificationListener` behavior: keep-alive notifications continue to
    /// bubble to outer ancestors.
    pub fn dispatch(&self, notification: KeepAliveNotification) -> bool {
        let handle = notification.handle;
        let id = handle.id();
        if handle.is_released() {
            return false;
        }
        {
            let state = self.state.borrow();
            if state.clients.contains_key(&id) {
                return false;
            }
        }
        let weak_state: Weak<RefCell<KeepAliveRegistryState>> = Rc::downgrade(&self.state);
        let Some(listener_id) = handle.add_listener(Rc::new(move || {
            if let Some(state) = weak_state.upgrade() {
                let listeners = {
                    let mut state = state.borrow_mut();
                    if state.clients.remove(&id).is_none() {
                        return;
                    }
                    state.active_changed(true)
                };
                for listener in listeners {
                    listener(false);
                }
            }
        })) else {
            return false;
        };
        let listeners = {
            let mut state = self.state.borrow_mut();
            let was_active = !state.clients.is_empty();
            state.clients.insert(id, (handle, listener_id));
            state.active_changed(was_active)
        };
        for listener in listeners {
            listener(true);
        }
        false
    }

    pub fn notify(&self, notification: KeepAliveNotification) -> bool {
        self.dispatch(notification)
    }

    #[must_use]
    pub fn keeping_alive(&self) -> bool {
        !self.state.borrow().clients.is_empty()
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.state.borrow().clients.len()
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.state.borrow().revision
    }

    /// Detaches every registered listener. Existing handles remain valid and
    /// can be dispatched to a different automatic ancestor.
    pub fn clear(&self) {
        let (clients, listeners) = {
            let mut state = self.state.borrow_mut();
            let was_active = !state.clients.is_empty();
            let clients = std::mem::take(&mut state.clients);
            (clients, state.active_changed(was_active))
        };
        for (_, (handle, listener_id)) in clients {
            handle.remove_listener(listener_id);
        }
        for listener in listeners {
            listener(false);
        }
    }

    fn watch(&self, listener: KeepAliveChange) -> KeepAliveWatch {
        static NEXT_WATCH: IdGenerator = IdGenerator::new();
        let id = NEXT_WATCH.next();
        self.state.borrow_mut().watchers.insert(id, listener);
        KeepAliveWatch {
            registry: Rc::downgrade(&self.state),
            id,
        }
    }
}

struct KeepAliveWatch {
    registry: Weak<RefCell<KeepAliveRegistryState>>,
    id: u64,
}

impl Drop for KeepAliveWatch {
    fn drop(&mut self) {
        if let Some(registry) = self.registry.upgrade() {
            registry.borrow_mut().watchers.remove(&self.id);
        }
    }
}

impl Default for KeepAliveRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parent-data intent equivalent to Flutter's `KeepAlive` widget.
///
/// The request travels as parent data on the child descriptor. Conversion to
/// a `Widget` stays transparent (no extra render object or widget kind): the
/// flag is read by sliver materializers, which keep a requesting child
/// retained outside the cache window so its state survives scrolling away.
#[derive(Clone)]
pub struct KeepAlive {
    keep_alive: bool,
    child: Widget,
}

impl KeepAlive {
    #[must_use]
    pub fn new(keep_alive: bool, child: impl Into<Widget>) -> Self {
        Self {
            keep_alive,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn keep_alive(&self) -> bool {
        self.keep_alive
    }

    #[must_use]
    pub fn child(&self) -> &Widget {
        &self.child
    }

    #[must_use]
    pub fn request(&self) -> KeepAliveRequest {
        KeepAliveRequest {
            keep_alive: self.keep_alive,
        }
    }

    #[must_use]
    pub fn into_child(self) -> Widget {
        self.child.with_keep_alive_request(self.keep_alive)
    }
}

impl From<KeepAlive> for Widget {
    fn from(keep_alive: KeepAlive) -> Self {
        keep_alive.into_child()
    }
}

/// The parent-data value a sliver materializer must read from `KeepAlive`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeepAliveRequest {
    pub keep_alive: bool,
}

/// An automatic ancestor that aggregates all active descendant handles.
#[derive(Clone)]
pub struct AutomaticKeepAlive {
    child: Widget,
    registry: Rc<OnceCell<KeepAliveRegistry>>,
}

impl AutomaticKeepAlive {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            registry: Rc::new(OnceCell::new()),
        }
    }

    #[must_use]
    pub fn with_registry(child: impl Into<Widget>, registry: KeepAliveRegistry) -> Self {
        Self {
            child: child.into(),
            registry: Rc::new(OnceCell::from(registry)),
        }
    }

    /// Exposes a registry that can receive requests directly. Obtain it
    /// before converting this value to a Widget if the mounted scope should
    /// use it. Ordinary descendants use KeepAliveNotification instead.
    #[must_use]
    pub fn registry(&self) -> KeepAliveRegistry {
        self.registry.get_or_init(KeepAliveRegistry::new).clone()
    }

    #[must_use]
    pub fn keeping_alive(&self) -> bool {
        self.registry
            .get()
            .is_some_and(KeepAliveRegistry::keeping_alive)
    }

    #[must_use]
    pub fn child(&self) -> &Widget {
        &self.child
    }

    /// Returns `false` so outer automatic ancestors can also observe the
    /// notification, matching Flutter's bubbling listener.
    pub fn on_notification(&self, notification: KeepAliveNotification) -> bool {
        self.registry().dispatch(notification)
    }

    #[must_use]
    pub fn into_child(self) -> Widget {
        let Self { child, registry } = self;
        let shared_registry = registry.get().cloned();
        let explicit = child.explicit_keep_alive_request();
        Widget::mount_scoped_builder(
            |_| MountedAutomaticKeepAlive::new(),
            move |state, context| state.build(context, shared_registry.as_ref(), child.clone()),
        )
        .with_automatic_keep_alive_scope()
        .with_keep_alive_request(explicit)
    }
}

impl From<AutomaticKeepAlive> for Widget {
    fn from(automatic: AutomaticKeepAlive) -> Self {
        automatic.into_child()
    }
}

type PendingKeepAlives = RefCell<HashMap<ConsumerId, bool>>;

/// One sparse queue per tree. Only a first acquisition or last release
/// changes parent data; inactive rows do not participate in frame work.
#[derive(Clone, Default)]
pub(crate) struct AutomaticKeepAliveQueue(Rc<PendingKeepAlives>);

impl AutomaticKeepAliveQueue {
    pub(crate) fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    pub(crate) fn take(&self) -> HashMap<ConsumerId, bool> {
        std::mem::take(&mut *self.0.borrow_mut())
    }

    fn listener(&self, id: ConsumerId) -> KeepAliveChange {
        Self::listener_for(Rc::downgrade(&self.0), id)
    }

    fn listener_for(pending: Weak<PendingKeepAlives>, id: ConsumerId) -> KeepAliveChange {
        Rc::new(move |keep_alive| {
            if let Some(pending) = pending.upgrade() {
                let was_empty = {
                    let mut pending = pending.borrow_mut();
                    let was_empty = pending.is_empty();
                    pending.insert(id, keep_alive);
                    was_empty
                };
                if was_empty {
                    incular_animation::scheduler::schedule_frame();
                }
            }
        })
    }
}

/// An implied delegate wrapper uses the child's existing element and key.
pub(crate) struct ImplicitAutomaticKeepAlive {
    scope: ListenerState,
    handlers: Rc<[NotificationHandler]>,
    // Only rows that actually request retention create a registry and a
    // watcher. A dispatcher retained past unmount holds just a weak pointer.
    _registry: Rc<OnceCell<(KeepAliveRegistry, KeepAliveWatch)>>,
}

impl ImplicitAutomaticKeepAlive {
    pub(crate) fn new(queue: &AutomaticKeepAliveQueue, id: ConsumerId) -> Self {
        let registry = Rc::new(OnceCell::new());
        let weak = Rc::downgrade(&registry);
        let pending = Rc::downgrade(&queue.0);
        let handler = NotificationHandler::new(move |notification: &KeepAliveNotification| {
            if let Some(registration) = weak.upgrade() {
                let (registry, _) = registration.get_or_init(|| {
                    let registry = KeepAliveRegistry::new();
                    let watch =
                        registry.watch(AutomaticKeepAliveQueue::listener_for(pending.clone(), id));
                    (registry, watch)
                });
                registry.dispatch(notification.clone());
            }
            false
        });
        Self {
            scope: ListenerState::new(),
            handlers: Rc::from([handler]),
            _registry: registry,
        }
    }

    pub(crate) fn inherit(
        &self,
        context: &incular_core::BuildContext,
    ) -> incular_core::BuildContext {
        self.scope.inherit(context, self.handlers.clone())
    }
}

impl Drop for ImplicitAutomaticKeepAlive {
    fn drop(&mut self) {
        self.scope.detach();
    }
}

struct MountedAutomaticKeepAlive {
    scope: ListenerState,
    registry: RefCell<Option<MountedRegistry>>,
}

struct MountedRegistry {
    registry: KeepAliveRegistry,
    shared: bool,
    watch: Option<KeepAliveWatch>,
    handlers: Rc<[NotificationHandler]>,
}

impl Drop for MountedRegistry {
    fn drop(&mut self) {
        self.watch.take();
        if !self.shared {
            self.registry.clear();
        }
    }
}

impl MountedAutomaticKeepAlive {
    fn new() -> Self {
        Self {
            scope: ListenerState::new(),
            registry: RefCell::new(None),
        }
    }

    fn build(
        &self,
        context: &BuildContext<'_>,
        shared: Option<&KeepAliveRegistry>,
        child: Widget,
    ) -> Widget {
        let mut mounted = self.registry.borrow_mut();
        if !mounted.as_ref().is_some_and(|old| {
            shared.map_or(!old.shared, |shared| {
                old.shared && Rc::ptr_eq(&old.registry.state, &shared.state)
            })
        }) {
            let registry = shared.cloned().unwrap_or_default();
            let listener = context
                .find::<AutomaticKeepAliveQueue>()
                .map(|queue| queue.listener(context.consumer_id()));
            let watch = listener.map(|listener| {
                if registry.keeping_alive() || mounted.is_some() {
                    listener(registry.keeping_alive());
                }
                registry.watch(listener)
            });
            let weak = Rc::downgrade(&registry.state);
            let handlers = Rc::from([NotificationHandler::new(
                move |notification: &KeepAliveNotification| {
                    weak.upgrade().is_some_and(|state| {
                        KeepAliveRegistry { state }.dispatch(notification.clone())
                    })
                },
            )]);
            let _ = mounted.replace(MountedRegistry {
                registry,
                shared: shared.is_some(),
                watch,
                handlers,
            });
        }
        let handlers = mounted
            .as_ref()
            .expect("automatic keep-alive registry")
            .handlers
            .clone();
        drop(mounted);
        self.scope.wrap(context, handlers, child)
    }
}

impl RetainedState for MountedAutomaticKeepAlive {
    const DISPOSE_ON_UNMOUNT: bool = true;

    fn dispose(&self) {
        self.scope.detach();
        self.registry.borrow_mut().take();
    }

    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}
