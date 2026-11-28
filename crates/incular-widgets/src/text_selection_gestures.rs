//! Gesture arbitration and callbacks used by text-selection providers.

use std::rc::Rc;

use incular_config::TargetPlatform;
use incular_core::{DragStartBehavior, PointerDeviceKind};
use incular_gestures::{
    ArenaGestureRecognizer, ForcePressDetails, ForcePressGestureRecognizer, GestureCallbacks,
    GestureRecognizerFactoryWithHandlers, LongPressEndDetails, LongPressGestureRecognizer,
    LongPressMoveUpdateDetails, LongPressStartDetails, PointerGestureRecognizer,
    TapAndDragCallbacks, TapAndDragGestureRecognizer, TapDownDetails, TapDragDownDetails,
    TapDragEndDetails, TapDragStartDetails, TapDragUpDetails, TapDragUpdateDetails,
};

use crate::{EditableText, HitTestBehavior, RawGestureDetector, Widget};

type EventHandler<T> = Option<Rc<dyn Fn(T)>>;
type EventSignal = Option<Rc<dyn Fn()>>;

/// Optional text selection gesture handlers. The detector arbitrates and
/// reports gestures but never mutates the selection or owns focus.
#[derive(Clone, Default)]
pub struct TextSelectionGestureCallbacks {
    pub on_tap_track_start: EventSignal,
    pub on_tap_track_reset: EventSignal,
    pub on_tap_down: EventHandler<TapDragDownDetails>,
    pub on_single_tap_up: EventHandler<TapDragUpDetails>,
    pub on_single_tap_cancel: EventSignal,
    pub on_double_tap_down: EventHandler<TapDragDownDetails>,
    pub on_triple_tap_down: EventHandler<TapDragDownDetails>,
    pub on_secondary_tap: EventSignal,
    pub on_secondary_tap_down: EventHandler<TapDownDetails>,
    pub on_single_long_tap_start: EventHandler<LongPressStartDetails>,
    pub on_single_long_tap_move: EventHandler<LongPressMoveUpdateDetails>,
    pub on_single_long_tap_end: EventHandler<LongPressEndDetails>,
    pub on_single_long_tap_cancel: EventSignal,
    pub on_drag_selection_start: EventHandler<TapDragStartDetails>,
    pub on_drag_selection_update: EventHandler<TapDragUpdateDetails>,
    pub on_drag_selection_end: EventHandler<TapDragEndDetails>,
    pub on_force_press_start: EventHandler<ForcePressDetails>,
    pub on_force_press_end: EventHandler<ForcePressDetails>,
    pub on_user_tap: EventSignal,
    pub on_user_tap_always_called: bool,
}

/// Installs the standard text-selection policy around an editable renderer.
/// The retained editor owns the policy, so adding it does not add an extra
/// node or layout pass. Use [TextSelectionGestureDetector] to supply custom
/// gesture handlers instead of the standard policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct TextSelectionGestureDetectorBuilder {
    force_press_enabled: bool,
}

impl TextSelectionGestureDetectorBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            force_press_enabled: false,
        }
    }

    /// Whether pressure-capable contacts may select words and show a
    /// toolbar. Ordinary touch and mouse selection remain available.
    #[must_use]
    pub const fn force_press_enabled(mut self, enabled: bool) -> Self {
        self.force_press_enabled = enabled;
        self
    }

    /// Uses the editor's renderer pointer policy to arbitrate between the
    /// renderer tap and the builder's tap-and-drag detector. An editor with
    /// `renderer_ignores_pointer(true)` gives the builder sole ownership.
    #[must_use]
    pub fn build_gesture_detector(self, editor: EditableText) -> EditableText {
        editor.with_selection_gesture_builder(self.force_press_enabled)
    }
}

/// Flutter's design-system-neutral text-selection gesture detector.
/// It can wrap an editor with its own pointer recognizers disabled.
#[derive(Clone)]
pub struct TextSelectionGestureDetector {
    child: Widget,
    callbacks: TextSelectionGestureCallbacks,
    behavior: HitTestBehavior,
    platform: TargetPlatform,
}

impl TextSelectionGestureDetector {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            callbacks: TextSelectionGestureCallbacks::default(),
            behavior: HitTestBehavior::DeferToChild,
            platform: TargetPlatform::current(),
        }
    }

    #[must_use]
    pub fn callbacks(mut self, callbacks: TextSelectionGestureCallbacks) -> Self {
        self.callbacks = callbacks;
        self
    }

    #[must_use]
    pub fn behavior(mut self, behavior: HitTestBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    #[must_use]
    pub fn platform(mut self, platform: TargetPlatform) -> Self {
        self.platform = platform;
        self
    }
}

