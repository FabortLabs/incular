//! The mobile embedder shell shared by Android and iOS.
//!
//! On phones the platform owns the process, the event loop and the views, so
//! Incular runs as an engine the native shell drives (see
//! `system-design/MOBILE.md`). [`MobileEngine`] is that engine without any
//! FFI: the Android and iOS crates translate native callbacks into calls on
//! it, and it presents through [`incular_host::HostSurface`] exactly like the
//! Winit shell does.
//!
//! The engine binds the application's first window to the native view. A
//! phone shows one Incular view per screen, so further windows are refused
//! with a typed `unsupported` error instead of being shown nowhere.
//!
//! Every method must be called on the platform main thread. Native callbacks
//! only queue state here; [`MobileEngine::frame`] runs the runtime.

mod entry;
mod environment;
pub mod text_offsets;

pub use entry::{install_application, take_application};
pub use environment::MobileEnvironment;

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PointerDeviceKind, PointerPhase, PointerSampleMetadata, StylusMetadata,
    time::{Duration, Instant},
};
use incular_host::HostSurface;
use incular_platform::{
    CapabilitySupport, Clipboard, NativeOperationCompletion, PhysicalSize, PlatformCapabilities,
    PlatformEvent, PlatformLifecycle, PlatformOperationError, SystemUiOverlayStyle,
    TextInputCommand, WindowCommand, WindowEvent, WindowId, WindowLifecycle, WindowMetrics,
    WindowOptions,
};
use incular_runtime::{Application, NativeWindowCommand, RuntimeWake};
use incular_wgpu::{SharedGpuContext, WindowSurfaceTarget};
use std::sync::Arc;

/// The capabilities every mobile view honors. Everything else a desktop
/// window can do (titles, moving, resizing, levels) does not exist on a phone.
#[must_use]
pub fn mobile_platform_capabilities() -> PlatformCapabilities {
    let mut capabilities = PlatformCapabilities::unsupported();
    capabilities.window.request_redraw = CapabilitySupport::Supported;
    capabilities
}

/// One touch, mouse or stylus sample as the native shell reports it, in
/// physical pixels of the view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchSample {
    /// Stable for the duration of one contact.
    pub pointer: u64,
    /// Identifies the source device, or zero when the shell has no device ID.
    pub device: u64,
    pub phase: PointerPhase,
    pub kind: PointerDeviceKind,
    /// Physical pixels from the view's top-left corner.
    pub x: f64,
    pub y: f64,
    /// Normalized pressure, when the device reports one.
    pub pressure: Option<incular_core::NormalizedPressure>,
    /// Pressed-button mask after this sample, in Incular's pointer-button
    /// bits. Finger and stylus tips report the primary bit while they touch;
    /// mice report their real buttons.
    pub buttons: u32,
    /// The button that went down or up on a Down or Up sample. `None` for
    /// movement and for platforms that do not expose the changed button.
    pub button: Option<u32>,
    /// Stylus tilt and orientation, when the device reports them.
    pub stylus: Option<StylusMetadata>,
    /// How long ago the platform recorded the sample. Historical samples
    /// batched into one delivery keep their own age, so velocity tracking
    /// sees their real spacing. Zero means "now".
    pub age: Duration,
}

/// A pointer-wheel or trackpad scroll as the native shell reports it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollSample {
    /// Physical pixels from the view's top-left corner, where the pointer was.
    pub x: f64,
    pub y: f64,
    /// Scroll delta in physical pixels, positive toward the bottom and right
    /// (the sign Incular's `InputEvent::ScrollAt` uses).
    pub delta_x: f64,
    pub delta_y: f64,
    /// How long ago the platform recorded the scroll.
    pub age: Duration,
}

/// The instant a sample recorded `age` ago. A sample older than the clock's
/// origin falls back to now.
fn sample_time(age: Duration) -> Instant {
    let now = Instant::now();
    now.checked_sub(age).unwrap_or(now)
}

/// The native view the application's first window is presented in.
struct BoundWindow {
    id: WindowId,
    options: WindowOptions,
    /// Present while the native surface exists. Android destroys surfaces
    /// while the activity lives (backgrounding, rotation in some modes), so
    /// the window outlives its surface.
    surface: Option<HostSurface>,
}

