//! Flutter-style gesture detection, pointer routing, and hover widgets.

use std::rc::Rc;

pub use incular_core::{DragStartBehavior, Offset, PointerPhase, TrackpadGesture};
pub use incular_gestures::{
    ActionResult, Actions, Command, DragCallbacks, DragDownDetails, DragEndDetails,
    DragGestureDetector, DragStartDetails, DragUpdateDetails, ErasedActionScope, ErasedIntent,
    ErasedShortcutScope, FlingLimits, FocusBehavior, FocusHighlightManager, FocusHighlightMode,
    FocusHighlightStrategy, ForcePressDetails, ForcePressGestureRecognizer, GestureAction,
    GestureArena, GestureArenaEntry, GestureArenaKey, GestureArenaMember, GestureCallbacks,
    GestureDecision, GestureDisposition, GestureRecognizer, LongPressDownDetails,
    LongPressEndDetails, LongPressGestureRecognizer, LongPressMoveUpdateDetails, LongPressStartDetails, PointerButtonClass,
    PointerDeviceKind, PointerEvent, PointerGestureRecognizer, ScaleEndDetails,
    ScaleGestureDetector, ScaleStartDetails, ScaleUpdateDetails, ShortcutKey, ShortcutMatch,
    Shortcuts, TapDownDetails, TapMoveDetails, TapUpDetails, Velocity,
};

pub use incular_gestures::{
    PointerSignalError, PointerSignalEvent, PointerSignalResolver as StandalonePointerSignalResolver,
};

use crate::{Widget, WidgetKind, macros::widget_from_kind};

/// How a gesture detector behaves during hit testing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HitTestBehavior {
    #[default]
    DeferToChild,
    Opaque,
    Translucent,
}

/// A first-class gesture detection widget with callbacks for tap, double-tap,
/// long-press, pan, drag, and pinch-to-scale.
#[derive(Clone, Default)]
pub struct GestureDetector {
    callbacks: GestureCallbacks,
    semantic_policy: crate::tree::GestureSemanticPolicy,
    behavior: Option<HitTestBehavior>,
    child: Option<Widget>,
}

