use crate::application_shell::ApplicationShellService;
use crate::application_types::WindowError;
use crate::environment::{BUILD_SCOPE, EnvironmentFields, install_focus_scope_watch};
use crate::file_dialogs::FileDialogService;
use crate::frame::Runtime;
use crate::global_shortcuts::GlobalShortcutService;
use crate::restoration::{self, Restorable, RestorationHandle};
#[cfg(not(target_arch = "wasm32"))]
use crate::tasks::TokioHandle;
use crate::tasks::{RuntimeSpawner, Task, TaskFailure, TaskHandle, TaskScope};
use crate::window_commands::{WindowHandle, WindowOpener};
use crate::window_state::WindowManager;
use incular_config::RuntimeEnvironment;
use incular_core::{RestorationKey, RestorationScope};
use incular_platform::WindowOptions;
use incular_widgets::{FocusScopeNode, Widget};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Framework-controlled build scope for declarative roots. It intentionally
/// exposes no element IDs or scheduler handles. Signals use the runtime's
/// scoped collector while a builder executes; reads outside a build simply
/// return the current value and create no dependency.
pub struct BuildContext {
    spawner: RuntimeSpawner,
    owner_scope: TaskScope,
    environment: Rc<RefCell<RuntimeEnvironment>>,
    environment_dependencies: Rc<Cell<EnvironmentFields>>,
    window_manager: Option<WindowManager>,
    restoration_scope: Option<RestorationScope>,
    restoration: Option<restoration::RestorationManager>,
}
impl BuildContext {
    pub(crate) fn new(
        spawner: RuntimeSpawner,
        owner_scope: TaskScope,
        environment: Rc<RefCell<RuntimeEnvironment>>,
        environment_dependencies: Rc<Cell<EnvironmentFields>>,
        window_manager: Option<WindowManager>,
        restoration_scope: Option<RestorationScope>,
    ) -> Self {
        Self {
            spawner,
            owner_scope,
            environment,
            environment_dependencies,
            restoration_scope,
            restoration: window_manager
                .as_ref()
                .and_then(|manager| manager.restoration.clone()),
            window_manager,
        }
    }
    /// Spawns Tokio work owned by this declarative owner. Its output must be
    /// `Send`; use [`Self::spawn_into`] to update UI-local state on completion.
    pub fn spawn<F, T>(&self, future: F) -> Task<T>
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.spawner.spawn_in(&self.owner_scope, future)
    }
    pub fn spawn_in<F, T>(&self, scope: &TaskScope, future: F) -> Task<T>
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.spawner.spawn_in(scope, future)
    }
    /// Starts owner-scoped Tokio work and handles the owned result on the UI
    /// thread. The callback may safely capture an Incular `Signal`.
    pub fn spawn_into<F, T>(
        &self,
        future: F,
        complete: impl FnOnce(Result<T, TaskFailure>, &mut Runtime) + 'static,
    ) -> TaskHandle
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.spawner
            .spawn_into_in(&self.owner_scope, future, complete)
    }
    /// Queues a `Send` callback for the UI owner without granting any Tokio
    /// worker mutable access to the widget/runtime tree.
    pub fn dispatch(&self, callback: impl FnOnce(&mut Runtime) + Send + 'static) {
        self.spawner.dispatcher().dispatch(callback);
    }
    pub fn spawn_blocking<T>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
        complete: impl FnOnce(Result<T, TaskFailure>, &mut Runtime) + 'static,
    ) -> TaskHandle
    where
        T: Send + 'static,
    {
        self.spawner
            .spawn_blocking_in(&self.owner_scope, work, complete)
    }
    #[must_use]
    pub fn task_scope(&self) -> TaskScope {
        self.owner_scope.child()
    }

    /// Makes the current retained builder depend on a focus scope.
    ///
    /// A scope is an application-owned handle rather than a signal, so this
    /// explicit watch registers a weak, owner-lifetime subscription in the
    /// runtime's ordinary reactive queue. The initial root build records the
    /// scope and installs the subscription once its element identity exists.
    pub fn watch_focus_scope(&self, scope: &FocusScopeNode) {
        let target = BUILD_SCOPE.with(|current| {
            let current = current.borrow();
            current.as_ref().map(|build| {
                (
                    build.element,
                    build.queue.clone(),
                    build.initial_dependencies.clone(),
                )
            })
        });
        let Some((element, queue, initial_dependencies)) = target else {
            return;
        };
        if let Some(element) = element {
            if let Some(queue) = queue.upgrade() {
                install_focus_scope_watch(&queue, element, scope);
            }
        } else if let Some(initial_dependencies) = initial_dependencies {
            initial_dependencies
                .borrow_mut()
                .focus_scopes
                .push(scope.clone());
        }
    }

    /// Returns the stable restoration scope for this build when the owning
    /// application explicitly enabled restoration. Ordinary applications have
    /// no restoration scope and therefore no persistence side effects.
    #[must_use]
    pub fn restoration_scope(&self) -> Option<RestorationScope> {
        self.restoration_scope.clone()
    }

    /// Returns a callback-safe capability for reset, flush, and diagnostics
    /// when restoration is enabled. It contains no widget tree or native
    /// filesystem handle.
    #[must_use]
    pub fn restoration(&self) -> Option<RestorationHandle> {
        self.restoration.clone().map(RestorationHandle::new)
    }

    /// Creates an opt-in reactive value under this build's restoration scope.
    /// `key` must be stable across reordering and rebuilds; use a separate
    /// child scope for dynamic collections rather than an element position.
    #[must_use]
    pub fn restorable<T>(&self, key: RestorationKey, default: T) -> Option<Restorable<T>>
    where
        T: Clone + PartialEq + serde::Serialize + serde::de::DeserializeOwned + 'static,
    {
        self.restoration_scope
            .clone()
            .map(|scope| Restorable::from_scope(scope, key, default))
    }

    /// Alias matching the common `cx.restored_signal("count", 0)` pattern.
    /// The returned [`Restorable`] wraps a normal reactive signal; write via its
    /// `set`/`update` methods so a mutation is captured for persistence.
    #[must_use]
    pub fn restored_signal<T>(&self, key: RestorationKey, default: T) -> Option<Restorable<T>>
    where
        T: Clone + PartialEq + serde::Serialize + serde::de::DeserializeOwned + 'static,
    {
        self.restorable(key, default)
    }

    /// Queues another retained root for native creation. Its state, focus,
    /// semantics, environment, compositor, and task scope are independent of
    /// this context's window; shared `Signal`s may still be captured by the
    /// supplied root and will subscribe each root independently.
    pub fn open_window(
        &self,
        options: WindowOptions,
        root: Widget,
    ) -> Result<WindowHandle, WindowError> {
        self.window_manager
            .as_ref()
            .ok_or(WindowError::ApplicationStopped)?
            .open_window(options, root)
    }

    /// Returns a retained UI capability that callbacks may use to open a
    /// window after this build has returned.
    #[must_use]
    pub fn window_opener(&self) -> Option<WindowOpener> {
        self.window_manager
            .as_ref()
            .cloned()
            .map(|manager| WindowOpener { manager })
    }

    /// Returns a command handle for the native window owning this build.
    /// Standalone/headless runtimes without an application window return None.
    #[must_use]
    pub fn window_handle(&self) -> Option<WindowHandle> {
        let window_id = self.spawner.window_id()?;
        self.window_manager.as_ref()?.handle(window_id)
    }

    /// Returns the native file-dialog service owned by the current window.
    /// Standalone/headless runtimes without an application window return None.
    #[must_use]
    pub fn file_dialogs(&self) -> Option<FileDialogService> {
        self.window_handle().map(|window| window.file_dialogs())
    }

    /// Returns the application-scoped native global-shortcut service. Unlike
    /// focused `Shortcuts`, registrations remain active while the application
    /// is in the background and are identified independently of any window.
    #[must_use]
    pub fn global_shortcuts(&self) -> Option<GlobalShortcutService> {
        let manager = self.window_manager.as_ref()?;
        Some(GlobalShortcutService::new(
            manager.global_shortcut_bridge.clone(),
            manager.application_capabilities.clone(),
        ))
    }

    /// Returns application-scoped tray/status, notification, and taskbar/Dock
    /// services. The service contains no native handles and is callback-safe.
    #[must_use]
    pub fn application_shell(&self) -> Option<ApplicationShellService> {
        self.window_manager
            .as_ref()
            .map(|manager| manager.application_shell.clone())
    }

    /// Returns the Tokio handle for advanced integrations. Tasks spawned
    /// directly through it are not owner-scoped; prefer [`Self::spawn`] or
    /// [`Self::spawn_into`] for UI work.
    #[must_use]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn tokio_handle(&self) -> TokioHandle {
        self.spawner.tokio_handle()
    }
    /// Reads all environment fields. Prefer the typed accessors below when a
    /// build uses only one value, so unrelated changes do not rebuild it.
    #[must_use]
    pub fn environment(&self) -> RuntimeEnvironment {
        self.environment_dependencies
            .set(self.environment_dependencies.get() | EnvironmentFields::all());
        self.environment.borrow().clone()
    }
    #[must_use]
    pub fn viewport(&self) -> incular_core::Size {
        self.record_environment(EnvironmentFields::VIEWPORT);
        self.environment.borrow().viewport
    }
    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        self.record_environment(EnvironmentFields::SCALE);
        self.environment.borrow().scale_factor
    }
    #[must_use]
    pub fn text_scale(&self) -> f32 {
        self.record_environment(EnvironmentFields::TEXT_SCALE);
        self.environment.borrow().text_scale
    }
    #[must_use]
    pub fn safe_insets(&self) -> incular_config::EdgeInsets {
        self.record_environment(EnvironmentFields::SAFE_INSETS);
        self.environment.borrow().safe_insets
    }
    #[must_use]
    pub fn view_insets(&self) -> incular_config::EdgeInsets {
        self.record_environment(EnvironmentFields::VIEW_INSETS);
        self.environment.borrow().view_insets
    }
    /// Persistent system-obstruction margin, independent of transient
    /// occlusion. Maintained `SafeArea` bottom edges read this alongside
    /// the safe margin.
    #[must_use]
    pub fn view_padding(&self) -> incular_config::EdgeInsets {
        self.record_environment(EnvironmentFields::VIEW_PADDING);
        self.environment.borrow().view_padding
    }
    /// Resolves `SafeArea` from the authoritative logical insets while
    /// recording only the dependencies it reads: the safe margin always,
    /// plus the persistent view padding when the descriptor maintains its
    /// bottom edge. This applies the same policy as retained
    /// construction; transient occlusion never participates.
    #[must_use]
    pub fn safe_area(&self, safe_area: incular_widgets::SafeArea) -> Widget {
        let safe = self.safe_insets();
        if safe_area.maintains_bottom_view_padding() {
            safe_area.resolve_with_padding(safe, self.view_padding())
        } else {
            safe_area.resolve(safe)
        }
    }
    #[must_use]
    pub fn brightness(&self) -> incular_config::Brightness {
        self.record_environment(EnvironmentFields::BRIGHTNESS);
        self.environment.borrow().brightness
    }
    #[must_use]
    pub fn locales(&self) -> Vec<incular_config::Locale> {
        self.record_environment(EnvironmentFields::LOCALE);
        self.environment.borrow().locales.clone()
    }
    /// Resolves a supported application locale while recording a locale-only
    /// dependency, so platform locale changes rebuild only consumers of it.
    #[must_use]
    pub fn resolve_locale(
        &self,
        supported: &[incular_config::Locale],
    ) -> Option<incular_config::Locale> {
        self.record_environment(EnvironmentFields::LOCALE);
        self.environment.borrow().resolve_locale(supported)
    }
    #[must_use]
    pub fn text_direction(&self) -> incular_config::TextDirection {
        self.record_environment(EnvironmentFields::DIRECTION);
        self.environment.borrow().text_direction
    }
    #[must_use]
    pub fn window_focused(&self) -> bool {
        self.record_environment(EnvironmentFields::WINDOW_FOCUS);
        self.environment.borrow().window_focused
    }
    #[must_use]
    pub fn reduced_motion(&self) -> bool {
        self.record_environment(EnvironmentFields::REDUCED_MOTION);
        self.environment.borrow().reduced_motion
    }
    #[must_use]
    pub fn high_contrast(&self) -> bool {
        self.record_environment(EnvironmentFields::HIGH_CONTRAST);
        self.environment.borrow().high_contrast
    }
    #[must_use]
    pub fn input_capabilities(&self) -> incular_config::InputCapabilities {
        self.record_environment(EnvironmentFields::INPUT);
        self.environment.borrow().input
    }
    #[must_use]
    pub fn window_occluded(&self) -> bool {
        self.record_environment(EnvironmentFields::WINDOW_OCCLUSION);
        self.environment.borrow().window_occluded
    }
    #[must_use]
    pub fn system_gesture_insets(&self) -> incular_config::EdgeInsets {
        self.record_environment(EnvironmentFields::SYSTEM_GESTURE_INSETS);
        self.environment.borrow().system_gesture_insets
    }
    #[must_use]
    pub fn always_use_24_hour_format(&self) -> bool {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().always_use_24_hour_format
    }
    #[must_use]
    pub fn accessible_navigation(&self) -> bool {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().accessible_navigation
    }
    #[must_use]
    pub fn invert_colors(&self) -> bool {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().invert_colors
    }
    #[must_use]
    pub fn on_off_switch_labels(&self) -> bool {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().on_off_switch_labels
    }
    #[must_use]
    pub fn bold_text(&self) -> bool {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().bold_text
    }
    #[must_use]
    pub fn supports_announce(&self) -> bool {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().supports_announce
    }
    #[must_use]
    pub fn navigation_mode(&self) -> incular_config::NavigationMode {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().navigation_mode
    }
    #[must_use]
    pub fn supports_showing_system_context_menu(&self) -> bool {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment
            .borrow()
            .supports_showing_system_context_menu
    }
    #[must_use]
    pub fn line_height_scale_factor_override(&self) -> Option<f32> {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().line_height_scale_factor_override
    }
    #[must_use]
    pub fn letter_spacing_override(&self) -> Option<f32> {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().letter_spacing_override
    }
    #[must_use]
    pub fn word_spacing_override(&self) -> Option<f32> {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().word_spacing_override
    }
    #[must_use]
    pub fn paragraph_spacing_override(&self) -> Option<f32> {
        self.record_environment(EnvironmentFields::ACCESSIBILITY);
        self.environment.borrow().paragraph_spacing_override
    }
    /// Flutter's `MediaQueryData.disableAnimations`, which Incular reads
    /// from the reduced-motion preference.
    #[must_use]
    pub fn disable_animations(&self) -> bool {
        self.reduced_motion()
    }
    fn record_environment(&self, field: EnvironmentFields) {
        self.environment_dependencies
            .set(self.environment_dependencies.get() | field);
    }
}