/// The mobile engine: one application, its shared GPU device and the native
/// view its first window is bound to.
pub struct MobileEngine {
    application: Application,
    window: Option<BoundWindow>,
    shared_gpu: Option<SharedGpuContext>,
    /// The native surface reported before the application asked for a
    /// window, applied when the window arrives.
    pending_surface: Option<(WindowSurfaceTarget, PhysicalSize, f64)>,
    environment: MobileEnvironment,
    metrics: WindowMetrics,
    /// A new or resized surface must be painted even when no retained
    /// state changed.
    force_frame: bool,
    /// The platform clipboard, installed on the window when it arrives.
    clipboard: Option<Box<dyn Clipboard>>,
}

impl MobileEngine {
    /// Starts an engine for `application`. `wake` must schedule
    /// [`Self::process_runtime_work`] on the main thread; it is called from
    /// Tokio worker threads when async work completes.
    pub fn new(mut application: Application, wake: Arc<dyn RuntimeWake>) -> Self {
        application.set_platform_capabilities(mobile_platform_capabilities());
        let launch =
            incular_platform::ApplicationActivation::Launch(application.launch_activation());
        application.handle_application_activation(launch);
        application.set_wake_handler(wake);
        let mut engine = Self {
            application,
            window: None,
            shared_gpu: None,
            pending_surface: None,
            environment: MobileEnvironment::default(),
            metrics: WindowMetrics::new(PhysicalSize::new(0, 0), 1.0),
            force_frame: false,
            clipboard: None,
        };
        engine.apply_window_commands();
        engine
    }

    /// The application, for platform crates that publish extra events.
    pub const fn application_mut(&mut self) -> &mut Application {
        &mut self.application
    }

