//! Correlated absolute pan/zoom input, distinct from OS aggregate callbacks.

use super::*;
use incular_gestures::{
    DEFAULT_TRACKPAD_SCROLL_TO_SCALE_FACTOR, DragCallbacks, PointerDeviceKind, PointerPanZoomEvent,
    PointerPanZoomPhase, TrackpadDragAxis, TrackpadDragRecognizer,
};

enum Recognizer {
    Scale,
    SharedDrag(TrackpadDragAxis),
    Drag(Box<TrackpadDragRecognizer>),
}

struct Member {
    element: ElementId,
    id: GestureArenaMember,
    recognizer: Recognizer,
}

pub(super) struct ActivePanZoom {
    target: ElementId,
    members: Vec<Member>,
}

impl ActivePanZoom {
    pub(super) fn contains_element(&self, element: ElementId) -> bool {
        self.members.iter().any(|member| member.element == element)
    }

    pub(super) fn contains_scale(&self, element: ElementId) -> bool {
        self.members.iter().any(|member| {
            member.element == element && matches!(member.recognizer, Recognizer::Scale)
        })
    }
}

pub(super) struct ApplicationDragBinding {
    pub(super) axis: TrackpadDragAxis,
    callbacks: DragCallbacks,
    down: Option<Rc<dyn Fn(incular_gestures::DragDownDetails)>>,
    cancel: Option<Rc<dyn Fn()>>,
}

impl ApplicationDragBinding {
    fn take_axis(callbacks: &mut GestureCallbacks, axis: TrackpadDragAxis) -> Self {
        let (down, start, update, end, cancel) = match axis {
            TrackpadDragAxis::Horizontal => (
                callbacks.on_horizontal_drag_down.take(),
                callbacks.on_horizontal_drag_start.take(),
                callbacks.on_horizontal_drag_update.take(),
                callbacks.on_horizontal_drag_end.take(),
                callbacks.on_horizontal_drag_cancel.take(),
            ),
            TrackpadDragAxis::Vertical => (
                callbacks.on_vertical_drag_down.take(),
                callbacks.on_vertical_drag_start.take(),
                callbacks.on_vertical_drag_update.take(),
                callbacks.on_vertical_drag_end.take(),
                callbacks.on_vertical_drag_cancel.take(),
            ),
            TrackpadDragAxis::Pan => (
                callbacks.on_pan_down.take(),
                callbacks.on_pan_start.take(),
                callbacks.on_pan_update.take(),
                callbacks.on_pan_end.take(),
                callbacks.on_pan_cancel.take(),
            ),
        };
        Self {
            axis,
            callbacks: DragCallbacks {
                on_start: start,
                on_update: update,
                on_end: end,
            },
            down,
            cancel,
        }
    }
}

impl WidgetTree {
    /// A pure application drag shares one recognizer across its contacts
    /// and across physical and absolute trackpad input. Mixed detectors,
    /// drag-and-drop and scrolling keep their existing specialized routes.
    pub(super) fn shared_application_drag(
        &self,
        element: ElementId,
        callbacks: &GestureCallbacks,
    ) -> Option<ApplicationDragBinding> {
        if !matches!(
            self.elements
                .get(element.0)
                .map(|element| element.widget.kind()),
            Some(WidgetKind::Gesture { .. })
        ) || callbacks.only_accept_drag_on_threshold
            || callbacks.on_scale_start.is_some()
            || callbacks.on_scale_update.is_some()
            || callbacks.on_scale_end.is_some()
            || callbacks.has_force_press()
        {
            return None;
        }
        let horizontal = callbacks.on_horizontal_drag_down.is_some()
            || callbacks.on_horizontal_drag_start.is_some()
            || callbacks.on_horizontal_drag_update.is_some()
            || callbacks.on_horizontal_drag_end.is_some()
            || callbacks.on_horizontal_drag_cancel.is_some();
        let vertical = callbacks.on_vertical_drag_down.is_some()
            || callbacks.on_vertical_drag_start.is_some()
            || callbacks.on_vertical_drag_update.is_some()
            || callbacks.on_vertical_drag_end.is_some()
            || callbacks.on_vertical_drag_cancel.is_some();
        let pan = callbacks.on_pan_down.is_some()
            || callbacks.on_pan_start.is_some()
            || callbacks.on_pan_update.is_some()
            || callbacks.on_pan_end.is_some()
            || callbacks.on_pan_cancel.is_some();
        let axis = match (horizontal, vertical, pan) {
            (true, false, false) => TrackpadDragAxis::Horizontal,
            (false, true, false) => TrackpadDragAxis::Vertical,
            (false, false, true) => TrackpadDragAxis::Pan,
            _ => return None,
        };
        let mut remaining = callbacks.clone();
        let binding = ApplicationDragBinding::take_axis(&mut remaining, axis);
        (!remaining.has_pointer_recognizer()).then_some(binding)
    }

