//! Native application-shell plumbing shared by the desktop facade crates.
//!
//! Native tray, menu and notification callbacks are process-global and can
//! fire after the shell that installed them has been torn down. Every backend
//! therefore emits through one process-wide sink whose registration is
//! generation-tagged: dropping a stale registration never removes a newer one.

use incular_runtime::NativeApplicationShellEvent;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Thread-safe delivery path for normalized application-shell events.
pub type ShellEventSink = Arc<dyn Fn(NativeApplicationShellEvent) + Send + Sync>;

struct SinkSlot {
    next_generation: u64,
    installed: Option<(u64, ShellEventSink)>,
}

static SINK: Mutex<SinkSlot> = Mutex::new(SinkSlot {
    next_generation: 0,
    installed: None,
});

// The slot holds no invariants a panicking holder could break, and native
// callbacks must not panic across the FFI boundary, so poisoning is ignored.
fn sink_slot() -> MutexGuard<'static, SinkSlot> {
    SINK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Keeps a shell event sink installed until dropped.
#[must_use = "the sink is uninstalled when the registration is dropped"]
pub struct ShellEventRegistration {
    generation: u64,
}

impl ShellEventRegistration {
    /// Installs `sink` as the process-wide application-shell event sink,
    /// replacing any previous installation.
    pub fn install(sink: ShellEventSink) -> Self {
        let mut slot = sink_slot();
        let generation = slot.next_generation;
        slot.next_generation += 1;
        slot.installed = Some((generation, sink));
        Self { generation }
    }
}

impl Drop for ShellEventRegistration {
    fn drop(&mut self) {
        let mut slot = sink_slot();
        if slot
            .installed
            .as_ref()
            .is_some_and(|(generation, _)| *generation == self.generation)
        {
            slot.installed = None;
        }
    }
}

