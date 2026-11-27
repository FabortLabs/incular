//! Observer subscriptions to retained scene publication.

use std::{
    cell::Cell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::LayerId;

struct Callback {
    active: Cell<bool>,
    notify: Box<dyn Fn(LayerId)>,
}

/// Keeps a retained layer's composition observer active.
///
/// Cancel explicitly or drop this subscription to unregister the observer.
/// Destroying the layer also cancels its subscriptions.
pub struct CompositionCallbackSubscription(Rc<Callback>);

impl CompositionCallbackSubscription {
    /// Cancels the observer. Repeated cancellation has no effect.
    pub fn cancel(&self) {
        self.0.active.set(false);
    }

    /// Whether the observer is still registered.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.0.active.get()
    }
}

impl Drop for CompositionCallbackSubscription {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[derive(Default)]
pub(crate) struct CompositionObservers(HashMap<LayerId, Vec<Weak<Callback>>>);

impl CompositionObservers {
    pub(crate) fn add(
        &mut self,
        layer: LayerId,
        callback: impl Fn(LayerId) + 'static,
    ) -> CompositionCallbackSubscription {
        let entry = Rc::new(Callback {
            active: Cell::new(true),
            notify: Box::new(callback),
        });
        self.0.entry(layer).or_default().push(Rc::downgrade(&entry));
        CompositionCallbackSubscription(entry)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn has_on(&self, layer: LayerId) -> bool {
        self.0.get(&layer).is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry.upgrade().is_some_and(|entry| entry.active.get()))
        })
    }

    pub(crate) fn observed_layers(&self) -> impl Iterator<Item = LayerId> + '_ {
        self.0.keys().copied().filter(|layer| self.has_on(*layer))
    }

    pub(crate) fn prune(&mut self) -> bool {
        if self.0.is_empty() {
            return false;
        }
        self.0.retain(|_, entries| {
            entries.retain(|entry| entry.upgrade().is_some_and(|entry| entry.active.get()));
            !entries.is_empty()
        });
        !self.0.is_empty()
    }

    pub(crate) fn clear_layer(&mut self, layer: LayerId) {
        if let Some(entries) = self.0.remove(&layer) {
            for entry in entries.iter().filter_map(Weak::upgrade) {
                entry.active.set(false);
            }
        }
    }

    pub(crate) fn notify(&self, layer: LayerId) {
        let Some(entries) = self.0.get(&layer) else {
            return;
        };
        let snapshot: Vec<_> = entries
            .iter()
            .filter_map(Weak::upgrade)
            .filter(|entry| entry.active.get())
            .collect();
        for entry in snapshot {
            (entry.notify)(layer);
        }
    }
}

impl Drop for CompositionObservers {
    fn drop(&mut self) {
        for callback in self.0.values().flatten().filter_map(Weak::upgrade) {
            callback.active.set(false);
        }
    }
}
