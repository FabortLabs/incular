//! Owned subscriptions that listen to several dependency sources.

use std::{fmt, rc::Rc};

use crate::reactivity::{DependencySource, Subscription};

/// Flutter's `Listenable.merge` for Incular dependency sources. A merge
/// does not itself register any observers. A listener is registered with
/// each source, in source order, until its subscription is dropped.
#[derive(Clone)]
pub struct MergedListenable {
    sources: Rc<[Option<DependencySource>]>,
}

impl MergedListenable {
    /// Collects the sources without attaching listeners. Absent entries
    /// are retained for diagnostics and ignored during dispatch; repeated
    /// sources are retained in order.
    #[must_use]
    pub fn new(sources: impl IntoIterator<Item = Option<DependencySource>>) -> Self {
        Self {
            sources: sources.into_iter().collect(),
        }
    }

    /// Equivalent to passing only present sources to [`Self::new`].
    #[must_use]
    pub fn from_sources(sources: impl IntoIterator<Item = DependencySource>) -> Self {
        Self::new(sources.into_iter().map(Some))
    }

    /// Tracks every constituent source in the active dependency scope.
    pub fn track(&self) {
        for source in self.sources.iter().flatten() {
            source.track();
        }
    }

    /// Calls the listener once for each constituent notification.
    pub fn add_listener(&self, listener: impl Fn() + 'static) -> MergedSubscription {
        let listener = Rc::new(listener);
        let subscriptions = self
            .sources
            .iter()
            .flatten()
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

impl fmt::Display for MergedListenable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Listenable.merge([")?;
        for (index, source) in self.sources.iter().enumerate() {
            if index != 0 {
                formatter.write_str(", ")?;
            }
            formatter.write_str(if source.is_some() {
                "DependencySource"
            } else {
                "null"
            })?;
        }
        formatter.write_str("])")
    }
}

impl fmt::Debug for MergedListenable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

/// Owns one listener registration in each source of a merge.
#[must_use = "keep the subscription to continue receiving merged notifications"]
pub struct MergedSubscription {
    _subscriptions: Vec<Subscription>,
}
