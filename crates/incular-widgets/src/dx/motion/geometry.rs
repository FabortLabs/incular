//! Laid-out geometry the motion node reacts to: drag bounds, in-view state,
//! and (through [`GeometryListener`]s) layout, shared and reorder motion.

use std::rc::Rc;

use incular_core::Rect;

use super::{cond::Interaction, node::MotionNode};
use crate::{Widget, tree::FrameGeometry};

/// Something that reacts to the node's geometry after each layout.
pub(crate) trait GeometryListener {
    fn geometry(&self, node: &MotionNode, previous: Option<FrameGeometry>, now: FrameGeometry);
}

/// Records the node's laid-out size when something reads it.
pub(crate) fn observe_size(node: &Rc<MotionNode>, widget: Widget) -> Widget {
    if node.spec().drag.is_none() {
        return widget;
    }
    let weak = Rc::downgrade(node);
    Widget::size_observer(
        move |size| {
            if let Some(node) = weak.upgrade() {
                node.size.set(size);
            }
        },
        widget,
    )
}

/// Whether the node needs its geometry reported.
pub(crate) fn wants(node: &MotionNode) -> bool {
    node.interactions_used().contains(Interaction::InView)
        || node
            .spec()
            .drag
            .as_ref()
            .is_some_and(|drag| drag.0.bounds == crate::dx::drag::Bounds::Parent)
        || !node.geometry_listeners.borrow().is_empty()
        || node
            .timeline
            .borrow()
            .as_ref()
            .is_some_and(|timeline| timeline.wants_geometry())
        || node
            .behaviors()
            .iter()
            .any(|behavior| behavior.wants_geometry())
}

fn area(rect: Rect) -> f32 {
    rect.size.width.max(0.) * rect.size.height.max(0.)
}

/// The visible fraction of `rect` inside `viewport` (1 for an empty rect
/// whose origin is visible).
pub(crate) fn visible_fraction(rect: Rect, viewport: Rect) -> f32 {
    let total = area(rect);
    if total <= 0. {
        return if viewport.contains(rect.origin) {
            1.
        } else {
            0.
        };
    }
    rect.intersection(viewport)
        .map_or(0., |visible| area(visible) / total)
}

/// Applies a newly reported geometry.
pub(crate) fn report(node: &MotionNode, geometry: FrameGeometry) {
    let previous = node.geometry.replace(Some(geometry));
    if previous == Some(geometry) {
        return;
    }
    if node.interactions_used().contains(Interaction::InView) {
        let options = node.spec().in_view;
        let fraction = visible_fraction(geometry.visual_rect, geometry.viewport);
        let visible = if options.amount <= 0. {
            fraction > 0.
        } else {
            fraction >= options.amount
        };
        let was = node.interactions().contains(Interaction::InView);
        if !(options.once && was) {
            node.set_flag(Interaction::InView, visible);
            if was != visible {
                incular_animation::schedule_frame();
            }
        }
    }
    let listeners = node.geometry_listeners.borrow().clone();
    for listener in listeners {
        listener.geometry(node, previous, geometry);
    }
}
