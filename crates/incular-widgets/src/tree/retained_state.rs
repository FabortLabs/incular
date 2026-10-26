//! Element-retained state for stateful builders: Flutter's `State`, which
//! survives a parent rebuilding its widget.
//!
//! A builder descriptor carries a [`RetainedSlot`]. When the retained tree
//! updates an element with a new descriptor whose slot holds the same state
//! type, the new slot adopts the old state and rebinds it to the new
//! descriptor's revision, so the state's animations keep rebuilding it.

use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    rc::Rc,
};

/// State kept by a retained builder across descriptor updates.
pub(crate) trait RetainedState: Any {
    /// The builder now runs under `revision`: bump it to rebuild.
    fn rebind_revision(&self, revision: Rc<Cell<u64>>);
}

pub(crate) struct RetainedSlot {
    type_id: TypeId,
    value: RefCell<Option<Rc<dyn Any>>>,
    revision: Rc<Cell<u64>>,
    rebind: fn(&Rc<dyn Any>, Rc<Cell<u64>>),
}

impl RetainedSlot {
    pub(crate) fn new<S: RetainedState>(revision: Rc<Cell<u64>>) -> Self {
        Self {
            type_id: TypeId::of::<S>(),
            value: RefCell::new(None),
            revision,
            rebind: |value, revision| {
                if let Some(state) = value.downcast_ref::<S>() {
                    state.rebind_revision(revision);
                }
            },
        }
    }

    /// The slot's state, created by `init` on first use.
    pub(crate) fn get_or_init<S: RetainedState>(
        &self,
        init: impl FnOnce(Rc<Cell<u64>>) -> S,
    ) -> Rc<S> {
        if let Some(state) = self
            .value
            .borrow()
            .clone()
            .and_then(|value| value.downcast::<S>().ok())
        {
            return state;
        }
        let state = Rc::new(init(self.revision.clone()));
        *self.value.borrow_mut() = Some(state.clone());
        state
    }

    /// Takes over `previous`'s state when it is of the same type and this
    /// slot has none yet.
    pub(crate) fn adopt(&self, previous: &Self) {
        if self.type_id != previous.type_id || self.value.borrow().is_some() {
            return;
        }
        let Some(state) = previous.value.borrow().clone() else {
            return;
        };
        (self.rebind)(&state, self.revision.clone());
        *self.value.borrow_mut() = Some(state);
    }
}
