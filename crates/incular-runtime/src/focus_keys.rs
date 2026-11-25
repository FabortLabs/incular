//! Window-local handlers surrounding the focused keyboard route.

use incular_core::KeyboardEvent;
use incular_widgets::KeyEventResult;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

type Handler = Rc<dyn Fn(&KeyboardEvent) -> KeyEventResult>;
type Snapshot = Rc<Vec<Handler>>;

/// Ordered copy-on-write list. Callbacks can remove registrations without
/// mutating the snapshot being dispatched.
#[derive(Default)]
pub(crate) struct FocusKeyHandlers {
    handlers: Rc<RefCell<Snapshot>>,
}

impl FocusKeyHandlers {
    pub(crate) fn add(
        &self,
        callback: impl Fn(&KeyboardEvent) -> KeyEventResult + 'static,
    ) -> FocusKeySubscription {
        let callback: Handler = Rc::new(callback);
        Rc::make_mut(&mut self.handlers.borrow_mut()).push(callback.clone());
        FocusKeySubscription {
            callback,
            handlers: Rc::downgrade(&self.handlers),
        }
    }

    pub(crate) fn dispatch(&self, event: &KeyboardEvent) -> KeyEventResult {
        let handlers = self.handlers.borrow().clone();
        let mut handled = false;
        let mut skip = false;
        for handler in handlers.iter() {
            match handler(event) {
                KeyEventResult::Handled => handled = true,
                KeyEventResult::SkipRemainingHandlers => skip = true,
                KeyEventResult::Ignored => {}
            }
        }
        if handled {
            KeyEventResult::Handled
        } else if skip {
            KeyEventResult::SkipRemainingHandlers
        } else {
            KeyEventResult::Ignored
        }
    }
}

/// Removes this individual early or late focus-key handler on drop.
#[must_use = "keep the subscription to receive focus key events"]
pub struct FocusKeySubscription {
    callback: Handler,
    handlers: Weak<RefCell<Snapshot>>,
}

impl Drop for FocusKeySubscription {
    fn drop(&mut self) {
        if let Some(registry) = self.handlers.upgrade() {
            let mut registry = registry.borrow_mut();
            Rc::make_mut(&mut registry).retain(|handler| !Rc::ptr_eq(handler, &self.callback));
        }
    }
}
