//! Retained routing for raw pointer widgets.

use std::collections::HashSet;

use crate::gestures::HitTestBehavior;
use incular_core::PointerPhase;
use incular_gestures::{
    ErasedGestureRecognizerFactory, GestureDecision, MouseCursor, PointerDeviceKind,
    RawPointerEvent,
};

use crate::raw_input::{
    ListenerCallbacks, MouseRegionCallbacks, RawInputKind, TapRegionCallbacks, TapRegionGroupId,
    TapRegionRouteCurrent, WindowInteraction,
};

use super::*;

#[derive(Clone)]
struct TapRegistration<'a> {
    id: ElementId,
    group_id: Option<&'a TapRegionGroupId>,
    callbacks: &'a TapRegionCallbacks,
    report_outside: bool,
    consume_outside_taps: bool,
}

#[derive(Default)]
struct TapRegionDelivery {
    consumed_by: Option<ElementId>,
    notified: bool,
}

impl WidgetTree {
    pub(super) fn dispatch_raw_trackpad_gesture(&self, position: Offset, event: TrackpadGesture) {
        for callback in self
            .listener_ids(&self.raw_hit_elements(position))
            .into_iter()
            .filter_map(|id| self.listener_callbacks(id))
            .filter_map(|callbacks| callbacks.on_trackpad_gesture)
        {
            callback(event);
        }
    }

    /// Pointer-routing table for this retained window. Its observers receive
    /// raw events after hit-path listeners and before the recognizer arena.
    #[must_use]
    pub fn pointer_router(&self) -> incular_gestures::PointerRouter {
        self.input
            .pointer_router
            .get_or_init(incular_gestures::PointerRouter::new)
            .clone()
    }

    fn route_pointer(&self, event: RawPointerEvent) {
        if let Some(router) = self.input.pointer_router.get() {
            router.route(event);
        }
    }

    /// Dispatches one metadata-rich pointer event through the standalone
    /// retained window.
    pub fn dispatch_raw_pointer(&mut self, event: RawPointerEvent) -> Option<ElementId> {
        self.dispatch_raw_pointer_in_window(0, event)
    }

    /// Dispatches raw listeners, mouse annotations, tap-region boundaries,
    /// and raw recognizers in one retained window. The return value is only a
    /// target that intentionally consumes ordinary runtime pointer fallback;
    /// Listener and MouseRegion callbacks never suppress buttons or text.
    pub fn dispatch_raw_pointer_in_window(
        &mut self,
        window: u64,
        event: RawPointerEvent,
    ) -> Option<ElementId> {
        let hit = if matches!(event.phase, PointerPhase::Enter | PointerPhase::Exit) {
            PointerHitTest::default()
        } else {
            self.pointer_hit_test(event.position)
        };
        self.dispatch_raw_pointer_with_hit(window, event, &hit)
    }

    /// [`Self::dispatch_raw_pointer_in_window`] over a hit test the caller
    /// already ran at `event.position`, so the runtime can reuse it for
    /// text editing and buttons. Enter and exit events ignore `hit`.
    pub fn dispatch_raw_pointer_with_hit(
        &mut self,
        window: u64,
        mut event: RawPointerEvent,
        hit: &PointerHitTest,
    ) -> Option<ElementId> {
        let key = GestureArenaKey::pointer_device(window, event.device, event.pointer);
        if event.phase == PointerPhase::Cancel
            || (event.phase == PointerPhase::Up && event.buttons == 0)
        {
            self.release_mounted_tap_contact(key);
        }
        // Flutter `PointerEvent.delta`: movement since this pointer's
        // previous event; a press starts from rest.
        event.delta = match event.phase {
            PointerPhase::Down | PointerPhase::Enter | PointerPhase::Exit => Offset::ZERO,
            PointerPhase::Move | PointerPhase::Up | PointerPhase::Cancel => self
                .input
                .raw_pointer_positions
                .get(&key)
                .map_or(Offset::ZERO, |previous| event.position - *previous),
        };
        let event = event.in_global_coordinates();
        match event.phase {
            PointerPhase::Exit | PointerPhase::Cancel => {
                self.input.raw_pointer_positions.remove(&key);
            }
            PointerPhase::Down | PointerPhase::Move | PointerPhase::Up | PointerPhase::Enter => {
                self.input.raw_pointer_positions.insert(key, event.position);
            }
        }
        self.dispatch_mouse_regions(window, event, hit, true);
        if matches!(event.phase, PointerPhase::Enter | PointerPhase::Exit) {
            self.route_pointer(event);
            return None;
        }
        self.dispatch_listeners(key, event, hit.raw());
        self.route_pointer(event);

        let tap_target = self.dispatch_tap_regions(event, hit.raw());
        if let Some(target) = tap_target {
            self.cancel_raw_gesture_stream(key, true);
            self.input.consumed_tap_pointers.insert(key);
            return Some(target);
        }
        if self.input.consumed_tap_pointers.contains(&key) {
            let last_button_up = event.phase == PointerPhase::Up && event.buttons == 0;
            if event.phase == PointerPhase::Cancel || last_button_up {
                self.input.consumed_tap_pointers.remove(&key);
            }
            return None;
        }
        self.dispatch_raw_gesture_stream(key, event, hit.raw())
    }

