//! Flutter's `CharacterActivator`, `ShortcutRegistry` and
//! `ShortcutRegistrar`: application-wide shortcuts that widgets anywhere in
//! the tree add and remove while they are mounted, such as menu accelerators.

use crate::{BuildContext, Widget, focus_keyboard::passive_shortcut_boundary};
use incular_core::{HardwareKeyboardState, KeyboardEvent, KeyboardKey, Modifiers};
pub use incular_gestures::CharacterActivator;
use incular_gestures::{LogicalKeySet, SingleActivator};
use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::{Rc, Weak},
};

/// An activator a [`ShortcutRegistry`] entry responds to.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RegistryActivator {
    Single(SingleActivator),
    Character(CharacterActivator),
    LogicalSet(LogicalKeySet),
}

impl RegistryActivator {
    #[must_use]
    pub fn accepts(&self, event: &KeyboardEvent) -> bool {
        self.accepts_with_keyboard_state(event, event.modifiers, None)
    }

    fn accepts_with_keyboard_state(
        &self,
        event: &KeyboardEvent,
        lock_modes: Modifiers,
        keyboard: Option<&HardwareKeyboardState>,
    ) -> bool {
        match self {
            Self::Single(activator) => activator.accepts_with_lock_modes(event, lock_modes),
            Self::Character(activator) => activator.accepts(event),
            Self::LogicalSet(activator) => keyboard.map_or_else(
                || activator.accepts(event),
                |keyboard| activator.accepts_with_keyboard_state(event, keyboard),
            ),
        }
    }
}

impl From<SingleActivator> for RegistryActivator {
    fn from(value: SingleActivator) -> Self {
        Self::Single(value)
    }
}

impl From<CharacterActivator> for RegistryActivator {
    fn from(value: CharacterActivator) -> Self {
        Self::Character(value)
    }
}

impl From<LogicalKeySet> for RegistryActivator {
    fn from(value: LogicalKeySet) -> Self {
        Self::LogicalSet(value)
    }
}

type Listener = Rc<dyn Fn(Modifiers)>;
type RegistryEntry = (u64, RegistryActivator, Rc<dyn Fn()>);

#[derive(Default)]
struct RegistryInner {
    entries: RefCell<Vec<RegistryEntry>>,
    listeners: RefCell<Vec<(u64, Listener)>>,
    next_id: Cell<u64>,
    modifiers: Cell<Modifiers>,
}

impl RegistryInner {
    fn next(&self) -> u64 {
        let id = self.next_id.get().wrapping_add(1);
        self.next_id.set(id);
        id
    }
}

/// Flutter's `ShortcutRegistry`: shortcuts added from anywhere below a
/// [`ShortcutRegistrar`], active while their [`ShortcutRegistryEntry`] lives.
///
/// The registry also tracks the modifier keys the last key event reported,
/// Incular's counterpart of reading `HardwareKeyboard.logicalKeysPressed`
/// for menu accelerators.
#[derive(Clone, Default)]
pub struct ShortcutRegistry {
    inner: Rc<RegistryInner>,
}

impl fmt::Debug for ShortcutRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShortcutRegistry")
            .field("entries", &self.inner.entries.borrow().len())
            .finish()
    }
}

impl PartialEq for ShortcutRegistry {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl ShortcutRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The registry of the nearest [`ShortcutRegistrar`].
    #[must_use]
    pub fn maybe_of(context: &BuildContext<'_>) -> Option<Self> {
        context.find::<Self>()
    }

    /// Adds a shortcut; it stays registered until the entry is dropped.
    pub fn add(
        &self,
        activator: impl Into<RegistryActivator>,
        callback: impl Fn() + 'static,
    ) -> ShortcutRegistryEntry {
        let id = self.inner.next();
        self.inner
            .entries
            .borrow_mut()
            .push((id, activator.into(), Rc::new(callback)));
        ShortcutRegistryEntry {
            registry: Rc::downgrade(&self.inner),
            id,
        }
    }

    /// The number of registered shortcuts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.entries.borrow().len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The modifier keys held as of the latest key event.
    #[must_use]
    pub fn modifiers(&self) -> Modifiers {
        self.inner.modifiers.get()
    }

    /// Calls `listener` whenever the held modifiers change, until the
    /// returned handle is dropped.
    pub fn add_modifier_listener(
        &self,
        listener: impl Fn(Modifiers) + 'static,
    ) -> ShortcutRegistryEntry {
        let id = self.inner.next();
        self.inner
            .listeners
            .borrow_mut()
            .push((id, Rc::new(listener)));
        ShortcutRegistryEntry {
            registry: Rc::downgrade(&self.inner),
            id,
        }
    }

