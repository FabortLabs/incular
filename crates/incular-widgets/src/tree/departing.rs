//! Departing elements: a child removed by reconciliation whose motion site
//! has an exit stays mounted (non-interactive, unfocusable, without
//! semantics) until the exit settles, then leaves for real. Re-inserting its
//! key mid-exit revives it.

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use super::{ElementId, FrameDelegate, WidgetKind, WidgetTree};

/// How deep the single-child spine of a removed element is searched for its
/// motion site (wrappers such as padding above `.motion()`).
const SPINE_DEPTH: usize = 24;

#[derive(Default)]
pub(super) struct Departures {
    /// Removed elements whose exit is playing.
    pub(super) active: HashSet<ElementId>,
    /// Departures whose exit settled, drained before the next layout.
    pub(super) done: Rc<RefCell<Vec<ElementId>>>,
}

impl WidgetTree {
    /// The frame delegate of the topmost motion site on `id`'s single-child
    /// spine.
    fn exit_delegate(&self, mut id: ElementId) -> Option<FrameDelegate> {
        for _ in 0..SPINE_DEPTH {
            let element = self.elements.get(id.0)?;
            if let WidgetKind::AnimationTicker {
                frame: Some(delegate),
                ..
            } = element.widget.kind()
            {
                return Some(delegate.clone());
            }
            match element.children.as_slice() {
                [only] => id = *only,
                _ => return None,
            }
        }
        None
    }

    /// Asks `id`'s motion site to exit instead of being removed. Returns
    /// whether the element stays mounted as departing.
    pub(super) fn defer_exit(&mut self, id: ElementId) -> bool {
        if self.departures.active.contains(&id) {
            return true;
        }
        let Some(delegate) = self.exit_delegate(id) else {
            return false;
        };
        let done = self.departures.done.clone();
        let finished = Rc::new(move || {
            done.borrow_mut().push(id);
            incular_animation::schedule_frame();
        });
        if !delegate.begin_exit(finished) {
            return false;
        }
        self.departures.active.insert(id);
        true
    }

    /// A departing element was matched again: its exit reverses.
    pub(super) fn revive(&mut self, id: ElementId) {
        if !self.departures.active.remove(&id) {
            return;
        }
        if let Some(delegate) = self.exit_delegate(id) {
            delegate.cancel_exit();
        }
    }

    /// Removes departing elements whose exit settled.
    pub(super) fn finish_departures(&mut self) {
        let finished: Vec<ElementId> = std::mem::take(&mut *self.departures.done.borrow_mut());
        for id in finished {
            if !self.departures.active.remove(&id) {
                continue;
            }
            let Some(parent) = self.elements.get(id.0).and_then(|element| element.parent) else {
                continue;
            };
            let Some(owner) = self.elements.get_mut(parent.0) else {
                continue;
            };
            owner.children.retain(|child| *child != id);
            self.retire_element(id, parent);
            self.sync_render_children(parent);
        }
    }

    /// Merges kept departing elements back into a reconciled child list at
    /// their previous positions.
    pub(super) fn place_departing(
        previous: &[ElementId],
        kept: &HashSet<ElementId>,
        next: &mut Vec<ElementId>,
    ) {
        for (index, id) in previous.iter().enumerate() {
            if !kept.contains(id) {
                continue;
            }
            let position = previous[..index]
                .iter()
                .rev()
                .find_map(|before| next.iter().position(|child| child == before))
                .map_or(0, |position| position + 1);
            next.insert(position, *id);
        }
    }
}
