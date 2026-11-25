//! Canonical, renderer-neutral identity for custom accessibility actions.

use crate::SemanticActionKind;
use std::{cell::RefCell, collections::HashMap, sync::Arc};

/// The canonical identifier of a custom semantics action on the current UI
/// thread. It identifies the action description, not a node or handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CustomSemanticsActionId(pub u32);

/// Flutter's `CustomSemanticsAction`: a localized label for an extra action,
/// or a localized hint overriding the description of a standard action.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomSemanticsAction(Action);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Action {
    Label(Arc<str>),
    Override {
        hint: Arc<str>,
        action: SemanticActionKind,
    },
}

#[derive(Default)]
struct Registry {
    ids: HashMap<CustomSemanticsAction, CustomSemanticsActionId>,
    actions: Vec<CustomSemanticsAction>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

impl CustomSemanticsAction {
    /// Creates a custom action with a nonempty user-visible label.
    #[must_use]
    pub fn new(label: impl Into<Arc<str>>) -> Self {
        let label = label.into();
        debug_assert!(
            !label.is_empty(),
            "custom semantics action label must not be empty"
        );
        Self(Action::Label(label))
    }

    /// Overrides a standard action's description with a nonempty hint.
    #[must_use]
    pub fn overriding_action(action: SemanticActionKind, hint: impl Into<Arc<str>>) -> Self {
        let hint = hint.into();
        debug_assert!(
            !hint.is_empty(),
            "custom semantics action hint must not be empty"
        );
        Self(Action::Override { hint, action })
    }

    #[must_use]
    pub fn label(&self) -> Option<&str> {
        match &self.0 {
            Action::Label(label) => Some(label),
            Action::Override { .. } => None,
        }
    }

    #[must_use]
    pub fn hint(&self) -> Option<&str> {
        match &self.0 {
            Action::Override { hint, .. } => Some(hint),
            Action::Label(_) => None,
        }
    }

    #[must_use]
    pub fn action(&self) -> Option<SemanticActionKind> {
        match &self.0 {
            Action::Override { action, .. } => Some(*action),
            Action::Label(_) => None,
        }
    }

    /// Registers this action lazily. Equal descriptions reuse the same ID,
    /// while separately named or overridden actions get distinct IDs.
    #[must_use]
    pub fn get_identifier(&self) -> CustomSemanticsActionId {
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            if let Some(id) = registry.ids.get(self) {
                return *id;
            }
            let id = CustomSemanticsActionId(
                u32::try_from(registry.actions.len())
                    .expect("custom semantics action IDs exhausted"),
            );
            registry.actions.push(self.clone());
            registry.ids.insert(self.clone(), id);
            id
        })
    }

    /// Looks up a registered action on this UI thread.
    #[must_use]
    pub fn get_action(id: CustomSemanticsActionId) -> Option<Self> {
        REGISTRY.with(|registry| registry.borrow().actions.get(id.0 as usize).cloned())
    }
}
