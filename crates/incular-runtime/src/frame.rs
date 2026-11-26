use crate::application_types::{
    ApplicationLifecycle, LifecycleTransition, RuntimeErrorReport, WindowError,
};
#[cfg(feature = "devtools")]
use crate::environment::DEV_TASK_COMPLETION;
#[cfg(feature = "devtools")]
use crate::environment::ReactiveRootId;
use crate::environment::{
    BuildScope, BuildScopeGuard, EnvironmentFields, InitialBuildDependencies, ReactiveQueue,
    environment_change_mask, install_focus_scope_watch,
};
use crate::focus_keys::{FocusKeyHandlers, FocusKeySubscription};
use crate::profiling;
use crate::profiling::FrameTimings;
#[cfg(not(target_arch = "wasm32"))]
use crate::tasks::TokioHandle;
use crate::tasks::{
    self, RuntimeDiagnostics, RuntimeSpawner, RuntimeWake, Task, TaskFailure, TaskHandle,
    TaskScope, UiDispatcher,
};
use crate::text_shortcuts::{TextIntent, TextShortcutPlatform};
use crate::undo::{UndoHistoryController, UndoHistoryState};
use crate::window_commands::{WindowHandle, WindowOpener};
use crate::window_state::WindowManager;
use incular_config::{Constraints, RuntimeEnvironment, TargetPlatform};
use incular_core::IdGenerator;
use incular_core::time::Instant;
use incular_core::{
    Code, ImeEvent, InputEvent, KeyboardEvent, Offset, PRIMARY_POINTER_BUTTON, PointerPhase,
    PointerSampleMetadata, Rect, Size,
};
use incular_platform::{
    BackGestureEvent, Clipboard, ClipboardCapabilities, ClipboardError, ClipboardWriteReport,
    DataTransfer, ExternalDragEvent, ExternalDragResponse, HoverDeviceChange, HoverDeviceEvent,
    MemoryClipboard, PlatformEvent, PlatformLifecycle, PlatformViewCommand, PlatformViewId,
    SystemUiOverlayStyle, TextInputAction, TextInputClientId, TextInputCommand,
    TextInputConfiguration, TextInputState, TextInputType, TransferReadRequest, UndoDirection,
    UndoManagerCommand, WindowId, WindowMetrics, WindowOperation, WindowOptions,
};
use incular_rendering::DisplayList;
use incular_semantics::{SemanticAction, SemanticNodeId};
#[cfg(feature = "devtools")]
use incular_widgets::internal::InvalidationCause;
use incular_widgets::internal::{
    ActionId, ButtonInk, Diagnostics, ElementId, MouseCursor, PlatformMenuBinding,
    PointerDeviceKind, PointerHitTest, RawPointerEvent, TextEditingController, TextRange,
    TextSelection, TreeError, WidgetTree, WindowInteraction,
};
use incular_widgets::{
    FocusHighlightManager, FocusHighlightMode, FocusNode, KeyEventResult, TextInputActionHint,
    Widget,
};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    rc::{Rc, Weak},
};

mod hover_devices;
mod system_chrome;
mod text_actions;
mod autofill;
mod text_gestures;

/// Sequence numbering runtime driver domains so outlet ownership errors
/// can name the runtimes involved.
static RUNTIME_SEQUENCE: IdGenerator = IdGenerator::new();

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    pub updated_elements: usize,
    pub rebuilt_elements: u64,
    pub laid_out_render_objects: u64,
    pub repainted_render_objects: u64,
    pub composited: u64,
    pub active_animations: u64,
    pub display_list_commands: usize,
    pub requested_another_frame: bool,
    /// Monotonic per-phase CPU durations. Phases never nest.
    pub timings: FrameTimings,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventTarget {
    pub element: ElementId,
    pub action: Option<ActionId>,
}
/// Debug-facing focus state without exposing native event-loop details.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FocusDiagnostics {
    pub focused_element: Option<ElementId>,
    pub text_pointer_capture: Option<ElementId>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditingDiagnostics {
    pub key_down_received: u64,
    pub key_up_received: u64,
    pub backspace_commands: u64,
    pub delete_commands: u64,
    pub text_commits: u64,
    pub ime_events: u64,
}

/// Open IME composition. Native IME events carry no session identity,
/// so the field and controller that received the latest Preedit are
/// the only routing authority for the matching Commit/End. The
/// controller handle is retained so cleanup always addresses the
/// composition's own editor, never the currently focused one.
///
/// Preedit → update → Commit → End: Commit retires the owner; End is ignored.
/// Preedit → End: cancellation; the owner's preedit clears, no text changes.
/// Preedit → focus moves: focus change cancels first; later events start fresh.
/// Preedit → controller replaced, field unmounted, or field no longer
///   editable: the owner stops validating and is cancelled, never committed.
/// Commit without Preedit: ownerless, applies to the focused field directly.
struct ImeCompositionOwner {
    field: ElementId,
    controller: TextEditingController,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum InitialFocus {
    AwaitingLayout,
    Settled,
}

/// Focus parked at the app root on desktop/web while inactive.
struct ParkedFocus {
    target: ElementId,
    node: Option<FocusNode>,
}

#[derive(Clone, Copy)]
struct MountedTextInputConnection {
    client: TextInputClientId,
    target: ElementId,
}

/// Retained application execution, input dispatch and frame coordination.
pub struct Runtime {
    pub(crate) tree: WidgetTree,
    system_sound: incular_platform::SystemSound,
    /// Input-dispatch time accumulated since the last frame consumed it.
    pub(crate) pending_event_processing_us: u32,
    /// The time stamped on input events, when a test drives the clock
    /// (Flutter's `TestPointer` time stamps); the wall clock otherwise.
    input_clock: Option<Instant>,
    pub(crate) pending: HashMap<ElementId, Widget>,
    pub(crate) order: VecDeque<ElementId>,
    pub(crate) reactive: Rc<RefCell<ReactiveQueue>>,
    pub(crate) builders: HashMap<ElementId, Box<dyn FnMut() -> Widget>>,
    pub(crate) handlers: HashMap<ActionId, Rc<dyn Fn()>>,
    /// Set when handlers were bound or elements unmounted, so the next
    /// prune scans the tree for live actions; otherwise none can be stale.
    handlers_stale: bool,
    pub(crate) legacy_action_handler: Option<Box<dyn FnMut(ActionId)>>,
    pub(crate) hovered_button: Option<ElementId>,
    pub(crate) pressed_button: Option<ElementId>,
    /// The contact currently allowed to drive single-contact controls. Other
    /// contacts still reach retained gesture regions for scale/pan input.
    pub(crate) legacy_pointer: Option<(u64, u64)>,
    pub(crate) last_pointer: Offset,
    hover_devices: hover_devices::HoverDevices,
    hover_hit_revision: u64,
    pub(crate) focused: Option<ElementId>,
    primary_focus_node: Option<FocusNode>,
    primary_focus_changes: incular_core::reactivity::DependencySource,
    parked_focus: Option<ParkedFocus>,
    focused_selection_lease: Option<incular_text::FocusedSelectionLease>,
    adopted_focus_epoch: Option<u64>,
    initial_focus: InitialFocus,
    pub(crate) captured_text_field: Option<ElementId>,
    /// An Android/iOS finger that may tap a different editable. Until a
    /// recognizer accepts it, it cannot move the active text input client.
    pending_mobile_text_touch: Option<text_gestures::PendingMobileTextTouch>,
    pub(crate) captured_selectable_text: Option<ElementId>,
    pub(crate) secondary_text_press: Option<text_gestures::SecondaryTextPress>,
    /// `SelectableRegionState._lastSecondaryTapDownPosition`.
    pub(crate) last_selectable_secondary: Option<Offset>,
    /// Consecutive primary taps on a text field (`TapAndPanGestureRecognizer`).
    pub(crate) text_taps: Option<text_gestures::TextTapRun>,
    /// A held text press that may become a long press (`kLongPressTimeout`).
    pub(crate) text_long_press: Option<text_gestures::TextLongPress>,
    /// A press on the focused field's selection handles, until it lifts.
    pub(crate) text_handle_drag: Option<text_gestures::TextHandleDrag>,
    /// Modifier keys held, from the latest key event (`HardwareKeyboard`).
    pub(crate) keyboard_modifiers: incular_core::Modifiers,
    hardware_keyboard: incular_core::HardwareKeyboardState,
    hardware_key_handlers: incular_core::HardwareKeyHandlers,
    early_focus_key_handlers: FocusKeyHandlers,
    late_focus_key_handlers: FocusKeyHandlers,
    keyboard_event_debug_listener: Option<Box<dyn Fn(&str)>>,
    /// The focused field, content revision and selection whose caret was
    /// last shown on screen (Flutter's `_scheduleShowCaretOnScreen`).
    pub(crate) caret_reveal_key: Option<(ElementId, u64, TextSelection)>,
    ime_composition: Option<ImeCompositionOwner>,
    pub(crate) text_histories: HashMap<ElementId, UndoHistoryController>,
    text_history_time: Option<Instant>,
    text_history_epoch: Option<(u64, u64)>,
    pub(crate) text_input_commands: VecDeque<TextInputCommand>,
    /// The style the app or an annotation asked for; `None` until one does.
    pub(crate) system_ui_overlay_style: Option<SystemUiOverlayStyle>,
    /// The style the host last received, so an unchanged style is not sent again.
    pub(crate) system_ui_overlay_style_applied: Option<SystemUiOverlayStyle>,
    /// Commands for the platform undo manager, drained by the host.
    pub(crate) undo_manager_commands: VecDeque<UndoManagerCommand>,
    /// The client field and availability last sent with `SetUndoState`.
    pub(crate) reported_undo_state: Option<(ElementId, UndoHistoryState)>,
    /// Mounted (or recently mounted) platform views and their last placement.
    pub(crate) platform_views: std::collections::BTreeMap<PlatformViewId, PlatformViewRecord>,
    pub(crate) platform_view_commands: VecDeque<PlatformViewCommand>,
    text_input_client: Option<MountedTextInputConnection>,
    last_text_input_client: Option<MountedTextInputConnection>,
    next_text_input_client: u64,
    pub(crate) text_input_configuration: Option<TextInputConfiguration>,
    pub(crate) text_input_state: Option<TextInputState>,
    pub(crate) text_input_caret_rect: Option<Rect>,
    /// A tap on the focused field asked for the keyboard (Flutter's
    /// `requestKeyboard`); the next client sync sends `Show`.
    pub(crate) text_input_show_requested: bool,
    pub(crate) clipboard: Box<dyn Clipboard>,
    pub(crate) editing_diagnostics: EditingDiagnostics,
    /// Which platform's `DefaultTextEditingShortcuts` table applies.
    pub(crate) text_shortcut_platform: TextShortcutPlatform,
    pub(crate) frame_requested: bool,
    warm_up_frame: Option<incular_animation::scheduler::WarmUpFrame>,
    pub(crate) scheduler: Rc<RefCell<tasks::TaskScheduler>>,
    pub(crate) environment: Rc<RefCell<RuntimeEnvironment>>,
    lifecycle: ApplicationLifecycle,
    frame_availability: incular_animation::scheduler::FrameAvailability,
    pub(crate) lifecycle_observers: Vec<Rc<dyn Fn(LifecycleTransition)>>,
    pub(crate) error_observers: Vec<Rc<dyn Fn(RuntimeErrorReport)>>,
    pub(crate) environment_generation: u64,
    pub(crate) environment_dependencies: Rc<Cell<EnvironmentFields>>,
    pub(crate) application_root: Option<ElementId>,
    pub(crate) owner_scopes: HashMap<ElementId, TaskScope>,
    pub(crate) window_id: Option<WindowId>,
    pub(crate) window_scope: TaskScope,
    pub(crate) window_manager: Option<WindowManager>,
    /// Stable driver identity shared with outlet ownership claims: the id
    /// names this runtime in [`OutletError`](super::route_outlet::OutletError)s,
    /// while the shared ownership doubles as a liveness token — dropping
    /// the runtime releases every claim without a global registry.
    driver_domain: Rc<u64>,
}

pub(crate) fn diagnostic_input_trigger(event: &InputEvent) -> String {
    match event {
        InputEvent::Pointer { phase, position } => {
            format!("pointer({phase:?}, {:.1}, {:.1})", position.x, position.y)
        }
        InputEvent::PointerWithId {
            pointer,
            phase,
            position,
        } => format!(
            "pointer({pointer}, {phase:?}, {:.1}, {:.1})",
            position.x, position.y
        ),
        InputEvent::PointerWithMetadata {
            pointer,
            device,
            kind,
            buttons,
            button,
            sample: _,
            phase,
            position,
        } => format!(
            "pointer({pointer}, device={device}, {kind:?}, buttons={buttons:#x}, changed={button:?}, {phase:?}, {:.1}, {:.1})",
            position.x, position.y
        ),
        InputEvent::TrackpadGesture(event) => format!(
            "trackpad(device={}, kind={:?}, phase={:?})",
            event.device(),
            event.kind(),
            event.phase()
        ),
        InputEvent::Scroll { delta } | InputEvent::ScrollAt { delta, .. } => {
            format!("scroll({:.1}, {:.1})", delta.x, delta.y)
        }
        InputEvent::Key(event) => format!("key({:?}, {:?})", event.state, event.code),
        InputEvent::Text(text) => format!("text_input(chars={})", text.chars().count()),
        InputEvent::Ime(ImeEvent::Preedit { text, .. }) => {
            format!("ime_preedit(chars={})", text.chars().count())
        }
        InputEvent::Ime(ImeEvent::Commit(text)) => {
            format!("ime_commit(chars={})", text.chars().count())
        }
        InputEvent::Ime(ImeEvent::End) => "ime_end".to_owned(),
        InputEvent::WindowResized { size, scale_factor } => format!(
            "window_resize({:.1}x{:.1}, scale={scale_factor:.2})",
            size.width, size.height
        ),
    }
}

fn kind_supports_hover(kind: PointerDeviceKind) -> bool {
    matches!(
        kind,
        PointerDeviceKind::Mouse
            | PointerDeviceKind::Trackpad
            | PointerDeviceKind::Stylus
            | PointerDeviceKind::InvertedStylus
    )
}

impl Runtime {
    pub fn new(root: Widget) -> Result<Self, TreeError> {
        Self::with_scheduler(root, tasks::TaskScheduler::new())
    }
    fn with_scheduler(
        root: Widget,
        scheduler: Rc<RefCell<tasks::TaskScheduler>>,
    ) -> Result<Self, TreeError> {
        let window_scope = tasks::TaskScheduler::spawner(&scheduler).scope();
        Self::with_window(root, scheduler, None, window_scope, None)
    }

