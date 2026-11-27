//! Local widget-preview restart without restarting the process.

use std::{cell::Cell, rc::Rc};

use incular_core::{
    Size,
    reactivity::{DependencySource, Subscription},
};

use crate::{Builder, SizedBox, Widget};

#[derive(Default)]
struct RestartState {
    pending: Cell<bool>,
    callback_armed: Cell<bool>,
    last_size: Cell<Size>,
    changes: DependencySource,
}

/// Owns the soft-restart lifecycle of one mounted preview.
#[derive(Clone, Default)]
pub struct PreviewSoftRestart(Rc<RestartState>);

impl PreviewSoftRestart {
    #[must_use]
    pub fn is_restarting(&self) -> bool {
        self.0.changes.track();
        self.0.pending.get()
    }

    #[must_use]
    pub fn last_child_size(&self) -> Size {
        self.0.last_size.get()
    }

    /// Returns false when an earlier restart is still pending.
    pub fn request(&self) -> bool {
        if self.0.pending.replace(true) {
            return false;
        }
        self.0.changes.notify();
        true
    }

    pub fn observe(&self, listener: impl Fn(bool) + 'static) -> Subscription {
        let weak = Rc::downgrade(&self.0);
        self.0.changes.subscribe((), move || {
            if let Some(state) = weak.upgrade() {
                listener(state.pending.get());
            }
        })
    }

    /// Removes the old subtree for one painted frame, using its last layout
    /// size to preserve the preview's footprint. The next frame mounts a new
    /// subtree from the factory; no other preview or worker restarts.
    #[must_use]
    pub fn wrap(&self, factory: impl Fn() -> Widget + 'static) -> Widget {
        let restart = self.clone();
        Builder::new(move |_| {
            if !restart.is_restarting() {
                let state = restart.0.clone();
                return Widget::size_observer(move |size| state.last_size.set(size), factory());
            }
            if !restart.0.callback_armed.replace(true) {
                let weak = Rc::downgrade(&restart.0);
                incular_animation::scheduler::add_post_frame_callback(move |_| {
                    if let Some(state) = weak.upgrade() {
                        state.callback_armed.set(false);
                        if state.pending.replace(false) {
                            state.changes.notify();
                        }
                    }
                });
            }
            Widget::from(SizedBox::from_size(restart.last_child_size()))
                .with_key("preview.restart.placeholder")
        })
        .into()
    }
}