    /// Seeds a host-positioned hover device without dispatching a mouse
    /// move to regions, listeners, or recognizers.
    #[doc(hidden)]
    pub fn add_positioned_hover_device(
        &mut self,
        window: u64,
        event: RawPointerEvent,
        hit: &PointerHitTest,
    ) {
        let key = GestureArenaKey::pointer_device(window, event.device, event.pointer);
        self.input.raw_pointer_positions.insert(key, event.position);
        self.dispatch_mouse_regions(window, event, hit, false);
    }

    /// Changes when hit-test properties change without moving a pointer.
    #[doc(hidden)]
    #[must_use]
    pub fn hover_hit_revision(&self) -> u64 {
        self.input.hover_hit_revision
    }

    /// Re-evaluates mouse-region and focus-behavior hover for a pointer that
    /// has not moved, after layout or scrolling moved content under it
    /// (Flutter's `MouseTracker.updateAllDevices`). Only enter and exit
    /// callbacks run: nothing hovered moved relative to the pointer, so no
    /// hover or listener event is sent. `event` is the device's latest
    /// sample and `hit` a hit test at its position.
    pub fn refresh_mouse_hover_in_window(
        &mut self,
        window: u64,
        event: RawPointerEvent,
        hit: &PointerHitTest,
    ) {
        if matches!(event.phase, PointerPhase::Enter | PointerPhase::Exit) {
            return;
        }
        self.dispatch_mouse_regions(window, event, hit, false);
    }

    /// Returns the cursor selected by the current raw hit route. A deferred
    /// cursor continues behind translucent/deferred regions, just as Flutter
    /// does for `MouseRegion` annotations.
    #[must_use]
    pub fn mouse_cursor_at(&self, point: Offset) -> MouseCursor {
        self.mouse_cursor_for_hit(&self.pointer_hit_test(point))
    }

    /// [`Self::mouse_cursor_at`] for a hit test the caller already ran.
    #[must_use]
    pub fn mouse_cursor_for_hit(&self, hit: &PointerHitTest) -> MouseCursor {
        let raw = hit
            .raw()
            .iter()
            .filter_map(|id| self.cursor_for_raw_input(*id))
            .find(|cursor| *cursor != MouseCursor::Defer)
            .unwrap_or(MouseCursor::Defer);
        if raw != MouseCursor::Defer {
            return raw;
        }

        let Some(render) = hit.target() else {
            return MouseCursor::Defer;
        };
        let Some(mut element) = self.element_for_render(render) else {
            return MouseCursor::Defer;
        };
        loop {
            if let Some(WidgetKind::Button(spec)) = self
                .elements
                .get(element.0)
                .map(|entry| entry.widget.kind())
                && spec.mouse_cursor != MouseCursor::Defer
            {
                return spec.mouse_cursor;
            }
            // Flutter's TextField wraps its EditableText in a MouseRegion with
            // WidgetStateMouseCursor.textable: text cursor when enabled, basic
            // cursor when disabled.
            if let Some(WidgetKind::TextField(spec)) = self
                .elements
                .get(element.0)
                .map(|entry| entry.widget.kind())
            {
                return if spec.enabled {
                    MouseCursor::Text
                } else {
                    MouseCursor::Basic
                };
            }
            let Some(parent) = self.parent(element) else {
                return MouseCursor::Defer;
            };
            element = parent;
        }
    }

    fn cursor_for_raw_input(&self, element: ElementId) -> Option<MouseCursor> {
        match self.raw_input_kind(element)? {
            RawInputKind::MouseRegion { cursor, .. } => Some(*cursor),
            RawInputKind::WindowResizeRegion { direction } => Some(match direction {
                incular_core::WindowResizeDirection::East
                | incular_core::WindowResizeDirection::West => MouseCursor::ResizeHorizontal,
                incular_core::WindowResizeDirection::North
                | incular_core::WindowResizeDirection::South => MouseCursor::ResizeVertical,
                incular_core::WindowResizeDirection::NorthWest
                | incular_core::WindowResizeDirection::SouthEast => {
                    MouseCursor::ResizeUpLeftDownRight
                }
                incular_core::WindowResizeDirection::NorthEast
                | incular_core::WindowResizeDirection::SouthWest => {
                    MouseCursor::ResizeUpRightDownLeft
                }
            }),
            _ => None,
        }
    }

    #[must_use]
    pub fn cursor_at(&self, point: Offset) -> MouseCursor {
        self.mouse_cursor_at(point)
    }

    /// Whether a tap at `point` belongs to an enabled text-field tap-region
    /// group. Runtime focus fallback uses this to keep the current editor
    /// focused while a spinner, clear button, or other grouped control is
    /// pressed.
    #[must_use]
    pub fn preserves_text_field_focus(&self, point: Offset) -> bool {
        self.raw_hit_elements(point).into_iter().any(|id| {
            matches!(
                self.raw_input_kind(id),
                Some(RawInputKind::TextFieldTapRegion { enabled: true, .. })
            )
        })
    }