    /// The runtime window bound to the native view, once the application
    /// created one.
    #[must_use]
    pub fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|window| window.id)
    }

    /// Runs completed async work. Call from the main thread after a wake.
    pub fn process_runtime_work(&mut self) {
        self.application.process_runtime_work();
        self.apply_window_commands();
    }

    /// The native surface was created or replaced.
    pub fn surface_created(&mut self, target: WindowSurfaceTarget, size: PhysicalSize, scale: f64) {
        self.pending_surface = Some((target, size, scale));
        self.bind_pending_surface();
    }

    /// The native surface changed size or density.
    pub fn surface_changed(&mut self, size: PhysicalSize, scale: f64) {
        if let Some((_, pending_size, pending_scale)) = self.pending_surface.as_mut() {
            *pending_size = size;
            *pending_scale = scale;
        }
        self.resize(size, scale);
    }

    /// The native surface is about to be destroyed. The renderer is dropped
    /// before this returns, as Android requires; the GPU device is kept.
    pub fn surface_destroyed(&mut self) {
        self.pending_surface = None;
        if let Some(window) = self.window.as_mut()
            && window.surface.take().is_some()
        {
            self.application
                .handle_window_event(WindowEvent::lifecycle(window.id, WindowLifecycle::Hidden));
        }
    }

    /// Applies an application lifecycle transition reported by the shell.
    pub fn lifecycle(&mut self, lifecycle: PlatformLifecycle) {
        self.application.handle_application_lifecycle(lifecycle);
    }

    /// Replaces the system-provided environment (insets, text scale,
    /// brightness, locales) and republishes it to the runtime.
    pub fn set_environment(&mut self, environment: MobileEnvironment) {
        if self.environment != environment {
            self.environment = environment;
            self.publish_environment();
        }
    }

    /// Forwards one touch contact, or a mouse or stylus hover sample.
    pub fn touch(&mut self, sample: TouchSample) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let scale = self.metrics.scale_factor as f32;
        let event = InputEvent::PointerWithMetadata {
            pointer: sample.pointer,
            device: sample.device,
            kind: sample.kind,
            buttons: sample.buttons,
            button: sample.button,
            sample: PointerSampleMetadata {
                pressure: sample.pressure,
                stylus: sample.stylus,
            },
            phase: sample.phase,
            position: Offset::new(sample.x as f32 / scale, sample.y as f32 / scale),
        };
        let time = sample_time(sample.age);
        self.application.handle_window_event(WindowEvent::platform(
            window.id,
            PlatformEvent::InputAt { input: event, time },
        ));
    }

    /// Forwards one scroll at the location reported by the shell.
    pub fn scroll(&mut self, sample: ScrollSample) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let scale = self.metrics.scale_factor as f32;
        let input = InputEvent::ScrollAt {
            position: Offset::new(sample.x as f32 / scale, sample.y as f32 / scale),
            delta: Offset::new(sample.delta_x as f32 / scale, sample.delta_y as f32 / scale),
        };
        self.application.handle_window_event(WindowEvent::platform(
            window.id,
            PlatformEvent::InputAt {
                input,
                time: sample_time(sample.age),
            },
        ));
    }

    /// Forwards a platform event, such as a key, a text input action or the
    /// input method's editing state, to the bound window.
    pub fn platform_event(&mut self, event: PlatformEvent) {
        if let Some(window) = self.window.as_ref() {
            self.application
                .handle_window_event(WindowEvent::platform(window.id, event));
        }
    }

    /// The system back button. Returns `false` when nothing in the app used
    /// it; the shell then leaves the app, as Flutter's `SystemNavigator.pop`.
    pub fn back_button(&mut self) -> bool {
        self.window
            .as_ref()
            .is_some_and(|window| self.application.handle_window_back_button(window.id))
    }

    /// Takes the text input commands (show the keyboard for a field, its new
    /// state, hide it) the runtime queued since the last call.
    pub fn take_text_input_commands(&mut self) -> Vec<TextInputCommand> {
        self.window.as_ref().map_or_else(Vec::new, |window| {
            self.application.take_window_text_input_commands(window.id)
        })
    }

    /// Takes the system bar style the app or its annotations asked for since the
    /// last call, to apply on the platform. iOS ignores it for now: its status
    /// bar style needs a view controller override, tracked separately.
    pub fn take_system_ui_overlay_style(&mut self) -> Option<SystemUiOverlayStyle> {
        let window = self.window.as_ref()?;
        self.application
            .take_window_system_ui_overlay_style(window.id)
    }

    /// Installs the system clipboard, replacing the runtime's in-memory one,
    /// so copy, cut and paste reach other apps.
    pub fn set_clipboard(&mut self, clipboard: Box<dyn Clipboard>) {
        match self.window.as_ref() {
            Some(window) => self.application.set_window_clipboard(window.id, clipboard),
            None => self.clipboard = Some(clipboard),
        }
    }

    /// Runs one frame if the runtime asked for one. Returns whether the shell
    /// should schedule another frame callback.
    pub fn frame(&mut self, now: Instant) -> bool {
        self.application.process_simulation_requests();
        self.apply_window_commands();
        let Some(window) = self.window.as_mut() else {
            return false;
        };
        let Some(surface) = window.surface.as_mut() else {
            return false;
        };
        let id = window.id;
        let wanted = self.force_frame || self.application.frame_requested(id);
        if !wanted || !surface.retry().attempt_due(now) {
            return surface.retry().retry_at().is_some();
        }
        self.force_frame = false;
        self.application.note_frame_requested(id);
        if self.application.simulation_capture_pending(id) {
            surface.renderer_mut().request_capture();
        }
        let logical = surface.metrics().logical_size();
        match self
            .application
            .run_window_frame_at(id, Constraints::tight(logical), now)
        {
            Ok(Some((list, _))) => {
                if let incular_host::PresentOutcome::Failed(error) =
                    surface.present(&mut self.application, &list)
                {
                    tracing::error!("Incular renderer error: {error}");
                }
            }
            Ok(None) => self.application.note_presented(id, false),
            Err(error) => {
                tracing::error!("Incular runtime error: {error:?}");
                self.application
                    .fail_simulation_frame(id, format!("{error:?}"));
            }
        }
        if let Some(deadline) = surface.renderer_mut().idle_trim_deadline()
            && now >= deadline
        {
            surface.renderer_mut().trim_idle();
        }
        self.application.frame_requested(id) || surface.retry().retry_at().is_some()
    }

    /// Whether the runtime has work for the next frame callback.
    #[must_use]
    pub fn wants_frame(&self) -> bool {
        self.window.as_ref().is_some_and(|window| {
            window.surface.is_some()
                && (self.force_frame || self.application.frame_requested(window.id))
        })
    }

    fn apply_window_commands(&mut self) {
        for command in self.application.take_native_window_commands() {
            match command {
                NativeWindowCommand::Create { window_id, options } => {
                    self.create_window(window_id, options);
                }
                NativeWindowCommand::Operate(command) => self.operate_window(command),
            }
        }
    }

    fn create_window(&mut self, id: WindowId, options: WindowOptions) {
        if self.window.is_some() {
            // A phone shows one Incular view per screen.
            let _ = self.application.record_platform_operation_error(
                id,
                PlatformOperationError::with_context(
                    incular_platform::PlatformOperationErrorKind::Unsupported,
                    "mobile hosts present only the application's first window",
                ),
            );
            let _ = self.application.close_window(id);
            return;
        }
        let _ = self
            .application
            .set_window_capabilities(id, mobile_platform_capabilities());
        self.window = Some(BoundWindow {
            id,
            options,
            surface: None,
        });
        if let Some(clipboard) = self.clipboard.take() {
            self.application.set_window_clipboard(id, clipboard);
        }
        self.publish_metrics();
        self.publish_environment();
        self.bind_pending_surface();
    }

    fn operate_window(&mut self, command: WindowCommand) {
        let supported = matches!(
            command.operation,
            incular_platform::WindowOperation::RequestRedraw
        );
        if supported {
            self.force_frame = true;
        }
        if let Some(request_id) = command.request_id {
            let result = if supported {
                Ok(())
            } else {
                Err(PlatformOperationError::unsupported())
            };
            let _ = self
                .application
                .complete_native_operation(NativeOperationCompletion::new(
                    command.window_id,
                    request_id,
                    result,
                ));
        }
    }

    fn bind_pending_surface(&mut self) {
        let Some(window) = self.window.as_mut() else {
            return;
        };
        let Some((target, size, scale)) = self.pending_surface.clone() else {
            return;
        };
        // The old renderer must go before a new surface is created for the
        // same native window.
        window.surface = None;
        let created = pollster::block_on(incular_host::create_renderer(
            self.shared_gpu.clone(),
            target,
            size,
            window.options.transparency_mode,
            window.options.background_color,
        ));
        let (renderer, shared) = match created {
            Ok(created) => created,
            Err(error) => {
                tracing::error!("Incular renderer initialization failed: {error}");
                return;
            }
        };
        self.shared_gpu.get_or_insert(shared);
        let metrics = WindowMetrics::new(size, scale);
        window.surface = Some(HostSurface::new(window.id, renderer, metrics));
        let id = window.id;
        self.application
            .handle_window_event(WindowEvent::lifecycle(id, WindowLifecycle::Visible));
        self.metrics = metrics;
        self.publish_metrics();
        self.publish_environment();
        self.force_frame = true;
    }

    fn resize(&mut self, size: PhysicalSize, scale: f64) {
        self.metrics = WindowMetrics::new(size, scale);
        let Some(window) = self.window.as_mut() else {
            return;
        };
        let redraw = window
            .surface
            .as_mut()
            .map(|surface| surface.resize(size, scale))
            .is_some_and(|resized| resized.redraw);
        self.publish_metrics();
        self.publish_environment();
        self.force_frame |= redraw;
    }

    fn publish_metrics(&mut self) {
        if let Some(window) = self.window.as_ref() {
            self.application.handle_window_event(WindowEvent::platform(
                window.id,
                PlatformEvent::Metrics(self.metrics),
            ));
        }
    }

    fn publish_environment(&mut self) {
        if let Some(window) = self.window.as_ref() {
            let environment = self.environment.snapshot(self.metrics);
            self.application.handle_window_event(WindowEvent::platform(
                window.id,
                PlatformEvent::Environment(environment),
            ));
        }
    }
}
