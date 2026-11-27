//! Rust lifecycle equivalent of Flutter's AutomaticKeepAliveClientMixin.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use crate::{
    BuildContext, NotificationDispatcher, Widget, notification::NotificationRoute,
    tree::RetainedState,
};

use super::{KeepAliveHandle, KeepAliveNotification};

/// Controls a mounted automatic keep-alive client. A change takes effect
/// even while the child's sliver row is parked.
#[derive(Clone, Default)]
pub struct AutomaticKeepAliveClientController(Rc<ClientControllerState>);

#[derive(Default)]
struct ClientControllerState {
    want_keep_alive: Cell<bool>,
    clients: RefCell<Vec<Weak<MountedClient>>>,
}

impl AutomaticKeepAliveClientController {
    #[must_use]
    pub fn new(want_keep_alive: bool) -> Self {
        let controller = Self::default();
        controller.0.want_keep_alive.set(want_keep_alive);
        controller
    }

    #[must_use]
    pub fn want_keep_alive(&self) -> bool {
        self.0.want_keep_alive.get()
    }

    pub fn set_want_keep_alive(&self, want_keep_alive: bool) {
        if self.0.want_keep_alive.replace(want_keep_alive) == want_keep_alive {
            return;
        }
        let clients = self.0.clients.borrow().clone();
        for client in clients.into_iter().filter_map(|client| client.upgrade()) {
            client.sync();
        }
    }

    fn detach(&self, client: &MountedClient) {
        self.0
            .clients
            .borrow_mut()
            .retain(|entry| !std::ptr::eq(entry.as_ptr(), client) && entry.strong_count() != 0);
    }
}

/// A descendant that holds a keep-alive request for its mounted slot.
/// Equivalent to Flutter's `AutomaticKeepAliveClientMixin`: it acquires on
/// first build, releases on removal, and transfers the request on reparenting.
#[derive(Clone)]
pub struct AutomaticKeepAliveClient {
    controller: AutomaticKeepAliveClientController,
    child: Widget,
}

impl AutomaticKeepAliveClient {
    #[must_use]
    pub fn new(controller: AutomaticKeepAliveClientController, child: impl Into<Widget>) -> Self {
        Self {
            controller,
            child: child.into(),
        }
    }
}

impl From<AutomaticKeepAliveClient> for Widget {
    fn from(client: AutomaticKeepAliveClient) -> Widget {
        Widget::mount_scoped_builder(
            |_| MountedClient::default(),
            move |state, context| {
                state.bind(&client.controller, context);
                client.child.clone()
            },
        )
    }
}

#[derive(Default)]
struct MountedClient {
    controller: RefCell<Option<AutomaticKeepAliveClientController>>,
    dispatcher: RefCell<Option<NotificationDispatcher>>,
    lease: RefCell<Option<(KeepAliveHandle, NotificationRoute)>>,
}

impl MountedClient {
    fn bind(
        self: &Rc<Self>,
        controller: &AutomaticKeepAliveClientController,
        context: &BuildContext<'_>,
    ) {
        let current = self.controller.borrow().clone();
        if !current
            .as_ref()
            .is_some_and(|old| Rc::ptr_eq(&old.0, &controller.0))
        {
            if let Some(old) = current {
                old.detach(self);
            }
            controller.0.clients.borrow_mut().push(Rc::downgrade(self));
            self.controller.replace(Some(controller.clone()));
        }
        self.dispatcher
            .replace(Some(NotificationDispatcher::of(context)));
        self.sync();
    }

    fn sync(&self) {
        let keep = self
            .controller
            .borrow()
            .as_ref()
            .is_some_and(AutomaticKeepAliveClientController::want_keep_alive);
        if !keep {
            self.release();
            return;
        }
        let Some(dispatcher) = self.dispatcher.borrow().clone() else {
            return;
        };
        if self
            .lease
            .borrow()
            .as_ref()
            .is_some_and(|(_, route)| route.matches(&dispatcher))
        {
            return;
        }
        self.release();
        let handle = KeepAliveHandle::new();
        self.lease
            .replace(Some((handle.clone(), dispatcher.route())));
        dispatcher.dispatch(&KeepAliveNotification::new(handle));
    }

    fn release(&self) {
        let previous = self.lease.take();
        if let Some((handle, _)) = previous {
            handle.release();
        }
    }
}

impl RetainedState for MountedClient {
    const DISPOSE_ON_UNMOUNT: bool = true;

    fn dispose(&self) {
        let controller = self.controller.take();
        if let Some(controller) = controller {
            controller.detach(self);
        }
        self.dispatcher.take();
        self.release();
    }

    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

impl Drop for MountedClient {
    fn drop(&mut self) {
        self.dispose();
    }
}
