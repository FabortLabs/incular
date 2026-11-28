//! Debug drawing that belongs to a logical view, not to its widget tree.

#[cfg(debug_assertions)]
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

#[cfg(debug_assertions)]
use incular_core::Size;
#[cfg(debug_assertions)]
use incular_rendering::DisplayList;

/// A registered view painter. Dropping the handle removes the painter
/// without changing the view's repaint schedule.
pub struct DebugViewPaintSubscription {
    #[cfg(debug_assertions)]
    owner: Weak<RefCell<Registry>>,
    #[cfg(debug_assertions)]
    id: u64,
}

impl Drop for DebugViewPaintSubscription {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        if let Some(owner) = self.owner.upgrade() {
            // Destroy captured application state outside the registry borrow.
            let removed = {
                let mut state = owner.borrow_mut();
                state
                    .painters
                    .iter()
                    .position(|&(id, _)| id == self.id)
                    .map(|index| state.painters.remove(index))
            };
            drop(removed);
        }
    }
}

#[cfg(debug_assertions)]
type Painter = Rc<dyn Fn(&mut DisplayList, Size)>;

#[cfg(debug_assertions)]
#[derive(Default)]
struct Registry {
    next_id: u64,
    painters: Vec<(u64, Painter)>,
}

#[cfg(debug_assertions)]
#[derive(Default)]
pub(crate) struct DebugViewPaint(Option<Rc<RefCell<Registry>>>);

#[cfg(debug_assertions)]
impl DebugViewPaint {
    pub(crate) fn add(
        &mut self,
        painter: impl Fn(&mut DisplayList, Size) + 'static,
    ) -> DebugViewPaintSubscription {
        let owner = self.0.get_or_insert_with(Rc::default);
        let mut state = owner.borrow_mut();
        let id = state.next_id;
        state.next_id = id
            .checked_add(1)
            .expect("debug view paint identity space exhausted");
        state.painters.push((id, Rc::new(painter)));
        DebugViewPaintSubscription {
            owner: Rc::downgrade(owner),
            id,
        }
    }

    pub(crate) fn paint(&self, output: &mut DisplayList, size: Size) {
        let Some(owner) = &self.0 else { return };
        let ids: Vec<_> = owner.borrow().painters.iter().map(|(id, _)| *id).collect();
        for id in ids {
            let painter = owner
                .borrow()
                .painters
                .iter()
                .find(|(current, _)| *current == id)
                .map(|(_, painter)| Rc::clone(painter));
            if let Some(painter) = painter {
                painter(output, size);
            }
        }
    }
}
