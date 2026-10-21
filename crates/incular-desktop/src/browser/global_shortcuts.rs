//! Browser variant of the application-level global shortcut backend.
//!
//! A page receives keys only while it has focus, so system-wide shortcuts do
//! not exist. Focused-window `Shortcuts` keep working through ordinary input.

use incular_platform::{
    CapabilitySupport, GlobalShortcutChord, GlobalShortcutError, GlobalShortcutId,
    NativeWindowSystem,
};
use std::sync::Arc;
use winit::event::KeyEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativeGlobalShortcutEvent {
    pub(crate) native_id: u32,
}

pub(crate) struct GlobalShortcutEventSinkGuard;

pub(crate) fn install_event_sink(
    _sink: Arc<dyn Fn(NativeGlobalShortcutEvent) + Send + Sync>,
) -> GlobalShortcutEventSinkGuard {
    GlobalShortcutEventSinkGuard
}

pub(crate) struct DesktopGlobalShortcuts;

impl DesktopGlobalShortcuts {
    pub(crate) const fn new(_system: NativeWindowSystem) -> Self {
        Self
    }

    #[must_use]
    pub(crate) const fn support(&self) -> CapabilitySupport {
        CapabilitySupport::Unsupported
    }

    pub(crate) fn register(
        &mut self,
        _id: GlobalShortcutId,
        _chord: GlobalShortcutChord,
    ) -> Result<(), GlobalShortcutError> {
        Err(GlobalShortcutError::Unsupported)
    }

    pub(crate) fn unregister(&mut self, id: GlobalShortcutId) -> Result<(), GlobalShortcutError> {
        Err(GlobalShortcutError::NotRegistered(id))
    }

    #[must_use]
    pub(crate) const fn stable_id(&self, _native_id: u32) -> Option<GlobalShortcutId> {
        None
    }

    #[must_use]
    pub(crate) const fn suppress_local_key_event(
        &self,
        _event: &KeyEvent,
        _modifiers: winit::keyboard::ModifiersState,
    ) -> bool {
        false
    }
}

pub(crate) const fn support_for(_system: NativeWindowSystem) -> CapabilitySupport {
    CapabilitySupport::Unsupported
}