    pub(super) fn shared_drag(
        &mut self,
        window: u64,
        element: ElementId,
        binding: ApplicationDragBinding,
        behavior: incular_core::DragStartBehavior,
    ) -> &mut incular_gestures::MonodragGestureRecognizer {
        let transform = self
            .render_id(element)
            .and_then(|render| self.pointer_world_to_local(render));
        let settings = self.gesture_settings_at(element);
        let drag = self
            .input
            .shared_drags
            .entry((window, element, binding.axis))
            .or_insert_with(|| {
                TrackpadDragRecognizer::new(binding.axis, binding.callbacks)
                    .with_pending_callbacks(binding.down, binding.cancel)
                    .with_drag_start_behavior(behavior)
            });
        drag.set_gesture_settings(settings);
        if let Some(transform) = transform {
            drag.set_local_transform(transform);
        }
        drag
    }

    /// Routes one absolute trackpad stream to mounted scale and drag
    /// recognizers. Its hit path is captured on Start. This is not the
    /// independent OS aggregate gesture route.
    pub fn dispatch_pointer_pan_zoom_in_window(
        &mut self,
        window: u64,
        event: PointerPanZoomEvent,
    ) -> Option<ElementId> {
        let key = GestureArenaKey::pointer_pan_zoom(window, event.device, event.pointer);
        if event.phase == PointerPanZoomPhase::Start {
            self.cancel_pointer_pan_zoom(key);
            self.begin_pointer_pan_zoom(key, event)?;
        }
        let stream = self.input.active_pan_zoom.get(&key)?;
        let target = stream.target;
        let count = stream.members.len();
        for index in 0..count {
            let member = &mut self.input.active_pan_zoom.get_mut(&key)?.members[index];
            let Some(status) = self.input.gesture_arena.disposition(key, member.id) else {
                continue;
            };
            if matches!(
                status,
                GestureDisposition::Rejected | GestureDisposition::Cancelled
            ) {
                continue;
            }
            let element = member.element;
            let id = member.id;
            match &mut member.recognizer {
                Recognizer::Scale => {
                    let scroll_gain =
                        self.elements.get(element.0).and_then(|mounted| {
                            match mounted.widget.kind() {
                                WidgetKind::Gesture { callbacks, .. }
                                    if callbacks.trackpad_scroll_causes_scale =>
                                {
                                    Some(
                                        callbacks
                                            .trackpad_scroll_to_scale_factor
                                            .unwrap_or(DEFAULT_TRACKPAD_SCROLL_TO_SCALE_FACTOR),
                                    )
                                }
                                _ => None,
                            }
                        });
                    let Some(scale) = self.input.scale_gestures.get_mut(&element) else {
                        continue;
                    };
                    scale.set_trackpad_scroll_to_scale(scroll_gain);
                    let observed = scale.observe_pan_zoom(event);
                    let details = if scale.wants_acceptance() {
                        self.accept_scale(window, element, observed)
                    } else if status == GestureDisposition::Accepted {
                        observed
                    } else {
                        None
                    };
                    if let Some(details) = details
                        && let Some(scale) = self.input.scale_gestures.get(&element)
                    {
                        scale.dispatch(details);
                    }
                }
                Recognizer::SharedDrag(axis) => {
                    let wants = self
                        .input
                        .shared_drags
                        .get_mut(&(window, element, *axis))
                        .is_some_and(|pan| pan.observe(event) && pan.wants_acceptance());
                    if wants {
                        let entries = self.input.gesture_arena.accept(key, id);
                        self.apply_pan_zoom_entries(key, &entries, false);
                    }
                }
                Recognizer::Drag(drag) => {
                    if drag.observe(event) && drag.wants_acceptance() {
                        let entries = self.input.gesture_arena.accept(key, id);
                        self.apply_pan_zoom_entries(key, &entries, false);
                    }
                }
            }
        }
        if event.phase == PointerPanZoomPhase::Start {
            let entries = self.input.gesture_arena.close_sole_member(key);
            self.apply_pan_zoom_entries(key, &entries, true);
        } else if event.phase == PointerPanZoomPhase::End {
            self.cancel_pointer_pan_zoom(key);
        }
        Some(target)
    }

