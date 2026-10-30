//! Flutter's notification bubbling (`widgets/notification_listener.dart`):
//! `Notification.dispatch` walks up from a widget to each enclosing
//! [`NotificationListener`](crate::NotificationListener) that handles the
//! notification's type, until one returns `true`.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    rc::Rc,
};

use crate::{BuildContext, Widget};

/// A callback over a notification of any type.
type ErasedCallback = Rc<dyn Fn(&dyn Any) -> bool>;

/// A typed `onNotification` callback. It returns `true` to stop the
/// notification from bubbling further.
#[derive(Clone)]
pub(crate) struct NotificationHandler {
    type_id: TypeId,
    callback: ErasedCallback,
}

impl NotificationHandler {
    pub(crate) fn new<T: Any>(callback: impl Fn(&T) -> bool + 'static) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            callback: Rc::new(move |notification| {
                notification.downcast_ref::<T>().is_some_and(&callback)
            }),
        }
    }
}

/// One listener in the bubbling chain. Its handlers and parent are
/// refreshed on every build of the listener, so a dispatcher captured by a
/// descendant always reaches the current callbacks.
struct ListenerNode {
    handlers: RefCell<Vec<NotificationHandler>>,
    parent: RefCell<Option<Rc<ListenerNode>>>,
}

/// The inherited link to the nearest listener.
struct ListenerScope(Rc<ListenerNode>);

/// Flutter's `Notification.dispatch(context)`, captured from a
/// [`BuildContext`] so a notification can be sent later, for example during
/// layout.
#[derive(Clone, Default)]
pub struct NotificationDispatcher {
    nearest: Option<Rc<ListenerNode>>,
}

impl NotificationDispatcher {
    /// The listeners enclosing `context`. Reading them does not make the
    /// build depend on them.
    #[must_use]
    pub fn of(context: &BuildContext<'_>) -> Self {
        Self {
            nearest: context
                .find_shared::<ListenerScope>()
                .map(|scope| scope.0.clone()),
        }
    }

    /// Bubbles `notification` to each enclosing listener with a handler for
    /// its type, innermost first, stopping at the first that returns `true`.
    /// Returns whether one did.
    pub fn dispatch<T: Any>(&self, notification: &T) -> bool {
        let mut node = self.nearest.clone();
        while let Some(current) = node {
            let handlers = current.handlers.borrow().clone();
            for handler in handlers {
                if handler.type_id == TypeId::of::<T>() && (handler.callback)(notification) {
                    return true;
                }
            }
            node = current.parent.borrow().clone();
        }
        false
    }
}

struct ListenerState(Rc<ListenerNode>);

impl crate::tree::RetainedState for ListenerState {
    fn rebind_revision(&self, _revision: Rc<std::cell::Cell<u64>>) {}
}

/// Installs `handlers` as one listener around `child`.
pub(crate) fn listen(handlers: Vec<NotificationHandler>, child: Widget) -> Widget {
    Widget::retained_builder(
        |_| {
            ListenerState(Rc::new(ListenerNode {
                handlers: RefCell::new(Vec::new()),
                parent: RefCell::new(None),
            }))
        },
        move |state, context, _| {
            let node = &state.0;
            *node.handlers.borrow_mut() = handlers.clone();
            *node.parent.borrow_mut() = context
                .find_shared::<ListenerScope>()
                .map(|scope| scope.0.clone());
            Widget::shared_environment_scope(Rc::new(ListenerScope(node.clone())), child.clone())
        },
        false,
    )
}
