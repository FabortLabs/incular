//! `layout()` / `layout_position()`: measured geometry animates.
//!
//! Position is FLIP on the compositor: after a layout moves the element,
//! the inverse offset is written into a translation channel and animated
//! to zero, so movement never relayouts. Rects are compared in the nearest
//! scrollable's content space, so scrolling never animates.
//!
//! Size (`layout()` only) lays the element out at the interpolated size each
//! frame, as its own relayout boundary, so text reflows instead of
//! stretching. Only that element relayouts.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use incular_animation::{MotionValue, Transition};
use incular_core::{Size, time::Instant};

use super::{
    LayoutMotion,
    geometry::GeometryListener,
    node::{ChannelSource, MotionNode},
    pose::{PROP_COUNT, Prop, Stage},
};
use crate::Widget;
use crate::custom_layout::{ChildSizedLayout, CustomLayoutDelegate};
use crate::tree::{FrameGeometry, RetainedState, WidgetKind};

/// The FLIP translation channel.
pub(crate) struct LayoutChannel {
    x: MotionValue,
    y: MotionValue,
    node: Weak<MotionNode>,
}

impl LayoutChannel {
    pub(crate) fn new(node: Weak<MotionNode>) -> Rc<Self> {
        Rc::new(Self {
            x: MotionValue::new(0.),
            y: MotionValue::new(0.),
            node,
        })
    }

    fn transition(&self) -> Transition {
        self.node
            .upgrade()
            .map_or_else(Transition::default, |node| node.layout_transition())
    }
}

/// Jumps `value` by `delta` (keeping its velocity) and animates it home.
fn retarget(value: &MotionValue, delta: f32, transition: &Transition) {
    if delta.abs() < 0.01 {
        return;
    }
    let velocity = value.velocity();
    value.set(value.get() + delta);
    value.set_velocity(velocity);
    value.animate_to(0., transition);
}

impl ChannelSource for LayoutChannel {
    fn tick(&self, now: Instant) -> bool {
        let x = self.x.tick(now);
        let y = self.y.tick(now);
        x || y
    }

    fn is_active(&self) -> bool {
        self.x.is_animating() || self.y.is_animating()
    }

    fn apply(&self, values: &mut [f32; PROP_COUNT]) {
        values[Prop::X.index()] += self.x.get();
        values[Prop::Y.index()] += self.y.get();
    }

    fn stages(&self, out: &mut [bool; 6]) {
        out[Stage::Transform as usize] = true;
    }
}

impl GeometryListener for LayoutChannel {
    fn geometry(&self, node: &MotionNode, previous: Option<FrameGeometry>, now: FrameGeometry) {
        let Some(previous) = previous else {
            return;
        };
        if previous.scroll_scope != now.scroll_scope {
            // Moved into another scrollable: no common space to animate in.
            return;
        }
        let from = previous.content_rect.origin;
        let to = now.content_rect.origin;
        if from == to {
            return;
        }
        // Offsets a behavior was showing (a reorder shift) become part of
        // the FLIP so the element continues from where it is on screen.
        let absorbed = node.absorb_offsets(to - from);
        let transition = self.transition();
        retarget(&self.x, from.x - to.x + absorbed.x, &transition);
        retarget(&self.y, from.y - to.y + absorbed.y, &transition);
        node.push();
        incular_animation::schedule_frame();
    }
}

/// The animated size of a `layout()` element.
pub(crate) struct SizeChannel {
    width: MotionValue,
    height: MotionValue,
    /// The child's own size last layout.
    target: Cell<Option<Size>>,
    node: Weak<MotionNode>,
    /// The size box's builder revision: bumping it relayouts the box.
    revision: RefCell<Option<Rc<Cell<u64>>>>,
}

impl SizeChannel {
    pub(crate) fn new(node: Weak<MotionNode>) -> Rc<Self> {
        Rc::new(Self {
            width: MotionValue::new(0.),
            height: MotionValue::new(0.),
            target: Cell::new(None),
            node,
            revision: RefCell::new(None),
        })
    }

    /// Called from layout with the child's natural size; returns the size to
    /// lay it out at.
    fn size_for(&self, natural: Size) -> Size {
        match self.target.replace(Some(natural)) {
            None => {
                self.width.set(natural.width);
                self.height.set(natural.height);
                natural
            }
            Some(previous) if previous != natural => {
                let transition = self
                    .node
                    .upgrade()
                    .map_or_else(Transition::default, |node| node.layout_transition());
                self.width.animate_to(natural.width, &transition);
                self.height.animate_to(natural.height, &transition);
                incular_animation::schedule_frame();
                self.current()
            }
            Some(_) => self.current(),
        }
    }

    fn current(&self) -> Size {
        Size::new(self.width.get().max(0.), self.height.get().max(0.))
    }

    fn relayout(&self) {
        if let Some(revision) = self.revision.borrow().as_ref() {
            revision.set(revision.get().wrapping_add(1));
        }
    }
}

impl ChannelSource for SizeChannel {
    fn tick(&self, now: Instant) -> bool {
        let width = self.width.tick(now);
        let height = self.height.tick(now);
        let changed = width || height;
        if changed {
            self.relayout();
        }
        // Size is applied by layout, not by compositor values.
        false
    }

    fn is_active(&self) -> bool {
        self.width.is_animating() || self.height.is_animating()
    }

    fn apply(&self, _values: &mut [f32; PROP_COUNT]) {}

    fn stages(&self, _out: &mut [bool; 6]) {}
}

struct SizeBox {
    channel: Rc<SizeChannel>,
}

impl RetainedState for SizeBox {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.channel.revision.borrow_mut() = Some(revision);
    }
}

/// Wraps `child` so it lays out at the channel's animated size.
pub(crate) fn sized(channel: &Rc<SizeChannel>, child: Widget) -> Widget {
    let channel = channel.clone();
    Widget::retained_builder(
        move |revision| {
            *channel.revision.borrow_mut() = Some(revision);
            SizeBox {
                channel: channel.clone(),
            }
        },
        move |state, _, _| {
            let weak = Rc::downgrade(&state.channel);
            let layout = ChildSizedLayout::reflowing(move |constraints, natural| {
                weak.upgrade().map_or(natural, |channel| {
                    constraints.constrain(channel.size_for(natural))
                })
            });
            Widget::from_kind(WidgetKind::CustomLayout {
                delegate: CustomLayoutDelegate::ChildSized(layout),
                children: vec![child.clone()],
            })
        },
        false,
    )
}

/// Whether the mode animates size as well as position.
pub(crate) fn animates_size(mode: LayoutMotion) -> bool {
    mode == LayoutMotion::Bounds
}
