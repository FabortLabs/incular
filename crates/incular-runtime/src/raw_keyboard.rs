//! Passive window-level subscriptions for the legacy raw-key route.

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use incular_core::KeyboardEvent;
use incular_widgets::{RawKeyDelivery, RawKeyEvent, RawKeyboardListenerError};

type Listener = Rc<dyn Fn(&RawKeyEvent)>;
type List = RefCell<Rc<Vec<Listener>>>;

#[derive(Default)]
pub(crate) struct RawKeyboardListeners(Option<Rc<List>>);

impl RawKeyboardListeners {
    pub(crate) fn add(
        &mut self,
        listener: impl Fn(&RawKeyEvent) + 'static,
    ) -> RawKeyboardSubscription {
        let listeners = self.0.get_or_insert_with(Rc::default);
        let listener: Listener = Rc::new(listener);
        Rc::make_mut(&mut listeners.borrow_mut()).push(listener.clone());
        RawKeyboardSubscription {
            listener,
            registry: Rc::downgrade(listeners),
        }
    }

    pub(crate) fn dispatch(
        &self,
        event: &KeyboardEvent,
        delivery: RawKeyDelivery<'_>,
        mut report: impl FnMut(RawKeyboardListenerError),
    ) {
        let Some(registry) = &self.0 else {
            return;
        };
        let listeners = registry.borrow().clone();
        if listeners.is_empty() {
            return;
        }
        let portable;
        let raw = match delivery {
            RawKeyDelivery::Absent => return,
            RawKeyDelivery::Supplied(raw) => raw,
            RawKeyDelivery::Mirror => {
                portable = RawKeyEvent::from(event.clone());
                &portable
            }
        };
        for listener in listeners.iter() {
            if let Some(error) = RawKeyboardListenerError::catch(raw, || listener(raw)) {
                report(error);
            }
        }
    }
}

/// Removes one raw-key registration on drop. Adding the same callback
/// again creates a separate registration that survives the first drop.
#[must_use = "keep the subscription to receive raw keyboard events"]
pub struct RawKeyboardSubscription {
    listener: Listener,
    registry: Weak<List>,
}

impl Drop for RawKeyboardSubscription {
    fn drop(&mut self) {
        if let Some(registry) = self.registry.upgrade() {
            Rc::make_mut(&mut registry.borrow_mut())
                .retain(|listener| !Rc::ptr_eq(listener, &self.listener));
        }
    }
}
