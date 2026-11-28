//! A replaceable shortcut map, shared by mounted shortcut scopes.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_core::{KeyboardEvent, reactivity::DependencySource};

use super::{Command, CommandId, ErasedShortcutScope, Shortcuts};

type KeyObserver = dyn Fn(&KeyboardEvent);

struct Inner<C: CommandId> {
    shortcuts: RefCell<Rc<Shortcuts<C>>>,
    modal: Cell<bool>,
    observer: RefCell<Option<Rc<KeyObserver>>>,
    changes: DependencySource,
}

/// Controls a mounted scope's bindings without replacing its focus node.
#[derive(Clone)]
pub struct ShortcutManager<C: CommandId = Command>(Rc<Inner<C>>);

impl ShortcutManager<Command> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<C: CommandId> Default for ShortcutManager<C> {
    fn default() -> Self {
        Self::with_shortcuts(Shortcuts::<C>::typed())
    }
}

impl<C: CommandId> ShortcutManager<C> {
    #[must_use]
    pub fn with_shortcuts(shortcuts: impl Into<Rc<Shortcuts<C>>>) -> Self {
        Self(Rc::new(Inner {
            shortcuts: RefCell::new(shortcuts.into()),
            modal: Cell::new(false),
            observer: RefCell::new(None),
            changes: DependencySource::default(),
        }))
    }

    #[must_use]
    pub fn shortcuts(&self) -> Rc<Shortcuts<C>> {
        self.0.changes.track();
        self.0.shortcuts.borrow().clone()
    }

    pub fn set_shortcuts(&self, shortcuts: impl Into<Rc<Shortcuts<C>>>) {
        let shortcuts = shortcuts.into();
        if Rc::ptr_eq(&self.0.shortcuts.borrow(), &shortcuts) {
            return;
        }
        self.0.shortcuts.replace(shortcuts);
        self.0.changes.notify();
    }

    #[must_use]
    pub fn is_modal(&self) -> bool {
        self.0.changes.track();
        self.0.modal.get()
    }

    /// An ignored key stops at this scope but remains available to the host.
    pub fn set_modal(&self, modal: bool) {
        if self.0.modal.replace(modal) != modal {
            self.0.changes.notify();
        }
    }

    /// Observes the key that actually reached this mounted manager.
    pub fn set_key_observer(&self, observer: Option<Rc<dyn Fn(&KeyboardEvent)>>) {
        self.0.observer.replace(observer);
    }

    #[doc(hidden)]
    #[must_use]
    pub fn erased_scope(&self) -> Rc<ErasedShortcutScope> {
        let manager = self.clone();
        let modal = self.clone();
        Rc::new(ErasedShortcutScope {
            finder: Rc::new(move |event, keyboard| {
                let observer = manager.0.observer.borrow().clone();
                if let Some(observer) = observer {
                    observer(event);
                }
                let bindings = manager.0.shortcuts.borrow().clone();
                bindings.find_match(event, keyboard)
            }),
            modal: Some(Rc::new(move || modal.0.modal.get())),
        })
    }
}
