//! Win32 application-shell integration.

use incular_desktop::{NativeTrays, ShellEventRegistration, ShellEventSink, emit_shell_event};
use incular_platform::{
    ApplicationShellError, ApplicationShellFeature, CapabilitySupport, NativeWindowSystem,
    PlatformCapabilities, WindowIcon,
};
use incular_runtime::{
    NativeApplicationShellEvent, NativeApplicationShellOperation, NativeApplicationShellRequest,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Default)]
pub(crate) struct WindowsApplicationShell {
    inner: Rc<WindowsApplicationShellInner>,
}

// Field order is drop order: native trays go before the event sink.
#[derive(Default)]
struct WindowsApplicationShellInner {
    trays: NativeTrays,
    notification_identity: RefCell<Option<String>>,
    events: RefCell<Option<ShellEventRegistration>>,
}

impl WindowsApplicationShell {
    pub(crate) fn refine_capabilities(
        &self,
        system: NativeWindowSystem,
        capabilities: &mut PlatformCapabilities,
    ) {
        let support = CapabilitySupport::from_supported(system == NativeWindowSystem::Win32);
        let unsupported = CapabilitySupport::Unsupported;
        let services = &mut capabilities.application_services;
        services.tray_or_status_item = support;
        services.notifications = support;
        services.notification_actions = support;
        services.notification_update = unsupported;
        services.notification_dismiss = unsupported;
        services.taskbar_progress = support;
        services.application_badge = unsupported;
        services.taskbar_overlay_icon = support;
    }

    pub(crate) fn start_watch(&self, sink: ShellEventSink) {
        let registration = ShellEventRegistration::install(sink);
        *self.inner.events.borrow_mut() = Some(registration);
    }

    pub(crate) fn set_notification_identity(&self, identity: Option<String>) {
        *self.inner.notification_identity.borrow_mut() = identity;
    }

    pub(crate) fn apply(
        &self,
        system: NativeWindowSystem,
        request: NativeApplicationShellRequest,
        target_window: Option<&winit::window::Window>,
    ) -> Result<(), ApplicationShellError> {
        if system != NativeWindowSystem::Win32 {
            return Err(ApplicationShellError::Unsupported(
                request.operation.required_feature(),
            ));
        }
        let trays = &self.inner.trays;
        match request.operation {
            NativeApplicationShellOperation::CreateTray {
                id,
                presentation,
                menu,
            } => trays.create(id, &presentation, &menu),
            NativeApplicationShellOperation::UpdateTray {
                id,
                presentation,
                menu,
            } => trays.update(id, &presentation, &menu),
            NativeApplicationShellOperation::RemoveTray { id } => trays.remove(id),
            NativeApplicationShellOperation::ShowNotification { id, presentation } => {
                self.show_notification(id, presentation)
            }
            operation @ (NativeApplicationShellOperation::UpdateNotification { .. }
            | NativeApplicationShellOperation::CloseNotification { .. }) => Err(
                ApplicationShellError::Unsupported(operation.required_feature()),
            ),
            NativeApplicationShellOperation::SetTaskbarDockState(state) => {
                set_taskbar_state(state, target_window)
            }
        }
    }

    fn show_notification(
        &self,
        id: incular_platform::NotificationId,
        presentation: incular_platform::NotificationPresentation,
    ) -> Result<(), ApplicationShellError> {
        let identity = self.inner.notification_identity.borrow();
        let identity = identity.as_deref().ok_or_else(|| {
            ApplicationShellError::PlatformConfigurationRequired(
                "Windows notifications require the installed application's AppUserModelID; call ApplicationShellService::set_notification_identity".to_owned(),
            )
        })?;
        let mut toast = tauri_winrt_notification::Toast::new(identity)
            .title(&presentation.title)
            .text1(&presentation.body);
        for action in &presentation.actions {
            toast = toast.add_button(&action.label, action.id.as_str());
        }
        toast = toast.on_activated(move |action| {
            emit_shell_event(match action {
                Some(action) => NativeApplicationShellEvent::NotificationAction {
                    id,
                    action: incular_platform::NotificationActionId::new(action),
                },
                None => NativeApplicationShellEvent::NotificationActivated { id },
            });
            Ok(())
        });
        toast = toast.on_dismissed(move |_| {
            emit_shell_event(NativeApplicationShellEvent::NotificationDismissed { id });
            Ok(())
        });
        toast.show().map_err(native_failure)
    }
}