fn effective_tap_count(platform: TargetPlatform, count: u32) -> u32 {
    match platform {
        TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::Linux => {
            if count <= 3 {
                count
            } else {
                (count - 1) % 3 + 1
            }
        }
        TargetPlatform::Ios | TargetPlatform::MacOs => count.min(3),
        TargetPlatform::Windows => {
            if count < 2 {
                count
            } else {
                2 + count % 2
            }
        }
    }
}

impl From<TextSelectionGestureDetector> for Widget {
    fn from(value: TextSelectionGestureDetector) -> Self {
        let c = value.callbacks;
        let platform = value.platform;
        let mut raw = RawGestureDetector::new(value.child)
            .behavior(value.behavior)
            .exclude_from_semantics(true);

        if c.on_secondary_tap.is_some() || c.on_secondary_tap_down.is_some() {
            let mut callbacks = GestureCallbacks::default();
            callbacks.secondary.on_tap = c.on_secondary_tap;
            callbacks.secondary.on_tap_down = c.on_secondary_tap_down;
            raw = raw.gesture(GestureRecognizerFactoryWithHandlers::new(
                || PointerGestureRecognizer::new(GestureCallbacks::default()),
                move |recognizer| recognizer.set_callbacks(callbacks.clone()),
            ));
        }

        if c.on_single_long_tap_start.is_some()
            || c.on_single_long_tap_move.is_some()
            || c.on_single_long_tap_end.is_some()
            || c.on_single_long_tap_cancel.is_some()
        {
            let mut callbacks = GestureCallbacks {
                on_long_press_start: c.on_single_long_tap_start,
                on_long_press_move_update: c.on_single_long_tap_move,
                on_long_press_end: c.on_single_long_tap_end,
                on_long_press_cancel: c.on_single_long_tap_cancel,
                ..Default::default()
            };
            callbacks.set_supported_devices([PointerDeviceKind::Touch]);
            raw = raw.gesture(GestureRecognizerFactoryWithHandlers::new(
                || {
                    ArenaGestureRecognizer::new(LongPressGestureRecognizer::new(
                        GestureCallbacks::default(),
                    ))
                },
                move |recognizer| recognizer.recognizer_mut().set_callbacks(callbacks.clone()),
            ));
        }

        if c.on_drag_selection_start.is_some()
            || c.on_drag_selection_update.is_some()
            || c.on_drag_selection_end.is_some()
        {
            let on_down = move |event: TapDragDownDetails| {
                if let Some(callback) = &c.on_tap_down {
                    callback(event);
                }
                let handler = match effective_tap_count(platform, event.consecutive_tap_count) {
                    2 => &c.on_double_tap_down,
                    3 => &c.on_triple_tap_down,
                    _ => return,
                };
                if let Some(callback) = handler {
                    callback(event);
                }
            };
            let on_up = move |event: TapDragUpDetails| {
                let single = effective_tap_count(platform, event.consecutive_tap_count) == 1;
                if single && let Some(callback) = &c.on_single_tap_up {
                    callback(event);
                }
                if (single || c.on_user_tap_always_called)
                    && let Some(callback) = &c.on_user_tap
                {
                    callback();
                }
            };
            let callbacks = TapAndDragCallbacks {
                on_tap_down: Some(Rc::new(on_down)),
                on_tap_up: Some(Rc::new(on_up)),
                on_drag_start: c.on_drag_selection_start,
                on_drag_update: c.on_drag_selection_update,
                on_drag_end: c.on_drag_selection_end,
                on_cancel: c.on_single_tap_cancel,
                on_tap_track_start: c.on_tap_track_start,
                on_tap_track_reset: c.on_tap_track_reset,
            };
            raw = raw.gesture(GestureRecognizerFactoryWithHandlers::new(
                move || {
                    let recognizer = if platform.is_desktop() {
                        TapAndDragGestureRecognizer::pan()
                    } else {
                        TapAndDragGestureRecognizer::horizontal_drag()
                    };
                    ArenaGestureRecognizer::new(
                        recognizer
                            .drag_start_behavior(DragStartBehavior::Down)
                            .eager_victory_on_drag(platform != TargetPlatform::Ios),
                    )
                },
                move |recognizer| recognizer.recognizer_mut().set_callbacks(callbacks.clone()),
            ));
        }

        if c.on_force_press_start.is_some() || c.on_force_press_end.is_some() {
            raw = raw.gesture(GestureRecognizerFactoryWithHandlers::new(
                || ArenaGestureRecognizer::new(ForcePressGestureRecognizer::new()),
                move |recognizer| {
                    recognizer.recognizer_mut().set_start_end_callbacks(
                        c.on_force_press_start.clone(),
                        c.on_force_press_end.clone(),
                    )
                },
            ));
        }
        raw.into()
    }
}
