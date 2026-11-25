//! Tree-wide [`GlobalKey`] identity: moving a keyed element to a new parent.
//!
//! Flutter's `Element._retakeInactiveElement` vocabulary, without its
//! lifecycle: a global-keyed element removed from one parent is parked
//! instead of unmounted, a mount of the same key under another parent takes
//! the parked (or still attached) element over, and whatever is still
//! parked when the layout pass ends is unmounted. Layout ends the window
//! because builders materialize their children during layout, so a move
//! into or out of a builder spans both phases.

use super::*;

#[derive(Default)]
pub(super) struct GlobalKeyRegistry {
    /// The element that currently owns each global key.
    elements: FastHashMap<GlobalKey, ElementId>,
    /// Detached global-keyed subtrees waiting to be taken over.
    parked: Vec<ElementId>,
    /// Keys an element was updated, moved or mounted for in the running
    /// reconciliation; a second claim is a duplicate.
    claimed: FastHashSet<GlobalKey>,
    /// Nesting of reconciliation entry points; claims reset at zero.
    depth: u32,
}

fn global_key(widget: &Widget) -> Option<GlobalKey> {
    match widget.key() {
        Some(Key::Global(key)) => Some(*key),
        _ => None,
    }
}

impl WidgetTree {
    /// Runs one reconciliation entry point. Claims live until the outermost
    /// one returns.
    pub(super) fn with_global_key_transaction<R>(&mut self, run: impl FnOnce(&mut Self) -> R) -> R {
        self.global_keys.depth += 1;
        let result = run(self);
        self.global_keys.depth -= 1;
        if self.global_keys.depth == 0 {
            self.global_keys.claimed.clear();
        }
        result
    }

    /// Claims `widget`'s global key for the running reconciliation.
    pub(super) fn claim_global_key(&mut self, widget: &Widget) -> Result<(), TreeError> {
        let Some(key) = global_key(widget) else {
            return Ok(());
        };
        if self.global_keys.depth > 0 && !self.global_keys.claimed.insert(key) {
            return Err(TreeError::DuplicateGlobalKey {
                key: Key::Global(key),
            });
        }
        Ok(())
    }

    pub(super) fn register_global_key(&mut self, id: ElementId, widget: &Widget) {
        if let Some(key) = global_key(widget) {
            self.global_keys.elements.insert(key, id);
        }
    }

    pub(super) fn unregister_global_key(&mut self, id: ElementId, widget: &Widget) {
        if let Some(key) = global_key(widget)
            && self.global_keys.elements.get(&key) == Some(&id)
        {
            self.global_keys.elements.remove(&key);
        }
    }

    fn is_inclusive_ancestor(&self, ancestor: ElementId, mut id: ElementId) -> bool {
        loop {
            if id == ancestor {
                return true;
            }
            match self.elements.get(id.0).and_then(|element| element.parent) {
                Some(parent) => id = parent,
                None => return false,
            }
        }
    }

    /// Detaches `id` from its parent's element and render children.
    fn detach_from_parent(&mut self, id: ElementId) {
        let Some(parent) = self.elements.get(id.0).and_then(|element| element.parent) else {
            self.global_keys.parked.retain(|parked| *parked != id);
            return;
        };
        let parent_render = {
            let element = self.element_live_mut(parent, "global key parent must remain live");
            if let Some(position) = element.children.iter().position(|child| *child == id) {
                let count = element.children.len();
                element.children.remove(position);
                // Dynamic parents keep per-child records parallel to
                // `children`.
                if let Some(scrolling) = element.scrolling_state.as_mut() {
                    if scrolling.sliver_child_ids.len() == count {
                        scrolling.sliver_child_ids.remove(position);
                    }
                    if scrolling.sliver_child_semantic_indices.len() == count {
                        scrolling.sliver_child_semantic_indices.remove(position);
                    }
                    if scrolling.advanced_child_keys.len() == count {
                        scrolling.advanced_child_keys.remove(position);
                    }
                }
            }
            element.reset_layout_builder_state();
            element.render
        };
        self.sync_render_children(parent);
        self.mark_render_dirty(parent_render, DirtyFlags::LAYOUT | DirtyFlags::PAINT, true);
        self.element_live_mut(id, "detached element must remain live")
            .parent = None;
    }