fn set_taskbar_state(
    state: incular_platform::TaskbarDockState,
    target_window: Option<&winit::window::Window>,
) -> Result<(), ApplicationShellError> {
    use windows::Win32::{
        Foundation::{HWND, RPC_E_CHANGED_MODE},
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        },
        UI::Shell::{
            ITaskbarList3, TBPF_ERROR, TBPF_INDETERMINATE, TBPF_NOPROGRESS, TBPF_NORMAL,
            TBPF_PAUSED, TaskbarList,
        },
    };
    use winit::raw_window_handle::RawWindowHandle;

    if state.badge != incular_platform::ApplicationBadge::None {
        return Err(ApplicationShellError::Unsupported(
            ApplicationShellFeature::ApplicationBadge,
        ));
    }
    if target_window.is_none()
        && state.progress == incular_platform::TaskbarProgress::None
        && state.overlay_icon.is_none()
    {
        return Ok(());
    }
    let window = target_window.ok_or(ApplicationShellError::StaleResource)?;
    let RawWindowHandle::Win32(handle) =
        incular_desktop::winit_adapter::raw_window_handles(window).window
    else {
        return Err(ApplicationShellError::Unsupported(
            ApplicationShellFeature::TaskbarProgress,
        ));
    };
    let hwnd = HWND(handle.hwnd.get() as *mut std::ffi::c_void);
    // COM initialization is thread-local. Winit does not promise that the
    // application event-loop thread has initialized COM, while taskbar APIs
    // require it. Balance S_OK/S_FALSE with CoUninitialize; if another
    // apartment model already owns the thread, RPC_E_CHANGED_MODE still means
    // COM is initialized and usable, so no uninitialize is owed by Incular.
    let coinit = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let _com = if coinit.is_ok() {
        Some(ComUninitializeGuard)
    } else if coinit == RPC_E_CHANGED_MODE {
        None
    } else {
        return Err(ApplicationShellError::NativeFailure(format!(
            "failed to initialize COM for taskbar integration: {coinit}"
        )));
    };

    // SAFETY: COM is initialized for this event-loop thread and `hwnd` belongs
    // to the live Winit window passed by the shared runner.
    unsafe {
        let taskbar: ITaskbarList3 =
            CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER).map_err(native_failure)?;
        taskbar.HrInit().map_err(native_failure)?;
        let (flag, fraction) = match state.progress {
            incular_platform::TaskbarProgress::None => (TBPF_NOPROGRESS, None),
            incular_platform::TaskbarProgress::Indeterminate => (TBPF_INDETERMINATE, None),
            incular_platform::TaskbarProgress::Normal(value) => (TBPF_NORMAL, Some(value)),
            incular_platform::TaskbarProgress::Paused(value) => (TBPF_PAUSED, Some(value)),
            incular_platform::TaskbarProgress::Error(value) => (TBPF_ERROR, Some(value)),
        };
        taskbar
            .SetProgressState(hwnd, flag)
            .map_err(native_failure)?;
        if let Some(fraction) = fraction {
            let completed = (fraction.clamp(0.0, 1.0) * 10_000.0).round() as u64;
            taskbar
                .SetProgressValue(hwnd, completed, 10_000)
                .map_err(native_failure)?;
        }
        if let Some(icon) = state.overlay_icon.as_ref() {
            let icon = OwnedTaskbarIcon::new(icon)?;
            taskbar
                .SetOverlayIcon(hwnd, icon.handle, windows::core::PCWSTR::null())
                .map_err(native_failure)?;
        } else {
            taskbar
                .SetOverlayIcon(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::HICON::default(),
                    windows::core::PCWSTR::null(),
                )
                .map_err(native_failure)?;
        }
    }
    Ok(())
}

struct ComUninitializeGuard;

impl Drop for ComUninitializeGuard {
    fn drop(&mut self) {
        // SAFETY: this guard is created only after a successful CoInitializeEx
        // on this event-loop thread and never leaves the synchronous call.
        unsafe { windows::Win32::System::Com::CoUninitialize() };
    }
}

struct OwnedTaskbarIcon {
    handle: windows::Win32::UI::WindowsAndMessaging::HICON,
}

impl OwnedTaskbarIcon {
    fn new(icon: &WindowIcon) -> Result<Self, ApplicationShellError> {
        use windows::Win32::UI::WindowsAndMessaging::CreateIcon;

        let mut bgra = icon.rgba().to_vec();
        let (pixels, remainder) = bgra.as_chunks_mut::<4>();
        debug_assert!(remainder.is_empty());
        for pixel in pixels {
            pixel.swap(0, 2);
        }
        let pixel_count = usize::try_from(icon.width())
            .ok()
            .and_then(|width| {
                usize::try_from(icon.height())
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or_else(|| {
                ApplicationShellError::NativeFailure(
                    "taskbar overlay icon dimensions overflow".to_owned(),
                )
            })?;
        // Match the proven 32-bpp Win32 icon path used by tray-icon/winit: the
        // monochrome mask inverts alpha while the XOR plane carries BGRA.
        let (rgba_pixels, remainder) = icon.rgba().as_chunks::<4>();
        debug_assert!(remainder.is_empty());
        let and_mask = rgba_pixels
            .iter()
            .map(|pixel| pixel[3].wrapping_sub(u8::MAX))
            .collect::<Vec<_>>();
        debug_assert_eq!(and_mask.len(), pixel_count);
        let width = i32::try_from(icon.width()).map_err(|_| {
            ApplicationShellError::NativeFailure("taskbar overlay icon is too wide".to_owned())
        })?;
        let height = i32::try_from(icon.height()).map_err(|_| {
            ApplicationShellError::NativeFailure("taskbar overlay icon is too tall".to_owned())
        })?;
        let handle =
            unsafe { CreateIcon(None, width, height, 1, 32, and_mask.as_ptr(), bgra.as_ptr()) }
                .map_err(native_failure)?;
        Ok(Self { handle })
    }
}

impl Drop for OwnedTaskbarIcon {
    fn drop(&mut self) {
        // SAFETY: handle was created by CreateIcon and this RAII owner is the
        // unique object responsible for releasing it after SetOverlayIcon has
        // copied the icon into the taskbar.
        let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyIcon(self.handle) };
    }
}

fn native_failure(error: impl std::fmt::Display) -> ApplicationShellError {
    ApplicationShellError::NativeFailure(error.to_string())
}