    pub(crate) fn with_window(
        root: Widget,
        scheduler: Rc<RefCell<tasks::TaskScheduler>>,
        window_id: Option<WindowId>,
        window_scope: TaskScope,
        window_manager: Option<WindowManager>,
    ) -> Result<Self, TreeError> {
        Self::with_window_and_reactive(
            root,
            scheduler,
            window_id,
            window_scope,
            window_manager,
            ReactiveQueue::new(),
        )
    }
    pub(crate) fn with_window_and_reactive(
        mut root: Widget,
        scheduler: Rc<RefCell<tasks::TaskScheduler>>,
        window_id: Option<WindowId>,
        window_scope: TaskScope,
        window_manager: Option<WindowManager>,
        reactive: Rc<RefCell<ReactiveQueue>>,
    ) -> Result<Self, TreeError> {
        let lifecycle = window_manager
            .as_ref()
            .map_or(ApplicationLifecycle::Starting, |manager| {
                manager.application_lifecycle.get()
            });
        let mut frame_availability = incular_animation::scheduler::FrameAvailability::default();
        frame_availability.set_enabled(lifecycle.allows_frames());

        #[cfg(not(target_arch = "wasm32"))]
        let highlight_platform = TargetPlatform::host();
        #[cfg(target_arch = "wasm32")]
        let highlight_platform = match TextShortcutPlatform::current() {
            TextShortcutPlatform::Android => TargetPlatform::Android,
            TextShortcutPlatform::Ios => TargetPlatform::Ios,
            TextShortcutPlatform::Linux => TargetPlatform::Linux,
            TextShortcutPlatform::MacOs => TargetPlatform::MacOs,
            TextShortcutPlatform::Windows => TargetPlatform::Windows,
        };
        Self::set_focus_highlight_platform_default(highlight_platform);
        let mut tree = WidgetTree::new();
        let system_sound = incular_platform::SystemSound::default();
        tree.install_system_sound(system_sound.clone());
        tree.enable_hosted_focus();
        let mut handlers = HashMap::new();
        root.bind_callbacks(&mut |callback| {
            let action = tree.allocate_action();
            handlers.insert(action, callback);
            action
        });
        tree.mount(root)?;
        for (action, handler) in tree.take_pending_handlers() {
            handlers.insert(action, handler);
        }
        let mut runtime = Self {
            pending_event_processing_us: 0,
            input_clock: None,
            tree,
            system_sound,
            pending: HashMap::new(),
            order: VecDeque::new(),
            reactive,
            builders: HashMap::new(),
            handlers,
            handlers_stale: false,
            legacy_action_handler: None,
            hovered_button: None,
            pressed_button: None,
            legacy_pointer: None,
            last_pointer: Offset::ZERO,
            hover_devices: hover_devices::HoverDevices::default(),
            hover_hit_revision: 0,
            focused: None,
            primary_focus_node: None,
            primary_focus_changes: incular_core::reactivity::DependencySource::default(),
            parked_focus: None,
            focused_selection_lease: None,
            adopted_focus_epoch: None,
            initial_focus: InitialFocus::AwaitingLayout,
            captured_text_field: None,
            pending_mobile_text_touch: None,
            captured_selectable_text: None,
            secondary_text_press: None,
            last_selectable_secondary: None,
            text_taps: None,
            text_long_press: None,
            text_handle_drag: None,
            keyboard_modifiers: incular_core::Modifiers::empty(),
            hardware_keyboard: incular_core::HardwareKeyboardState::default(),
            hardware_key_handlers: incular_core::HardwareKeyHandlers::default(),
            early_focus_key_handlers: FocusKeyHandlers::default(),
            late_focus_key_handlers: FocusKeyHandlers::default(),
            keyboard_event_debug_listener: None,
            caret_reveal_key: None,
            ime_composition: None,
            text_histories: HashMap::new(),
            text_history_time: None,
            text_history_epoch: None,
            text_input_commands: VecDeque::new(),
            system_ui_overlay_style: None,
            system_ui_overlay_style_applied: None,
            undo_manager_commands: VecDeque::new(),
            reported_undo_state: None,
            platform_views: std::collections::BTreeMap::new(),
            platform_view_commands: VecDeque::new(),
            text_input_client: None,
            last_text_input_client: None,
            next_text_input_client: 1,
            text_input_configuration: None,
            text_input_state: None,
            text_input_caret_rect: None,
            text_input_show_requested: false,
            clipboard: Box::new(MemoryClipboard::default()),
            editing_diagnostics: EditingDiagnostics::default(),
            text_shortcut_platform: TextShortcutPlatform::current(),
            frame_requested: true,
            warm_up_frame: None,
            scheduler,
            environment: Rc::new(RefCell::new(RuntimeEnvironment::default())),
            lifecycle,
            frame_availability,
            lifecycle_observers: Vec::new(),
            error_observers: Vec::new(),
            environment_generation: 0,
            environment_dependencies: Rc::new(Cell::new(EnvironmentFields::empty())),
            application_root: None,
            owner_scopes: HashMap::new(),
            window_id,
            window_scope,
            window_manager,
            driver_domain: Rc::new(RUNTIME_SEQUENCE.next()),
        };
        // Focus nodes live outside the retained tree so they may be shared by
        // rebuildable descriptors. Resolve the first mounted autofocus listener
        // once the element IDs exist, then keep runtime and node focus mirrored
        // for subsequent keyboard traversal.
        if let Some(autofocus) = runtime.tree.autofocus_element() {
            runtime.set_focus_with_external_selection(Some(autofocus), false);
        }
        Ok(runtime)
    }
    /// Stable identity of this runtime's driver domain, for outlet
    /// ownership diagnostics: pairs with the outlet id to name both sides
    /// of a driver conflict.
    #[must_use]
    pub fn id(&self) -> u64 {
        *self.driver_domain
    }

    /// Liveness token for outlet driver claims. Dies with the runtime, so
    /// teardown releases outlet ownership without any registry.
    pub(crate) fn driver_token(&self) -> Weak<u64> {
        Rc::downgrade(&self.driver_domain)
    }

    /// Starts work owned by this retained root. Component code should prefer
    /// the build context's spawn API so completion lifetime follows its owner.
    #[must_use]
    pub fn spawner(&self) -> RuntimeSpawner {
        self.window_id.map_or_else(
            || tasks::TaskScheduler::spawner(&self.scheduler),
            |window_id| tasks::TaskScheduler::spawner_for(&self.scheduler, window_id),
        )
    }
    /// Sends a `Send` callback from any worker thread to the UI owner. The
    /// callback is never run concurrently with retained-tree mutation.
    #[must_use]
    pub fn dispatcher(&self) -> UiDispatcher {
        self.scheduler.borrow().dispatcher_for(self.window_id)
    }
    /// Returns the application's Tokio handle for advanced libraries. Direct
    /// Tokio tasks have Tokio/application lifetime and are not component-owned.
    #[must_use]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn tokio_handle(&self) -> TokioHandle {
        self.scheduler.borrow().tokio_handle()
    }
    pub fn set_wake_handler(&mut self, wake: std::sync::Arc<dyn RuntimeWake>) {
        crate::tasks::install_scheduled_task_wake(&wake);
        self.scheduler.borrow_mut().set_wake(wake);
    }

    /// Returns the owning normalized window when this runtime is managed by a
    /// multi-window application. Standalone runtimes remain windowless.
    #[must_use]
    pub const fn window_id(&self) -> Option<WindowId> {
        self.window_id
    }

    /// Returns default application-menu bridges retained by this window root.
    /// Native desktop adapters use these after native window creation; normal
    /// application code should use `PlatformMenuBar` instead.
    #[doc(hidden)]
    #[must_use]
    pub fn platform_menu_bindings(&self) -> Vec<PlatformMenuBinding> {
        self.tree.platform_menu_bindings()
    }

    /// Marks only this retained root for a future presentation.
    pub fn request_frame(&mut self) {
        self.frame_requested = true;
    }

    /// Drains native text-input commands produced since the last platform
    /// event-loop turn. A desktop runner applies these to Winit; mobile hosts
    /// can translate the same data to their platform text services.
    pub fn take_text_input_commands(&mut self) -> Vec<TextInputCommand> {
        self.collect_autofill_context_completions();
        self.text_input_commands.drain(..).collect()
    }

    fn collect_autofill_context_completions(&mut self) {
        self.text_input_commands.extend(
            self.tree.take_autofill_context_completions().into_iter()
                .map(|action| TextInputCommand::FinishAutofillContext { action }),
        );
    }

    /// Drains the platform view commands for this window: each mounted
    /// view's controller commands (creation, forwarded pointers, focus,
    /// disposal) and the placements that changed in the last frame. A host
    /// that cannot show native views drops them; see
    /// [`incular_platform::PlatformViewCompositing`].
    pub fn take_platform_view_commands(&mut self) -> Vec<PlatformViewCommand> {
        self.collect_platform_view_commands();
        self.platform_views
            .retain(|_, record| record.mounted || !record.controller.is_disposed());
        self.platform_view_commands.drain(..).collect()
    }

    /// Drains the platform undo manager commands produced since the last
    /// platform event-loop turn (Flutter's `UndoManager` channel). A host
    /// applies them to the system undo state, or drops them when the system
    /// has none.
    pub fn take_undo_manager_commands(&mut self) -> Vec<UndoManagerCommand> {
        self.undo_manager_commands.drain(..).collect()
    }