    /// Returns the deepest/frontmost custom-chrome interaction annotation at
    /// `point`. Interactive descendants are resolved separately by Runtime and
    /// take precedence before this annotation is acted upon.
    #[doc(hidden)]
    #[must_use]
    pub fn window_interaction_at(&self, point: Offset) -> Option<(ElementId, WindowInteraction)> {
        self.raw_hit_elements(point).into_iter().find_map(|id| {
            let interaction = match self.raw_input_kind(id)? {
                RawInputKind::WindowDragRegion => WindowInteraction::Move,
                RawInputKind::WindowResizeRegion { direction } => {
                    WindowInteraction::Resize(*direction)
                }
                _ => return None,
            };
            (!self.window_interaction_blocked_by_descendant(id, point)).then_some((id, interaction))
        })
    }

    fn window_interaction_blocked_by_descendant(&self, region: ElementId, point: Offset) -> bool {
        let Some(mut current) = self
            .hit_test(point)
            .and_then(|render| self.element_for_render(render))
        else {
            return true;
        };
        loop {
            if current == region {
                return false;
            }
            let Some(element) = self.elements.get(current.0) else {
                return true;
            };
            if window_chrome_blocks_at(element.widget.kind()) {
                return true;
            }
            let Some(parent) = element.parent else {
                // The raw annotation is not an ancestor of the visual target;
                // another frontmost branch owns the press.
                return true;
            };
            current = parent;
        }
    }

    pub(super) fn sync_raw_input_state(&mut self, element: ElementId) {
        let factories = self
            .raw_input_kind(element)
            .and_then(RawInputKind::factory_map)
            .map(ToOwned::to_owned);
        let Some(factories) = factories else {
            self.cancel_raw_streams_for_element(element);
            if let Some(mut recognizers) = self.input.raw_recognizers.remove(&element) {
                for recognizer in recognizers.values_mut() {
                    recognizer.dispose();
                }
            }
            return;
        };

        let next_types = factories
            .iter()
            .map(ErasedGestureRecognizerFactory::type_id)
            .collect::<HashSet<_>>();
        let streams_with_removed_recognizers = self
            .input
            .raw_gesture_streams
            .iter()
            .filter(|(_, active)| {
                active.members.iter().any(|member| {
                    member.element == element && !next_types.contains(&member.type_id)
                })
            })
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        for key in streams_with_removed_recognizers {
            self.cancel_raw_gesture_stream(key, true);
        }

        let mut previous = self
            .input
            .raw_recognizers
            .remove(&element)
            .unwrap_or_default();
        let mut next = std::collections::HashMap::new();
        let mut seen = HashSet::new();
        for factory in factories {
            if !seen.insert(factory.type_id()) {
                continue;
            }
            let type_id = factory.type_id();
            let mut recognizer = previous
                .remove(&type_id)
                .unwrap_or_else(|| factory.create());
            if factory.initialize(recognizer.as_mut()).is_ok() {
                next.insert(type_id, recognizer);
            } else {
                recognizer.dispose();
            }
        }
        for recognizer in previous.values_mut() {
            recognizer.dispose();
        }
        self.input.raw_recognizers.insert(element, next);
    }

    fn raw_input_kind(&self, element: ElementId) -> Option<&RawInputKind> {
        match self.elements.get(element.0)?.widget.kind() {
            WidgetKind::RawInput { kind, .. } => Some(kind),
            _ => None,
        }
    }

    fn listener_callbacks(&self, element: ElementId) -> Option<ListenerCallbacks> {
        match self.raw_input_kind(element)? {
            RawInputKind::Listener { callbacks, .. } => Some(callbacks.clone()),
            _ => None,
        }
    }

    /// The focus behavior carried by a detector element, if any. Hover
    /// tracking resolves these through the same hit sets as mouse
    /// regions so both lifecycles share one map and one diff.
    fn focus_behavior_for(
        &self,
        element: ElementId,
    ) -> Option<std::rc::Rc<incular_gestures::FocusBehavior>> {
        match self
            .elements
            .get(element.0)
            .map(|element| element.widget.kind())
        {
            Some(WidgetKind::Gesture { callbacks, .. }) => callbacks.focus_behavior.clone(),
            _ => None,
        }
    }

    /// Every detector element enclosing the point: detectors enter hit
    /// testing through hovered descendants, so climb from the deepest
    /// hit (raw hit lists only carry RawInput kinds). Nested detectors
    /// all appear; an absorbing overlay keeps the detector behind it
    /// out of the chain.
    fn focus_behavior_elements(&self, target: Option<RenderObjectId>) -> Vec<ElementId> {
        let mut tracked = Vec::new();
        let mut current = target.and_then(|render| self.element_for_render(render));
        while let Some(candidate) = current {
            if self.focus_behavior_for(candidate).is_some() {
                tracked.push(candidate);
            }
            current = self.parent(candidate);
        }
        tracked
    }

    fn mouse_region_config(
        &self,
        element: ElementId,
    ) -> Option<(
        MouseRegionCallbacks,
        MouseCursor,
        bool,
        Option<HitTestBehavior>,
    )> {
        match self.raw_input_kind(element)? {
            RawInputKind::MouseRegion {
                callbacks,
                cursor,
                opaque,
                behavior,
            } => Some((callbacks.clone(), *cursor, *opaque, *behavior)),
            _ => None,
        }
    }

