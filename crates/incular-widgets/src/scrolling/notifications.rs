use super::*;

/// Flutter's `NotificationListener`: listens for notifications bubbling up
/// the widget tree.
///
/// [`NotificationListener::on_notification`] receives scroll notifications
/// from descendant viewports. [`NotificationListener::on`] receives any
/// other notification type that a descendant sends with
/// [`NotificationDispatcher::dispatch`](crate::NotificationDispatcher::dispatch),
/// such as [`SizeChangedLayoutNotification`](crate::SizeChangedLayoutNotification).
#[derive(Clone, TypedBuilder)]
pub struct NotificationListener {
    #[builder(default)]
    callback: Option<Rc<dyn Fn(ScrollNotification) -> bool>>,
    #[builder(default, setter(skip))]
    handlers: Vec<crate::notification::NotificationHandler>,
    #[builder(setter(into))]
    child: Widget,
}

impl NotificationListener {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            callback: None,
            handlers: Vec::new(),
            child: child.into(),
        }
    }

    /// Flutter `NotificationListener<T>.onNotification` for notifications
    /// of type `T`. Returning `true` stops the notification from reaching an
    /// outer listener.
    #[must_use]
    pub fn on<T: std::any::Any>(mut self, callback: impl Fn(&T) -> bool + 'static) -> Self {
        self.handlers
            .push(crate::notification::NotificationHandler::new(callback));
        self
    }

    /// Receives scroll notifications from descendant viewports. Returning
    /// `true` stops the notification from reaching an outer listener.
    #[must_use]
    pub fn on_notification(
        mut self,
        callback: impl Fn(ScrollNotification) -> bool + 'static,
    ) -> Self {
        self.callback = Some(Rc::new(callback));
        self
    }
}

impl From<NotificationListener> for Widget {
    fn from(value: NotificationListener) -> Self {
        let child = Widget::notification_listener(value.callback, value.child);
        if value.handlers.is_empty() {
            child
        } else {
            crate::notification::listen(value.handlers, child)
        }
    }
}

/// Observes scroll notifications.
#[derive(Clone, TypedBuilder)]
pub struct ScrollNotificationObserver {
    #[builder(setter(into))]
    child: Widget,
}

impl ScrollNotificationObserver {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }
}

impl From<ScrollNotificationObserver> for Widget {
    fn from(value: ScrollNotificationObserver) -> Self {
        value.child
    }
}