    /// Records modifiers and runs the best matching shortcut.
    /// Keyed triggers precede character fallbacks; the newest matching
    /// registration within the same tier wins.
    pub fn handle_key(&self, event: &KeyboardEvent) -> bool {
        self.handle_key_with_keyboard_state(event, event.modifiers, None)
    }

    /// Routes an event after the host has updated its hardware snapshot.
    pub fn handle_key_with_keyboard_state(
        &self,
        event: &KeyboardEvent,
        lock_modes: Modifiers,
        keyboard: Option<&HardwareKeyboardState>,
    ) -> bool {
        let mut modifiers = event.modifiers;
        let alt_key = matches!(event.key, KeyboardKey::Named(incular_core::NamedKey::Alt));
        if alt_key {
            modifiers.set(Modifiers::ALT, event.state.is_down());
        }
        if modifiers != self.inner.modifiers.get() {
            self.inner.modifiers.set(modifiers);
            let listeners = self
                .inner
                .listeners
                .borrow()
                .iter()
                .map(|(_, listener)| listener.clone())
                .collect::<Vec<_>>();
            for listener in listeners {
                listener(modifiers);
            }
        }
        let callback = {
            let entries = self.inner.entries.borrow();
            let mut character_fallback = None;
            let keyed = entries.iter().rev().find_map(|(_, activator, callback)| {
                match activator {
                    RegistryActivator::Single(_) | RegistryActivator::LogicalSet(_) => activator
                        .accepts_with_keyboard_state(event, lock_modes, keyboard)
                        .then(|| callback.clone()),
                    RegistryActivator::Character(_) => {
                        if character_fallback.is_none()
                            && activator.accepts_with_keyboard_state(event, lock_modes, keyboard)
                        {
                            character_fallback = Some(callback.clone());
                        }
                        None
                    }
                }
            });
            keyed.or(character_fallback)
        };
        callback.is_some_and(|callback| {
            callback();
            true
        })
    }
}

/// Keeps a [`ShortcutRegistry`] shortcut or listener registered; dropping it
/// removes the registration (Flutter's `ShortcutRegistryEntry.dispose`).
pub struct ShortcutRegistryEntry {
    registry: Weak<RegistryInner>,
    id: u64,
}

impl fmt::Debug for ShortcutRegistryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ShortcutRegistryEntry")
            .field(&self.id)
            .finish()
    }
}

impl Drop for ShortcutRegistryEntry {
    fn drop(&mut self) {
        if let Some(inner) = self.registry.upgrade() {
            inner
                .entries
                .borrow_mut()
                .retain(|(id, _, _)| *id != self.id);
            inner
                .listeners
                .borrow_mut()
                .retain(|(id, _)| *id != self.id);
        }
    }
}

/// Flutter's `ShortcutRegistrar`: owns a [`ShortcutRegistry`] for its
/// subtree and runs its shortcuts for key events that reach it.
#[derive(Clone)]
pub struct ShortcutRegistrar {
    registry: Option<ShortcutRegistry>,
    child: Widget,
}

impl ShortcutRegistrar {
    /// A registrar whose registry lives as long as the registrar's element,
    /// across rebuilds.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            registry: None,
            child: child.into(),
        }
    }

    /// A registrar over an application-owned registry.
    #[must_use]
    pub fn with_registry(registry: ShortcutRegistry, child: impl Into<Widget>) -> Self {
        Self {
            registry: Some(registry),
            child: child.into(),
        }
    }
}

struct RegistrarState(ShortcutRegistry);

impl crate::tree::RetainedState for RegistrarState {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

fn registrar(registry: ShortcutRegistry, child: Widget) -> Widget {
    let handler = registry.clone();
    let callbacks = incular_gestures::GestureCallbacks {
        on_shortcut_with_keyboard_state: Some(Rc::new(move |event, lock_modes, keyboard| {
            handler.handle_key_with_keyboard_state(&event, lock_modes, keyboard)
        })),
        ..incular_gestures::GestureCallbacks::default()
    };
    Widget::environment_scope(
        registry,
        passive_shortcut_boundary(
            child,
            callbacks,
            Some(Rc::from("Shortcuts: <Shortcut Registrar>")),
            false,
            true,
        ),
    )
}

impl From<ShortcutRegistrar> for Widget {
    fn from(value: ShortcutRegistrar) -> Self {
        match value.registry {
            Some(registry) => registrar(registry, value.child),
            None => {
                let child = value.child;
                Widget::retained_builder(
                    |_| RegistrarState(ShortcutRegistry::new()),
                    move |state, _, _| registrar(state.0.clone(), child.clone()),
                    false,
                )
            }
        }
    }
}
