//! Flutter's `SizeChangedLayoutNotifier`
//! (`widgets/size_changed_layout_notifier.dart`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_core::Size;

use crate::tree::RetainedState;
use crate::{NotificationDispatcher, Widget};

/// Flutter's `SizeChangedLayoutNotification`, sent by a
/// [`SizeChangedLayoutNotifier`] to the enclosing
/// [`NotificationListener`](crate::NotificationListener)s, received with
/// [`NotificationListener::on`](crate::NotificationListener::on).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SizeChangedLayoutNotification;

/// Flutter's `SizeChangedLayoutNotifier`: on each layout in which the
/// child's size differs from its previous size, but not the first, it
/// dispatches a [`SizeChangedLayoutNotification`] to the enclosing
/// listeners. It runs during layout.
pub struct SizeChangedLayoutNotifier {
    on_changed: Option<Rc<dyn Fn(Size)>>,
    child: Widget,
}

impl SizeChangedLayoutNotifier {
    /// Notifies the enclosing listeners and also calls `on_changed` with the
    /// new size, before the notification is dispatched.
    #[must_use]
    pub fn new(on_changed: impl Fn(Size) + 'static, child: impl Into<Widget>) -> Self {
        Self {
            on_changed: Some(Rc::new(on_changed)),
            child: child.into(),
        }
    }

    /// Flutter's `SizeChangedLayoutNotifier(child: ...)`: only notifies the
    /// enclosing listeners.
    #[must_use]
    pub fn notifying(child: impl Into<Widget>) -> Self {
        Self {
            on_changed: None,
            child: child.into(),
        }
    }
}

/// `_RenderSizeChangedWithCallback._oldSize`.
struct LastSize(Rc<Cell<Option<Size>>>);

impl RetainedState for LastSize {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

impl From<SizeChangedLayoutNotifier> for Widget {
    fn from(value: SizeChangedLayoutNotifier) -> Self {
        let value = RefCell::new(value);
        Widget::retained_builder(
            |_| LastSize(Rc::new(Cell::new(None))),
            move |state, context, _| {
                let value = value.borrow();
                let last = state.0.clone();
                let on_changed = value.on_changed.clone();
                let dispatcher = NotificationDispatcher::of(context);
                Widget::size_observer(
                    move |size| {
                        if last.replace(Some(size)).is_some_and(|old| old != size) {
                            if let Some(on_changed) = &on_changed {
                                on_changed(size);
                            }
                            dispatcher.dispatch(&SizeChangedLayoutNotification);
                        }
                    },
                    value.child.clone(),
                )
            },
            false,
        )
    }
}
