//! Flutter's inline stateful builder, with a mount-scoped invalidation handle.

use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

use incular_core::reactivity::DependencySource;

use crate::{BuildContext, Widget, tree::RetainedState};

pub(crate) struct StatefulBuilderState {
    changes: DependencySource,
    mounted: Cell<bool>,
}

impl RetainedState for StatefulBuilderState {
    const DISPOSE_ON_UNMOUNT: bool = true;

    fn dispose(&self) {
        self.mounted.set(false);
    }

    // Invalidation tracks the stable dependency source, not the descriptor's
    // layout revision. A retained replacement keeps existing setters live.
    fn rebind_revision(&self, _: Rc<Cell<u64>>) {}
}

/// An update handle for one mounted [`StatefulBuilder`] instance.
///
/// Cloning the handle does not keep the mounted builder alive.
#[derive(Clone)]
pub struct StateSetter(Weak<StatefulBuilderState>);

impl StateSetter {
    /// Runs an update synchronously and requests a rebuild of this builder.
    ///
    /// Returns false without running the update when the builder is no
    /// longer mounted. Store mutable values outside the build callback so
    /// they survive the rebuild.
    pub fn set_state(&self, update: impl FnOnce()) -> bool {
        let Some(state) = self.0.upgrade().filter(|state| state.mounted.get()) else {
            return false;
        };
        update();
        state.changes.notify();
        true
    }
}

type Build = dyn for<'a> Fn(&BuildContext<'a>, StateSetter) -> Widget;

/// Builds a subtree with a local update handle, without introducing a
/// separate widget implementation. Only the mounted builder that issued the
/// handle is invalidated when [`StateSetter::set_state`] runs.
#[derive(Clone)]
pub struct StatefulBuilder(Rc<Build>);

impl StatefulBuilder {
    /// Creates an inline builder that rebuilds in response to its setter,
    /// parent updates, and inherited dependencies read while building.
    #[must_use]
    pub fn new(
        builder: impl for<'a> Fn(&BuildContext<'a>, StateSetter) -> Widget + 'static,
    ) -> Self {
        Self(Rc::new(builder))
    }
}

impl From<StatefulBuilder> for Widget {
    fn from(value: StatefulBuilder) -> Self {
        Widget::mount_scoped_builder(
            |_| StatefulBuilderState {
                changes: DependencySource::default(),
                mounted: Cell::new(true),
            },
            move |state, context| {
                state.changes.track();
                (value.0)(context, StateSetter(Rc::downgrade(state)))
            },
        )
    }
}
