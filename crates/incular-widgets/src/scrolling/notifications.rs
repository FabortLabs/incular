use super::*;

/// Flutter's `NotificationListener`: listens for notifications bubbling up
/// the widget tree.
///
/// [`NotificationListener::on_notification`] receives scroll notifications
/// from descendant viewports. [`NotificationListener::on`] also accepts
/// typed notifications and [`LayoutChangedNotification`](crate::LayoutChangedNotification),
/// which receives both size changes and scrolling (but not metrics changes).
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
        use crate::LayoutChangedNotification;

        let scroll_handlers: Vec<_> = value
            .handlers
            .iter()
            .filter(|handler| {
                handler.handles::<ScrollNotification>()
                    || handler.handles::<LayoutChangedNotification>()
            })
            .cloned()
            .collect();
        let mut callback = value.callback;
        if !scroll_handlers.is_empty() {
            let direct = callback.take();
            callback = Some(Rc::new(move |scroll| {
                if direct.as_ref().is_some_and(|direct| direct(scroll)) {
                    return true;
                }
                let layout_change = LayoutChangedNotification::for_notification(&scroll);
                scroll_handlers
                    .iter()
                    .any(|handler| handler.notify(&scroll, layout_change.as_ref()))
            }));
        }
        let child = Widget::notification_listener(callback, value.child);
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