    fn begin_pointer_pan_zoom(
        &mut self,
        key: GestureArenaKey,
        event: PointerPanZoomEvent,
    ) -> Option<()> {
        let hit = self.pointer_hit_test(event.position);
        let gestures = hit.coincident_gestures();
        let target = hit
            .target()
            .and_then(|render| self.element_for_render(render))
            .or_else(|| gestures.first().copied())?;
        let mut members = Vec::new();
        for element in self.gesture_route_elements(target, gestures) {
            let Some(callbacks) = self.gesture_callbacks(element) else {
                continue;
            };
            let admits_drag = callbacks.allows_pointer_device(PointerDeviceKind::Trackpad);
            if admits_drag && let Some(binding) = self.shared_application_drag(element, &callbacks) {
                let axis = binding.axis;
                self.shared_drag(key.window, element, binding, callbacks.drag_start_behavior);
                members.push(Member {
                    element,
                    id: self.input.gesture_arena.add(key, false),
                    recognizer: Recognizer::SharedDrag(axis),
                });
                continue;
            }
            let transform = self
                .render_id(element)
                .and_then(|render| self.pointer_world_to_local(render));
            let settings = self.gesture_settings_at(element);
            for (axis, down, start, update, end, cancel) in [
                (
                    TrackpadDragAxis::Vertical,
                    callbacks.on_vertical_drag_down,
                    callbacks.on_vertical_drag_start,
                    callbacks.on_vertical_drag_update,
                    callbacks.on_vertical_drag_end,
                    callbacks.on_vertical_drag_cancel,
                ),
                (
                    TrackpadDragAxis::Horizontal,
                    callbacks.on_horizontal_drag_down,
                    callbacks.on_horizontal_drag_start,
                    callbacks.on_horizontal_drag_update,
                    callbacks.on_horizontal_drag_end,
                    callbacks.on_horizontal_drag_cancel,
                ),
                (
                    TrackpadDragAxis::Pan,
                    callbacks.on_pan_down,
                    callbacks.on_pan_start,
                    callbacks.on_pan_update,
                    callbacks.on_pan_end,
                    callbacks.on_pan_cancel,
                ),
            ] {
                if !admits_drag
                    || (down.is_none()
                        && start.is_none()
                        && update.is_none()
                        && end.is_none()
                        && cancel.is_none())
                {
                    continue;
                }
                let mut drag = TrackpadDragRecognizer::new(
                    axis,
                    DragCallbacks {
                        on_start: start,
                        on_update: update,
                        on_end: end,
                    },
                )
                .with_drag_start_behavior(callbacks.drag_start_behavior)
                .with_pending_callbacks(down, cancel);
                drag.set_gesture_settings(settings);
                if let Some(transform) = transform {
                    drag.set_local_transform(transform);
                }
                members.push(Member {
                    element,
                    id: self.input.gesture_arena.add(key, false),
                    recognizer: Recognizer::Drag(Box::new(drag)),
                });
            }
            if callbacks.on_scale_start.is_some()
                || callbacks.on_scale_update.is_some()
                || callbacks.on_scale_end.is_some()
            {
                let scale = self.input.scale_gestures.entry(element).or_insert_with(|| {
                    ScaleGestureDetector::with_callbacks(
                        callbacks.on_scale_update,
                        callbacks.on_scale_start,
                        callbacks.on_scale_end,
                    )
                    .with_drag_start_behavior(callbacks.drag_start_behavior)
                });
                scale.set_gesture_settings(settings);
                if let Some(transform) = transform {
                    scale.set_local_transform(transform);
                }
                members.push(Member {
                    element,
                    id: self.input.gesture_arena.add(key, true),
                    recognizer: Recognizer::Scale,
                });
            }
        }
        if members.is_empty() {
            return None;
        }
        self.input
            .active_pan_zoom
            .insert(key, ActivePanZoom { target, members });
        Some(())
    }

