//! Free dragging on the compositor: the offset is a pair of motion values
//! added to the element's translation.

use std::{cell::Cell, rc::Rc};

use incular_animation::{MotionValue, Spring, Transition, schedule_frame};
use incular_core::{Offset, Rect, time::Instant};

use super::{
    Motion,
    cond::Interaction,
    node::{ChannelSource, MotionNode},
    pose::{PROP_COUNT, Prop, Stage},
};
use crate::dx::drag::{Bounds, Drag, DragAxis, Release};
use crate::{GestureDetector, Widget, tree::FrameGeometry};

/// How far the release velocity carries a momentum drag (seconds).
const MOMENTUM_POWER: f32 = 0.25;

pub(crate) struct DragChannel {
    pub(crate) x: MotionValue,
    pub(crate) y: MotionValue,
    raw: Cell<Offset>,
    revision: Cell<(u64, u64)>,
}

impl DragChannel {
    fn new() -> Self {
        Self {
            x: MotionValue::new(0.),
            y: MotionValue::new(0.),
            raw: Cell::new(Offset::ZERO),
            revision: Cell::new((0, 0)),
        }
    }

    pub(crate) fn offset(&self) -> Offset {
        Offset::new(self.x.get(), self.y.get())
    }

    fn set(&self, offset: Offset) {
        self.x.set(offset.x);
        self.y.set(offset.y);
        schedule_frame();
    }
}

impl ChannelSource for DragChannel {
    fn tick(&self, now: Instant) -> bool {
        let mut changed = self.x.tick(now);
        changed |= self.y.tick(now);
        let revision = (self.x.revision(), self.y.revision());
        if self.revision.replace(revision) != revision {
            changed = true;
        }
        changed
    }

    fn is_active(&self) -> bool {
        self.x.is_animating()
            || self.y.is_animating()
            || self.revision.get() != (self.x.revision(), self.y.revision())
    }

    fn apply(&self, values: &mut [f32; PROP_COUNT]) {
        values[Prop::X.index()] += self.x.get();
        values[Prop::Y.index()] += self.y.get();
    }

    fn stages(&self, out: &mut [bool; 6]) {
        out[Stage::Transform as usize] = true;
    }
}

fn clamp_axis(value: f32, min: f32, max: f32, elastic: f32) -> f32 {
    if value < min {
        min + (value - min) * elastic
    } else if value > max {
        max + (value - max) * elastic
    } else {
        value
    }
}

/// The allowed offset range for `Bounds::Parent`: how far the element may
/// move before an edge leaves its parent's box.
pub(crate) fn parent_offsets(geometry: Option<FrameGeometry>) -> Option<Rect> {
    let geometry = geometry?;
    let own = geometry.layout_rect;
    let parent = geometry.parent_rect;
    let min = Offset::new(
        parent.origin.x - own.origin.x,
        parent.origin.y - own.origin.y,
    );
    let max = Offset::new(
        parent.origin.x + parent.size.width - (own.origin.x + own.size.width),
        parent.origin.y + parent.size.height - (own.origin.y + own.size.height),
    );
    Some(Rect::from_origin_size(
        min,
        incular_core::Size::new((max.x - min.x).max(0.), (max.y - min.y).max(0.)),
    ))
}

pub(crate) fn bounded(offset: Offset, drag: &Drag, elastic: f32, parent: Option<Rect>) -> Offset {
    let rect = match drag.bounds {
        Bounds::Offsets(rect) => Some(rect),
        Bounds::Parent => parent,
        Bounds::None => None,
    };
    match rect {
        Some(rect) => Offset::new(
            clamp_axis(
                offset.x,
                rect.origin.x,
                rect.origin.x + rect.size.width,
                elastic,
            ),
            clamp_axis(
                offset.y,
                rect.origin.y,
                rect.origin.y + rect.size.height,
                elastic,
            ),
        ),
        None => offset,
    }
}

fn axis_filter(delta: Offset, axis: DragAxis) -> Offset {
    match axis {
        DragAxis::X => Offset::new(delta.x, 0.),
        DragAxis::Y => Offset::new(0., delta.y),
        DragAxis::Both => delta,
    }
}

/// Where a release at `current` moving at `velocity` comes to rest.
pub(crate) fn release_target(
    drag: &Drag,
    current: Offset,
    velocity: Offset,
    parent: Option<Rect>,
) -> Option<Offset> {
    let velocity = axis_filter(velocity, drag.axis);
    let projected = Offset::new(
        current.x + velocity.x * MOMENTUM_POWER,
        current.y + velocity.y * MOMENTUM_POWER,
    );
    match &drag.release {
        Release::Stay => None,
        Release::SnapBack => Some(Offset::ZERO),
        Release::Momentum => Some(bounded(projected, drag, 0., parent)),
        Release::SnapTo(points) => Some(
            points
                .iter()
                .copied()
                .min_by(|a, b| {
                    let da = (a.x - projected.x).powi(2) + (a.y - projected.y).powi(2);
                    let db = (b.x - projected.x).powi(2) + (b.y - projected.y).powi(2);
                    da.total_cmp(&db)
                })
                .unwrap_or(Offset::ZERO),
        ),
    }
}