    /// Opens a second retained root through the owning application. This can
    /// be called from a build or a UI completion; native creation is queued
    /// for the desktop event-loop callback.
    pub fn open_window(
        &mut self,
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

    /// Returns a command handle for this retained root's owning native window.
    #[must_use]
    pub fn window_handle(&self) -> Option<WindowHandle> {
        let id = self.window_id?;
        self.window_manager.as_ref()?.handle(id)
    }

    /// Returns the scope cancelled as soon as this window closes. Child
    /// component scopes inherit from it automatically.
    #[must_use]
    pub fn window_task_scope(&self) -> TaskScope {
        self.window_scope.clone()
    }

    /// Starts bounded blocking work. Its completion callback runs later on the
    /// UI thread, so it may safely update a local `Signal` or schedule a build.
    pub fn spawn_blocking<T>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
        complete: impl FnOnce(Result<T, TaskFailure>, &mut Runtime) + 'static,
    ) -> TaskHandle
    where
        T: Send + 'static,
    {
        self.spawner()
            .spawn_blocking_in(&self.window_scope, work, complete)
    }
    /// Starts Tokio work cancelled with this window. Use a `BuildContext` for
    /// work whose completion must be tied to a mounted declarative owner.
    pub fn spawn<F, T>(&self, future: F) -> Task<T>
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.spawner().spawn_in(&self.window_scope, future)
    }
    /// Starts window-scoped Tokio work with a UI-thread completion.
    pub fn spawn_into<F, T>(
        &self,
        future: F,
        complete: impl FnOnce(Result<T, TaskFailure>, &mut Runtime) + 'static,
    ) -> TaskHandle
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.spawner()
            .spawn_into_in(&self.window_scope, future, complete)
    }
    /// Processes a bounded batch of UI-relevant Tokio completion messages and
    /// dispatch callbacks. Native runners call this only from their user-event
    /// path; it never polls Tokio or implies a redraw.
    pub fn process_runtime_work_at(&mut self, _now: Instant) {
        let scheduler = self.scheduler.clone();
        // Take message-owned work while borrowing the local scheduler, then
        // release that borrow before any UI callback runs. A completion may
        // safely start its next Tokio task without re-entering `RefCell`.
        let work = scheduler.borrow_mut().take_turn();
        for work in work {
            self.process_ui_work(&scheduler, work);
        }
        // A UI completion may also complete a value a widget build reads,
        // such as a `UiFuture` behind a `FutureBuilder`.
        if self.reactive.borrow().has_work()
            || !self.pending.is_empty()
            || self.tree.has_dirty_dependencies()
        {
            self.frame_requested = true;
        }
    }

    pub(crate) fn process_ui_work(
        &mut self,
        scheduler: &Rc<RefCell<tasks::TaskScheduler>>,
        work: tasks::UiWork,
    ) {
        let mut failures = Vec::new();
        match work {
            tasks::UiWork::Dispatch { callback, .. } => {
                if scheduler.borrow().accepting()
                    && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(self)))
                        .is_err()
                {
                    failures.push(TaskFailure::Panicked);
                }
            }
            tasks::UiWork::Task(pending) => {
                scheduler.borrow().detach_scope(&pending.control);
                let stale_owner = pending.control.owner_is_stale(self);
                if !scheduler.borrow().accepting() || pending.control.is_cancelled() || stale_owner
                {
                    (pending.discard)();
                    scheduler
                        .borrow_mut()
                        .record_discard(pending.blocking, stale_owner);
                } else {
                    #[cfg(feature = "devtools")]
                    let previous_task_context = DEV_TASK_COMPLETION.with(|context| {
                        let previous = context.get();
                        context.set(true);
                        previous
                    });
                    let failure = (pending.finish)(self);
                    #[cfg(feature = "devtools")]
                    DEV_TASK_COMPLETION.with(|context| context.set(previous_task_context));
                    if let Some(failure) = scheduler
                        .borrow_mut()
                        .record_completion(pending.blocking, failure)
                    {
                        failures.push(failure);
                    }
                }
            }
        }
        for failure in failures {
            if self.error_observers.is_empty() {
                // Flutter's default FlutterError.onError (presentError) reports
                // the error instead of dropping it; registered observers replace
                // that default.
                eprintln!("Incular runtime error: {failure:?}");
            }
            for observer in &self.error_observers {
                observer(RuntimeErrorReport {
                    failure: failure.clone(),
                });
            }
        }
        // A UI completion may also complete a value a widget build reads,
        // such as a `UiFuture` behind a `FutureBuilder`.
        if self.reactive.borrow().has_work()
            || !self.pending.is_empty()
            || self.tree.has_dirty_dependencies()
        {
            self.frame_requested = true;
        }
    }
    pub fn process_runtime_work(&mut self) {
        self.process_runtime_work_at(Instant::now());
    }
    #[must_use]
    pub fn runtime_diagnostics(&self) -> RuntimeDiagnostics {
        self.scheduler.borrow().diagnostics()
    }
    /// Compact ownership-facing state for framework diagnostics. Tokio's own
    /// tracing ecosystem remains the source of executor-level telemetry.
    #[must_use]
    pub fn debug_dump(&self) -> String {
        let diagnostics = self.runtime_diagnostics();
        let (application_tasks, owner_tasks) = self.scheduler.borrow().active_ownership();
        format!(
            "Lifecycle: {:?}\nTokio tasks tracked: {}\n  application: {}\n  owner-scoped: {}\nPending UI messages: {}\nEnvironment generation: {}",
            self.lifecycle,
            diagnostics.active_tracked_tasks,
            application_tasks,
            owner_tasks,
            diagnostics
                .ui_messages_enqueued
                .saturating_sub(diagnostics.ui_messages_processed),
            self.environment_generation,
        )
    }
    #[must_use]
    pub fn environment(&self) -> RuntimeEnvironment {
        self.environment.borrow().clone()
    }
    #[must_use]
    pub const fn lifecycle(&self) -> ApplicationLifecycle {
        self.lifecycle
    }
    #[must_use]
    pub const fn environment_generation(&self) -> u64 {
        self.environment_generation
    }
    /// Replaces normalized runtime values from a platform event. Rebuild is
    /// requested only when the declarative root read a changed value.
    pub fn set_environment(&mut self, environment: RuntimeEnvironment) -> bool {
        let environment = environment.normalized();
        let previous = self.environment.borrow().clone();
        if previous == environment {
            return false;
        }
        let changed = environment_change_mask(&previous, &environment);
        *self.environment.borrow_mut() = environment.clone();
        // WidgetTree owns renderer-neutral runtime geometry and direction used
        // by retained policies such as SafeArea and transient placement. Keep
        // that snapshot synchronized even when no declarative environment
        // dependency requires rebuilding the application root.
        let retained_environment_dirty = self.tree.set_environment(environment);
        self.environment_generation = self
            .environment_generation
            .checked_add(1)
            .expect("runtime environment generation exhausted");
        if retained_environment_dirty {
            self.frame_requested = true;
        }
        if self.environment_dependencies.get().intersects(changed)
            && let Some(root) = self.application_root
        {
            #[cfg(feature = "devtools")]
            self.tree
                .note_invalidation(root, InvalidationCause::EnvironmentChanged);
            let _ = self.rebuild_from_builder(root);
            self.frame_requested = true;
        }
        true
    }
    /// Applies one literal lifecycle transition. Host events go through
    /// the ordered platform projection in [`Self::handle_platform_event`].
    pub fn transition_lifecycle(&mut self, lifecycle: ApplicationLifecycle) -> bool {
        if self.lifecycle == lifecycle {
            return false;
        }
        let transition = LifecycleTransition {
            previous: self.lifecycle,
            current: lifecycle,
        };
        self.lifecycle = lifecycle;
        self.frame_availability
            .set_enabled(lifecycle.allows_frames());
        self.update_focus_for_lifecycle(lifecycle);
        for observer in &self.lifecycle_observers {
            observer(transition);
        }
        if matches!(
            lifecycle,
            ApplicationLifecycle::Stopping | ApplicationLifecycle::Terminated
        ) {
            self.reactive.borrow_mut().clear();
            self.scheduler.borrow_mut().shutdown();
        }
        true
    }
    fn handle_platform_lifecycle(&mut self, lifecycle: PlatformLifecycle) {
        use ApplicationLifecycle::{Active, Inactive, Starting, Suspended};

        let target = ApplicationLifecycle::from(lifecycle);
        // Flutter traverses intermediate states when its host skips one.
        // Project those edges on states Incular actually exposes; Hidden
        // and reversible Detached are not aliases for terminal states.
        if self.lifecycle == Starting && matches!(target, Inactive | Suspended) {
            self.transition_lifecycle(Active);
        }
        if matches!(
            (self.lifecycle, target),
            (Active, Suspended) | (Suspended, Active)
        ) {
            self.transition_lifecycle(Inactive);
        }
        self.transition_lifecycle(target);
    }

    fn update_focus_for_lifecycle(&mut self, lifecycle: ApplicationLifecycle) {
        match lifecycle {
            ApplicationLifecycle::Inactive | ApplicationLifecycle::Suspended
                if cfg!(target_arch = "wasm32")
                    || !matches!(
                        self.text_shortcut_platform,
                        TextShortcutPlatform::Android | TextShortcutPlatform::Ios
                    ) =>
            {
                if let Some(target) = self.focused {
                    self.parked_focus = Some(ParkedFocus {
                        target,
                        node: self.tree.focus_node_at(target),
                    });
                    // Parking replaces a request already queued at the time of
                    // deactivation. Requests made after that boundary may win.
                    self.tree.clear_hosted_focus_requests();
                    self.set_focus(None);
                }
            }
            ApplicationLifecycle::Active => {
                if let Some(parked) = self.parked_focus.take()
                    && self.focused.is_none()
                {
                    self.adopt_external_focus_change();
                    if self.focused.is_none()
                        && self.tree.can_request_focus(parked.target)
                        && self.tree.focus_node_at(parked.target) == parked.node
                    {
                        // Restoring the same editor is not traversal; preserve
                        // its caret or selected range instead of selecting all.
                        self.set_focus_with_external_selection(Some(parked.target), false);
                    }
                }
            }
            ApplicationLifecycle::Stopping | ApplicationLifecycle::Terminated => {
                self.parked_focus = None;
            }
            _ => {}
        }
    }

    pub fn observe_lifecycle(&mut self, observer: impl Fn(LifecycleTransition) + 'static) {
        self.lifecycle_observers.push(Rc::new(observer));
    }
    pub fn observe_runtime_error(&mut self, observer: impl Fn(RuntimeErrorReport) + 'static) {
        self.error_observers.push(Rc::new(observer));
    }
    /// Cancels runtime-owned work and prevents later queued callbacks from
    /// touching retained state. Native runners should call this before their
    /// renderer/window teardown.
    pub fn shutdown(&mut self) {
        self.system_sound.unbind_host();
        if self.focused.is_some() {
            self.set_focus(None);
        } else {
            self.cancel_ime_composition();
            self.sync_text_input_client();
        }
        self.tree.cancel_all_gesture_streams();
        self.captured_text_field = None;
        self.captured_selectable_text = None;
        self.hovered_button = None;
        self.pressed_button = None;
        self.legacy_pointer = None;
        self.text_histories.clear();
        self.text_history_epoch = None;
        self.tree.release_edit_subscriptions();
        let _ = self.transition_lifecycle(ApplicationLifecycle::Stopping);
        for (_, scope) in self.owner_scopes.drain() {
            scope.cancel();
        }
        self.scheduler.borrow_mut().shutdown();
        self.lifecycle = ApplicationLifecycle::Terminated;
    }

    /// Releases window-owned subscriptions and callback state without
    /// stopping the shared application scheduler. `Application` calls this
    /// before dropping a closed retained root.
    pub(crate) fn dispose_window(&mut self) {
        self.system_sound.unbind_host();
        if self.focused.is_some() {
            self.set_focus(None);
        } else {
            self.cancel_ime_composition();
            self.sync_text_input_client();
        }
        self.tree.cancel_all_gesture_streams();
        self.tree.release_edit_subscriptions();
        self.legacy_pointer = None;
        self.window_scope.cancel();
        for (_, scope) in self.owner_scopes.drain() {
            scope.cancel();
        }
        self.reactive.borrow_mut().clear();
        self.builders.clear();
        self.pending.clear();
        self.order.clear();
        self.handlers.clear();
        self.legacy_action_handler = None;
        self.text_histories.clear();
        self.text_history_epoch = None;
        self.text_input_commands.clear();
        self.undo_manager_commands.clear();
        for record in self.platform_views.values() {
            record.controller.dispose();
        }
        self.platform_views.clear();
        self.platform_view_commands.clear();
        self.reported_undo_state = None;
        self.text_input_client = None;
        self.last_text_input_client = None;
        self.text_input_configuration = None;
        self.text_input_state = None;
        self.text_input_caret_rect = None;
        self.captured_text_field = None;
        self.captured_selectable_text = None;
        self.hovered_button = None;
        self.pressed_button = None;
    }
    /// System sound requests for this runtime's window.
    #[must_use]
    pub fn system_sound(&self) -> incular_platform::SystemSound {
        self.system_sound.clone()
    }

    #[must_use]
    pub fn tree(&self) -> &WidgetTree {
        &self.tree
    }

    #[cfg(feature = "devtools")]
    pub(crate) fn devtools_reactive_root(&self) -> ReactiveRootId {
        self.reactive.borrow().root
    }
    #[must_use]
    pub fn tree_mut(&mut self) -> &mut WidgetTree {
        &mut self.tree
    }

    /// Registers an embedder's default spelling service for editors that
    /// opt in without supplying their own service.
    pub fn set_default_spell_check_provider(
        &mut self,
        provider: Option<incular_widgets::DefaultSpellCheckProvider>,
    ) {
        self.tree.set_default_spell_check_provider(provider);
        self.frame_requested = true;
    }

    /// Returns the effective capture-protection policy contributed by the
    /// mounted retained tree. The window/application boundary is responsible
    /// for forwarding changes to its native adapter.
    #[must_use]
    pub fn content_sensitivity(&self) -> incular_config::ContentSensitivity {
        self.tree.content_sensitivity()
    }

    /// Visible transient portals after the latest layout. Desktop adapters use
    /// this retained snapshot to bind popup presentation to the owning window
    /// without coupling controls to a native event loop.
    #[must_use]
    pub fn transient_surfaces(&self) -> Vec<incular_widgets::TransientSurfaceSnapshot> {
        self.tree.transient_surfaces()
    }

    /// Updates the parent-local logical rectangle that confirmed native
    /// transient hosts may use for collision placement.
    pub fn set_native_transient_bounds(&mut self, bounds: Option<incular_core::Rect>) -> bool {
        let changed = self.tree.set_native_transient_bounds(bounds);
        self.frame_requested |= changed;
        changed
    }

    /// Updates the set of retained transients whose native hosts have completed
    /// negotiation. A presentation change requests a frame so placement,
    /// semantics, hit testing, and rendering switch coordinate spaces together.
    pub fn set_native_transient_presentations(
        &mut self,
        ids: impl IntoIterator<Item = incular_widgets::TransientSurfaceId>,
    ) -> bool {
        let changed = self.tree.set_native_transient_presentations(ids);
        self.frame_requested |= changed;
        changed
    }

    /// Applies a framework-level transient dismissal cause without changing
    /// native focus ownership. Controllers receive the callback and the normal
    /// retained rebuild path removes their popup subtree.
    pub fn dismiss_transients(&mut self, reason: incular_widgets::TransientDismissReason) -> bool {
        let dismissed = self.tree.dismiss_transients(reason) > 0;
        self.frame_requested |= dismissed;
        dismissed
    }
    #[cfg(feature = "devtools")]
    pub(crate) fn devtools_edit_property(
        &mut self,
        id: incular_devtools_protocol::DevWidgetId,
        name: &str,
        value: &incular_devtools_protocol::DebugValue,
    ) -> bool {
        let edit = self.tree.devtools_edit_property(id, name, value);
        self.frame_requested |= edit == incular_widgets::devtools::DevEdit::Applied;
        edit != incular_widgets::devtools::DevEdit::Rejected
    }
    #[cfg(feature = "devtools")]
    pub(crate) fn devtools_revert_property(
        &mut self,
        id: incular_devtools_protocol::DevWidgetId,
        name: &str,
        original: &incular_devtools_protocol::DebugValue,
    ) -> bool {
        let edit = self.tree.devtools_revert_property(id, name, original);
        self.frame_requested |= edit == incular_widgets::devtools::DevEdit::Applied;
        edit != incular_widgets::devtools::DevEdit::Rejected
    }
    /// Sets the animation-only time scale for this retained root. Runtime
    /// messages, Tokio tasks and profiler wall-clock measurements remain on
    /// the platform monotonic clock.
    pub fn set_animation_time_scale(&mut self, scale: f32) {
        self.tree.set_animation_time_scale(scale);
    }
    #[must_use]
    pub fn focused_element(&self) -> Option<ElementId> {
        self.focused
    }
    /// Observes changes of this window's primary focus, after focus state
    /// and the text input client have both been updated. Retain the token
    /// for as long as the callback should remain registered.
    #[must_use]
    pub fn observe_primary_focus(
        &self,
        callback: impl Fn() + 'static,
    ) -> incular_core::reactivity::Subscription {
        self.primary_focus_changes.subscribe((), callback)
    }

    /// Dispatches an owned semantic action through the same control state used
    /// by pointer and keyboard input. Native adapters queue these requests;
    /// they never borrow mutable element storage. Eligibility reuses the
    /// existing owners: explicit semantic callbacks always execute;
    /// built-in Activate additionally requires the advertisement predicate
    /// (enabled button with a bound callback), matching the pointer and
    /// keyboard paths, which never see a disabled button past their own
    /// enabled gate. SetText requires an editable field while SetSelection
    /// requires only a controller — selection stays available in read-only
    /// and disabled fields on every input path. Focus accepts only targets
    /// the tree's own focusability owner admits (`can_request_focus`:
    /// enabled controls, selectable text, or a focus node, unobstructed by
    /// `exclude_focus` or a hidden IndexedStack slot), with an
    /// already-focused valid target still reporting success; `set_focus`
    /// remains the single focus owner for all paths.
    pub fn dispatch_semantic_action(
        &mut self,
        node: SemanticNodeId,
        action: SemanticAction,
    ) -> bool {
        let Some(element) = self.tree.element_for_semantic_node(node) else {
            let handled = action == SemanticAction::ShowOnScreen
                && self.tree.semantic_show_table_row_on_screen(node);
            if handled {
                self.tree.note_semantic_action();
                self.frame_requested = true;
            }
            return handled;
        };
        // A native adapter may still hold a node ID announced before the
        // mask changed. Reject that request without invoking callbacks or
        // synthesizing a TapRegion event.
        let kind = action.kind();
        if !kind.is_accessibility_focus_notification() && self.tree.pointer_actions_blocked(element) {
            return false;
        }
        if matches!(action, SemanticAction::Activate | SemanticAction::LongPress)
            && let Some(bounds) = self.tree.semantics().node(node).map(|node| node.bounds)
        {
            let notified = self
                .tree
                .notify_tap_regions_of_semantic_action(bounds, self.input_now());
            self.frame_requested |= notified;
        }
        self.tree.note_semantic_action();
        if matches!(
            action,
            SemanticAction::Focus
                | SemanticAction::DidGainAccessibilityFocus
                | SemanticAction::DidLoseAccessibilityFocus
                | SemanticAction::SetText(_)
                | SemanticAction::SetSelection { .. }
                | SemanticAction::Copy
                | SemanticAction::Cut
                | SemanticAction::Paste
                | SemanticAction::Expand
                | SemanticAction::Collapse
                | SemanticAction::MoveCursorForwardByCharacter { .. }
                | SemanticAction::MoveCursorBackwardByCharacter { .. }
        ) {
            if let Some(callback) = self.tree.semantic_bound_callback(element, kind) {
                if !self
                    .tree
                    .semantics()
                    .node(node)
                    .is_some_and(|node| node.actions.contains(&kind))
                {
                    return false;
                }
                callback(action);
                self.frame_requested = true;
                return true;
            }
        }
        let handled = match action {
            SemanticAction::Focus => {
                let blocked =
                    self.tree.semantics().node(node).is_some_and(|node| {
                        node.state.accessibility_focus_block_type.blocks_focus()
                            || node.state.declared_input_focusable() == Some(false)
                    });
                if !blocked && self.tree.can_dispatch_semantic_focus(element) {
                    self.set_focus(Some(element));
                    true
                } else {
                    false
                }
            }
            SemanticAction::Activate => {
                self.tree.semantic_activation_target(element).is_some_and(|target| {
                    if let Some(callback) = self.tree.semantic_action_callback(
                        target,
                        incular_semantics::SemanticActionKind::Activate,
                    ) {
                        callback();
                        true
                    } else {
                        self.tree
                            .action_for_element(target)
                            .is_some_and(|action| self.activate_action(action))
                    }
                })
            }
            SemanticAction::Dismiss => self
                .tree
                .semantic_action_callback(element, incular_semantics::SemanticActionKind::Dismiss)
                .is_some_and(|callback| {
                    callback();
                    true
                }),
            SemanticAction::SetText(text) => {
                if !self.tree.text_field_is_editable(element) {
                    false
                } else {
                    self.tree
                        .text_controller(element)
                        .map(|controller| {
                            // Flutter's SetText handler is a user edit
                            // (RenderEditable._handleSetText calls
                            // userUpdateTextEditingValue), so input formatters
                            // and onChanged run. Programmatic set_text does not.
                            let length = controller.text().len();
                            let end = text.len();
                            controller.apply_delta(&incular_text::TextEditingDelta {
                                range: TextRange::new(0, length),
                                replacement: text,
                                selection: Some(TextSelection::collapsed(end)),
                                composing: None,
                            });
                            true
                        })
                        .unwrap_or(false)
                }
            }
            SemanticAction::SetSelection { base, extent } => self
                .tree
                .text_controller(element)
                .map(|controller| {
                    let length = controller.text().len();
                    controller.set_selection(TextSelection {
                        base: base.min(length),
                        extent: extent.min(length),
                    });
                    true
                })
                .unwrap_or(false),
            SemanticAction::ScrollToOffset { x, y } => {
                self.tree.semantic_scroll_to_offset(element, x, y)
            }
            SemanticAction::ScrollForward => self
                .tree
                .semantic_action_callback(
                    element,
                    incular_semantics::SemanticActionKind::ScrollForward,
                )
                .map(|callback| {
                    callback();
                    true
                })
                .unwrap_or_else(|| self.tree.semantic_scroll(element, true)),
            SemanticAction::ScrollBackward => self
                .tree
                .semantic_action_callback(
                    element,
                    incular_semantics::SemanticActionKind::ScrollBackward,
                )
                .map(|callback| {
                    callback();
                    true
                })
                .unwrap_or_else(|| self.tree.semantic_scroll(element, false)),
            direction @ (SemanticAction::ScrollLeft
            | SemanticAction::ScrollRight
            | SemanticAction::ScrollUp
            | SemanticAction::ScrollDown) => self
                .tree
                .semantic_action_callback(element, direction.kind())
                .map(|callback| {
                    callback();
                    true
                })
                .unwrap_or_else(|| {
                    self.tree
                        .semantics()
                        .node(node)
                        .is_some_and(|semantic| semantic.actions.contains(&direction.kind()))
                        && self
                            .tree
                            .semantic_scroll_direction(element, direction.kind())
                }),
            SemanticAction::Increment => self
                .tree
                .semantic_action_callback(element, incular_semantics::SemanticActionKind::Increment)
                .map(|callback| {
                    callback();
                    true
                })
                .unwrap_or_else(|| self.tree.dispatch_semantic_increment(element, true)),
            SemanticAction::Decrement => self
                .tree
                .semantic_action_callback(element, incular_semantics::SemanticActionKind::Decrement)
                .map(|callback| {
                    callback();
                    true
                })
                .unwrap_or_else(|| self.tree.dispatch_semantic_increment(element, false)),
            SemanticAction::LongPress => self
                .tree
                .semantic_action_callback(element, incular_semantics::SemanticActionKind::LongPress)
                .map(|callback| {
                    callback();
                    true
                })
                .unwrap_or(false),
            SemanticAction::Copy
            | SemanticAction::Cut
            | SemanticAction::Paste
            | SemanticAction::Expand
            | SemanticAction::Collapse
            | SemanticAction::MoveCursorForwardByCharacter { .. }
            | SemanticAction::MoveCursorBackwardByCharacter { .. }
            | SemanticAction::DidGainAccessibilityFocus
            | SemanticAction::DidLoseAccessibilityFocus => false,
            SemanticAction::ShowOnScreen => self.tree.semantic_show_on_screen(element),
            SemanticAction::Custom(id) => self
                .tree
                .custom_semantic_action_callback(element, id)
                .is_some_and(|callback| {
                    callback();
                    true
                }),
        };
        if handled {
            self.frame_requested = true;
        }
        handled
    }
    /// Dispatches an action to the retained semantic target at a window
    /// position. Positions are logical pixels, like other Runtime input.
    /// A position with no node supporting the action is left untouched.
    pub fn dispatch_semantic_action_at(
        &mut self,
        position: Offset,
        action: SemanticAction,
    ) -> bool {
        self.tree
            .semantic_action_target_at(position, &action)
            .is_some_and(|node| self.dispatch_semantic_action(node, action))
    }

    #[must_use]
    pub fn focus_diagnostics(&self) -> FocusDiagnostics {
        FocusDiagnostics {
            focused_element: self.focused,
            text_pointer_capture: self.captured_text_field,
        }
    }
    #[must_use]
    pub const fn editing_diagnostics(&self) -> EditingDiagnostics {
        self.editing_diagnostics
    }
    /// Chooses the platform for native text-editing and focus shortcuts
    /// (Flutter's `defaultTargetPlatform`): Option for words and Command for
    /// lines on macOS and iOS, Control for words elsewhere. Defaults to the
    /// platform this build targets.
    pub fn set_text_shortcut_platform(&mut self, platform: TextShortcutPlatform) {
        self.text_shortcut_platform = platform;
        self.tree
            .set_focus_semantic_actions_enabled(platform != TextShortcutPlatform::Ios);
        self.sync_focused_selection_lease();
    }

    /// Overrides the initial focus-ring guess for this UI thread before
    /// real input. Desktop targets start traditional; mobile starts touch
    /// unless a mouse is present.
    pub fn set_focus_highlight_platform_default(platform: TargetPlatform) {
        let mode = if platform.is_desktop() {
            FocusHighlightMode::Traditional
        } else {
            FocusHighlightMode::Touch
        };
        FocusHighlightManager::new().set_platform_default_mode(mode);
    }

    /// The platform whose text-editing shortcuts are active.
    #[must_use]
    pub fn text_shortcut_platform(&self) -> TextShortcutPlatform {
        self.text_shortcut_platform
    }

    pub fn set_clipboard(&mut self, clipboard: Box<dyn Clipboard>) {
        self.clipboard = clipboard;
    }
    /// Returns the capabilities of the clipboard backend currently installed
    /// for this retained window.
    #[must_use]
    pub fn clipboard_capabilities(&self) -> ClipboardCapabilities {
        self.clipboard.capabilities()
    }
    /// Reads rich clipboard data without collapsing it to plain text.
    pub fn read_clipboard(
        &mut self,
        request: TransferReadRequest,
    ) -> Result<DataTransfer, ClipboardError> {
        self.clipboard.read(request)
    }
    /// Writes one rich clipboard transfer. The report distinguishes formats
    /// written atomically by the backend from representations it skipped.
    pub fn write_clipboard(
        &mut self,
        transfer: DataTransfer,
    ) -> Result<ClipboardWriteReport, ClipboardError> {
        self.clipboard.write(transfer)
    }
    pub fn set_clipboard_text(&mut self, text: impl Into<String>) {
        self.clipboard.set_text(text.into());
    }
    /// Routes platform-owned transfer data through external drop targets. This
    /// path is intentionally independent of the local typed gesture arena.
    #[must_use]
    pub fn handle_external_drag(&mut self, event: ExternalDragEvent) -> ExternalDragResponse {
        let response = self.tree.dispatch_external_drag(event);
        self.frame_requested = true;
        response
    }
    /// Processes a normalized platform event without exposing native window or
    /// event-loop types to the runtime API.
    pub fn handle_platform_event(&mut self, event: PlatformEvent) -> Option<EventTarget> {
        match event {
            PlatformEvent::Input(input) => self.handle_input(input),
            PlatformEvent::InputAt { input, time } => self.handle_input_at(input, time),
            PlatformEvent::HoverDevice(event) => {
                self.handle_hover_device_event(event);
                None
            }
            PlatformEvent::TextInputAction(action) => {
                self.handle_text_input_action(action);
                None
            }
            PlatformEvent::TextEditingState(state) => {
                self.handle_text_editing_state(state);
                None
            }
            PlatformEvent::TextInputConnectionClosed(client) => {
                self.text_input_connection_closed(client);
                None
            }
            PlatformEvent::RequestExistingTextInputState => {
                if self.text_input_client.is_some() {
                    self.sync_text_input_client_for_host(true);
                }
                None
            }
            PlatformEvent::Metrics(metrics) => {
                self.update_window_metrics(metrics);
                None
            }
            PlatformEvent::Environment(environment) => {
                self.set_environment(environment);
                None
            }
            PlatformEvent::Lifecycle(lifecycle) => {
                self.handle_platform_lifecycle(lifecycle);
                None
            }
            PlatformEvent::ExternalDrag(event) => {
                let _ = self.handle_external_drag(event);
                None
            }
            PlatformEvent::Undo(direction) => {
                self.handle_platform_undo(direction);
                None
            }
            PlatformEvent::BackGesture(event) => {
                self.handle_back_gesture(event);
                None
            }
            PlatformEvent::CloseRequested => {
                self.shutdown();
                None
            }
        }
    }
    /// Applies the authoritative logical/physical conversion supplied by the
    /// platform metrics owner. Existing environment values such as locale are
    /// preserved while size and scale change together.
    pub fn update_window_metrics(&mut self, metrics: WindowMetrics) -> bool {
        let mut environment = self.environment();
        environment.viewport = metrics.logical_size();
        environment.physical_width = metrics.physical_size.width;
        environment.physical_height = metrics.physical_size.height;
        environment.scale_factor = metrics.scale_factor;
        self.set_environment(environment)
    }
    pub fn register_builder(
        &mut self,
        id: ElementId,
        builder: impl FnMut() -> Widget + 'static,
    ) -> Result<(), TreeError> {
        self.register_builder_without_rebuild(id, builder)?;
        self.rebuild_from_builder(id)
    }
    pub(crate) fn register_builder_without_rebuild(
        &mut self,
        id: ElementId,
        builder: impl FnMut() -> Widget + 'static,
    ) -> Result<(), TreeError> {
        if !self.tree.element_exists(id) {
            return Err(TreeError::MissingElement(id));
        }
        self.builders.insert(id, Box::new(builder));
        Ok(())
    }
    pub(crate) fn install_initial_dependencies(
        &mut self,
        id: ElementId,
        initial_dependencies: Rc<RefCell<InitialBuildDependencies>>,
    ) {
        let initial_dependencies = std::mem::take(&mut *initial_dependencies.borrow_mut());
        let queue = self.reactive.clone();
        let root = queue.borrow().root;
        let queue_weak = Rc::downgrade(&queue);
        for dependency in initial_dependencies.signals {
            let tracked =
                crate::reactive::subscribe_element(dependency, root, id, queue_weak.clone());
            queue.borrow_mut().record(id, tracked);
        }
        for scope in initial_dependencies.focus_scopes {
            install_focus_scope_watch(&queue, id, &scope);
        }
    }
    pub fn schedule_update(&mut self, id: ElementId, widget: Widget) -> Result<(), TreeError> {
        if !self.tree.element_exists(id) {
            return Err(TreeError::MissingElement(id));
        }
        #[cfg(feature = "devtools")]
        self.tree
            .note_invalidation(id, InvalidationCause::WidgetConfigurationChanged);
        self.tree.mark_build(id)?;
        let mut widget = widget;
        self.prepare_widget(&mut widget);
        if self.pending.insert(id, widget).is_none() {
            self.order.push_back(id);
        }
        self.frame_requested = true;
        Ok(())
    }
    #[must_use]
    pub fn frame_requested(&self) -> bool {
        if self.warm_up_frame.is_some() {
            return true;
        }
        // A warm-up owned elsewhere on the UI thread must complete before
        // this runtime can admit a redraw. Keep demand, but do not make
        // a native host spin requesting the same blocked event.
        if incular_animation::scheduler::warm_up_frame_pending() {
            return false;
        }
        self.frame_requested
            || self.reactive.borrow().has_work()
            || self.tree.has_dirty_dependencies()
            || self.tree.has_pending_application_fonts()
            || self.tree.has_pending_scroll_animations()
            || self.next_frame_deadline().is_some_and(|deadline| deadline <= Instant::now())
            || incular_animation::scheduler::has_scheduled_frame()
            || (self.lifecycle != ApplicationLifecycle::Suspended
                && incular_animation::scheduler::has_ready_scheduled_task())
    }
    /// Earliest deadline that needs a frame but does not need continuous
    /// redraws while waiting. Hosts can sleep until this instant.
    #[must_use]
    pub fn next_frame_deadline(&self) -> Option<Instant> {
        self.text_histories.values().filter_map(UndoHistoryController::next_deadline).min()
    }

    #[must_use]
    pub fn handle_input(&mut self, event: InputEvent) -> Option<EventTarget> {
        self.tree
            .set_diagnostic_input_trigger(None, &event, diagnostic_input_trigger);
        self.handle_input_timed(event)
    }

    /// Handles `event` as if it arrived at `at`: velocity tracking and
    /// gesture timing see `at` instead of the wall clock. Frame-by-frame
    /// parity tests stamp input with the frame clock, and mobile hosts stamp
    /// touch samples with the time the platform recorded them.
    pub fn handle_input_at(&mut self, event: InputEvent, at: Instant) -> Option<EventTarget> {
        self.input_clock = Some(at);
        let target = self.handle_input(event);
        self.input_clock = None;
        target
    }

    /// Dispatches a complete pointer sample, preserving synthesized motion
    /// and the supplied timestamp through listeners and retained gestures.
    pub fn handle_raw_pointer_event(&mut self, event: RawPointerEvent) -> Option<EventTarget> {
        let traced_at = incular_core::time::Instant::now();
        let diagnostic = InputEvent::PointerWithMetadata {
            pointer: event.pointer,
            device: event.device,
            kind: event.kind,
            buttons: event.buttons,
            button: event.button,
            sample: event.sample,
            phase: event.phase,
            position: event.position,
        };
        self.tree.set_diagnostic_input_trigger(
            event.synthesized.then_some("synthesized"),
            &diagnostic,
            diagnostic_input_trigger,
        );
        let previous_clock = self.input_clock.replace(event.time);
        let result = self.handle_pointer_input(event, true);
        self.input_clock = previous_clock;
        self.pending_event_processing_us = self
            .pending_event_processing_us
            .saturating_add(us_since_instant(traced_at));
        result
    }

    fn input_now(&self) -> Instant {
        self.input_clock.unwrap_or_else(Instant::now)
    }

    pub(crate) fn handle_input_with_trigger(
        &mut self,
        event: InputEvent,
        trigger: impl Into<String>,
    ) -> Option<EventTarget> {
        self.tree.set_diagnostic_trigger(trigger);
        self.handle_input_timed(event)
    }

    pub(crate) fn handle_input_timed(&mut self, event: InputEvent) -> Option<EventTarget> {
        let started = incular_core::time::Instant::now();
        self.advance_text_history_clock(self.input_now());
        let target = self.handle_input_inner(event);
        self.pending_event_processing_us = self
            .pending_event_processing_us
            .saturating_add(us_since_instant(started));
        target
    }

    /// Associates subsequent frame work with a higher-level command such as a
    /// semantic simulation click. The trigger survives the input callback so
    /// deferred build/layout failures retain their original cause.
    pub fn set_diagnostic_trigger(&self, trigger: impl Into<String>) {
        self.tree.set_diagnostic_trigger(trigger);
    }
    /// The retained tree's window key for gesture arenas and hover state.
    fn gesture_window(&self) -> u64 {
        self.window_id.map_or(0, |window| {
            (u64::from(window.index()) << 32) | u64::from(window.generation())
        })
    }
    /// Handles the events that carry no pointer sample.
    fn handle_non_pointer_input(&mut self, event: InputEvent) -> Option<EventTarget> {
        match event {
            InputEvent::Scroll { delta } => {
                let position = self
                    .hover_devices
                    .latest()
                    .map_or(self.last_pointer, |event| event.position);
                self.handle_wheel(position, delta);
            }
            InputEvent::ScrollAt { position, delta } => self.handle_wheel(position, delta),
            InputEvent::Key(key) => {
                self.dispatch_keyboard_event(key);
            }
            InputEvent::Text(text) => self.insert_text(&text),
            InputEvent::Ime(ime) => self.handle_ime(ime),
            InputEvent::TrackpadGesture(gesture) => {
                // Aggregate gestures do not carry a focal position in
                // Winit. Use only a genuine cursor position already
                // observed for this window; never invent a touch point.
                let gesture_window = self.gesture_window();
                if let Some(sample) = self.hover_devices.latest()
                    && let Some(element) = self.tree.dispatch_trackpad_gesture_in_window(
                        gesture_window,
                        sample.position,
                        gesture,
                    )
                {
                    self.frame_requested = true;
                    return Some(EventTarget {
                        element,
                        action: None,
                    });
                }
            }
            InputEvent::WindowResized { .. } => {}
            InputEvent::Pointer { .. }
            | InputEvent::PointerWithId { .. }
            | InputEvent::PointerWithMetadata { .. } => {
                unreachable!("pointer events are handled by handle_input_inner")
            }
        }
        None
    }

    fn handle_wheel(&mut self, position: Offset, delta: Offset) {
        // Shift turns a wheel into a horizontal scroll (Flutter's
        // `_kScrollAxisModifierKeys`).
        if self
            .tree
            .scroll_at_with_axis_flip(position, delta, self.keyboard_modifiers.shift())
        {
            self.frame_requested = true;
        }
    }

    /// Handles a wheel sample from a host that can perform a default scroll
    /// when none of the retained listeners or viewports claims the event.
    pub fn handle_wheel_with_platform_response(
        &mut self,
        position: Offset,
        delta: Offset,
        respond: impl FnOnce(bool),
    ) -> bool {
        let activity = self.tree.scroll_at_with_platform_response(
            position,
            delta,
            self.keyboard_modifiers.shift(),
            respond,
        );
        self.frame_requested |= activity;
        activity
    }

    fn handle_input_inner(&mut self, event: InputEvent) -> Option<EventTarget> {
        let (pointer, device, kind, buttons, button, sample, phase, position, metadata_rich) =
            match event {
                InputEvent::Pointer { phase, position } => (
                    0,
                    0,
                    PointerDeviceKind::Mouse,
                    0,
                    None,
                    PointerSampleMetadata::default(),
                    phase,
                    position,
                    false,
                ),
                InputEvent::PointerWithId {
                    pointer,
                    phase,
                    position,
                } => (
                    pointer,
                    0,
                    PointerDeviceKind::Mouse,
                    0,
                    None,
                    PointerSampleMetadata::default(),
                    phase,
                    position,
                    false,
                ),
                InputEvent::PointerWithMetadata {
                    pointer,
                    device,
                    kind,
                    buttons,
                    button,
                    sample,
                    phase,
                    position,
                } => (
                    pointer, device, kind, buttons, button, sample, phase, position, true,
                ),
                other => return self.handle_non_pointer_input(other),
            };
        let raw_event = RawPointerEvent::new(
            pointer,
            device,
            kind,
            buttons,
            position,
            phase,
            self.input_now(),
        )
        .with_button(button)
        .with_sample(sample);
        self.handle_pointer_input(raw_event, metadata_rich)
    }

    fn handle_hover_device_event(&mut self, event: HoverDeviceEvent) {
        if !kind_supports_hover(event.kind)
            || !event.position.x.is_finite()
            || !event.position.y.is_finite()
        {
            return;
        }
        let window = self.gesture_window();
        let phase = match event.change {
            HoverDeviceChange::Added => PointerPhase::Move,
            HoverDeviceChange::Removed => PointerPhase::Exit,
        };
        let raw = RawPointerEvent::new(
            event.pointer,
            event.device,
            event.kind,
            0,
            event.position,
            phase,
            self.input_now(),
        );
        match event.change {
            HoverDeviceChange::Added => {
                self.last_pointer = event.position;
                self.hover_devices.record(raw);
                let hit = self.tree.pointer_hit_test(event.position);
                self.tree.add_positioned_hover_device(window, raw, &hit);
                if self.legacy_pointer.is_none() {
                    let target = self.button_target(&hit);
                    self.set_hover(target.map(|(element, _)| element));
                }
            }
            HoverDeviceChange::Removed => {
                let _ = self.tree.dispatch_raw_pointer_with_hit(
                    window,
                    raw,
                    &PointerHitTest::default(),
                );
                if self.hover_devices.remove(raw) {
                    self.refresh_primary_button_hover();
                    self.frame_requested |= self
                        .tree
                        .scrollbar_pointer(PointerPhase::Exit, event.position);
                }
            }
        }
        if event.kind == PointerDeviceKind::Mouse {
            self.sync_initial_focus_highlight_mouse();
        }
    }

    fn sync_initial_focus_highlight_mouse(&self) {
        FocusHighlightManager::new().set_initial_mouse_connected(
            self.hover_devices
                .iter()
                .any(|event| event.kind == PointerDeviceKind::Mouse),
        );
    }

    fn handle_pointer_input(
        &mut self,
        raw_event: RawPointerEvent,
        metadata_rich: bool,
    ) -> Option<EventTarget> {
        let gesture_window = self.gesture_window();
        let RawPointerEvent {
            pointer,
            device,
            kind,
            button,
            phase,
            position,
            ..
        } = raw_event;
        let hover_capable = kind_supports_hover(kind);
        let mut departed_primary_hover = false;
        if hover_capable {
            if matches!(phase, PointerPhase::Enter | PointerPhase::Exit) {
                // A window boundary carries no new positioned hover sample.
                departed_primary_hover = self.hover_devices.remove(raw_event);
            } else {
                self.hover_devices.record(raw_event);
            }
        }
        if kind == PointerDeviceKind::Mouse {
            self.sync_initial_focus_highlight_mouse();
        } else {
            FocusHighlightManager::new().note_pointer_input(kind);
        }
        if phase != PointerPhase::Enter {
            self.last_pointer = position;
        }
        if phase == PointerPhase::Down {
            // A held press owns `kPressTimeout` and `kLongPressTimeout`
            // deadlines that fire on the frame clock, so a frame must run.
            self.frame_requested = true;
        }
        let text_menu_was_showing = phase == PointerPhase::Down
            && (button == Some(incular_core::SECONDARY_POINTER_BUTTON)
                || (self.text_shortcut_platform == TextShortcutPlatform::Ios
                    && matches!(kind, PointerDeviceKind::Touch | PointerDeviceKind::Unknown)))
            && self.text_menu_open_at(position);
        // A press on the focused field's handle is part of that field, so it
        // does not dismiss the field's toolbar.
        let on_text_handle =
            phase == PointerPhase::Down && self.text_handle_under(position, kind).is_some();
        if phase == PointerPhase::Down
            && !on_text_handle
            && self.tree.dismiss_transients_for_pointer(position) > 0
        {
            self.frame_requested = true;
        }
        // One hit test serves the whole event: raw regions and listeners,
        // retained gestures, text editing and buttons.
        let hit = if matches!(phase, PointerPhase::Enter | PointerPhase::Exit) {
            PointerHitTest::default()
        } else {
            self.tree.pointer_hit_test(position)
        };
        let raw_target = self
            .tree
            .dispatch_raw_pointer_with_hit(gesture_window, raw_event, &hit);
        if metadata_rich
            && self.text_secondary_pointer(
                phase,
                button,
                device,
                pointer,
                position,
                &hit,
                text_menu_was_showing,
            )
        {
            self.frame_requested = true;
        }
        if matches!(phase, PointerPhase::Enter | PointerPhase::Exit) {
            if departed_primary_hover {
                self.refresh_primary_button_hover();
                if phase == PointerPhase::Exit
                    && self.tree.scrollbar_pointer(PointerPhase::Exit, position)
                {
                    self.frame_requested = true;
                }
            }
            return None;
        }

        let primary_changed = !metadata_rich || button == Some(PRIMARY_POINTER_BUTTON);
        let stream = (device, pointer);
        let active_primary = self.legacy_pointer == Some(stream);
        let legacy_pointer = match phase {
            PointerPhase::Down if primary_changed && self.legacy_pointer.is_none() => {
                self.legacy_pointer = Some(stream);
                true
            }
            PointerPhase::Down => primary_changed && active_primary,
            PointerPhase::Move => {
                active_primary || (hover_capable && self.legacy_pointer.is_none())
            }
            PointerPhase::Up => primary_changed && active_primary,
            PointerPhase::Cancel => active_primary,
            PointerPhase::Enter | PointerPhase::Exit => false,
        };

        if let Some(element) = raw_target {
            if active_primary && phase != PointerPhase::Down {
                self.cancel_pending_mobile_text_touch();
            }
            self.frame_requested = true;
            self.release_legacy_pointer(device, pointer, phase, primary_changed);
            return Some(EventTarget {
                element,
                action: None,
            });
        }
        // A secondary or tertiary button press and its release join the gesture
        // arena too: Flutter routes them to that button's tap and long-press
        // recognizers, and their release is not a primary transition.
        let other_button_event = metadata_rich
            && matches!(phase, PointerPhase::Down | PointerPhase::Up)
            && button.is_some_and(|button| button != PRIMARY_POINTER_BUTTON);
        let ordinary_gesture_allowed = legacy_pointer
            || !metadata_rich
            || !hover_capable
            || other_button_event
            || self.tree.has_active_pointer_gesture(gesture_window, device, pointer);
        if ordinary_gesture_allowed
            && let Some(element) =
                self.tree
                    .dispatch_pointer_gesture_raw_with_hit(gesture_window, raw_event, &hit)
        {
            self.frame_requested = true;
            if active_primary && phase != PointerPhase::Down {
                self.cancel_pending_mobile_text_touch();
            }
            // A drag, long press or scroll that wins the arena after the
            // press reached a button cancels the button (Flutter's
            // `InkWell` loses the arena and gets `onTapCancel`).
            if phase != PointerPhase::Down
                && active_primary
                && let Some(pressed) = self.pressed_button.take()
            {
                let _ = self
                    .tree
                    .set_button_interaction(pressed, None, Some(false), None);
                self.tree.button_ink(pressed, ButtonInk::Cancel);
            }
            self.release_legacy_pointer(device, pointer, phase, primary_changed);
            return Some(EventTarget {
                element,
                action: None,
            });
        }
        // Legacy controls and scrollbars admit just the primary contact.
        // Additional contacts remain available to retained gesture regions.
        if !legacy_pointer {
            return None;
        }
        if self.tree.scrollbar_pointer_with_kind(phase, position, kind) {
            self.cancel_pending_mobile_text_touch();
            self.frame_requested = true;
            self.release_legacy_pointer(device, pointer, phase, primary_changed);
            return None;
        }
        let text_target = self.tree.text_field_for_hit(&hit);
        let selectable_target = self.tree.selectable_text_for_hit(&hit);
        let target = self.button_target(&hit);
        let result = match phase {
            PointerPhase::Enter | PointerPhase::Exit => {
                unreachable!("pointer boundary handled above")
            }
            PointerPhase::Move => {
                self.text_handle_drag_move(position);
                if self
                    .pending_mobile_text_touch
                    .is_some_and(|press| !press.within_slop(position))
                {
                    self.cancel_pending_mobile_text_touch();
                }
                self.text_long_press_moved(position);
                // A started long press extends by word and owns the move.
                let long_press_owns_move = self.text_long_press_move(position, selectable_target);
                if let Some(field) = self
                    .captured_text_field
                    .filter(|_| !long_press_owns_move && self.pending_mobile_text_touch.is_none())
                {
                    self.text_field_primary_drag(field, position, kind);
                    self.frame_requested = true;
                }
                if let Some(label) = self
                    .captured_selectable_text
                    .filter(|_| !long_press_owns_move)
                {
                    if let Some(target) = selectable_target {
                        if self
                            .tree
                            .selectable_text_set_selection(target, position, true)
                        {
                            self.captured_selectable_text = Some(target);
                            self.set_focus(Some(target));
                            self.frame_requested = true;
                        }
                    } else if self.tree.is_selectable_text(label) {
                        self.frame_requested = true;
                    }
                }
                if hover_capable {
                    self.set_hover(target.map(|(element, _)| element));
                }
                target.map(|(element, action)| EventTarget { element, action })
            }
            PointerPhase::Down => {
                // Long press follows the pointer kind: touch and stylus only on
                // selectable text (`_kLongPressSelectionDevices`), any device
                // on text fields.
                let touch = matches!(
                    kind,
                    PointerDeviceKind::Touch
                        | PointerDeviceKind::Stylus
                        | PointerDeviceKind::InvertedStylus
                );
                // A touch on a visible selection handle drags it instead of
                // pressing the field: no focus change, no long press.
                if let Some(field) = self.text_handle_press(position, kind) {
                    self.frame_requested = true;
                    return Some(EventTarget {
                        element: field,
                        action: None,
                    });
                }
                if let Some(label) = selectable_target {
                    self.set_focus(Some(label));
                    self.captured_selectable_text = Some(label);
                    if self
                        .tree
                        .selectable_text_set_selection(label, position, false)
                    {
                        if touch {
                            self.text_long_press_begin(
                                text_gestures::LongPressTarget::Label(label),
                                position,
                                true,
                                false,
                            );
                        }
                        self.frame_requested = true;
                        return Some(EventTarget {
                            element: label,
                            action: None,
                        });
                    }
                }
                if selectable_target.is_none()
                    && text_target.is_none()
                    && target.is_none()
                    && let Some((element, interaction)) = self.tree.window_interaction_at(position)
                    && let (Some(window_id), Some(manager)) =
                        (self.window_id, self.window_manager.as_ref())
                {
                    let operation = match interaction {
                        WindowInteraction::Move => WindowOperation::BeginMoveDrag,
                        WindowInteraction::Resize(direction) => {
                            WindowOperation::BeginResizeDrag(direction)
                        }
                    };
                    if manager.send_window_operation(window_id, operation) {
                        self.release_legacy_pointer(device, pointer, phase, primary_changed);
                        return Some(EventTarget {
                            element,
                            action: None,
                        });
                    }
                }
                let text_target_was_focused =
                    text_target.is_some_and(|field| self.focused == Some(field));
                // `EditableText._defaultOnTapOutside`: on Android and iOS a
                // finger tapping outside the field leaves it focused, with
                // the keyboard up; a mouse or stylus unfocuses it. A tap on
                // another text field is not "outside": that field requests
                // focus itself.
                let mobile_touch = kind == PointerDeviceKind::Touch
                    && matches!(
                        self.text_shortcut_platform,
                        TextShortcutPlatform::Android | TextShortcutPlatform::Ios
                    );
                let deferred_text_touch =
                    mobile_touch && !text_target_was_focused && text_target.is_some();
                let keep_text_field_focus = text_target.is_none()
                    && self
                        .focused
                        .is_some_and(|focused| self.tree.is_text_field(focused))
                    && (mobile_touch || self.tree.preserves_text_field_focus(position));
                // Flutter on desktop: a press focuses the text field it hits, a
                // press outside a focused text field unfocuses it
                // (`EditableText` tap-outside), and any other press leaves
                // focus where it is. Buttons and gesture detectors do not
                // request focus on tap, so a focused node (a media player's
                // key handler) keeps its keys after a click elsewhere.
                let focused_text_field = self
                    .focused
                    .is_some_and(|focused| self.tree.is_text_field(focused));
                if !deferred_text_touch {
                    self.set_focus_with_external_selection(
                        if keep_text_field_focus {
                            self.focused
                        } else if text_target.is_some() || focused_text_field {
                            text_target
                        } else {
                            self.focused
                        },
                        false,
                    );
                }
                // Flutter's EditableText only installs its selection gestures
                // while `_userSelectionEnabled` (`!readOnly || !obscureText`)
                // holds; a read-only obscured field keeps focus but no caret.
                self.captured_text_field =
                    text_target.filter(|field| self.text_field_pointer_selection_enabled(*field));
                if let Some(field) = text_target {
                    if deferred_text_touch {
                        self.pending_mobile_text_touch = Some(text_gestures::PendingMobileTextTouch {
                            field,
                            position,
                            menu_was_showing: text_menu_was_showing,
                        });
                    }
                    if self.captured_text_field.is_some() {
                        if !deferred_text_touch {
                            self.text_field_primary_down(
                                field,
                                position,
                                text_target_was_focused,
                                kind,
                                text_menu_was_showing,
                            );
                        }
                        self.text_long_press_begin(
                            text_gestures::LongPressTarget::Field(field),
                            position,
                            touch,
                            !text_target_was_focused,
                        );
                    }
                    self.frame_requested = true;
                    return Some(EventTarget {
                        element: field,
                        action: None,
                    });
                }
                if hover_capable {
                    self.set_hover(target.map(|(element, _)| element));
                }
                self.pressed_button = target.map(|(element, _)| element);
                if let Some(element) = self.pressed_button {
                    let _ = self
                        .tree
                        .set_button_interaction(element, None, Some(true), None);
                    self.tree.button_ink(element, ButtonInk::Down(position));
                    self.frame_requested = true;
                }
                target.map(|(element, action)| EventTarget { element, action })
            }
            PointerPhase::Up => {
                if let Some(press) = self.pending_mobile_text_touch {
                    if press.within_slop(position)
                        && text_target == Some(press.field)
                        && self.tree.can_request_focus(press.field)
                    {
                        self.pending_mobile_text_touch = None;
                        let was_focused = self.focused == Some(press.field);
                        self.set_focus_with_external_selection(Some(press.field), false);
                        if self.captured_text_field == Some(press.field) {
                            self.text_field_primary_down(
                                press.field,
                                press.position,
                                was_focused,
                                kind,
                                press.menu_was_showing,
                            );
                        }
                    } else {
                        self.cancel_pending_mobile_text_touch();
                    }
                }
                self.text_handle_drag_end(position);
                let was_long_press = self.text_long_press.as_ref().is_some_and(|press| press.started);
                self.text_long_press_end(position);
                if self.captured_text_field.take().is_some() {
                    self.text_field_primary_up(position, kind, was_long_press);
                }
                self.captured_selectable_text = None;
                let pressed = self.pressed_button.take();
                let valid = pressed
                    .zip(target)
                    .filter(|(pressed, (target, _))| pressed == target)
                    .map(|(_, target)| target);
                if let Some(element) = pressed {
                    let _ = self
                        .tree
                        .set_button_interaction(element, None, Some(false), None);
                    let ink = if valid.is_some() {
                        ButtonInk::Confirm
                    } else {
                        ButtonInk::Cancel
                    };
                    self.tree.button_ink(element, ink);
                }
                if let Some((element, action)) = valid {
                    if let Some(action) = action {
                        self.activate_action(action);
                    }
                    Some(EventTarget { element, action })
                } else {
                    None
                }
            }
            PointerPhase::Cancel => {
                self.captured_text_field = None;
                self.text_long_press = None;
                self.text_handle_drag = None;
                self.captured_selectable_text = None;
                if let Some(element) = self.pressed_button.take() {
                    let _ = self
                        .tree
                        .set_button_interaction(element, None, Some(false), None);
                    self.tree.button_ink(element, ButtonInk::Cancel);
                    self.frame_requested = true;
                }
                None
            }
        };
        self.release_legacy_pointer(device, pointer, phase, primary_changed);
        result
    }
    fn button_target(&self, hit: &PointerHitTest) -> Option<(ElementId, Option<ActionId>)> {
        hit.target()
            .and_then(|render| self.tree.element_for_render(render))
            .and_then(|element| self.tree.button_ancestor(element))
    }
    /// Flutter's `MouseTracker.updateAllDevices` after layout: a still
    /// pointer over content that moved (a programmatic or wheel scroll, a
    /// relayout) hovers whatever is under it now. Mouse regions get their
    /// enter and exit callbacks, and button hover follows unless a press
    /// owns the single-contact controls.
    fn refresh_hover_after_layout(&mut self) {
        let Some(primary) = self.hover_devices.latest() else {
            return;
        };
        self.tree.refresh_follower_links_for_geometry();
        let window = self.gesture_window();
        let now = self.input_now();
        let primary_key = primary.hover_device_identity();
        let mut target = None;
        for sample in self.hover_devices.iter() {
            let event = RawPointerEvent {
                local_position: sample.position,
                delta: Offset::ZERO,
                local_delta: Offset::ZERO,
                time: now,
                ..sample
            };
            let hit = self.tree.pointer_hit_test(sample.position);
            self.tree.refresh_mouse_hover_in_window(window, event, &hit);
            if self.legacy_pointer.is_none() && sample.hover_device_identity() == primary_key {
                target = self.button_target(&hit).map(|(id, _)| id);
            }
        }
        if self.legacy_pointer.is_none() {
            self.set_hover(target);
        }
    }

    fn refresh_primary_button_hover(&mut self) {
        if self.legacy_pointer.is_none() {
            let target = self.hover_devices.latest().and_then(|event| {
                let hit = self.tree.pointer_hit_test(event.position);
                self.button_target(&hit).map(|(id, _)| id)
            });
            self.set_hover(target);
        }
    }

    /// Whether at least one hover-capable device has a known window position.
    #[must_use]
    pub fn has_hover_device(&self) -> bool {
        !self.hover_devices.is_empty()
    }

    fn release_legacy_pointer(
        &mut self,
        device: u64,
        pointer: u64,
        phase: PointerPhase,
        primary_changed: bool,
    ) {
        if (phase == PointerPhase::Cancel || (phase == PointerPhase::Up && primary_changed))
            && self.legacy_pointer == Some((device, pointer))
        {
            self.cancel_pending_mobile_text_touch();
            self.legacy_pointer = None;
        }
    }

    /// Cursor selected by the current retained hover route. `Defer` is resolved
    /// to the ordinary platform cursor before crossing the native boundary.
    #[must_use]
    pub fn effective_mouse_cursor(&self) -> MouseCursor {
        let Some(event) = self.hover_devices.latest() else {
            return MouseCursor::Basic;
        };
        match self.tree.mouse_cursor_at(event.position) {
            MouseCursor::Defer => MouseCursor::Basic,
            cursor => cursor,
        }
    }
    pub(crate) fn set_focus(&mut self, next: Option<ElementId>) {
        self.set_focus_with_external_selection(next, true);
    }

    fn set_focus_with_external_selection(&mut self, next: Option<ElementId>, external: bool) {
        // Admit the new target before touching the old one. A hit on a
        // disabled or excluded editor is not permission to retire the
        // current native client or cancel its composing text.
        if next.is_some_and(|target| !self.tree.can_request_focus(target)) {
            return;
        }
        // Explicit input (including clearing focus) wins over a pending initial
        // autofocus request, even when it leaves the focused element unchanged.
        self.initial_focus = InitialFocus::Settled;
        let next_node = next.and_then(|element| self.tree.focus_node_at(element));
        if self.focused == next && self.primary_focus_node == next_node {
            return;
        }
        // A focus change retires the native client behind any open
        // composition: without a session id later IME events are
        // unattributable, so the old composition ends here and later
        // events start fresh on the newly focused field.
        self.cancel_ime_composition();
        if let Some(next) = next
            && self.tree.dismiss_transients_for_focus(next) > 0
        {
            self.frame_requested = true;
        }
        if let Some(previous) = self.focused {
            self.tree.set_keyboard_focus(previous, false);
            let _ = self.tree.set_focused(previous, false, self.input_now());
            self.hide_text_field_overlays(previous);
        }
        self.focused = next;
        self.primary_focus_node = next_node;
        let focus_policy = self.sync_focused_selection_lease();
        self.tree.sync_hosted_focus_chain(next);
        if let Some(current) = next {
            if external && let Some((controller, true)) = focus_policy {
                controller.select_all();
            }
            let _ = self.tree.set_focused(current, true, self.input_now());
            self.tree.set_keyboard_focus(current, true);
            if self.tree.is_text_field(current) {
                let _ = self.ensure_text_history(current);
            }
        }
        self.frame_requested = true;
        self.sync_text_input_client();
        for failure in self.primary_focus_changes.notify_collecting_panics() {
            let message = failure
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| failure.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("non-string primary focus listener panic");
            tracing::error!(%message, "primary focus listener failed");
        }
    }

    fn sync_focused_selection_lease(&mut self) -> Option<(TextEditingController, bool)> {
        let policy = self.focused.and_then(|current| {
            let desktop = cfg!(target_arch = "wasm32")
                || matches!(
                    self.text_shortcut_platform,
                    TextShortcutPlatform::Linux
                        | TextShortcutPlatform::MacOs
                        | TextShortcutPlatform::Windows,
                );
            self.tree.editable_focus_selection_policy(current, desktop)
        });
        match (&self.focused_selection_lease, &policy) {
            (None, None) => {}
            (Some(current), Some((controller, select_all)))
                if current.matches(controller, *select_all) => {}
            _ => {
                self.focused_selection_lease = None;
                self.focused_selection_lease = policy
                    .as_ref()
                    .map(|(controller, select_all)| controller.bind_focused_selection(*select_all));
            }
        }
        policy
    }

    /// Applies a focus change made directly on a [`FocusNode`]: the requested
    /// node becomes the runtime's focus (releasing the previous one), and an
    /// unfocused current node clears it.
    fn adopt_external_focus_change(&mut self) {
        self.tree.refresh_hosted_scope_membership();
        // Scanning focusable elements allocates, so only look when some
        // focus node changed since the last check.
        let epoch = self.tree.focus_epoch();
        let replaced = self.primary_focus_node_replaced();
        if !replaced && self.adopted_focus_epoch == Some(epoch) {
            return;
        }
        let current = if replaced { None } else { self.focused };
        if let Some(next) = self.tree.external_focus_change(current) {
            self.set_focus(next);
        } else if replaced {
            // An Element reused for a different explicit FocusNode must
            // not silently transfer focus to the replacement. The next
            // focused node still needs a real request or autofocus.
            self.set_focus(None);
        }
        // Focus observers can queue another request while a transition is
        // being delivered. Do not acknowledge that newer request here.
        self.adopted_focus_epoch = Some(epoch);
    }

    fn primary_focus_node_replaced(&self) -> bool {
        self.focused.is_some_and(|element| {
            self.primary_focus_node
                .as_ref()
                .is_some_and(|node| self.tree.focus_node_at(element).as_ref() != Some(node))
        })
    }

    fn reconcile_focus_and_selection_captures(&mut self) {
        if self.parked_focus.as_ref().is_some_and(|parked| {
            !self.tree.element_exists(parked.target)
                || self.tree.focus_node_at(parked.target) != parked.node
        }) {
            self.parked_focus = None;
        }
        if self.pending_mobile_text_touch.is_some_and(|press| {
            !self.tree.is_text_field(press.field) || !self.tree.can_request_focus(press.field)
        }) {
            self.cancel_pending_mobile_text_touch();
        }
        if self.captured_text_field.is_some_and(|field| {
            !self.tree.is_text_field(field) || !self.tree.can_request_focus(field)
        }) {
            self.captured_text_field = None;
        }
        if self.captured_selectable_text.is_some_and(|label| {
            !self.tree.is_selectable_text(label) || !self.tree.can_request_focus(label)
        }) {
            self.captured_selectable_text = None;
        }
        if self.primary_focus_node_replaced() {
            self.adopt_external_focus_change();
        }
        if self
            .focused
            .is_some_and(|focused| !self.tree.can_request_focus(focused))
        {
            self.set_focus(None);
        }
        self.tree.sync_hosted_focus_chain(self.focused);
        self.sync_focused_selection_lease();
        self.reconcile_button_tenure();
    }

    fn reconcile_button_tenure(&mut self) {
        // Pressed tenure belongs to a still-pressable button element.
        // `button_ancestor` already filters disabled and missing elements,
        // so reuse it instead of restating button liveness here. A stale
        // press clears its visual when still mounted and never activates.
        if self.pressed_button.is_some_and(|pressed| {
            self.tree
                .button_ancestor(pressed)
                .map(|(ancestor, _)| ancestor)
                != Some(pressed)
        }) && let Some(pressed) = self.pressed_button.take()
        {
            let _ = self
                .tree
                .set_button_interaction(pressed, None, Some(false), None);
            self.frame_requested = true;
        }
        // Hover tenure reuses the single `set_hover` owner so exit callbacks
        // fire for still-mounted buttons; pruned handlers make unmounted exit
        // a silent visual release.
        if self.hovered_button.is_some_and(|hovered| {
            self.tree
                .button_ancestor(hovered)
                .map(|(ancestor, _)| ancestor)
                != Some(hovered)
        }) {
            self.set_hover(None);
        }
    }

    fn advance_text_history_clock(&mut self, now: Instant) {
        let now = self.text_history_time.map_or(now, |last| last.max(now));
        self.text_history_time = Some(now);
        for history in self.text_histories.values() {
            history.advance_to(now);
        }
    }

    fn sync_mounted_text_histories(&mut self) {
        let epoch = self.tree.text_field_history_epoch();
        if self.text_history_epoch == Some(epoch) {
            return;
        }
        self.text_history_epoch = Some(epoch);
        let clock = self.text_history_time.unwrap_or_else(|| self.input_now());
        let android = self.text_shortcut_platform == TextShortcutPlatform::Android;
        for field in self.tree.text_field_elements() {
            Self::mounted_text_history(&self.tree, &mut self.text_histories, field, clock, android);
        }
    }

    fn ensure_text_history(&mut self, field: ElementId) -> Option<UndoHistoryController> {
        let clock = self.text_history_time.unwrap_or_else(|| self.input_now());
        let android = self.text_shortcut_platform == TextShortcutPlatform::Android;
        Self::mounted_text_history(&self.tree, &mut self.text_histories, field, clock, android).cloned()
    }

    fn mounted_text_history<'histories>(
        tree: &WidgetTree,
        histories: &'histories mut HashMap<ElementId, UndoHistoryController>,
        field: ElementId,
        clock: Instant,
        android: bool,
    ) -> Option<&'histories UndoHistoryController> {
        let editor = tree.text_controller(field)?;
        let replace = histories.get(&field).is_none_or(|history| history.editor() != editor);
        if replace {
            histories.insert(field, UndoHistoryController::new_flutter(editor, clock, android));
        }
        let history = histories.get(&field)?;
        if let Some(max_entries) = tree.text_field_history_max_entries(field)
            && history.max_entries() != max_entries
        {
            history.set_max_entries(max_entries);
        }
        Some(history)
    }

    /// Flutter's `EditableTextState._scheduleShowCaretOnScreen`: when the
    /// focused field gains focus or its value or selection changes, its
    /// ancestor viewports scroll its caret, with `scrollPadding`, into view.
    fn show_focused_caret_on_screen(&mut self) -> bool {
        let Some(field) = self.focused.filter(|id| self.tree.is_text_field(*id)) else {
            self.caret_reveal_key = None;
            return false;
        };
        let Some(controller) = self.tree.text_controller(field) else {
            return false;
        };
        let key = (field, controller.revisions().0, controller.selection());
        if self.caret_reveal_key == Some(key) {
            return false;
        }
        self.caret_reveal_key = Some(key);
        self.tree.text_field_show_caret_on_screen(field)
    }

    /// Retires the current host connection and clears its editor focus. It
    /// was already closed by the host, so no Hide/Clear is echoed back.
    /// An older connection ID cannot affect a reattached editor.
    pub fn text_input_connection_closed(&mut self, client: TextInputClientId) -> bool {
        let Some(active) = self.text_input_client.filter(|active| active.client == client) else {
            return false;
        };
        self.text_input_client = None;
        self.text_input_commands.retain(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => configuration.client != client,
            TextInputCommand::Update { client: id, .. }
            | TextInputCommand::SetCaretRect { client: id, .. }
            | TextInputCommand::Show { client: id }
            | TextInputCommand::Hide { client: id }
            | TextInputCommand::Clear { client: id } => *id != client,
            TextInputCommand::FinishAutofillContext { .. } => true,
        });
        if self.focused == Some(active.target) {
            self.set_focus(None);
        }
        true
    }

    /// Tries to refocus the last attached editor on behalf of the host.
    /// The retained element must still be mounted and eligible.
    pub fn text_input_focus_received(&mut self, client: TextInputClientId) -> bool {
        let Some(previous) = self.last_text_input_client.filter(|previous| previous.client == client) else {
            return false;
        };
        self.request_text_input_focus_for(previous.target)
    }

    /// Delivers a focus request to a specific retained editor, including
    /// one newly mounted by a lazy list and not yet attached to the host.
    /// Use the connection-ID entry point for asynchronous channel events.
    pub fn request_text_input_focus_for(&mut self, target: ElementId) -> bool {
        if self.focused == Some(target)
            || !self.tree.is_text_field(target)
            || !self.tree.can_request_focus(target)
        {
            return false;
        }
        self.set_focus(Some(target));
        true
    }

    fn sync_text_input_client(&mut self) {
        self.sync_text_input_client_for_host(false);
    }

    fn sync_text_input_client_for_host(&mut self, resend_client: bool) {
        self.collect_autofill_context_completions();
        // This runs every frame. While the focused field still holds the
        // value last sent to the input client, that state is reused instead
        // of copying the field's text again, which matters for a long
        // document: only the caret rectangle and configuration are read.
        let unchanged = self.focused.is_some_and(|id| {
            self.text_input_state.as_ref().is_some_and(|state| {
                self.tree.text_field_value_matches(
                    id,
                    &state.text,
                    TextSelection::new(state.selection_start, state.selection_end),
                    state
                        .composing
                        .map(|(start, end)| incular_text::TextRange::new(start, end)),
                )
            })
        });
        let snapshot = self.focused.and_then(|id| {
            if unchanged {
                self.tree.text_field_input_geometry(id)
            } else {
                self.tree.text_field_input_snapshot(id)
            }
        });
        let Some((target, snapshot)) = self.focused.zip(snapshot) else {
            if let Some(MountedTextInputConnection { client, .. }) = self.text_input_client.take() {
                self.text_input_commands
                    .push_back(TextInputCommand::Hide { client });
                self.text_input_commands
                    .push_back(TextInputCommand::Clear { client });
            }
            self.text_input_configuration = None;
            self.text_input_state = None;
            self.text_input_show_requested = false;
            self.text_input_caret_rect = None;
            return;
        };

        let current = self.text_input_client.filter(|current| current.target == target);
        let client_changed = current.is_none();
        let connection = current.unwrap_or_else(|| {
            let client = TextInputClientId::new(self.next_text_input_client);
            self.next_text_input_client = self.next_text_input_client.checked_add(1)
                .expect("text input connection IDs exhausted");
            MountedTextInputConnection { client, target }
        });
        let client = connection.client;
        let input_type = snapshot.input_type;
        let action = match snapshot.input_action {
            TextInputActionHint::Unspecified => {
                if snapshot.multiline {
                    TextInputAction::Newline
                } else {
                    TextInputAction::Done
                }
            }
            TextInputActionHint::None => TextInputAction::None,
            TextInputActionHint::Done => TextInputAction::Done,
            TextInputActionHint::Go => TextInputAction::Go,
            TextInputActionHint::Search => TextInputAction::Search,
            TextInputActionHint::Send => TextInputAction::Send,
            TextInputActionHint::Next => TextInputAction::Next,
            TextInputActionHint::Previous => TextInputAction::Previous,
            TextInputActionHint::Newline => TextInputAction::Newline,
        };
        let configuration = TextInputConfiguration {
            client,
            input_type: if snapshot.obscure_text && input_type == TextInputType::Text {
                TextInputType::Password
            } else {
                input_type
            },
            action,
            multiline: snapshot.multiline,
            enabled: snapshot.enabled,
            read_only: snapshot.read_only,
            obscure_text: snapshot.obscure_text,
            traits: snapshot.traits,
        };
        let state = match self.text_input_state.take() {
            Some(previous) if unchanged => previous,
            _ => TextInputState {
                text: snapshot.text,
                selection_start: snapshot.selection.base,
                selection_end: snapshot.selection.extent,
                composing: snapshot.composing.map(|range| (range.start, range.end)),
            },
        };
        let configuration_changed = self.text_input_configuration.as_ref() != Some(&configuration);
        let set_client = resend_client || client_changed || configuration_changed;
        if set_client {
            self.text_input_commands
                .push_back(TextInputCommand::SetClient {
                    configuration: configuration.clone(),
                    state: state.clone(),
                    caret_rect: snapshot.caret_rect,
                });
        } else if !unchanged {
            self.text_input_commands
                .push_back(TextInputCommand::Update {
                    client,
                    state: state.clone(),
                });
        }
        if !set_client && self.text_input_caret_rect != Some(snapshot.caret_rect) {
            self.text_input_commands
                .push_back(TextInputCommand::SetCaretRect {
                    client,
                    rect: snapshot.caret_rect,
                });
        }
        // `SetClient` only attaches the field. A new client is shown because
        // focusing a field opens the keyboard; a configuration change on the
        // same client does not reopen a keyboard the user dismissed.
        let show_requested = std::mem::take(&mut self.text_input_show_requested);
        if show_requested || client_changed {
            self.text_input_commands
                .push_back(TextInputCommand::Show { client });
        }
        self.text_input_client = Some(connection);
        self.last_text_input_client = Some(connection);
        self.text_input_configuration = Some(configuration);
        self.text_input_state = Some(state);
        self.text_input_caret_rect = Some(snapshot.caret_rect);
    }
    fn focus_next(&mut self, reverse: bool) {
        let fields = self.tree.focus_traversal_elements(self.focused);
        if fields.is_empty() {
            return;
        }
        let index = self
            .focused
            .and_then(|focused| fields.iter().position(|id| *id == focused));
        let next = match index {
            Some(index) if reverse => fields[(index + fields.len() - 1) % fields.len()],
            Some(index) => fields[(index + 1) % fields.len()],
            None if reverse => *fields.last().expect("not empty"),
            None => fields[0],
        };
        self.set_focus(Some(next));
    }
    /// Flutter's native `WidgetsApp` defaults: unmodified arrows traverse
    /// focus, the platform's modifier with an arrow scrolls by a line, and
    /// unmodified PageUp/PageDown scroll by a page.
    fn run_default_focus_key(&mut self, event: &KeyboardEvent) -> bool {
        if event.modifiers.shift() || event.modifiers.alt() {
            return false;
        }
        let ctrl = event.modifiers.ctrl();
        let meta = event.modifiers.meta();
        let unmodified = !ctrl && !meta;
        let line_modifier = if matches!(
            self.text_shortcut_platform,
            TextShortcutPlatform::MacOs | TextShortcutPlatform::Ios
        ) {
            meta && !ctrl
        } else {
            ctrl && !meta
        };
        match event.code {
            Code::PageUp | Code::PageDown if unmodified => {
                self.tree
                    .scroll_nearest_viewport(self.focused, event.code, true)
            }
            Code::ArrowUp | Code::ArrowDown | Code::ArrowLeft | Code::ArrowRight
                if line_modifier =>
            {
                self.tree
                    .scroll_nearest_viewport(self.focused, event.code, false)
            }
            Code::ArrowUp | Code::ArrowDown | Code::ArrowLeft | Code::ArrowRight if unmodified => {
                let Some(next) = self.tree.directional_focus_target(self.focused, event.code)
                else {
                    return false;
                };
                self.set_focus(Some(next));
                true
            }
            _ => false,
        }
    }

    /// The portable pressed-key and lock state observed by this runtime.
    #[must_use]
    pub const fn hardware_keyboard(&self) -> &incular_core::HardwareKeyboardState {
        &self.hardware_keyboard
    }

    /// Adds a window-wide hardware key handler. Keep the subscription to
    /// receive events before focused widgets and fallback editing actions.
    pub fn add_hardware_key_handler(
        &self,
        handler: impl Fn(&KeyboardEvent) -> bool + 'static,
    ) -> incular_core::HardwareKeySubscription {
        self.hardware_key_handlers.add(handler)
    }

    /// Adds a window-local focus-key handler before the focused node and
    /// its ancestors. A handled result prevents the focus route from running.
    pub fn add_early_focus_key_handler(
        &self,
        handler: impl Fn(&KeyboardEvent) -> KeyEventResult + 'static,
    ) -> FocusKeySubscription {
        self.early_focus_key_handlers.add(handler)
    }

    /// Adds a window-local focus-key handler after the focused route,
    /// when the route did not handle the key. Keep the returned subscription.
    pub fn add_late_focus_key_handler(
        &self,
        handler: impl Fn(&KeyboardEvent) -> KeyEventResult + 'static,
    ) -> FocusKeySubscription {
        self.late_focus_key_handlers.add(handler)
    }

    /// Enables key and pressed-state diagnostics for this window. Normal
    /// event handling produces no such messages unless a listener is set.
    pub fn set_keyboard_event_debug_listener(&mut self, listener: impl Fn(&str) + 'static) {
        self.keyboard_event_debug_listener = Some(Box::new(listener));
    }

    pub fn clear_keyboard_event_debug_listener(&mut self) {
        self.keyboard_event_debug_listener = None;
    }

    /// Invokes an inherited action from a live element (Flutter's
    /// `Actions.invoke`), without sending key events to unrelated handlers.
    pub fn invoke_action_at<C: incular_widgets::CommandId>(
        &mut self,
        element: ElementId,
        intent: &incular_widgets::Intent<C>,
    ) -> Option<incular_widgets::ActionResult> {
        let result = self.tree.invoke_action_at(element, intent);
        if result == Some(incular_widgets::ActionResult::Handled) {
            self.frame_requested = true;
        }
        result
    }

    /// Dispatches a portable key event and returns whether it was handled.
    pub fn dispatch_keyboard_event(&mut self, event: KeyboardEvent) -> bool {
        FocusHighlightManager::new().note_keyboard_input();
        if let Some(debug) = &self.keyboard_event_debug_listener {
            debug(&format!("KEYBOARD: Key event received: {event:?}"));
            debug(&format!(
                "KEYBOARD: Pressed state before processing the event: {:?}",
                self.hardware_keyboard
            ));
            let irregularity = self.hardware_keyboard.observe_checked(&event);
            if let Some(irregularity) = irregularity {
                debug(&format!("KEYBOARD: ERROR: {irregularity:?}: {event:?}"));
            }
            debug(&format!(
                "KEYBOARD: Pressed state after processing the event: {:?}",
                self.hardware_keyboard
            ));
        } else {
            self.hardware_keyboard.observe(&event);
        }
        self.track_keyboard_modifiers(&event);
        // Widgets that show held modifiers (menu accelerator underlines)
        // rebuild when one is pressed or released.
        if matches!(
            event.code,
            Code::AltLeft
                | Code::AltRight
                | Code::ControlLeft
                | Code::ControlRight
                | Code::ShiftLeft
                | Code::ShiftRight
                | Code::MetaLeft
                | Code::MetaRight
        ) {
            self.frame_requested = true;
        }
        if event.state.is_down() {
            self.editing_diagnostics.key_down_received += 1;
        } else {
            self.editing_diagnostics.key_up_received += 1;
        }

        // A FocusNode may be requested by application code or a FocusScopeNode
        // without going through runtime pointer/Tab handling. Adopt that
        // external selection before routing the event through retained parents.
        self.adopt_external_focus_change();

        let hardware = self.hardware_key_handlers.dispatch(&event);
        for failure in hardware.failures {
            let message = failure
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| failure.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("non-string hardware key handler panic");
            tracing::error!(?event, %message, "hardware key handler failed");
        }
        // A window-wide handler cannot suppress notification of the focused
        // keyboard scope. Both results gate fallback editing and traversal.
        if self.focused.is_some()
            && self.early_focus_key_handlers.dispatch(&event) == KeyEventResult::Handled
        {
            self.frame_requested = true;
            return true;
        }
        // Match RawTooltip's early Escape handler: an open tooltip takes
        // precedence over the ancestor shortcut that would dismiss a route.
        if event.state.is_down()
            && event.key == incular_core::KeyboardKey::Named(incular_core::NamedKey::Escape)
            && self.tree.dismiss_tooltips_on_escape() > 0
        {
            self.frame_requested = true;
            return true;
        }
        let lock_modes = event.modifiers | self.hardware_keyboard.lock_modes_enabled();
        let focus_result =
            self.tree
                .route_keyboard_with_lock_modes(self.focused, event.clone(), lock_modes);
        let late_result = if self.focused.is_some() && focus_result != KeyEventResult::Handled {
            self.late_focus_key_handlers.dispatch(&event)
        } else {
            KeyEventResult::Ignored
        };
        if hardware.handled
            || focus_result == KeyEventResult::Handled
            || late_result == KeyEventResult::Handled
        {
            self.frame_requested = true;
            return true;
        }
        if focus_result == KeyEventResult::SkipRemainingHandlers
            || late_result == KeyEventResult::SkipRemainingHandlers
        {
            self.frame_requested = true;
            return false;
        }
        if !event.state.is_down() {
            return false;
        }
        if event.code == Code::Escape
            && self
                .tree
                .dismiss_transients(incular_widgets::TransientDismissReason::Escape)
                > 0
        {
            self.frame_requested = true;
            return true;
        }
        if matches!(
            event.code,
            Code::ArrowUp | Code::ArrowDown | Code::ArrowLeft | Code::ArrowRight
        ) && let Some(focused) = self.focused
            && let Some(next) = self.tree.menu_directional_focus_target(focused, event.code)
        {
            self.set_focus(Some(next));
            return true;
        }
        // Activate the focused surface itself. Only menus delegate activation
        // from a focused descendant; nested editors retain their own keys.
        if matches!(event.code, Code::Enter | Code::NumpadEnter | Code::Space)
            && let Some(focused) = self.focused
            && let Some((owner, Some(action))) = self.tree.button_ancestor(focused)
            && (owner == focused || self.tree.is_menu_item_focus(focused))
            && {
                // Flutter `InkWell.activateOnIntent`: a confirmed splash at
                // the center.
                self.tree.button_ink(owner, ButtonInk::Activate);
                true
            }
            && self.activate_action(action)
        {
            return true;
        }
        // Flutter's NextFocusIntent/PreviousFocusIntent activators require
        // control, alt and meta to be released; Ctrl+Tab matches nothing.
        if event.code == Code::Tab
            && !event.modifiers.ctrl()
            && !event.modifiers.alt()
            && !event.modifiers.meta()
        {
            self.focus_next(event.modifiers.shift());
            return true;
        }
        if let Some(label) = self.focused.filter(|id| self.tree.is_selectable_text(*id)) {
            let handled = self.run_selectable_text_key(label, &event);
            self.frame_requested |= handled;
            return handled;
        }
        // Text fields keep arrows and paging for their own editing, so the
        // default focus and scroll shortcuts only serve other focus targets.
        if self.focused.is_none_or(|id| !self.tree.is_text_field(id))
            && self.run_default_focus_key(&event)
        {
            self.frame_requested = true;
            return true;
        }
        let Some(field) = self.focused.filter(|id| self.tree.is_text_field(*id)) else {
            return false;
        };
        let Some(controller) = self.tree.text_controller(field) else {
            return false;
        };
        // Flutter's `DefaultTextEditingShortcuts` pick the intent for the
        // chord on the target platform and the field's actions run it.
        if !self.run_text_field_key(field, &controller, &event) {
            return false;
        }
        self.hide_text_field_overlays(field);
        controller.reset_caret(self.input_now());
        self.frame_requested = true;
        true
    }
    /// Flutter's `UndoHistoryState.handlePlatformUndo`: a system undo or redo
    /// command acts on the history of the focused text field, the
    /// `UndoManager.client`, exactly like the undo and redo shortcuts.
    /// Returns whether the history moved; without a focused text field
    /// nothing happens.
    pub fn handle_platform_undo(&mut self, direction: UndoDirection) -> bool {
        // Like keyboard dispatch, resolve pending programmatic focus before
        // choosing the recipient. Native events need not wait for a frame.
        self.adopt_external_focus_change();
        self.advance_text_history_clock(self.input_now());
        let Some(field) = self.focused.filter(|id| self.tree.is_text_field(*id)) else {
            return false;
        };
        let Some(controller) = self.tree.text_controller(field) else {
            return false;
        };
        let intent = match direction {
            UndoDirection::Undo => TextIntent::Undo,
            UndoDirection::Redo => TextIntent::Redo,
        };
        let editable = self.tree.text_field_is_editable(field);
        if !self.run_text_intent(field, &controller, intent, editable) {
            return false;
        }
        self.hide_text_field_overlays(field);
        controller.reset_caret(self.input_now());
        self.frame_requested = true;
        true
    }

    /// Flutter's `WidgetsBinding` back gesture handlers: delivers one stage
    /// of a system predictive back gesture to the back gesture observers,
    /// such as navigators (which register themselves when created), and
    /// requests a frame so a route
    /// following the gesture repaints. Returns whether an observer follows
    /// the gesture. Incular has no system pop-route channel, so a commit
    /// nobody followed (`false`) is left to the host, which reports it as
    /// its ordinary back; Flutter falls back to `handlePopRoute`.
    pub fn handle_back_gesture(&mut self, event: BackGestureEvent) -> bool {
        let followed = incular_widgets::dispatch_back_gesture(&event);
        if followed {
            self.frame_requested = true;
        }
        followed
    }

    /// The system back button (Android's back key or a committed back
    /// gesture), as Flutter handles it: the key goes to the focused widgets
    /// first (`LogicalKeyboardKey.goBack`, [`Code::BrowserBack`] here);
    /// unhandled, it becomes `popRoute`, which closes the topmost popup
    /// (Flutter's popup routes) and otherwise asks the back observers, such
    /// as navigators, to pop ([`incular_widgets::dispatch_pop_route`]).
    /// Returns `false` when nothing used it; the host then leaves the app,
    /// as Flutter's `SystemNavigator.pop` does.
    pub fn handle_back_button(&mut self) -> bool {
        let key = |state| KeyboardEvent {
            state,
            key: incular_core::KeyboardKey::Named(incular_core::NamedKey::BrowserBack),
            code: Code::BrowserBack,
            ..KeyboardEvent::default()
        };
        self.adopt_external_focus_change();
        let handled = self
            .tree
            .dispatch_keyboard(self.focused, key(incular_core::KeyState::Down));
        let _ = self
            .tree
            .dispatch_keyboard(self.focused, key(incular_core::KeyState::Up));
        let handled = handled
            || self.dismiss_transients(incular_widgets::TransientDismissReason::Escape)
            || incular_widgets::dispatch_pop_route();
        if handled {
            self.frame_requested = true;
        }
        handled
    }

    /// Flutter's `UndoHistoryState._updateState` for the platform: on iOS,
    /// the focused text field (`UndoManager.client`) reports its undo and
    /// redo availability when it gains focus and whenever it changes.
    /// Other target platforms report nothing, as in Flutter.
    fn sync_undo_manager_state(&mut self) {
        let client = self
            .focused
            .filter(|id| self.tree.is_text_field(*id))
            .filter(|_| self.text_shortcut_platform == TextShortcutPlatform::Ios);
        let Some(field) = client else {
            self.reported_undo_state = None;
            return;
        };
        let Some(history) = self.ensure_text_history(field) else {
            self.reported_undo_state = None;
            return;
        };
        let state = history.status();
        if self.reported_undo_state == Some((field, state)) {
            return;
        }
        self.reported_undo_state = Some((field, state));
        self.undo_manager_commands
            .push_back(UndoManagerCommand::SetUndoState {
                can_undo: state.can_undo,
                can_redo: state.can_redo,
            });
    }

    /// Whether the field hides its text (Flutter `obscureText`).
    fn text_field_obscured(&self, field: ElementId) -> bool {
        self.tree
            .text_field_input_geometry(field)
            .is_some_and(|snapshot| snapshot.obscure_text)
    }

    /// Flutter's `EditableText._userSelectionEnabled` with the default
    /// `enableInteractiveSelection`: `!readOnly || !obscureText`.
    fn text_field_pointer_selection_enabled(&self, field: ElementId) -> bool {
        self.tree
            .text_field_input_geometry(field)
            .is_none_or(|snapshot| !(snapshot.read_only && snapshot.obscure_text))
    }

    /// Flutter's `copySelection` gate: a collapsed selection or obscured text
    /// copies nothing and leaves the clipboard untouched.
    fn text_field_copy_allowed(
        &self,
        field: ElementId,
        controller: &TextEditingController,
    ) -> bool {
        !controller.value().selection.is_collapsed() && !self.text_field_obscured(field)
    }

    /// Flutter's `cutSelection` gate: read-only or obscured fields and collapsed
    /// selections leave both the clipboard and the text unchanged.
    fn cut_text_field_selection(
        &mut self,
        field: ElementId,
        controller: &TextEditingController,
    ) -> bool {
        if !self.tree.text_field_is_editable(field)
            || !self.text_field_copy_allowed(field, controller)
        {
            return false;
        }
        self.clipboard.set_text(controller.selected_text());
        controller.replace_selection("");
        true
    }

    fn insert_text(&mut self, text: &str) {
        let Some(field) = self.focused.filter(|id| self.tree.is_text_field(*id)) else {
            return;
        };
        if !self.tree.text_field_is_editable(field) {
            return;
        }
        if text.chars().any(char::is_control) {
            return;
        }
        if let Some(controller) = self.tree.text_controller(field) {
            controller.insert(text);
            self.editing_diagnostics.text_commits += 1;
            controller.reset_caret(self.input_now());
            self.frame_requested = true;
            // Flutter `updateEditingValue`: a text change, unlike a selection-only
            // update, closes the toolbar.
            self.hide_text_field_overlays(field);
        }
    }

    /// Handles the action chosen by a native keyboard. Multiline input
    /// arrives separately as editing state; a newline action is only its
    /// notification, not another edit.
    pub fn handle_text_input_action(&mut self, action: TextInputAction) -> bool {
        let Some(field) = self.focused.filter(|id| self.tree.is_text_field(*id)) else {
            return false;
        };
        let editable = self.tree.text_field_is_editable(field);
        if !editable {
            return false;
        }
        let handled = match action {
            TextInputAction::Newline => {
                if self.tree.is_multiline_text_field(field) {
                    return true;
                } else {
                    self.finish_text_editing(field)
                }
            }
            TextInputAction::Next | TextInputAction::Previous => {
                if !self.tree.text_field_has_editing_complete(field) {
                    if let Some(controller) = self.tree.text_controller(field) {
                        controller.set_composing(None);
                    }
                    self.focus_next(action == TextInputAction::Previous);
                }
                self.tree.submit_text_field(field)
            }
            TextInputAction::Done
            | TextInputAction::Go
            | TextInputAction::Search
            | TextInputAction::Send => self.finish_text_editing(field),
            TextInputAction::Unspecified | TextInputAction::None => false,
        };
        if handled {
            self.frame_requested = true;
        }
        handled
    }

    /// Flutter's `_finalizeEditing` for a completion action. Without an
    /// `on_editing_complete` callback the field gives up focus first, then
    /// `on_submit` runs; with one, the callback runs before `on_submit` and
    /// focus is left alone.
    pub(crate) fn finish_text_editing(&mut self, field: ElementId) -> bool {
        if !self.tree.text_field_has_editing_complete(field) {
            self.set_focus(None);
        }
        self.tree.submit_text_field(field)
    }
    /// Drops the open composition, clearing the owner's stale preedit.
    /// Single cleanup path: the owner's own controller is the only
    /// writer, so cleanup never lands in another editor. Clearing an
    /// unset preedit notifies nothing.
    fn cancel_ime_composition(&mut self) {
        let owner = self.ime_composition.take();
        if let Some(owner) = owner {
            owner.controller.clear_preedit();
        }
    }

    /// Whether the retained owner still names the live editing target:
    /// the focused field, with the same controller identity, still
    /// editable. Anything else means the composition's client is gone.
    fn ime_owner_current(&self) -> bool {
        self.ime_composition.as_ref().is_some_and(|owner| {
            self.focused == Some(owner.field)
                && self.tree.is_text_field(owner.field)
                && self.tree.text_field_is_editable(owner.field)
                && self
                    .tree
                    .text_controller(owner.field)
                    .is_some_and(|current| current == owner.controller)
        })
    }

    fn ime_focused_target(&self) -> Option<ImeCompositionOwner> {
        let field = self
            .focused
            .filter(|id| self.tree.text_field_is_editable(*id))?;
        Some(ImeCompositionOwner {
            field,
            controller: self.tree.text_controller(field)?,
        })
    }

    fn validate_ime_composition(&mut self) {
        if !self.ime_owner_current() {
            self.cancel_ime_composition();
        }
    }

    /// Routes one native IME event. An owner that no longer validates
    /// (focus moved off-path, controller replaced, field unmounted or
    /// no longer editable) is cancelled before routing, so its preedit
    /// clears on its own controller and is never committed elsewhere.
    /// Owned events address the owner; only ownerless Commits fall back
    /// to the focused field, and only ownerless Preedits adopt a target.
    fn handle_ime(&mut self, event: ImeEvent) {
        self.editing_diagnostics.ime_events += 1;
        self.validate_ime_composition();
        let target = match &event {
            ImeEvent::Preedit { .. } => {
                if self.ime_composition.is_none() {
                    self.ime_composition = self.ime_focused_target();
                }
                self.ime_composition
                    .as_ref()
                    .map(|owner| owner.controller.clone())
            }
            ImeEvent::Commit(_) => self
                .ime_composition
                .take()
                .or_else(|| self.ime_focused_target())
                .map(|owner| owner.controller),
            ImeEvent::End => self.ime_composition.take().map(|owner| owner.controller),
        };
        let Some(controller) = target else {
            return;
        };
        match event {
            ImeEvent::Preedit { text, selection } => {
                controller.set_preedit(
                    text,
                    selection.map(|(start, end)| TextRange::new(start, end)),
                );
            }
            ImeEvent::Commit(text) => {
                // Ownerless commit policy: without a session or client
                // identity, a late Commit after focus changes cannot be
                // distinguished from a legitimate direct commit, so it
                // applies to the focused editable field. This is a routing
                // policy, not stale-commit protection.
                controller.commit_preedit(&text);
            }
            ImeEvent::End => {
                controller.clear_preedit();
            }
        }
        controller.reset_caret(self.input_now());
        self.frame_requested = true;
    }

    /// Flutter `EditableTextState.updateEditingValue`: applies the input
    /// method's whole editing state to the focused field as one user edit.
    /// The state becomes the last one the platform knows, so the next
    /// [`TextInputCommand::Update`] goes out only when an input formatter
    /// changed it.
    pub fn handle_text_editing_state(&mut self, state: TextInputState) {
        self.editing_diagnostics.ime_events += 1;
        self.cancel_ime_composition();
        let Some(owner) = self.ime_focused_target() else {
            return;
        };
        let controller = owner.controller;
        let mut value = incular_text::TextEditingValue::new(state.text);
        value.selection =
            TextSelection::new(state.selection_start, state.selection_end).clamp_to(&value.text);
        value.composing = state
            .composing
            .map(|(start, end)| incular_text::TextRange::new(start, end).clamp_to(&value.text));
        // Flutter `updateEditingValue`: a change of text or composing range hides
        // the toolbar; a selection-only change does not.
        let text_changed = controller.with_value(|current| {
            current.text != value.text || current.composing != value.composing
        });
        let delta = controller.with_value(|current| {
            (current != &value)
                .then(|| incular_text::TextEditingDelta::between(&current.text, &value))
        });
        if let Some(delta) = delta {
            controller.apply_delta(&delta);
        }
        if text_changed {
            self.hide_text_field_overlays(owner.field);
        }
        if self.text_input_client.is_some() {
            self.text_input_state = Some(TextInputState {
                selection_start: value.selection.base,
                selection_end: value.selection.extent,
                composing: value.composing.map(|range| (range.start, range.end)),
                text: value.text,
            });
        }
        controller.reset_caret(self.input_now());
        self.frame_requested = true;
    }
    pub fn run_frame(
        &mut self,
        constraints: Constraints,
    ) -> Result<(DisplayList, FrameStats), TreeError> {
        self.run_frame_at(constraints, Instant::now())
    }
    /// Reserves one warm-up begin/draw pair on this UI thread. Duplicate
    /// requests and requests during an active frame are ignored.
    ///
    /// The next ordinary runtime frame drives both phases automatically.
    /// A host with separate turns can instead call the begin and draw
    /// methods directly. Prioritized UI tasks wait until draw completes.
    pub fn schedule_warm_up_frame(&mut self) -> bool {
        if self.warm_up_frame.is_some() {
            return false;
        }
        self.warm_up_frame = incular_animation::scheduler::WarmUpFrame::reserve();
        self.warm_up_frame.is_some()
    }

    /// Starts the reserved warm-up frame's transient callbacks once.
    pub fn begin_warm_up_frame_at(&mut self, now: Instant) -> bool {
        self.warm_up_frame
            .as_mut()
            .is_some_and(|frame| frame.begin(now))
    }

    /// Drains microtasks, then builds, lays out, paints, and finishes a
    /// warm-up frame that has begun. Returns None before its begin phase.
    pub fn draw_warm_up_frame(
        &mut self,
        constraints: Constraints,
    ) -> Result<Option<(DisplayList, FrameStats)>, TreeError> {
        let Some(now) = self
            .warm_up_frame
            .as_ref()
            .and_then(|frame| frame.began_at())
        else {
            return Ok(None);
        };
        let frame = self.warm_up_frame.take().expect("begun warm-up frame");
        let result = self.run_frame_mode(constraints, now, true);
        match result {
            Ok((list, mut stats)) => {
                frame.complete();
                stats.requested_another_frame = self.frame_requested();
                Ok(Some((list, stats)))
            }
            Err(error) => Err(error),
        }
    }

    /// Deterministic frame entry point used by animation tests and embedders
    /// with an existing monotonic clock.
    pub fn run_frame_at(
        &mut self,
        constraints: Constraints,
        now: Instant,
    ) -> Result<(DisplayList, FrameStats), TreeError> {
        if self.warm_up_frame.is_some() {
            self.begin_warm_up_frame_at(now);
            return self
                .draw_warm_up_frame(constraints)
                .map(|frame| frame.expect("warm-up frame starts before draw"));
        }
        if self.defer_native_frame_during_warm_up() {
            return Err(TreeError::WarmUpFrameInProgress);
        }
        self.run_frame_mode(constraints, now, false)
    }

    /// Consumes a colliding host redraw without losing its demand. Repeated
    /// collisions coalesce, and [Self::frame_requested] exposes the retry
    /// only after the warm-up owner's draw releases the reservation.
    pub(crate) fn defer_native_frame_during_warm_up(&mut self) -> bool {
        if !incular_animation::scheduler::warm_up_frame_pending() {
            return false;
        }
        self.frame_requested = true;
        true
    }

    fn run_frame_mode(
        &mut self,
        constraints: Constraints,
        now: Instant,
        warm_up: bool,
    ) -> Result<(DisplayList, FrameStats), TreeError> {
        let message_guard = tracing::info_span!("incular.messages").entered();
        let message_span = profiling::PhaseSpan::start();
        // Writes made during the previous build invalidate their dependents
        // now, so they rebuild in this frame.
        crate::environment::flush_deferred_invalidations();
        self.advance_text_history_clock(now);
        self.sync_mounted_text_histories();
        if !warm_up {
            self.process_runtime_work_at(now);
            if self.lifecycle != ApplicationLifecycle::Suspended {
                // Ordinary prioritized tasks yield between frames.
                let _ = incular_animation::scheduler::run_one_scheduled_task();
            }
            incular_animation::scheduler::begin_frame(now);
        }
        incular_animation::scheduler::flush_frame_microtasks();
        // The controllers that builds read tick here too, so their new values
        // build in this frame rather than the next.
        self.tree.tick_animations(now);
        // Deadline callbacks can invalidate widgets. Deliver them before
        // rebuilding so a stationary press shows a tooltip on the due frame.
        let gesture_timers_live = self.tree.pump_gesture_timers(now);
        let tooltip_timers_live = self.tree.pump_tooltip_timers(now);
        let runtime_messages = message_span.elapsed_us();
        drop(message_guard);
        let before = self.tree.diagnostics();
        let build_guard =
            tracing::info_span!("incular.build", elements_updated = tracing::field::Empty,)
                .entered();
        let build_span = profiling::PhaseSpan::start();
        let focused_at_start = self.focused;
        let mut updated = 0;
        while let Some(id) = self.order.pop_front() {
            if let Some(widget) = self.pending.remove(&id)
                && self.tree.element_exists(id)
            {
                self.tree.update(id, widget)?;
                updated += 1;
            }
        }
        loop {
            while let Some((root, node)) = { self.reactive.borrow_mut().take_node() } {
                if let Some(node) = node.upgrade() {
                    node.run(root);
                }
            }
            while let Some(id) = { self.reactive.borrow_mut().take() } {
                if self.tree.element_exists(id) && self.builders.contains_key(&id) {
                    #[cfg(feature = "devtools")]
                    for cause in self.reactive.borrow_mut().take_causes(id) {
                        self.tree.note_invalidation(id, cause);
                    }
                    self.rebuild_from_builder(id)?;
                    updated += 1;
                }
            }
            if !self.reactive.borrow().has_work() {
                break;
            }
        }
        // `FocusNode::request_focus` / `unfocus` from application code (a
        // callback, a timer, a build) take effect on the next frame.
        self.adopt_external_focus_change();
        for id in self.tree.take_unmounted() {
            self.handlers_stale = true;
            self.builders.remove(&id);
            self.reactive.borrow_mut().forget(id);
            if let Some(scope) = self.owner_scopes.remove(&id) {
                scope.cancel();
            }
            self.text_histories.remove(&id);
        }
        self.reconcile_focus_and_selection_captures();
        self.validate_ime_composition();
        self.prune_handlers();
        let build = build_span.elapsed_us();
        drop(build_guard);
        // Ballistic tenures pump on the frame clock before layout, so
        // pumped offsets lay out and paint in the same frame. The pump
        // reports live drivers; that liveness is captured here and
        // ORed into the end-of-frame demand below, so the reset point
        // cannot discard it — the scheduler stays awake while a driver
        // lives and idles otherwise. No timer loop, just frame demand.
        let reduced_motion = self.environment.borrow().reduced_motion;
        let fling_live = self.tree.pump_scroll_flings(now, reduced_motion);
        // The overlay scrollbar's reveal/fade-out follows scrolling and hover,
        // so it steps after flings and before paint.
        let scrollbar_live = self.tree.pump_scrollbar_fades(now);
        let text_long_press_live = self.pump_text_long_press(now);
        let _layout_guard = tracing::info_span!("incular.layout").entered();
        let layout_span = profiling::PhaseSpan::start();
        let focus_attachments_before_layout = self.tree.pending_focus_attachment_revision();
        self.tree.layout(constraints)?;
        let layout = layout_span.elapsed_us();
        drop(_layout_guard);
        for (action, handler) in self.tree.take_pending_handlers() {
            self.handlers_stale = true;
            self.handlers.insert(action, handler);
        }
        // Lazy viewport expiry occurs during layout, after the ordinary dirty
        // queue drain above. Release those builder subscriptions and callbacks
        // in the same frame rather than retaining one stale cache generation.
        for id in self.tree.take_unmounted() {
            self.handlers_stale = true;
            self.builders.remove(&id);
            self.reactive.borrow_mut().forget(id);
            if let Some(scope) = self.owner_scopes.remove(&id) {
                scope.cancel();
            }
            self.text_histories.remove(&id);
        }
        self.reconcile_focus_and_selection_captures();
        self.validate_ime_composition();
        self.prune_handlers();
        // A request on a node first attached during layout is now actionable.
        // Honor it before autofocus, without scanning on every clean frame.
        if self.tree.pending_focus_attachment_revision() != focus_attachments_before_layout {
            self.adopted_focus_epoch = None;
            self.adopt_external_focus_change();
        }
        // Layout builders create their retained controls during the first
        // layout. Resolve initial autofocus once those controls exist, without
        // reclaiming focus on later frames or after an explicit input decision.
        if self.initial_focus == InitialFocus::AwaitingLayout {
            self.set_focus_with_external_selection(self.tree.autofocus_element(), false);
        }
        let replaces_detached_focus = self.focused.is_none()
            && focused_at_start.is_some_and(|focused| !self.tree.element_exists(focused));
        if let Some(next) = self.tree.finish_mounted_autofocus(replaces_detached_focus) {
            self.set_focus_with_external_selection(Some(next), false);
        }
        let _composite_guard = tracing::info_span!("incular.composite").entered();
        let composite_span = profiling::PhaseSpan::start();
        let (composited, animations_active) = self.tree.update_compositor(now)?;
        let composite = composite_span.elapsed_us();
        drop(_composite_guard);
        // Layout and the compositor's scroll translations are final here,
        // so hover catches up with content that moved under the pointer
        // and its paint lands in this frame.
        let geometry = self.tree.diagnostics();
        let hit_revision = self.tree.hover_hit_revision();
        if geometry.layouts != before.layouts
            || geometry.scroll_offset_updates != before.scroll_offset_updates
            || self.hover_hit_revision != hit_revision
        {
            self.hover_hit_revision = hit_revision;
            self.refresh_hover_after_layout();
        }
        self.sync_text_selection_handles();
        let _semantics_guard = tracing::info_span!("incular.semantics").entered();
        let semantics_span = profiling::PhaseSpan::start();
        self.tree.update_semantics();
        let semantics = semantics_span.elapsed_us();
        drop(_semantics_guard);
        let _paint_guard = tracing::info_span!("incular.paint").entered();
        let paint_span = profiling::PhaseSpan::start();
        let display_list = self.tree.try_paint()?;
        let paint = paint_span.elapsed_us();
        drop(_paint_guard);
        // Flutter `RenderView._updateSystemChrome`: the paint flattened the
        // compositor, so its annotations describe this frame.
        self.update_system_ui_overlay_style_from_annotations();
        self.sync_text_input_client();
        self.sync_mounted_text_histories();
        self.sync_undo_manager_state();
        self.sync_platform_views();
        // A focused field whose value or selection changed scrolls its
        // ancestors to show the caret; the next frame lays that out.
        let caret_revealed = self.show_focused_caret_on_screen();
        // End-of-frame reset: recomputed from every demand source, not
        // blindly cleared. `fling_live` above is one source — a write
        // before layout alone would not survive this assignment, so it
        // is ORed back in. Nothing else in this frame clears the flag:
        // intermediate paths only ever set it.
        self.frame_requested = fling_live
            || caret_revealed
            || gesture_timers_live
            || tooltip_timers_live
            || text_long_press_live
            || scrollbar_live
            || !self.pending.is_empty()
            || self.reactive.borrow().has_work()
            || crate::environment::has_deferred_invalidations()
            || animations_active;
        // Flutter `addPostFrameCallback`: after layout and paint.
        incular_animation::scheduler::end_frame(now);
        let after = self.tree.diagnostics();
        let timings = FrameTimings {
            event_processing: std::mem::take(&mut self.pending_event_processing_us),
            runtime_messages,
            build,
            layout,
            composite,
            semantics,
            paint,
            cpu_total: runtime_messages + build + layout + composite + semantics + paint,
        };
        tracing::debug!(
            target: "incular::frame",
            cpu_total_us = timings.cpu_total,
            build_us = timings.build,
            layout_us = timings.layout,
            paint_us = timings.paint,
        );
        Ok((
            display_list.clone(),
            FrameStats {
                updated_elements: updated,
                rebuilt_elements: after.rebuilds - before.rebuilds,
                laid_out_render_objects: after.layouts - before.layouts,
                repainted_render_objects: after.paints - before.paints,
                composited: u64::from(composited),
                active_animations: u64::from(animations_active),
                display_list_commands: display_list.len(),
                requested_another_frame: self.frame_requested || self.tree.has_dirty_dependencies(),
                timings,
            },
        ))
    }
    #[must_use]
    pub fn diagnostics(&self) -> Diagnostics {
        self.tree.diagnostics()
    }
    /// Queues every registered builder for the next frame's build drain,
    /// application root first so descendants it unmounts are skipped.
    #[cfg(feature = "hot-reload")]
    pub(crate) fn reassemble(&mut self) {
        // Hot reload shows changed enter animations right away.
        incular_widgets::motion::replay_enter_animations();
        let root = self.application_root;
        let mut reactive = self.reactive.borrow_mut();
        for id in root
            .into_iter()
            .chain(self.builders.keys().copied().filter(|id| Some(*id) != root))
        {
            #[cfg(feature = "devtools")]
            reactive.note_cause(id, InvalidationCause::Manual);
            reactive.enqueue(id);
        }
        drop(reactive);
        self.frame_requested = true;
    }
    /// Rebuilds one registered builder outside the frame drain. Route
    /// outlets use this to refresh mounted content deterministically
    /// before framing, rather than only when reactive dependencies fire.
    pub(crate) fn rebuild_from_builder(&mut self, id: ElementId) -> Result<(), TreeError> {
        self.reactive.borrow_mut().refresh(id);
        let widget = {
            let _build_scope = BuildScopeGuard::enter(BuildScope {
                root: self.reactive.borrow().root,
                element: Some(id),
                queue: Rc::downgrade(&self.reactive),
                initial_dependencies: None,
                spawner: self.spawner(),
                owner_scope: self
                    .owner_scopes
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| self.window_scope.clone()),
            });
            self.builders.get_mut(&id).expect("registered builder")()
        };
        let mut widget = widget;
        self.prepare_widget(&mut widget);
        self.tree.update(id, widget)?;
        Ok(())
    }
    fn prepare_widget(&mut self, widget: &mut Widget) {
        self.handlers_stale = true;
        let handlers = &mut self.handlers;
        let tree = &mut self.tree;
        widget.bind_callbacks(&mut |callback| {
            let id = tree.allocate_action();
            handlers.insert(id, callback);
            id
        });
    }
    fn activate_action(&mut self, action: ActionId) -> bool {
        let mut handled = false;
        if let Some(callback) = self.handlers.get(&action).cloned() {
            callback();
            handled = true;
        }
        if let Some(callback) = self.legacy_action_handler.as_mut() {
            callback(action);
            handled = true;
        }
        self.frame_requested |= handled;
        handled
    }

    fn prune_handlers(&mut self) {
        if !std::mem::take(&mut self.handlers_stale) {
            return;
        }
        let active = self.tree.action_ids();
        self.handlers.retain(|id, _| active.contains(id));
    }
    fn set_hover(&mut self, next: Option<ElementId>) {
        if self.hovered_button == next {
            return;
        }
        if let Some(previous) = self.hovered_button {
            let _ = self
                .tree
                .set_button_interaction(previous, Some(false), None, None);
            if let Some(action) = self.tree.hover_actions_for_element(previous).1
                && let Some(callback) = self.handlers.get(&action).cloned()
            {
                callback();
            }
        }
        self.hovered_button = next;
        if let Some(current) = next {
            if self.pressed_button != Some(current) {
                let _ = self
                    .tree
                    .set_button_interaction(current, Some(true), None, None);
            }
            if let Some(action) = self.tree.hover_actions_for_element(current).0
                && let Some(callback) = self.handlers.get(&action).cloned()
            {
                callback();
            }
        }
        self.frame_requested = true;
    }
}

