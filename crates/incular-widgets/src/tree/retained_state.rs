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

type BuilderFactory = dyn Fn(Rc<RetainedSlot>) -> Rc<super::context::LayoutBuilderCallback>;

pub(crate) struct RetainedSlot {
    type_id: TypeId,
    value: RefCell<Option<Rc<dyn Any>>>,
    #[cfg(debug_assertions)]
    allocation: RefCell<Option<Rc<incular_core::memory::allocations::ObjectLifetime>>>,
    revision: Rc<Cell<u64>>,
    rebind: fn(&Rc<dyn Any>, Rc<Cell<u64>>),
    factory: Option<Rc<BuilderFactory>>,
}

impl RetainedSlot {
    pub(crate) fn new<S: RetainedState>(revision: Rc<Cell<u64>>) -> Self {
        Self {
            type_id: TypeId::of::<S>(),
            value: RefCell::new(None),
            #[cfg(debug_assertions)]
            allocation: RefCell::new(None),
            revision,
            factory: None,
            rebind: |value, revision| {
                if let Some(state) = value.downcast_ref::<S>() {
                    state.rebind_revision(revision);
                }
            },
        }
    }

    pub(crate) fn with_mount_factory(mut self, factory: Rc<BuilderFactory>) -> Self {
        self.factory = Some(factory);
        self
    }

    /// Concrete builder state is allocated per mounted element, even when
    /// multiple elements use clones of the same immutable descriptor.
    pub(crate) fn mount_instance(
        &self,
    ) -> Option<(Rc<Self>, Rc<super::context::LayoutBuilderCallback>)> {
        let factory = self.factory.as_ref()?;
        let slot = Rc::new(Self {
            type_id: self.type_id,
            value: RefCell::new(None),
            #[cfg(debug_assertions)]
            allocation: RefCell::new(None),
            revision: Rc::new(Cell::new(0)),
            rebind: self.rebind,
            factory: Some(factory.clone()),
        });
        let builder = factory(slot.clone());
        Some((slot, builder))
    }

    pub(crate) fn revision(&self) -> Rc<Cell<u64>> {
        self.revision.clone()
    }

    pub(crate) fn same_mount_template(&self, other: &Self) -> bool {
        matches!((&self.factory, &other.factory), (Some(a), Some(b)) if Rc::ptr_eq(a, b))
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
        #[cfg(debug_assertions)]
        {
            let allocation =
                incular_core::memory::allocations::ObjectLifetime::new("widgets", "State");
            *self.allocation.borrow_mut() = Some(Rc::new(allocation));
        }
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
        #[cfg(debug_assertions)]
        self.allocation
            .borrow_mut()
            .clone_from(&previous.allocation.borrow());
    }
}

#[cfg(all(test, debug_assertions))]
mod allocation_tests {
    use super::*;
    use crate::{Padding, SizedBox, Widget, internal::WidgetTree};
    use incular_config::{Constraints, EdgeInsets};
    use incular_core::{
        Size,
        memory::allocations::{MemoryAllocations, ObjectEventKind},
    };

    struct Fixture;
    impl RetainedState for Fixture {
        fn rebind_revision(&self, _: Rc<Cell<u64>>) {}
    }

    #[test]
    fn state_allocation_survives_update_and_disposes_when_replaced() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let _listener = MemoryAllocations::current().add_listener({
            let events = events.clone();
            move |event| {
                if event.library == "widgets" && event.class_name == "State" {
                    events.borrow_mut().push(event);
                }
            }
        });
        let pane = || Widget::mount_scoped_builder(|_| Fixture, |_, _| SizedBox::shrink().into());
        let descriptor = pane();
        assert!(events.borrow().is_empty());
        let scoped = |child: Widget| Padding::new(EdgeInsets::ZERO, child).into();
        let mut tree = WidgetTree::new();
        let root = tree.mount(scoped(descriptor.clone())).unwrap();
        let bounds = Constraints::tight(Size::new(100., 100.));
        tree.layout(bounds).unwrap();
        assert_eq!(events.borrow().len(), 1);
        let created = events.borrow()[0];
        assert_eq!(created.kind, ObjectEventKind::Created);

        tree.update(root, scoped(pane())).unwrap();
        tree.layout(bounds).unwrap();
        assert_eq!(*events.borrow(), [created]);

        tree.update(root, scoped(SizedBox::shrink().into()))
            .unwrap();
        tree.layout(bounds).unwrap();
        assert_eq!(events.borrow().len(), 2);
        let disposed = events.borrow()[1];
        assert_eq!(disposed.kind, ObjectEventKind::Disposed);
        assert_eq!(disposed.id, created.id);
        drop(descriptor);
        assert_eq!(events.borrow().len(), 2);
    }
}