impl GestureDetector {
    /// Creates a GestureDetector wrapping a child.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            callbacks: GestureCallbacks::default(),
            semantic_policy: Default::default(),
            behavior: None,
            child: Some(child.into()),
        }
    }

    /// Limits new pointer presses and trackpad gestures to these devices.
    /// Without an allowlist, all pointer devices are eligible.
    #[must_use]
    pub fn supported_devices(mut self, kinds: impl IntoIterator<Item = PointerDeviceKind>) -> Self {
        self.callbacks.set_supported_devices(kinds);
        self
    }

    /// Excludes the detector's gestures from accessibility without disabling pointer handling.
    #[must_use]
    pub fn exclude_from_semantics(mut self, exclude: bool) -> Self {
        self.semantic_policy.included = !exclude;
        self
    }

    /// Restricts which of the installed gesture callbacks accessibility may invoke.
    /// Omit this setting to expose every installed semantic gesture.
    #[must_use]
    pub fn valid_semantic_actions(
        mut self,
        actions: impl IntoIterator<Item = incular_semantics::SemanticActionKind>,
    ) -> Self {
        self.semantic_policy.set_allowed(actions);
        self
    }

    /// Fraction of the detector's width or height used by a semantic drag.
    /// Defaults to 0.8; a nonfinite factor restores the default.
    #[must_use]
    pub fn semantic_scroll_factor(mut self, factor: f32) -> Self {
        self.semantic_policy.scroll_factor = if factor.is_finite() { factor } else { 0.8 };
        self
    }

    /// Sets where drags and scales measure their start from (Flutter
    /// `GestureDetector.dragStartBehavior`, default
    /// [`DragStartBehavior::Start`]).
    #[must_use]
    pub fn drag_start_behavior(mut self, behavior: DragStartBehavior) -> Self {
        self.callbacks.drag_start_behavior = behavior;
        self
    }

    /// Sets the on_tap callback.
    #[must_use]
    pub fn on_tap(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_tap = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tap_down callback.
    #[must_use]
    pub fn on_tap_down(mut self, callback: impl Fn(TapDownDetails) + 'static) -> Self {
        self.callbacks.on_tap_down = Some(Rc::new(callback));
        self
    }

    /// Reports a pointer tap regardless of its stable button mask, without
    /// adding a semantic activation callback.
    #[doc(hidden)]
    #[must_use]
    pub fn on_any_pointer_tap(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_any_tap = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tap_up callback.
    #[must_use]
    pub fn on_tap_up(mut self, callback: impl Fn(TapUpDetails) + 'static) -> Self {
        self.callbacks.on_tap_up = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tap_cancel callback.
    #[must_use]
    pub fn on_tap_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_tap_cancel = Some(Rc::new(callback));
        self
    }

    /// Sets the on_double_tap callback.
    #[must_use]
    pub fn on_double_tap(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_double_tap = Some(Rc::new(callback));
        self
    }

    /// Sets the on_double_tap_down callback.
    #[must_use]
    pub fn on_double_tap_down(mut self, callback: impl Fn(TapDownDetails) + 'static) -> Self {
        self.callbacks.on_double_tap_down = Some(Rc::new(callback));
        self
    }

    /// Sets the on_double_tap_cancel callback.
    #[must_use]
    pub fn on_double_tap_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_double_tap_cancel = Some(Rc::new(callback));
        self
    }

    /// Sets the on_long_press callback.
    #[must_use]
    pub fn on_long_press(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_long_press = Some(Rc::new(callback));
        self
    }

    /// Sets the on_long_press_start callback.
    #[must_use]
    pub fn on_long_press_start(
        mut self,
        callback: impl Fn(LongPressStartDetails) + 'static,
    ) -> Self {
        self.callbacks.on_long_press_start = Some(Rc::new(callback));
        self
    }

    /// Sets the on_long_press_move_update callback.
    #[must_use]
    pub fn on_long_press_move_update(
        mut self,
        callback: impl Fn(LongPressMoveUpdateDetails) + 'static,
    ) -> Self {
        self.callbacks.on_long_press_move_update = Some(Rc::new(callback));
        self
    }

    /// Sets the on_long_press_up callback.
    #[must_use]
    pub fn on_long_press_up(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_long_press_up = Some(Rc::new(callback));
        self
    }

    /// Sets the on_long_press_end callback.
    #[must_use]
    pub fn on_long_press_end(mut self, callback: impl Fn(LongPressEndDetails) + 'static) -> Self {
        self.callbacks.on_long_press_end = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tap_move callback: each movement of a pending primary-button
    /// tap (Flutter `GestureDetector.onTapMove`).
    #[must_use]
    pub fn on_tap_move(mut self, callback: impl Fn(TapMoveDetails) + 'static) -> Self {
        self.callbacks.on_tap_move = Some(Rc::new(callback));
        self
    }

    /// Sets the on_long_press_down callback, fired when a press goes down
    /// before any deadline (Flutter `GestureDetector.onLongPressDown`).
    #[must_use]
    pub fn on_long_press_down(mut self, callback: impl Fn(LongPressDownDetails) + 'static) -> Self {
        self.callbacks.on_long_press_down = Some(Rc::new(callback));
        self
    }

    /// Sets the on_long_press_cancel callback, fired when a press that reported
    /// `onLongPressDown` loses before its long press starts (Flutter
    /// `GestureDetector.onLongPressCancel`).
    #[must_use]
    pub fn on_long_press_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_long_press_cancel = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_tap callback (secondary-button tap, Flutter
    /// `GestureDetector.onSecondaryTap`).
    #[must_use]
    pub fn on_secondary_tap(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.secondary.on_tap = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_tap_down callback (Flutter `onSecondaryTapDown`).
    #[must_use]
    pub fn on_secondary_tap_down(mut self, callback: impl Fn(TapDownDetails) + 'static) -> Self {
        self.callbacks.secondary.on_tap_down = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_tap_up callback (Flutter `onSecondaryTapUp`).
    #[must_use]
    pub fn on_secondary_tap_up(mut self, callback: impl Fn(TapUpDetails) + 'static) -> Self {
        self.callbacks.secondary.on_tap_up = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_tap_cancel callback (Flutter `onSecondaryTapCancel`).
    #[must_use]
    pub fn on_secondary_tap_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.secondary.on_tap_cancel = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_long_press_down callback (Flutter
    /// `onSecondaryLongPressDown`).
    #[must_use]
    pub fn on_secondary_long_press_down(
        mut self,
        callback: impl Fn(LongPressDownDetails) + 'static,
    ) -> Self {
        self.callbacks.secondary.on_long_press_down = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_long_press_cancel callback (Flutter
    /// `onSecondaryLongPressCancel`).
    #[must_use]
    pub fn on_secondary_long_press_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.secondary.on_long_press_cancel = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_long_press callback (Flutter `onSecondaryLongPress`).
    #[must_use]
    pub fn on_secondary_long_press(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.secondary.on_long_press = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_long_press_start callback (Flutter
    /// `onSecondaryLongPressStart`).
    #[must_use]
    pub fn on_secondary_long_press_start(
        mut self,
        callback: impl Fn(LongPressStartDetails) + 'static,
    ) -> Self {
        self.callbacks.secondary.on_long_press_start = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_long_press_move_update callback (Flutter
    /// `onSecondaryLongPressMoveUpdate`).
    #[must_use]
    pub fn on_secondary_long_press_move_update(
        mut self,
        callback: impl Fn(LongPressMoveUpdateDetails) + 'static,
    ) -> Self {
        self.callbacks.secondary.on_long_press_move_update = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_long_press_up callback (Flutter
    /// `onSecondaryLongPressUp`).
    #[must_use]
    pub fn on_secondary_long_press_up(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.secondary.on_long_press_up = Some(Rc::new(callback));
        self
    }

    /// Sets the on_secondary_long_press_end callback (Flutter
    /// `onSecondaryLongPressEnd`).
    #[must_use]
    pub fn on_secondary_long_press_end(
        mut self,
        callback: impl Fn(LongPressEndDetails) + 'static,
    ) -> Self {
        self.callbacks.secondary.on_long_press_end = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tertiary_tap_down callback (middle-button press, Flutter
    /// `onTertiaryTapDown`).
    #[must_use]
    pub fn on_tertiary_tap_down(mut self, callback: impl Fn(TapDownDetails) + 'static) -> Self {
        self.callbacks.tertiary.on_tap_down = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tertiary_tap_up callback (Flutter `onTertiaryTapUp`).
    #[must_use]
    pub fn on_tertiary_tap_up(mut self, callback: impl Fn(TapUpDetails) + 'static) -> Self {
        self.callbacks.tertiary.on_tap_up = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tertiary_tap_cancel callback (Flutter `onTertiaryTapCancel`).
    #[must_use]
    pub fn on_tertiary_tap_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.tertiary.on_tap_cancel = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tertiary_long_press_down callback (Flutter
    /// `onTertiaryLongPressDown`).
    #[must_use]
    pub fn on_tertiary_long_press_down(
        mut self,
        callback: impl Fn(LongPressDownDetails) + 'static,
    ) -> Self {
        self.callbacks.tertiary.on_long_press_down = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tertiary_long_press_cancel callback (Flutter
    /// `onTertiaryLongPressCancel`).
    #[must_use]
    pub fn on_tertiary_long_press_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.tertiary.on_long_press_cancel = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tertiary_long_press callback (Flutter `onTertiaryLongPress`).
    #[must_use]
    pub fn on_tertiary_long_press(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.tertiary.on_long_press = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tertiary_long_press_start callback (Flutter
    /// `onTertiaryLongPressStart`).
    #[must_use]
    pub fn on_tertiary_long_press_start(
        mut self,
        callback: impl Fn(LongPressStartDetails) + 'static,
    ) -> Self {
        self.callbacks.tertiary.on_long_press_start = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tertiary_long_press_move_update callback (Flutter
    /// `onTertiaryLongPressMoveUpdate`).
    #[must_use]
    pub fn on_tertiary_long_press_move_update(
        mut self,
        callback: impl Fn(LongPressMoveUpdateDetails) + 'static,
    ) -> Self {
        self.callbacks.tertiary.on_long_press_move_update = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tertiary_long_press_up callback (Flutter
    /// `onTertiaryLongPressUp`).
    #[must_use]
    pub fn on_tertiary_long_press_up(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.tertiary.on_long_press_up = Some(Rc::new(callback));
        self
    }

    /// Sets the on_tertiary_long_press_end callback (Flutter
    /// `onTertiaryLongPressEnd`).
    #[must_use]
    pub fn on_tertiary_long_press_end(
        mut self,
        callback: impl Fn(LongPressEndDetails) + 'static,
    ) -> Self {
        self.callbacks.tertiary.on_long_press_end = Some(Rc::new(callback));
        self
    }

    /// Sets the on_pan_down callback.
    #[must_use]
    pub fn on_pan_down(mut self, callback: impl Fn(DragDownDetails) + 'static) -> Self {
        self.callbacks.on_pan_down = Some(Rc::new(callback));
        self
    }

    /// Sets the on_pan_start callback.
    #[must_use]
    pub fn on_pan_start(mut self, callback: impl Fn(DragStartDetails) + 'static) -> Self {
        self.callbacks.on_pan_start = Some(Rc::new(callback));
        self
    }

    /// Sets the on_pan_update callback.
    #[must_use]
    pub fn on_pan_update(mut self, callback: impl Fn(DragUpdateDetails) + 'static) -> Self {
        self.callbacks.on_pan_update = Some(Rc::new(callback));
        self
    }

    /// Sets the on_pan_end callback.
    #[must_use]
    pub fn on_pan_end(mut self, callback: impl Fn(DragEndDetails) + 'static) -> Self {
        self.callbacks.on_pan_end = Some(Rc::new(callback));
        self
    }

    /// Sets the on_pan_cancel callback.
    #[must_use]
    pub fn on_pan_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_pan_cancel = Some(Rc::new(callback));
        self
    }

    /// Sets the on_horizontal_drag_down callback.
    #[must_use]
    pub fn on_horizontal_drag_down(mut self, callback: impl Fn(DragDownDetails) + 'static) -> Self {
        self.callbacks.on_horizontal_drag_down = Some(Rc::new(callback));
        self
    }

    /// Sets the on_horizontal_drag_start callback.
    #[must_use]
    pub fn on_horizontal_drag_start(
        mut self,
        callback: impl Fn(DragStartDetails) + 'static,
    ) -> Self {
        self.callbacks.on_horizontal_drag_start = Some(Rc::new(callback));
        self
    }

    /// Sets the on_horizontal_drag_update callback.
    #[must_use]
    pub fn on_horizontal_drag_update(
        mut self,
        callback: impl Fn(DragUpdateDetails) + 'static,
    ) -> Self {
        self.callbacks.on_horizontal_drag_update = Some(Rc::new(callback));
        self
    }

    /// Sets the on_horizontal_drag_end callback.
    #[must_use]
    pub fn on_horizontal_drag_end(mut self, callback: impl Fn(DragEndDetails) + 'static) -> Self {
        self.callbacks.on_horizontal_drag_end = Some(Rc::new(callback));
        self
    }

    /// Sets the on_horizontal_drag_cancel callback.
    #[must_use]
    pub fn on_horizontal_drag_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_horizontal_drag_cancel = Some(Rc::new(callback));
        self
    }

    /// Sets the on_vertical_drag_down callback.
    #[must_use]
    pub fn on_vertical_drag_down(mut self, callback: impl Fn(DragDownDetails) + 'static) -> Self {
        self.callbacks.on_vertical_drag_down = Some(Rc::new(callback));
        self
    }

    /// Sets the on_vertical_drag_start callback.
    #[must_use]
    pub fn on_vertical_drag_start(mut self, callback: impl Fn(DragStartDetails) + 'static) -> Self {
        self.callbacks.on_vertical_drag_start = Some(Rc::new(callback));
        self
    }

    /// Sets the on_vertical_drag_update callback.
    #[must_use]
    pub fn on_vertical_drag_update(
        mut self,
        callback: impl Fn(DragUpdateDetails) + 'static,
    ) -> Self {
        self.callbacks.on_vertical_drag_update = Some(Rc::new(callback));
        self
    }

    /// Sets the on_vertical_drag_end callback.
    #[must_use]
    pub fn on_vertical_drag_end(mut self, callback: impl Fn(DragEndDetails) + 'static) -> Self {
        self.callbacks.on_vertical_drag_end = Some(Rc::new(callback));
        self
    }

    /// Sets the on_vertical_drag_cancel callback.
    #[must_use]
    pub fn on_vertical_drag_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_vertical_drag_cancel = Some(Rc::new(callback));
        self
    }

    /// Converts trackpad scroll to exponential zoom while keeping the
    /// focal point fixed. Disabled by default.
    #[must_use]
    pub fn trackpad_scroll_causes_scale(mut self, enabled: bool) -> Self {
        self.callbacks.trackpad_scroll_causes_scale = enabled;
        self
    }

    /// Sets the horizontal and vertical gains used by scroll-to-scale.
    /// The default is `(0, -1/200)`; this does not enable the conversion.
    #[must_use]
    pub fn trackpad_scroll_to_scale_factor(mut self, factor: incular_core::Offset) -> Self {
        self.callbacks.trackpad_scroll_to_scale_factor = Some(factor);
        self
    }

    /// Sets the on_scale_start callback.
    #[must_use]
    pub fn on_scale_start(mut self, callback: impl Fn(ScaleStartDetails) + 'static) -> Self {
        self.callbacks.on_scale_start = Some(Rc::new(callback));
        self
    }

    /// Sets the on_scale_update callback.
    #[must_use]
    pub fn on_scale_update(mut self, callback: impl Fn(ScaleUpdateDetails) + 'static) -> Self {
        self.callbacks.on_scale_update = Some(Rc::new(callback));
        self
    }

    /// Sets the on_scale_end callback.
    #[must_use]
    pub fn on_scale_end(mut self, callback: impl Fn(ScaleEndDetails) + 'static) -> Self {
        self.callbacks.on_scale_end = Some(Rc::new(callback));
        self
    }

    /// Flutter `onForcePressStart`: a pressure-sensing press passed 0.4 of
    /// its pressure range. Devices that report no pressure never start one.
    #[must_use]
    pub fn on_force_press_start(mut self, callback: impl Fn(ForcePressDetails) + 'static) -> Self {
        self.callbacks.on_force_press_start = Some(Rc::new(callback));
        self
    }

    /// Flutter `onForcePressPeak`: the pressure passed 0.85 of its range.
    #[must_use]
    pub fn on_force_press_peak(mut self, callback: impl Fn(ForcePressDetails) + 'static) -> Self {
        self.callbacks.on_force_press_peak = Some(Rc::new(callback));
        self
    }

    /// Flutter `onForcePressUpdate`: each change once the force press started.
    #[must_use]
    pub fn on_force_press_update(mut self, callback: impl Fn(ForcePressDetails) + 'static) -> Self {
        self.callbacks.on_force_press_update = Some(Rc::new(callback));
        self
    }

    /// Flutter `onForcePressEnd`: the pointer lifted after the force press
    /// started.
    #[must_use]
    pub fn on_force_press_end(mut self, callback: impl Fn(ForcePressDetails) + 'static) -> Self {
        self.callbacks.on_force_press_end = Some(Rc::new(callback));
        self
    }

    /// Receives native aggregate trackpad gestures after the retained arena
    /// resolves ownership. No synthetic touch contacts are created.
    #[must_use]
    pub fn on_trackpad_gesture(mut self, callback: impl Fn(TrackpadGesture) + 'static) -> Self {
        self.callbacks.on_trackpad_gesture = Some(Rc::new(callback));
        self
    }

    /// Sets the on_cancel callback.
    #[must_use]
    pub fn on_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks.on_cancel = Some(Rc::new(callback));
        self
    }

    /// Sets the hit test behavior. Defaults to [`HitTestBehavior::DeferToChild`]
    /// when a child is present and [`HitTestBehavior::Translucent`] when it is not.
    #[must_use]
    pub fn behavior(mut self, behavior: HitTestBehavior) -> Self {
        self.behavior = Some(behavior);
        self
    }

    /// Sets the callbacks configuration directly.
    #[must_use]
    pub fn callbacks(mut self, callbacks: GestureCallbacks) -> Self {
        self.callbacks = callbacks;
        self
    }

    /// Associates a retained focus node with this detector. Focus metadata is
    /// kept on the gesture callback record so the widget tree can include the
    /// detector in traversal without introducing a second focus subsystem.
    #[must_use]
    pub fn focus_node(mut self, node: crate::FocusNode) -> Self {
        self.callbacks.focus_node = Some(node);
        self
    }

    /// Requests the associated focus node when the retained tree mounts.
    #[must_use]
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.callbacks.autofocus = autofocus;
        self
    }
}

impl From<GestureDetector> for Widget {
    fn from(value: GestureDetector) -> Self {
        let behavior = value.behavior.unwrap_or(if value.child.is_some() {
            HitTestBehavior::DeferToChild
        } else {
            HitTestBehavior::Translucent
        });
        // A childless detector fills its constraints, like Flutter's
        // `RenderPointerListener.computeSizeForNoChild` (`constraints.biggest`).
        let child = value
            .child
            .unwrap_or_else(|| crate::SizedBox::expand().into());
        Widget::from_kind(WidgetKind::Gesture {
            behavior,
            callbacks: Box::new(value.callbacks),
            child,
        })
        .with_gesture_semantic_policy(value.semantic_policy)
    }
}

/// A widget that makes its subtree invisible to hit testing.
#[derive(Clone, Debug, PartialEq)]
pub struct IgnorePointer {
    ignoring: bool,
    child: Widget,
}

impl IgnorePointer {
    /// Creates an IgnorePointer widget.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            ignoring: true,
            child: child.into(),
        }
    }

    /// Sets whether ignoring is enabled.
    #[must_use]
    pub fn ignoring(mut self, ignoring: bool) -> Self {
        self.ignoring = ignoring;
        self
    }
}

widget_from_kind!(IgnorePointer => IgnorePointer { ignoring, child });

/// A widget that absorbs pointer events during hit testing to prevent descendants and siblings behind it from receiving them.
#[derive(Clone, Debug, PartialEq)]
pub struct AbsorbPointer {
    absorbing: bool,
    child: Widget,
}

impl AbsorbPointer {
    /// Creates an AbsorbPointer widget.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            absorbing: true,
            child: child.into(),
        }
    }

    /// Sets whether absorbing is enabled.
    #[must_use]
    pub fn absorbing(mut self, absorbing: bool) -> Self {
        self.absorbing = absorbing;
        self
    }
}

widget_from_kind!(AbsorbPointer => AbsorbPointer { absorbing, child });