    // Shared with touch: a scale can have both kinds of contact enrolled.
    pub(super) fn accept_scale_on_pan_zoom(&mut self, window: u64, element: ElementId) -> bool {
        let members: Vec<_> = self
            .input
            .active_pan_zoom
            .iter()
            .filter(|(key, _)| key.window == window)
            .flat_map(|(key, stream)| {
                stream.members.iter().filter_map(|member| {
                    (member.element == element && matches!(member.recognizer, Recognizer::Scale))
                        .then_some((*key, member.id))
                })
            })
            .collect();
        let mut accepted = false;
        for (key, member) in members {
            let entries = self.input.gesture_arena.accept(key, member);
            accepted |= disposition_for(&entries, member) == GestureDisposition::Accepted;
            self.apply_pan_zoom_entries(key, &entries, false);
        }
        accepted
    }

    fn apply_pan_zoom_entries(
        &mut self,
        key: GestureArenaKey,
        entries: &[GestureArenaEntry],
        default_win: bool,
    ) {
        let Some(stream) = self.input.active_pan_zoom.get_mut(&key) else {
            return;
        };
        let (Some(device), Some(pointer)) = (key.pointer_device_id(), key.pointer_id()) else {
            return;
        };
        for entry in entries {
            let Some(member) = stream
                .members
                .iter_mut()
                .find(|member| member.id == entry.member)
            else {
                continue;
            };
            match (&mut member.recognizer, entry.disposition) {
                (Recognizer::SharedDrag(axis), disposition) => {
                    if let Some(pan) = self
                        .input
                        .shared_drags
                        .get_mut(&(key.window, member.element, *axis))
                    {
                        match disposition {
                            GestureDisposition::Accepted => pan.accept_stream(key),
                            GestureDisposition::Rejected | GestureDisposition::Cancelled => {
                                pan.reject_stream(key)
                            }
                            GestureDisposition::Pending => {}
                        }
                    }
                }
                (Recognizer::Drag(drag), GestureDisposition::Accepted) => drag.accept(),
                (
                    Recognizer::Drag(drag),
                    GestureDisposition::Rejected | GestureDisposition::Cancelled,
                ) => drag.reject(device, pointer),
                (
                    Recognizer::Scale,
                    GestureDisposition::Rejected | GestureDisposition::Cancelled,
                ) => {
                    if let Some(scale) = self.input.scale_gestures.get_mut(&member.element) {
                        scale.reject_pan_zoom(device, pointer);
                    }
                }
                (Recognizer::Scale, GestureDisposition::Accepted) if default_win => {
                    if let Some(scale) = self.input.scale_gestures.get_mut(&member.element) {
                        scale.accept_without_update();
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn cancel_pointer_pan_zoom(&mut self, key: GestureArenaKey) {
        if !self.input.active_pan_zoom.contains_key(&key) {
            return;
        }
        let entries = self.input.gesture_arena.cancel(key);
        self.apply_pan_zoom_entries(key, &entries, false);
        self.input.active_pan_zoom.remove(&key);
        self.prune_shared_drags();
        self.remove_scale_recognizers_when_idle();
    }

    pub(super) fn prune_shared_drags(&mut self) {
        self.input.shared_drags.retain(|_, pan| pan.is_tracking());
    }
}