    fn clear_render_parent(&mut self, id: ElementId) {
        let render = self
            .element_live(id, "global key element must remain live")
            .render;
        let node = self.render_live_mut(render, "global key render must remain live");
        node.parent = None;
        node.parked = false;
    }

    /// Takes over the element that owns `widget`'s global key for a mount
    /// under `parent`, wherever it is: parked, or still attached elsewhere
    /// (its old parent then reconciles without it). Returns `None` when the
    /// key is new or names an element of another type.
    pub(super) fn retake_global_element(
        &mut self,
        parent: ElementId,
        widget: &Widget,
    ) -> Result<Option<ElementId>, TreeError> {
        let Some(key) = global_key(widget) else {
            return Ok(None);
        };
        let Some(&id) = self.global_keys.elements.get(&key) else {
            return Ok(None);
        };
        if self.global_keys.claimed.contains(&key) {
            return Err(TreeError::DuplicateGlobalKey {
                key: Key::Global(key),
            });
        }
        let compatible = self
            .elements
            .get(id.0)
            .is_some_and(|element| element.widget.type_() == widget.type_());
        // A widget cannot move below itself.
        if !compatible || self.is_inclusive_ancestor(id, parent) {
            return Ok(None);
        }
        self.detach_from_parent(id);
        self.clear_render_parent(id);
        self.element_live_mut(id, "retaken element must remain live")
            .parent = Some(parent);
        // Inherited values resolve from the new position.
        self.rebind_inherited_subtree(id);
        let render = self
            .element_live(id, "retaken element must remain live")
            .render;
        self.mark_render_dirty(render, DirtyFlags::LAYOUT | DirtyFlags::PAINT, true);
        self.update_existing(id, widget)?;
        self.diagnostics.elements_reused += 1;
        Ok(Some(id))
    }

    /// Removes `id`, a child `owner` no longer wants. An element another
    /// parent took over is left alone, and global-keyed elements inside the
    /// removed subtree are parked until the layout pass ends.
    pub(super) fn retire_element(&mut self, id: ElementId, owner: ElementId) {
        if self.elements.get(id.0).and_then(|element| element.parent) != Some(owner) {
            return;
        }
        if !self.global_keys.elements.is_empty() {
            let candidates: Vec<ElementId> = self
                .global_keys
                .elements
                .values()
                .copied()
                .filter(|candidate| self.is_inclusive_ancestor(id, *candidate))
                .collect();
            // Park only outermost ones; nested keyed elements travel inside.
            let outermost: Vec<ElementId> = candidates
                .iter()
                .copied()
                .filter(|candidate| {
                    !candidates.iter().any(|other| {
                        other != candidate && self.is_inclusive_ancestor(*other, *candidate)
                    })
                })
                .collect();
            for parked in outermost {
                self.detach_from_parent(parked);
                self.clear_render_parent(parked);
                self.global_keys.parked.push(parked);
                if parked == id {
                    return;
                }
            }
        }
        self.unmount_element(id);
    }

    /// Unmounts every parked element nobody took over.
    pub(super) fn finalize_parked_global_elements(&mut self) {
        for id in std::mem::take(&mut self.global_keys.parked) {
            if self
                .elements
                .get(id.0)
                .is_some_and(|element| element.parent.is_none())
                && self.root != Some(id)
            {
                self.unmount_element(id);
            }
        }
    }

    /// Whether `widget`'s global key was already seen in this description.
    pub(super) fn note_static_global_key(
        widget: &Widget,
        seen: &mut HashSet<GlobalKey>,
    ) -> Result<(), TreeError> {
        if let Some(key) = global_key(widget)
            && !seen.insert(key)
        {
            return Err(TreeError::DuplicateGlobalKey {
                key: Key::Global(key),
            });
        }
        Ok(())
    }
}
