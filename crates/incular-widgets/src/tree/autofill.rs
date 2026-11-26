//! Mount-ordered registration of fields in retained autofill groups.

use incular_platform::autofill::AutofillContextAction;

use super::{ElementId, FastHashMap, FastHashSet, Widget, WidgetKind, WidgetTree};

#[derive(Default)]
pub(super) struct AutofillRegistry {
    clients: FastHashMap<ElementId, Vec<ElementId>>,
    parent_scope: FastHashMap<ElementId, ElementId>,
    parked_under_group: FastHashSet<ElementId>,
}

impl AutofillRegistry {
    fn move_client(&mut self, client: ElementId, scope: Option<ElementId>) {
        let previous = self.parent_scope.get(&client).copied();
        if previous == scope {
            return;
        }
        if let Some(old_scope) = previous {
            self.parent_scope.remove(&client);
            if let Some(clients) = self.clients.get_mut(&old_scope) {
                clients.retain(|&item| item != client);
            }
        }
        if let Some(scope) = scope
            && let Some(clients) = self.clients.get_mut(&scope)
        {
            clients.push(client);
            self.parent_scope.insert(client, scope);
        }
    }
}

impl WidgetTree {
    /// The closest mounted autofill scope, including the element if it is a scope.
    #[must_use]
    pub fn autofill_scope(&self, mut id: ElementId) -> Option<ElementId> {
        loop {
            let element = self.elements.get(id.0)?;
            if element.widget.autofill_action().is_some() {
                return Some(id);
            }
            id = element.parent?;
        }
    }

    /// The enabled mounted clients in registration order. Reordering
    /// existing elements in the same group does not re-register them.
    #[must_use]
    pub fn autofill_clients(&self, scope: ElementId) -> Vec<ElementId> {
        let Some(clients) = self
            .autofill_registry
            .as_ref()
            .and_then(|registry| registry.clients.get(&scope))
        else {
            return Vec::new();
        };
        clients.iter().copied().filter(|id| {
            self.elements.get(id.0).is_some_and(|element| {
                matches!(element.widget.kind(), WidgetKind::TextField(spec) if spec.traits.autofill_hints.is_some())
            })
        }).collect()
    }

    /// Takes the completed top-level groups in disposal order.
    pub fn take_autofill_context_completions(&mut self) -> Vec<AutofillContextAction> {
        std::mem::take(&mut self.autofill_context_completions)
    }

    pub(super) fn register_autofill_element(&mut self, id: ElementId, widget: &Widget) {
        if widget.autofill_action().is_some() {
            self.autofill_registry
                .get_or_insert_with(Box::default)
                .clients
                .entry(id)
                .or_default();
        } else if self.autofill_registry.is_some()
            && matches!(widget.kind(), WidgetKind::TextField(_))
            && let Some(scope) = self.autofill_scope(id)
        {
            self.autofill_registry
                .as_mut()
                .expect("registry exists")
                .move_client(id, Some(scope));
        }
    }

    pub(super) fn unregister_autofill_element(&mut self, id: ElementId, widget: &Widget) {
        let Some(registry) = self.autofill_registry.as_mut() else {
            return;
        };
        if widget.autofill_action().is_some() {
            if let Some(clients) = registry.clients.remove(&id) {
                for client in clients {
                    registry.parent_scope.remove(&client);
                }
            }
        } else if matches!(widget.kind(), WidgetKind::TextField(_)) {
            registry.move_client(id, None);
        }
        if registry.clients.is_empty() && registry.parked_under_group.is_empty() {
            self.autofill_registry = None;
        }
    }

    /// The closest group can change when a keyed subtree moves. Stop at
    /// groups that move with it; their own clients are still registered.
    pub(super) fn reparent_autofill_subtree(&mut self, id: ElementId) {
        if self.autofill_registry.is_none() {
            return;
        }
        self.take_parked_autofill_ancestor(id);
        let new_scope = self
            .parent(id)
            .and_then(|parent| self.autofill_scope(parent));
        let mut work = vec![id];
        while let Some(id) = work.pop() {
            let Some(element) = self.elements.get(id.0) else {
                continue;
            };
            if element.widget.autofill_action().is_some() {
                continue;
            }
            if matches!(element.widget.kind(), WidgetKind::TextField(_)) {
                self.autofill_registry
                    .as_mut()
                    .expect("registry exists")
                    .move_client(id, new_scope);
            }
            work.extend(element.children.iter().rev().copied());
        }
    }

    /// Keep ancestry across a temporary detach, so an untaken inner
    /// group is not treated as topmost when it is finally disposed.
    pub(super) fn note_parked_autofill_ancestor(&mut self, id: ElementId) {
        if self.autofill_registry.is_some()
            && self
                .parent(id)
                .and_then(|parent| self.autofill_scope(parent))
                .is_some()
        {
            self.autofill_registry
                .as_mut()
                .expect("registry exists")
                .parked_under_group
                .insert(id);
        }
    }

    pub(super) fn take_parked_autofill_ancestor(&mut self, id: ElementId) -> bool {
        self.autofill_registry
            .as_mut()
            .is_some_and(|registry| registry.parked_under_group.remove(&id))
    }
}
