//! Flutter's notification bubbling (`widgets/notification_listener.dart`):
//! `Notification.dispatch` walks up from a widget to each enclosing
//! [`NotificationListener`](crate::NotificationListener) that handles the
//! notification's type, until one returns `true`.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    rc::Rc,
};

use crate::{
    BuildContext, ScrollNotification, ScrollNotificationType, SizeChangedLayoutNotification, Widget,
};

/// A layout-change notification or one of the two framework notifications
/// that extend it in Flutter. Viewport metrics changes are not scroll
/// activity and do not reach this listener.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LayoutChangedNotification {
    #[default]
    Unspecified,
    SizeChanged(SizeChangedLayoutNotification),
    Scroll(ScrollNotification),
}

impl LayoutChangedNotification {
    pub(crate) fn for_notification(event: &dyn Any) -> Option<Self> {
        if let Some(event) = event.downcast_ref::<SizeChangedLayoutNotification>() {
            return Some(Self::SizeChanged(*event));
        }
        let event = event.downcast_ref::<ScrollNotification>()?;
        (event.kind != ScrollNotificationType::Metrics).then_some(Self::Scroll(*event))
    }
}

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

    pub(crate) fn handles<T: Any>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    pub(crate) fn notify(
        &self,
        notification: &dyn Any,
        layout_change: Option<&LayoutChangedNotification>,
    ) -> bool {
        if self.type_id == notification.type_id() {
            (self.callback)(notification)
        } else if self.handles::<LayoutChangedNotification>() {
            layout_change.is_some_and(|change| (self.callback)(change))
        } else {
            false
        }
    }
}

/// One listener in the bubbling chain. Its handlers and parent are
/// refreshed on every build of the listener, so a dispatcher captured by a
/// descendant always reaches the current callbacks.
struct ListenerNode {
    handlers: RefCell<Rc<[NotificationHandler]>>,
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

    pub(crate) fn route(&self) -> NotificationRoute {
        NotificationRoute(self.ancestors().collect())
    }

    fn ancestors(&self) -> impl Iterator<Item = Rc<ListenerNode>> {
        let mut node = self.nearest.clone();
        std::iter::from_fn(move || {
            let current = node.take()?;
            node = current.parent.borrow().clone();
            Some(current)
        })
    }

    /// Bubbles `notification` to each enclosing listener with a handler for
    /// its type, innermost first, stopping at the first that returns `true`.
    /// Layout-change listeners can receive qualifying concrete notifications.
    /// Returns whether a listener stopped propagation.
    pub fn dispatch<T: Any>(&self, notification: &T) -> bool {
        let layout_change = LayoutChangedNotification::for_notification(notification);
        let mut node = self.nearest.clone();
        while let Some(current) = node {
            let handlers = current.handlers.borrow().clone();
            for handler in handlers.iter() {
                if handler.notify(notification, layout_change.as_ref()) {
                    return true;
                }
            }
            node = current.parent.borrow().clone();
        }
        false
    }
}

/// The exact chain at dispatch time, before a retained nearest listener may
/// have been moved under a different outer listener.
pub(crate) struct NotificationRoute(Vec<Rc<ListenerNode>>);

impl NotificationRoute {
    pub(crate) fn matches(&self, dispatcher: &NotificationDispatcher) -> bool {
        let mut ancestors = dispatcher.ancestors();
        self.0
            .iter()
            .all(|bound| ancestors.next().is_some_and(|now| Rc::ptr_eq(bound, &now)))
            && ancestors.next().is_none()
    }
}

pub(crate) struct ListenerState(Rc<ListenerNode>);

impl ListenerState {
    pub(crate) fn new() -> Self {
        Self(Rc::new(ListenerNode {
            handlers: RefCell::new(Rc::from([])),
            parent: RefCell::new(None),
        }))
    }

    pub(crate) fn wrap(
        &self,
        context: &BuildContext<'_>,
        handlers: Rc<[NotificationHandler]>,
        child: Widget,
    ) -> Widget {
        *self.0.handlers.borrow_mut() = handlers;
        *self.0.parent.borrow_mut() = context
            .find_shared::<ListenerScope>()
            .map(|scope| scope.0.clone());
        Widget::shared_environment_scope(Rc::new(ListenerScope(self.0.clone())), child)
    }

    /// A listener which is parent data on an existing element has no
    /// declarative wrapper. Keep its BuildContext chain just like a widget
    /// listener, including when the element is moved under a new parent.
    pub(crate) fn inherit(
        &self,
        context: &incular_core::BuildContext,
        handlers: Rc<[NotificationHandler]>,
    ) -> incular_core::BuildContext {
        *self.0.handlers.borrow_mut() = handlers;
        *self.0.parent.borrow_mut() = context
            .read_shared::<ListenerScope>()
            .map(|scope| scope.0.clone());
        context.provide(ListenerScope(self.0.clone()))
    }

    pub(crate) fn detach(&self) {
        self.0.handlers.replace(Rc::from([]));
        self.0.parent.borrow_mut().take();
    }
}

impl crate::tree::RetainedState for ListenerState {
    fn rebind_revision(&self, _revision: Rc<std::cell::Cell<u64>>) {}
}

/// Installs `handlers` as one listener around `child`.
pub(crate) fn listen(handlers: Vec<NotificationHandler>, child: Widget) -> Widget {
    let handlers: Rc<[NotificationHandler]> = handlers.into();
    Widget::retained_builder(
        |_| ListenerState::new(),
        move |state, context, _| state.wrap(context, handlers.clone(), child.clone()),
        false,
    )
}