    fn listener_ids(&self, elements: &[ElementId]) -> Vec<ElementId> {
        let mut out = Vec::with_capacity(elements.len());
        for &id in elements {
            if !out.contains(&id)
                && matches!(self.raw_input_kind(id), Some(RawInputKind::Listener { .. }))
            {
                out.push(id);
            }
        }
        out
    }

    fn dispatch_listeners(
        &mut self,
        key: GestureArenaKey,
        event: RawPointerEvent,
        hit: &[ElementId],
    ) {
        let hover_event = event.phase == PointerPhase::Move
            && event.buttons == 0
            && !self.input.raw_pointer_routes.contains_key(&key);
        if event.phase == PointerPhase::Down && !self.input.raw_pointer_routes.contains_key(&key) {
            // Flutter retains both the recipient and its hit-test transform
            // for the contact. Layout or scrolling may move it afterwards.
            let route = self
                .listener_ids(hit)
                .into_iter()
                .map(|element| RawPointerRouteEntry {
                    element,
                    world_to_local: self
                        .render_id(element)
                        .and_then(|id| self.pointer_world_to_local(id)),
                })
                .collect();
            self.input.raw_pointer_routes.insert(key, route);
        }
        let finished;
        let route = match event.phase {
            PointerPhase::Cancel => {
                finished = self.input.raw_pointer_routes.remove(&key);
                finished.as_ref()
            }
            PointerPhase::Up if event.buttons == 0 => {
                finished = self.input.raw_pointer_routes.remove(&key);
                finished.as_ref()
            }
            PointerPhase::Down | PointerPhase::Move | PointerPhase::Up => {
                self.input.raw_pointer_routes.get(&key)
            }
            PointerPhase::Enter | PointerPhase::Exit => None,
        };
        let callbacks = route
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                self.listener_callbacks(entry.element).map(|callbacks| {
                    (
                        callbacks,
                        localize_with_transform(entry.world_to_local, event),
                    )
                })
            })
            .collect::<Vec<_>>();
        for (callback, event) in callbacks {
            match event.phase {
                PointerPhase::Down => {
                    if let Some(callback) = callback.on_pointer_down {
                        callback(event);
                    }
                }
                PointerPhase::Move => {
                    if let Some(callback) = callback.on_pointer_move {
                        callback(event);
                    }
                }
                PointerPhase::Up => {
                    if let Some(callback) = callback.on_pointer_up {
                        callback(event);
                    }
                }
                PointerPhase::Cancel => {
                    if let Some(callback) = callback.on_pointer_cancel {
                        callback(event);
                    }
                }
                PointerPhase::Enter | PointerPhase::Exit => {}
            }
        }
        if hover_event {
            let hovers = self
                .listener_ids(hit)
                .into_iter()
                .filter_map(|id| {
                    self.listener_callbacks(id)
                        .map(|callbacks| (callbacks, self.localized_event(id, event)))
                })
                .collect::<Vec<_>>();
            for (callback, event) in hovers {
                if let Some(callback) = callback.on_pointer_hover {
                    callback(event);
                }
            }
        }
    }

    /// Delivers a wheel scroll to the `on_pointer_signal` listeners under
    /// `point`, innermost first. Returns whether any listener received it.
    pub(super) fn dispatch_pointer_signal(&self, point: Offset, scroll_delta: Offset) -> bool {
        let hit = self.pointer_hit_test(point);
        let event = RawPointerEvent::scroll(
            0,
            0,
            crate::raw_input::PointerDeviceKind::Mouse,
            point,
            scroll_delta,
            incular_core::time::Instant::now(),
        );
        let listeners = self
            .listener_ids(hit.raw())
            .into_iter()
            .filter_map(|id| {
                let callback = self.listener_callbacks(id)?.on_pointer_signal?;
                Some((id, callback, self.localized_event(id, event)))
            })
            .collect::<Vec<_>>();
        let delivered = !listeners.is_empty();
        for (id, callback, event) in listeners {
            crate::raw_input::PointerSignalResolver::set_listener(Some(id));
            callback(event);
        }
        crate::raw_input::PointerSignalResolver::set_listener(None);
        self.route_pointer(event);
        delivered
    }

    /// Flutter `PointerEvent.transformed`: the event's `localPosition` and
    /// `localDelta` in `element`'s coordinates.
    pub(super) fn localized_event(
        &self,
        element: ElementId,
        event: RawPointerEvent,
    ) -> RawPointerEvent {
        let inverse = self
            .render_id(element)
            .and_then(|id| self.pointer_world_to_local(id));
        localize_with_transform(inverse, event)
    }

    /// Diffs the device's hovered regions against `hit` and runs their enter
    /// and exit callbacks; `send_hover` also delivers hover callbacks for a
    /// buttonless move.
    fn dispatch_mouse_regions(
        &mut self,
        window: u64,
        event: RawPointerEvent,
        hit: &PointerHitTest,
        send_hover: bool,
    ) {
        if !matches!(
            event.kind,
            PointerDeviceKind::Mouse
                | PointerDeviceKind::Trackpad
                | PointerDeviceKind::Stylus
                | PointerDeviceKind::InvertedStylus
        ) {
            // Flutter's MouseTracker ignores non-hover devices, but the
            // region itself still receives a buttonless PointerHoverEvent
            // (for example a synthetic touch hover outside active contact).
            let stream = GestureArenaKey::pointer_device(window, event.device, event.pointer);
            if send_hover
                && event.phase == PointerPhase::Move
                && !event.down
                && event.buttons == 0
                && !self.input.raw_pointer_routes.contains_key(&stream)
            {
                for &element in hit.raw() {
                    if let Some(callback) = self
                        .mouse_region_config(element)
                        .and_then(|(callbacks, ..)| callbacks.on_hover)
                    {
                        let transform = self
                            .render_id(element)
                            .and_then(|id| self.pointer_world_to_local(id));
                        callback(localize_with_transform(transform, event));
                    }
                }
            }
            return;
        }
        let (device, pointer) = event.hover_device_identity();
        let key = GestureArenaKey::pointer_device(window, device, pointer);
        if event.phase == PointerPhase::Enter {
            // Winit's CursorEntered carries no position. Waiting for the first
            // CursorMoved avoids firing an enter callback for stale coordinates
            // from a previous surface visit.
            return;
        }
        // Mouse regions enter the hover sets directly; focus behaviors
        // join through their hovered descendants so nested detectors all
        // track without a second hover map.
        let mut next = Vec::new();
        if event.phase != PointerPhase::Exit {
            next.extend(hit.raw().iter().copied().filter_map(|element| {
                self.mouse_region_config(element)?;
                Some(RawPointerRouteEntry {
                    element,
                    world_to_local: self
                        .render_id(element)
                        .and_then(|id| self.pointer_world_to_local(id)),
                })
            }));
            for element in self.focus_behavior_elements(hit.target()) {
                if !next.iter().any(|entry| entry.element == element) {
                    next.push(RawPointerRouteEntry {
                        element,
                        world_to_local: None,
                    });
                }
            }
        }
        let previous = if next.is_empty() {
            self.input.mouse_hover.remove(&key)
        } else {
            self.input.mouse_hover.insert(key, next)
        }
        .unwrap_or_default();
        let next = self
            .input
            .mouse_hover
            .get(&key)
            .map_or(&[][..], Vec::as_slice);
        let exits = previous.iter().filter(|entry| {
            self.elements.contains(entry.element.0)
                && !next.iter().any(|next| next.element == entry.element)
        });
        // Flutter sends exits in old hit-test order and enters in reverse
        // new order, so the outer region is entered before its inner one.
        let enters = next.iter().filter(|entry| {
            !previous
                .iter()
                .any(|previous| previous.element == entry.element)
        });
        let exit_callbacks = exits
            .clone()
            .filter_map(|entry| {
                self.mouse_region_config(entry.element)
                    .and_then(|(callbacks, ..)| callbacks.on_exit)
                    .map(|callback| (callback, entry.world_to_local))
            })
            .collect::<Vec<_>>();
        let enter_callbacks = enters
            .clone()
            .rev()
            .filter_map(|entry| {
                self.mouse_region_config(entry.element)
                    .and_then(|(callbacks, ..)| callbacks.on_enter)
                    .map(|callback| (callback, entry.world_to_local))
            })
            .collect::<Vec<_>>();
        let exit_behaviors = exits
            .filter_map(|entry| self.focus_behavior_for(entry.element))
            .collect::<Vec<_>>();
        let enter_behaviors = enters
            .filter_map(|entry| self.focus_behavior_for(entry.element))
            .collect::<Vec<_>>();
        for (callback, transform) in exit_callbacks {
            callback(localize_with_transform(
                transform,
                RawPointerEvent {
                    phase: PointerPhase::Exit,
                    ..event
                },
            ));
        }
        for behavior in exit_behaviors {
            behavior.mouse_exit();
        }
        for (callback, transform) in enter_callbacks {
            callback(localize_with_transform(
                transform,
                RawPointerEvent {
                    phase: PointerPhase::Enter,
                    ..event
                },
            ));
        }
        for behavior in enter_behaviors {
            behavior.mouse_enter();
        }
        if send_hover && event.phase == PointerPhase::Move && event.buttons == 0 {
            let hover_callbacks = next
                .iter()
                .filter_map(|entry| {
                    self.mouse_region_config(entry.element)
                        .and_then(|(callbacks, ..)| callbacks.on_hover)
                        .map(|callback| (callback, entry.world_to_local))
                })
                .collect::<Vec<_>>();
            for (callback, transform) in hover_callbacks {
                callback(localize_with_transform(transform, event));
            }
        }
    }

    fn tap_region_registrations(&self, surface: ElementId) -> Vec<TapRegistration<'_>> {
        self.tracked
            .tap_regions
            .iter()
            .filter_map(|&id| {
                let element = self.elements.get(id.0)?;
                let WidgetKind::RawInput { kind, .. } = element.widget.kind() else {
                    return None;
                };
                let (callbacks, group_id, consume_outside_taps) = match &**kind {
                    RawInputKind::TapRegion {
                        callbacks,
                        enabled: true,
                        group_id,
                        consume_outside_taps,
                        ..
                    } => (callbacks, group_id.as_ref(), *consume_outside_taps),
                    RawInputKind::TextFieldTapRegion {
                        callbacks,
                        enabled: true,
                        group_id,
                        consume_outside_taps,
                        ..
                    } => (callbacks, Some(group_id), *consume_outside_taps),
                    _ => return None,
                };
                if self.nearest_tap_surface(id) != Some(surface) {
                    return None;
                }
                // Like Flutter, a covered route is still in the registry
                // for hit groups and inside callbacks. Only outside phases
                // and their arena consumption depend on route currency.
                let report_outside = element
                    .build_context
                    .read::<TapRegionRouteCurrent>()
                    .is_none_or(|status| status.0);
                Some(TapRegistration {
                    id,
                    group_id,
                    callbacks,
                    report_outside,
                    consume_outside_taps,
                })
            })
            .collect()
    }

    fn nearest_tap_surface(&self, element: ElementId) -> Option<ElementId> {
        let mut current = self.parent(element);
        while let Some(id) = current {
            if matches!(
                self.raw_input_kind(id),
                Some(RawInputKind::TapRegionSurface)
            ) {
                return Some(id);
            }
            current = self.parent(id);
        }
        None
    }

    fn dispatch_tap_regions(&self, event: RawPointerEvent, hit: &[ElementId]) -> Option<ElementId> {
        if !matches!(event.phase, PointerPhase::Down | PointerPhase::Up) {
            return None;
        }
        // A surface classifies pointer down/up, not a primary-only tap
        // gesture. Keep the original button metadata for each callback.
        let mut consumed = None;
        for surface in hit.iter().copied().filter(|id| {
            matches!(
                self.raw_input_kind(*id),
                Some(RawInputKind::TapRegionSurface)
            )
        }) {
            let delivery = self.notify_tap_region_surface(surface, hit, event, true);
            consumed = consumed.or(delivery.consumed_by);
        }
        consumed
    }

    /// Mirrors the surface's accessibility listener. The notification is
    /// not an input stream: it cannot start gestures, reach pointer listeners,
    /// or consume the actual semantic action.
    #[doc(hidden)]
    pub fn notify_tap_regions_of_semantic_action(&self, bounds: Rect, time: Instant) -> bool {
        if self.tracked.tap_regions.is_empty() {
            return false;
        }
        let position = Offset::new(
            bounds.origin.x + bounds.size.width * 0.5,
            bounds.origin.y + bounds.size.height * 0.5,
        );
        let event = RawPointerEvent::new(
            0,
            0,
            PointerDeviceKind::Touch,
            incular_core::PRIMARY_POINTER_BUTTON,
            position,
            PointerPhase::Down,
            time,
        )
        .with_button(Some(incular_core::PRIMARY_POINTER_BUTTON));
        let mut notified = false;
        for &surface in &self.tracked.tap_region_surfaces {
            // Each surface listens independently. Start at that subtree,
            // not at the window hit route (an overlapping sibling may
            // otherwise drop the surface).
            if let Some(hit) = self.raw_hit_elements_within(surface, position)
                && hit.contains(&surface)
            {
                notified |= self
                    .notify_tap_region_surface(surface, &hit, event, false)
                    .notified;
            }
        }
        notified
    }

    fn notify_tap_region_surface(
        &self,
        surface: ElementId,
        hit: &[ElementId],
        event: RawPointerEvent,
        can_consume: bool,
    ) -> TapRegionDelivery {
        let down = event.phase == PointerPhase::Down;
        let registrations = self.tap_region_registrations(surface);
        let hit_groups = registrations
            .iter()
            .filter(|registration| hit.contains(&registration.id))
            .filter_map(|registration| registration.group_id)
            .collect::<Vec<_>>();
        let mut delivery = TapRegionDelivery::default();
        // Flutter reports outside before inside in each phase, including
        // when a point's hit counts as inside for an entire group.
        for registration in registrations.iter().filter(|registration| {
            registration.report_outside && !is_inside_tap_region(registration, hit, &hit_groups)
        }) {
            let callback = if down {
                &registration.callbacks.on_tap_outside
            } else {
                &registration.callbacks.on_tap_up_outside
            };
            if let Some(callback) = callback {
                callback(event);
                delivery.notified = true;
            }
            if down
                && can_consume
                && delivery.consumed_by.is_none()
                && registration.consume_outside_taps
            {
                delivery.consumed_by = Some(registration.id);
            }
        }
        for registration in registrations
            .iter()
            .filter(|registration| is_inside_tap_region(registration, hit, &hit_groups))
        {
            let callback = if down {
                &registration.callbacks.on_tap_inside
            } else {
                &registration.callbacks.on_tap_up_inside
            };
            if let Some(callback) = callback {
                callback(event);
                delivery.notified = true;
            }
        }
        delivery
    }

    fn raw_gesture_elements(&self, hit: &[ElementId]) -> Vec<ElementId> {
        let mut out = Vec::with_capacity(hit.len());
        for &id in hit {
            if !out.contains(&id)
                && self
                    .raw_input_kind(id)
                    .and_then(RawInputKind::factory_map)
                    .is_some_and(|factories| !factories.is_empty())
            {
                out.push(id);
            }
        }
        out
    }

    fn dispatch_raw_gesture_stream(
        &mut self,
        key: GestureArenaKey,
        event: RawPointerEvent,
        hit: &[ElementId],
    ) -> Option<ElementId> {
        if event.phase == PointerPhase::Down && !self.input.raw_gesture_streams.contains_key(&key) {
            self.cancel_raw_gesture_stream(key, true);
            let elements = self.raw_gesture_elements(hit);
            let element = elements.first().copied()?;
            let mut active = ActiveRawGesture {
                element,
                members: Vec::new(),
            };
            let mut rejected = Vec::new();
            for element in elements {
                let factories = self
                    .elements
                    .get(element.0)
                    .and_then(|entry| match entry.widget.kind() {
                        WidgetKind::RawInput { kind, .. } => kind.factory_map(),
                        _ => None,
                    })
                    .unwrap_or(&[]);
                for factory in factories {
                    let type_id = factory.type_id();
                    let Some(recognizer) = self
                        .input
                        .raw_recognizers
                        .get_mut(&element)
                        .and_then(|recognizers| recognizers.get_mut(&type_id))
                    else {
                        continue;
                    };
                    let member = self.input.gesture_arena.add(key, false);
                    let decision = recognizer.observe_raw(event);
                    active.members.push(ActiveRawGestureMember {
                        element,
                        member,
                        type_id,
                        cancelled: false,
                    });
                    if matches!(
                        decision,
                        GestureDecision::Reject | GestureDecision::Cancelled
                    ) {
                        rejected.push(member);
                    }
                }
            }
            if active.members.is_empty() {
                let _ = self.input.gesture_arena.cancel(key);
                return None;
            }
            self.input.raw_gesture_streams.insert(key, active);
            self.input.pointer_captures.insert(key, element);
            for member in rejected {
                self.input.gesture_arena.reject(key, member);
            }
            return Some(element);
        }

        let active_members = self.input.raw_gesture_streams.get(&key).map(|active| {
            active
                .members
                .iter()
                .map(|member| (member.element, member.member, member.type_id))
                .collect::<Vec<_>>()
        })?;
        let dispositions = self.input.gesture_arena.entries(key);
        let mut accepts = Vec::new();
        let mut rejects = Vec::new();
        for (element, member, type_id) in active_members {
            match disposition_for(&dispositions, member) {
                GestureDisposition::Pending => {}
                // A winner keeps receiving its pointer's events, as in
                // Flutter, where the arena only decides who listens on.
                GestureDisposition::Accepted => {
                    if let Some(recognizer) = self
                        .input
                        .raw_recognizers
                        .get_mut(&element)
                        .and_then(|recognizers| recognizers.get_mut(&type_id))
                    {
                        match recognizer.observe_raw(event) {
                            GestureDecision::Accept(action) => recognizer.dispatch(action),
                            GestureDecision::Cancelled => recognizer.cancel(),
                            GestureDecision::Pending | GestureDecision::Reject => {}
                        }
                    }
                    continue;
                }
                GestureDisposition::Rejected | GestureDisposition::Cancelled => continue,
            }
            let Some(recognizer) = self
                .input
                .raw_recognizers
                .get_mut(&element)
                .and_then(|recognizers| recognizers.get_mut(&type_id))
            else {
                rejects.push(member);
                continue;
            };
            match recognizer.observe_raw(event) {
                GestureDecision::Accept(action) => accepts.push((member, action)),
                GestureDecision::Reject | GestureDecision::Cancelled => rejects.push(member),
                GestureDecision::Pending => {}
            }
        }
        for member in rejects {
            self.input.gesture_arena.reject(key, member);
            self.apply_raw_arena_entries(key, self.input.gesture_arena.entries(key));
        }
        for (member, action) in accepts {
            let entries = self.input.gesture_arena.accept(key, member);
            self.apply_raw_arena_entries(key, entries);
            if disposition_for(&self.input.gesture_arena.entries(key), member)
                == GestureDisposition::Accepted
            {
                self.dispatch_raw_gesture_action(key, member, action);
            }
        }
        let element = self.input.raw_gesture_streams.get(&key)?.element;
        if event.phase == PointerPhase::Cancel
            || (event.phase == PointerPhase::Up && event.buttons == 0)
        {
            if event.phase == PointerPhase::Cancel {
                self.cancel_raw_gesture_stream(key, true);
            } else {
                self.remove_raw_gesture_stream(key);
            }
        }
        Some(element)
    }

    fn dispatch_raw_gesture_action(
        &mut self,
        key: GestureArenaKey,
        member: GestureArenaMember,
        action: GestureAction,
    ) {
        let Some((element, type_id)) =
            self.input.raw_gesture_streams.get(&key).and_then(|active| {
                active
                    .members
                    .iter()
                    .find(|candidate| candidate.member == member)
                    .map(|candidate| (candidate.element, candidate.type_id))
            })
        else {
            return;
        };
        if let Some(recognizer) = self
            .input
            .raw_recognizers
            .get_mut(&element)
            .and_then(|recognizers| recognizers.get_mut(&type_id))
        {
            recognizer.dispatch(action);
        }
    }

    fn apply_raw_arena_entries(&mut self, key: GestureArenaKey, entries: Vec<GestureArenaEntry>) {
        let to_cancel = {
            let Some(active) = self.input.raw_gesture_streams.get_mut(&key) else {
                return;
            };
            entries
                .into_iter()
                .filter(|entry| {
                    matches!(
                        entry.disposition,
                        GestureDisposition::Rejected | GestureDisposition::Cancelled
                    )
                })
                .filter_map(|entry| {
                    let member = active
                        .members
                        .iter_mut()
                        .find(|member| member.member == entry.member)?;
                    if member.cancelled {
                        return None;
                    }
                    member.cancelled = true;
                    Some((member.element, member.type_id))
                })
                .collect::<Vec<_>>()
        };
        for (element, type_id) in to_cancel {
            if let Some(recognizer) = self
                .input
                .raw_recognizers
                .get_mut(&element)
                .and_then(|recognizers| recognizers.get_mut(&type_id))
            {
                recognizer.cancel();
            }
        }
    }

    pub(super) fn cancel_raw_gesture_stream(&mut self, key: GestureArenaKey, notify: bool) {
        let Some(active) = self.input.raw_gesture_streams.get(&key) else {
            return;
        };
        let members = active
            .members
            .iter()
            .map(|member| (member.element, member.type_id))
            .collect::<Vec<_>>();
        let element = active.element;
        let _ = self.input.gesture_arena.cancel(key);
        if notify {
            for (member_element, type_id) in members {
                if let Some(recognizer) = self
                    .input
                    .raw_recognizers
                    .get_mut(&member_element)
                    .and_then(|recognizers| recognizers.get_mut(&type_id))
                {
                    recognizer.cancel();
                }
            }
        }
        self.input.raw_gesture_streams.remove(&key);
        if self.input.pointer_captures.get(&key) == Some(&element) {
            self.input.pointer_captures.remove(&key);
        }
    }

    fn cancel_raw_streams_for_element(&mut self, element: ElementId) {
        let streams = self
            .input
            .raw_gesture_streams
            .iter()
            .filter(|(_, active)| {
                active
                    .members
                    .iter()
                    .any(|member| member.element == element)
            })
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        for key in streams {
            self.cancel_raw_gesture_stream(key, true);
        }
    }

    fn remove_raw_gesture_stream(&mut self, key: GestureArenaKey) {
        let Some(active) = self.input.raw_gesture_streams.remove(&key) else {
            return;
        };
        let _ = self.input.gesture_arena.cancel(key);
        if self.input.pointer_captures.get(&key) == Some(&active.element) {
            self.input.pointer_captures.remove(&key);
        }
    }

    pub(super) fn raw_input_unmounted(&mut self, element: ElementId) {
        self.input
            .deferred_single_taps
            .retain(|tap| tap.owner != element && tap.double_tap_owner != element);
        self.input.tap_memories.remove(&element);
        self.cancel_raw_streams_for_element(element);
        // An empty captured route still marks a pressed contact. A later
        // buttonless move on that stream is not an independent hover.
        for route in self.input.raw_pointer_routes.values_mut() {
            route.retain(|candidate| candidate.element != element);
        }
        self.input.mouse_hover.retain(|_, hover| {
            hover.retain(|candidate| candidate.element != element);
            !hover.is_empty()
        });
        if let Some(mut recognizers) = self.input.raw_recognizers.remove(&element) {
            for recognizer in recognizers.values_mut() {
                recognizer.dispose();
            }
        }
    }
}