fn release(channel: &DragChannel, drag: &Drag, velocity: Offset, parent: Option<Rect>) {
    let velocity = axis_filter(velocity, drag.axis);
    let Some(target) = release_target(drag, channel.offset(), velocity, parent) else {
        channel.x.stop();
        channel.y.stop();
        return;
    };
    channel.x.set_velocity(velocity.x);
    channel.y.set_velocity(velocity.y);
    let transition = Transition::spring(Spring::smooth());
    channel.x.animate_to(target.x, &transition);
    channel.y.animate_to(target.y, &transition);
    schedule_frame();
}

/// The node's drag channel, created on first use.
pub(crate) fn channel(node: &Rc<MotionNode>) -> Rc<DragChannel> {
    node.drag_channel
        .borrow_mut()
        .get_or_insert_with(|| {
            let channel = Rc::new(DragChannel::new());
            node.add_extra(channel.clone());
            channel
        })
        .clone()
}

fn drag_of(spec: &Motion) -> Option<&Drag> {
    spec.drag.as_ref().map(|drag| &drag.0)
}

/// A drag began at `global` (from the element or one of its handles).
pub(crate) fn drag_start(node: &Rc<MotionNode>, global: Offset) {
    let channel = channel(node);
    channel.x.stop();
    channel.y.stop();
    channel.raw.set(channel.offset());
    node.set_flag(Interaction::Dragging, true);
    for behavior in node.behaviors() {
        behavior.drag_start(node, global);
    }
}

/// The pointer moved by `delta` to `global`.
pub(crate) fn drag_update(node: &Rc<MotionNode>, delta: Offset, global: Offset) {
    let spec = node.spec();
    let Some(drag) = drag_of(&spec) else { return };
    let channel = channel(node);
    let axis = node.drag_axis.get().unwrap_or(drag.axis);
    let raw = channel.raw.get() + axis_filter(delta, axis);
    channel.raw.set(raw);
    let parent = parent_offsets(node.geometry.get());
    channel.set(bounded(raw, drag, drag.elastic, parent));
    for behavior in node.behaviors() {
        behavior.drag_update(node, global);
    }
}

/// The drag ended (or was cancelled) at `global`.
pub(crate) fn drag_end(node: &Rc<MotionNode>, global: Offset, velocity: Offset, cancelled: bool) {
    let spec = node.spec();
    let Some(drag) = drag_of(&spec) else { return };
    node.set_flag(Interaction::Dragging, false);
    let mut settled = false;
    for behavior in node.behaviors() {
        settled |= behavior.drag_end(node, global, velocity, cancelled);
    }
    if !settled {
        let velocity = if cancelled { Offset::ZERO } else { velocity };
        release(
            &channel(node),
            drag,
            velocity,
            parent_offsets(node.geometry.get()),
        );
    }
}

/// Shifts the drag offset (for example while auto-scrolling) so the element
/// stays under the pointer.
pub(crate) fn nudge(node: &Rc<MotionNode>, delta: Offset) {
    let channel = channel(node);
    let raw = channel.raw.get() + delta;
    channel.raw.set(raw);
    channel.set(raw);
}

/// Springs the drag offset to `target` (used to settle into a slot).
pub(crate) fn settle_to(node: &Rc<MotionNode>, target: Offset, transition: &Transition) {
    let channel = channel(node);
    channel.raw.set(target);
    channel.x.animate_to(target.x, transition);
    channel.y.animate_to(target.y, transition);
    schedule_frame();
}

/// The current drag offset, zeroed (absorbed by a layout change).
pub(crate) fn take_offset(node: &MotionNode) -> Offset {
    let Some(channel) = node.drag_channel.borrow().clone() else {
        return Offset::ZERO;
    };
    let offset = channel.offset();
    channel.x.set(0.);
    channel.y.set(0.);
    channel.raw.set(Offset::ZERO);
    offset
}