/// Framework-controlled build scope for declarative roots. It intentionally
/// exposes no element IDs or scheduler handles. Signals use the runtime's
/// scoped collector while a builder executes; reads outside a build simply
/// do not create dependencies.
fn us_since_instant(started: incular_core::time::Instant) -> u32 {
    started.elapsed().as_micros().try_into().unwrap_or(u32::MAX)
}

/// A platform view the runtime has seen mounted.
pub(crate) struct PlatformViewRecord {
    controller: incular_widgets::PlatformViewController,
    /// The last `Place` command sent.
    placed: Option<PlatformViewCommand>,
    /// Whether the view was in the tree at the last frame.
    mounted: bool,
}

impl Runtime {
    /// Flutter's platform view render objects report their size and offset
    /// to the host after layout (`resize` and `offset`). Collects each
    /// mounted view's commands, then sends a `Place` for every view whose
    /// placement changed, and hides views that left the tree.
    fn sync_platform_views(&mut self) {
        let placements = self.tree.platform_view_placements();
        for record in self.platform_views.values_mut() {
            record.mounted = false;
        }
        for placement in placements {
            let id = placement.controller.id();
            let place = PlatformViewCommand::Place {
                id,
                bounds: placement.bounds,
                visible_bounds: placement
                    .visible_bounds
                    .unwrap_or(Rect::from_origin_size(placement.bounds.origin, Size::ZERO)),
                visible: placement.visible_bounds.is_some(),
            };
            let record = self
                .platform_views
                .entry(id)
                .or_insert_with(|| PlatformViewRecord {
                    controller: placement.controller.clone(),
                    placed: None,
                    mounted: true,
                });
            record.mounted = true;
            // Creation precedes the first placement.
            self.platform_view_commands
                .extend(record.controller.take_commands());
            if record.controller.is_disposed() || record.placed.as_ref() == Some(&place) {
                continue;
            }
            record.placed = Some(place.clone());
            self.platform_view_commands.push_back(place);
        }
        for record in self.platform_views.values_mut() {
            if record.mounted {
                continue;
            }
            self.platform_view_commands
                .extend(record.controller.take_commands());
            if let Some(PlatformViewCommand::Place {
                id,
                bounds,
                visible_bounds,
                visible: true,
            }) = record.placed
                && !record.controller.is_disposed()
            {
                let hidden = PlatformViewCommand::Place {
                    id,
                    bounds,
                    visible_bounds,
                    visible: false,
                };
                record.placed = Some(hidden.clone());
                self.platform_view_commands.push_back(hidden);
            }
        }
    }

    /// Moves the commands queued on every known controller since the last
    /// frame, such as forwarded pointers, into the window's queue.
    fn collect_platform_view_commands(&mut self) {
        for record in self.platform_views.values() {
            self.platform_view_commands
                .extend(record.controller.take_commands());
        }
    }
}