fn localize_with_transform(
    world_to_local: Option<CoreTransform>,
    event: RawPointerEvent,
) -> RawPointerEvent {
    event.transformed(world_to_local)
}

fn is_inside_tap_region(
    registration: &TapRegistration<'_>,
    hit: &[ElementId],
    hit_groups: &[&TapRegionGroupId],
) -> bool {
    hit.contains(&registration.id)
        || registration
            .group_id
            .is_some_and(|group| hit_groups.contains(&group))
}

fn window_chrome_blocks_at(kind: &WidgetKind) -> bool {
    match kind {
        WidgetKind::Button(_)
        | WidgetKind::SelectableText { .. }
        | WidgetKind::SelectionArea { .. }
        | WidgetKind::SelectionContainer { .. }
        | WidgetKind::TextField(_)
        | WidgetKind::Gesture { .. }
        | WidgetKind::Draggable { .. } => true,
        WidgetKind::AbsorbPointer { absorbing, .. } => *absorbing,
        WidgetKind::RawInput { kind, .. } => matches!(
            **kind,
            RawInputKind::Listener { .. }
                | RawInputKind::RawGestureDetector { .. }
                | RawInputKind::TapRegion { .. }
                | RawInputKind::TextFieldTapRegion { .. }
        ),
        _ => false,
    }
}