/// A pan gesture driving `node`'s drag. With `long_press_touch`, touch
/// starts the drag with a long press so quick swipes keep scrolling; once
/// lifted, the drag follows the finger whichever recognizer reports it.
pub(crate) fn pan(node: &Rc<MotionNode>, widget: Widget) -> Widget {
    let long_press = node.long_press_touch.get();
    let last = Rc::new(Cell::new(Offset::ZERO));
    // A touch pan left to scrolling (not a drag of this element).
    let ignored = Rc::new(Cell::new(false));
    // A drag a long press started and has not ended yet.
    let lifted = Rc::new(Cell::new(false));
    let weak = Rc::downgrade(node);
    let follow = {
        let (weak, last) = (weak.clone(), last.clone());
        move |global: Offset| {
            let Some(node) = weak.upgrade() else { return };
            let delta = global - last.get();
            last.set(global);
            if delta != Offset::ZERO {
                drag_update(&node, delta, global);
            }
        }
    };
    let finish = {
        let (weak, last, lifted) = (weak.clone(), last.clone(), lifted.clone());
        move |global: Option<Offset>, velocity: Offset, cancelled: bool| {
            if !lifted.replace(false) {
                return;
            }
            let Some(node) = weak.upgrade() else { return };
            drag_end(&node, global.unwrap_or(last.get()), velocity, cancelled);
        }
    };
    let mut detector = GestureDetector::new(widget)
        .drag_start_behavior(incular_core::DragStartBehavior::Down)
        .on_pan_start({
            let (weak, last, ignored, lifted) =
                (weak.clone(), last.clone(), ignored.clone(), lifted.clone());
            move |details| {
                let Some(node) = weak.upgrade() else { return };
                let touch = details.kind == Some(incular_core::PointerDeviceKind::Touch);
                ignored.set(long_press && touch);
                if ignored.get() || lifted.get() {
                    return;
                }
                last.set(details.global_position);
                drag_start(&node, details.global_position);
            }
        })
        .on_pan_update({
            let (weak, last, ignored, lifted, follow) = (
                weak.clone(),
                last.clone(),
                ignored.clone(),
                lifted.clone(),
                follow.clone(),
            );
            move |details| {
                if lifted.get() {
                    follow(details.global_position);
                    return;
                }
                let Some(node) = weak.upgrade() else { return };
                if ignored.get() {
                    return;
                }
                last.set(details.global_position);
                drag_update(&node, details.delta, details.global_position);
            }
        })
        .on_pan_end({
            let (weak, ignored, lifted, finish) = (
                weak.clone(),
                ignored.clone(),
                lifted.clone(),
                finish.clone(),
            );
            move |details| {
                let ignored = ignored.replace(false);
                if lifted.get() {
                    finish(
                        Some(details.global_position),
                        details.velocity.pixels_per_second,
                        false,
                    );
                    return;
                }
                let Some(node) = weak.upgrade() else { return };
                if ignored {
                    return;
                }
                drag_end(
                    &node,
                    details.global_position,
                    details.velocity.pixels_per_second,
                    false,
                );
            }
        })
        .on_pan_cancel({
            let (weak, last, ignored, lifted, finish) = (
                weak.clone(),
                last.clone(),
                ignored.clone(),
                lifted.clone(),
                finish.clone(),
            );
            move || {
                let ignored = ignored.replace(false);
                if lifted.get() {
                    finish(None, Offset::ZERO, true);
                    return;
                }
                let Some(node) = weak.upgrade() else { return };
                if ignored {
                    return;
                }
                drag_end(&node, last.get(), Offset::ZERO, true);
            }
        });
    if long_press {
        detector = detector
            .on_long_press_start({
                let (weak, last, lifted) = (weak.clone(), last.clone(), lifted.clone());
                move |details| {
                    let Some(node) = weak.upgrade() else { return };
                    if lifted.replace(true) {
                        return;
                    }
                    last.set(details.global_position);
                    drag_start(&node, details.global_position);
                }
            })
            .on_long_press_move_update({
                let (lifted, follow) = (lifted.clone(), follow.clone());
                move |details| {
                    if lifted.get() {
                        follow(details.global_position);
                    }
                }
            })
            .on_long_press_end({
                let finish = finish.clone();
                move |details| {
                    finish(
                        Some(details.global_position),
                        details.velocity.pixels_per_second,
                        false,
                    );
                }
            })
            .on_long_press_cancel(move || finish(None, Offset::ZERO, true));
    }
    detector.into()
}

/// Published by a handle-only drag so a `reorder_handle()` finds its item.
#[derive(Clone)]
pub(crate) struct HandleScope(pub(crate) std::rc::Weak<MotionNode>);

/// Installs the drag gesture and its channel.
pub(crate) fn attach(node: &Rc<MotionNode>, drag: &Drag, widget: Widget) -> Widget {
    let _ = channel(node);
    if drag.handle_only {
        return Widget::environment_scope(HandleScope(Rc::downgrade(node)), widget);
    }
    pan(node, widget)
}