/// Delivers `event` to the installed sink, if any. The sink runs without the
/// slot lock held so it may freely re-enter shell APIs.
pub fn emit_shell_event(event: NativeApplicationShellEvent) {
    let sink = sink_slot()
        .installed
        .as_ref()
        .map(|(_, sink)| Arc::clone(sink));
    if let Some(sink) = sink {
        sink(event);
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use tray::NativeTrays;

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod tray {
    use super::emit_shell_event;
    use incular_platform::{ApplicationShellError, TrayItemId, TrayItemPresentation, WindowIcon};
    use incular_runtime::NativeApplicationShellEvent;
    use incular_widgets::{
        MenuItemId, PlatformMenuEvent, PlatformMenuSnapshot, PlatformMenuSnapshotNode,
    };
    use std::{cell::RefCell, collections::HashMap, sync::Once};
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

    const TRAY_PREFIX: &str = "incular-tray-icon:";
    const MENU_PREFIX: &str = "incular-tray:";

    /// `tray_icon` status items owned by one application shell.
    #[derive(Default)]
    pub struct NativeTrays(RefCell<HashMap<TrayItemId, tray_icon::TrayIcon>>);

    impl NativeTrays {
        pub fn create(
            &self,
            id: TrayItemId,
            presentation: &TrayItemPresentation,
            menu: &PlatformMenuSnapshot,
        ) -> Result<(), ApplicationShellError> {
            install_event_handlers();
            let mut builder = tray_icon::TrayIconBuilder::new()
                .with_id(format!("{TRAY_PREFIX}{}:{}", id.index(), id.generation()))
                .with_menu(Box::new(build_menu(id, menu)?));
            if let Some(icon) = &presentation.icon {
                builder = builder.with_icon(native_icon(icon)?);
            }
            if let Some(tooltip) = &presentation.tooltip {
                builder = builder.with_tooltip(tooltip);
            }
            if let Some(title) = &presentation.title {
                builder = builder.with_title(title);
            }
            let tray = builder.build().map_err(native_failure)?;
            tray.set_visible(presentation.visible)
                .map_err(native_failure)?;
            self.0.borrow_mut().insert(id, tray);
            Ok(())
        }

        pub fn update(
            &self,
            id: TrayItemId,
            presentation: &TrayItemPresentation,
            menu: &PlatformMenuSnapshot,
        ) -> Result<(), ApplicationShellError> {
            let trays = self.0.borrow();
            let tray = trays.get(&id).ok_or(ApplicationShellError::StaleResource)?;
            tray.set_menu(Some(Box::new(build_menu(id, menu)?)));
            tray.set_icon(presentation.icon.as_ref().map(native_icon).transpose()?)
                .map_err(native_failure)?;
            tray.set_tooltip(presentation.tooltip.as_deref())
                .map_err(native_failure)?;
            tray.set_title(presentation.title.as_deref());
            tray.set_visible(presentation.visible)
                .map_err(native_failure)
        }

        pub fn remove(&self, id: TrayItemId) -> Result<(), ApplicationShellError> {
            self.0
                .borrow_mut()
                .remove(&id)
                .map(drop)
                .ok_or(ApplicationShellError::StaleResource)
        }
    }

    /// Routes `tray_icon`'s process-global callbacks to the shell event sink.
    fn install_event_handlers() {
        static INSTALLED: Once = Once::new();
        INSTALLED.call_once(|| {
            MenuEvent::set_event_handler(Some(|event: MenuEvent| {
                if let Some((id, item)) = decode_menu_id(event.id.as_ref()) {
                    emit_shell_event(NativeApplicationShellEvent::TrayMenu {
                        id,
                        event: PlatformMenuEvent::Selected(MenuItemId::new(item)),
                    });
                }
            }));
            tray_icon::TrayIconEvent::set_event_handler(Some(|event| {
                if let tray_icon::TrayIconEvent::Click {
                    id,
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                } = event
                    && let Some(id) = decode_tray_id(id.as_ref())
                {
                    emit_shell_event(NativeApplicationShellEvent::TrayActivated { id });
                }
            }));
        });
    }

    fn decode_tray_id(value: &str) -> Option<TrayItemId> {
        let (index, generation) = value.strip_prefix(TRAY_PREFIX)?.split_once(':')?;
        Some(TrayItemId::from_parts(
            index.parse().ok()?,
            generation.parse().ok()?,
        ))
    }

    fn menu_id(tray: TrayItemId, item: &str) -> String {
        format!("{MENU_PREFIX}{}:{}:{item}", tray.index(), tray.generation())
    }

    fn decode_menu_id(value: &str) -> Option<(TrayItemId, String)> {
        let mut parts = value.strip_prefix(MENU_PREFIX)?.splitn(3, ':');
        let index = parts.next()?.parse().ok()?;
        let generation = parts.next()?.parse().ok()?;
        Some((
            TrayItemId::from_parts(index, generation),
            parts.next()?.to_owned(),
        ))
    }

    fn native_icon(icon: &WindowIcon) -> Result<tray_icon::Icon, ApplicationShellError> {
        tray_icon::Icon::from_rgba(icon.rgba().to_vec(), icon.width(), icon.height())
            .map_err(native_failure)
    }

    fn build_menu(
        id: TrayItemId,
        snapshot: &PlatformMenuSnapshot,
    ) -> Result<Menu, ApplicationShellError> {
        let root = Menu::new();
        for node in &snapshot.menus {
            root.append(&build_submenu(id, node)?)
                .map_err(native_failure)?;
        }
        Ok(root)
    }

    fn build_submenu(
        id: TrayItemId,
        node: &PlatformMenuSnapshotNode,
    ) -> Result<Submenu, ApplicationShellError> {
        let submenu = Submenu::with_id(menu_id(id, node.id.as_str()), &node.label, node.enabled);
        for child in &node.children {
            let appended = if child.separator {
                submenu.append(&PredefinedMenuItem::separator())
            } else if child.children.is_empty() {
                submenu.append(&MenuItem::with_id(
                    menu_id(id, child.id.as_str()),
                    &child.label,
                    child.enabled,
                    None,
                ))
            } else {
                submenu.append(&build_submenu(id, child)?)
            };
            appended.map_err(native_failure)?;
        }
        Ok(submenu)
    }

    fn native_failure(error: impl std::fmt::Display) -> ApplicationShellError {
        ApplicationShellError::NativeFailure(error.to_string())
    }
}
