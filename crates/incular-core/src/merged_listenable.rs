//! Owned subscriptions that listen to several dependency sources.

use std::rc::Rc;

use crate::reactivity::{DependencySource, Subscription};

/// Flutter's `Listenable.merge` for Incular dependency sources. A merge
/// does not itself register any observers. A listener is registered with
/// each source, in source order, until its subscription is dropped.
#[derive(Clone)]
pub struct MergedListenable {
    sources: Rc<[DependencySource]>,
}

impl MergedListenable {
    /// Collects the sources without attaching listeners. Absent entries
    /// are ignored, while repeated sources are retained.
    #[must_use]
    pub fn new(sources: impl IntoIterator<Item = Option<DependencySource>>) -> Self {
        Self {
            sources: sources.into_iter().flatten().collect(),
        }
    }

    /// Equivalent to passing only present sources to [`Self::new`].
    #[must_use]
    pub fn from_sources(sources: impl IntoIterator<Item = DependencySource>) -> Self {
        Self::new(sources.into_iter().map(Some))
    }

    /// Tracks every constituent source in the active dependency scope.
    pub fn track(&self) {
        for source in self.sources.iter() {
            source.track();
        }
    }

    /// Calls the listener once for each constituent notification.
    pub fn add_listener(&self, listener: impl Fn() + 'static) -> MergedSubscription {
        let listener = Rc::new(listener);
        let subscriptions = self
            .sources
            .iter()
            .map(|source| {
                let listener = listener.clone();
                source.subscribe((), move || listener())
            })
            .collect();
        MergedSubscription {
            _subscriptions: subscriptions,
        }
    }
}

/// Owns one listener registration in each source of a merge.
#[must_use = "keep the subscription to continue receiving merged notifications"]
pub struct MergedSubscription {
    _subscriptions: Vec<Subscription>,
}
