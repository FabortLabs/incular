use super::{ElementId, Runtime, TextInputClientId};
use incular_core::{Offset, Rect, Size, time::Instant};
use incular_platform::{FloatingCursorDragState, FlutterFloatingCursorPoint};
use incular_text::{TextAffinity, TextEditingController, TextSelection};
use incular_widgets::FloatingCaretGeometry;
use std::time::Duration;

const RESET_DURATION: Duration = Duration::from_millis(125);

pub(super) struct FloatingCaretRun {
    field: ElementId,
    client: TextInputClientId,
    controller: TextEditingController,
    anchor: Offset,
    offset: Offset,
    text: (usize, TextAffinity),
    x: AxisBounds,
    y: AxisBounds,
    settling_at: Option<Instant>,
}

#[derive(Clone, Copy, Default)]
struct AxisBounds {
    relative_origin: f32,
    previous: Option<f32>,
    overshoot: i8,
}

impl AxisBounds {
    fn bound(&mut self, raw: f32, min: f32, max: f32) -> f32 {
        let delta = self.previous.map_or(0., |previous| raw - previous);
        if self.overshoot < 0 && delta > 0. {
            self.relative_origin = raw - min;
            self.overshoot = 0;
        } else if self.overshoot > 0 && delta < 0. {
            self.relative_origin = raw - max;
            self.overshoot = 0;
        }
        let local = raw - self.relative_origin;
        if local < min && delta < 0. {
            self.overshoot = -1;
        } else if local > max && delta > 0. {
            self.overshoot = 1;
        }
        self.previous = Some(raw);
        local.clamp(min, max)
    }
}

impl FloatingCaretRun {
    fn bound(&mut self, point: Offset, geometry: FloatingCaretGeometry) {
        self.offset = Offset::new(
            self.x
                .bound(point.x, geometry.minimum.x, geometry.maximum.x),
            self.y
                .bound(point.y, geometry.minimum.y, geometry.maximum.y),
        );
    }

    fn rect(&self, size: Size, inflation: f32) -> Rect {
        Rect::from_origin_size(
            self.offset - Offset::new(0.5 * inflation, inflation),
            Size::new(size.width + inflation, size.height + 2. * inflation),
        )
    }
}

impl Runtime {
    pub(crate) fn dispatch_flutter_floating_cursor(
        &mut self,
        client: TextInputClientId,
        point: FlutterFloatingCursorPoint,
    ) {
        let Some(field) = self.focused else {
            return;
        };
        match point.state {
            FloatingCursorDragState::Start => {
                if let Some(previous) = self.floating_caret.take() {
                    self.tree.set_floating_caret(previous.field, None);
                }
                let Some(controller) = self.tree.text_controller(field) else {
                    return;
                };
                let Some(selection) = controller.selection_if_set() else {
                    return;
                };
                let text = (selection.base, controller.caret_affinity());
                let Some(geometry) = self.tree.floating_caret_geometry(field, text.0, text.1)
                else {
                    return;
                };
                let caret = geometry.caret;
                let anchor = Offset::new(
                    caret.origin.x + caret.size.width * 0.5,
                    caret.origin.y + (caret.size.height - geometry.preferred_height) * 0.5,
                );
                let mut run = FloatingCaretRun {
                    field,
                    client,
                    controller,
                    text,
                    anchor,
                    offset: anchor,
                    x: AxisBounds::default(),
                    y: AxisBounds::default(),
                    settling_at: None,
                };
                run.bound(anchor, geometry);
                self.tree
                    .set_floating_caret(field, Some(run.rect(caret.size, 1.)));
                self.floating_caret = Some(run);
            }
            FloatingCursorDragState::Update => {
                if !self.floating_caret.as_ref().is_some_and(|run| {
                    run.field == field && run.client == client && run.settling_at.is_none()
                }) {
                    return;
                }
                let mut run = self
                    .floating_caret
                    .take()
                    .expect("validated floating caret");
                if self.tree.text_controller(field).as_ref() != Some(&run.controller) {
                    self.tree.set_floating_caret(field, None);
                    return;
                }
                let Some(geometry) = self
                    .tree
                    .floating_caret_geometry(field, run.text.0, run.text.1)
                else {
                    self.tree.set_floating_caret(field, None);
                    return;
                };
                let raw = run.anchor + point.offset;
                if !raw.x.is_finite() || !raw.y.is_finite() {
                    self.floating_caret = Some(run);
                    return;
                }
                run.bound(raw, geometry);
                let center = run.offset + Offset::new(0., geometry.preferred_height * 0.5);
                let world = geometry.world.transform_point(center);
                if world.x.is_finite()
                    && world.y.is_finite()
                    && let Some(text) = self.tree.text_field_position_at(field, world)
                {
                    run.text = text;
                }
                self.tree
                    .set_floating_caret(field, Some(run.rect(geometry.caret.size, 1.)));
                self.floating_caret = Some(run);
            }
            FloatingCursorDragState::End => {
                let now = self.input_now();
                let Some(run) = self.floating_caret.as_mut().filter(|run| {
                    run.field == field && run.client == client && run.settling_at.is_none()
                }) else {
                    return;
                };
                run.settling_at = Some(now);
            }
        }
        self.frame_requested = true;
    }

    pub(super) fn sample_floating_caret(&mut self, now: Instant) -> bool {
        let Some(mut run) = self.floating_caret.take() else {
            return false;
        };
        if self.focused != Some(run.field)
            || self.active_flutter_editable_client() != Some(run.client)
            || self.tree.text_controller(run.field).as_ref() != Some(&run.controller)
        {
            self.tree.set_floating_caret(run.field, None);
            return false;
        }
        let Some(started) = run.settling_at else {
            self.floating_caret = Some(run);
            return false;
        };
        let Some(geometry) = self
            .tree
            .floating_caret_geometry(run.field, run.text.0, run.text.1)
        else {
            self.tree.set_floating_caret(run.field, None);
            return false;
        };
        let elapsed = now.saturating_duration_since(started);
        if elapsed >= RESET_DURATION {
            self.tree.set_floating_caret(run.field, None);
            if let Some(controller) = self.tree.text_controller(run.field)
                && controller
                    .selection_if_set()
                    .is_some_and(TextSelection::is_collapsed)
            {
                controller
                    .set_selection_with_affinity(TextSelection::collapsed(run.text.0), run.text.1);
                controller.reset_caret(now);
            }
            return false;
        }

        let time = elapsed.as_secs_f32() / RESET_DURATION.as_secs_f32();
        let amount = 1. - (1. - time) * (1. - time);
        let destination = geometry.caret.origin
            + Offset::new(
                0.,
                (geometry.caret.size.height - geometry.preferred_height) * 0.5,
            );
        let start = run.offset;
        let delta = destination - start;
        run.offset = start + Offset::new(delta.x * amount, delta.y * amount);
        self.tree
            .set_floating_caret(run.field, Some(run.rect(geometry.caret.size, 1. - amount)));
        run.offset = start;
        self.floating_caret = Some(run);
        true
    }
}
