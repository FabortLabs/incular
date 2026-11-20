//! `shared(id)` / `shared_bounds(id)`: one element in two places (§8.1).
//!
//! Every shared site joins a per-thread registry entry keyed by its id (and
//! the nearest `shared_scope()`). The newest member holds the element. When
//! a site joins, it starts at the rect the previous holder was showing (or
//! had just shown, if it already left) and animates home on the compositor
//! (translate and scale, never relayout). When the holder leaves, the newest
//! remaining member takes over the same way, so a detail page closing hands
//! the element back to its thumbnail.
//!
//! `Element` hides the other members (one element on screen). `Bounds` is
//! different content sharing bounds: the newcomer fades in while a previous
//! holder that is still mounted (an exiting element) morphs to the new
//! bounds and fades out.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    time::Duration,
};

use incular_animation::{MotionValue, Transition};
use incular_core::{Offset, Rect, time::Instant};

use super::{
    SharedId, SharedMode,
    geometry::GeometryListener,
    node::{ChannelSource, MotionNode},
    pose::{PROP_COUNT, Prop, Stage},
};
use crate::tree::FrameGeometry;

/// How long after an element left a newcomer still starts from its rect.
const HANDOFF_WINDOW: Duration = Duration::from_millis(500);

/// Published by `.motion(|m| m.shared_scope())`: ids inside do not match ids
/// outside.
#[derive(Clone, Copy)]
pub(crate) struct SharedScope(pub(crate) u64);

/// A fresh namespace id.
pub(crate) fn next_scope() -> u64 {
    thread_local! {
        static NEXT: Cell<u64> = const { Cell::new(1) };
    }
    NEXT.with(|next| {
        let id = next.get();
        next.set(id + 1);
        id
    })
}

type Key = (u64, SharedId);

#[derive(Default)]
struct Entry {
    holder: Option<Weak<SharedChannel>>,
    /// What the holder last showed on screen.
    last: Option<Rect>,
    /// When the holder left, if it did and nobody took over yet.
    left_at: Option<Instant>,
    /// Members in join order.
    members: Vec<Weak<SharedChannel>>,
}

thread_local! {
    static REGISTRY: RefCell<HashMap<Key, Entry>> = RefCell::new(HashMap::new());
}

fn is(weak: &Weak<SharedChannel>, channel: &SharedChannel) -> bool {
    std::ptr::eq(weak.as_ptr(), channel)
}

/// The FLIP transform and crossfade of a shared site.
pub(crate) struct SharedChannel {
    node: Weak<MotionNode>,
    key: RefCell<Option<Key>>,
    mode: Cell<SharedMode>,
    joined: Cell<bool>,
    /// The site's own layout rect on screen (no compositor transforms).
    rect: Cell<Option<Rect>>,
    x: MotionValue,
    y: MotionValue,
    scale_x: MotionValue,
    scale_y: MotionValue,
    fade: MotionValue,
}

impl SharedChannel {
    pub(crate) fn new(node: Weak<MotionNode>) -> Rc<Self> {
        Rc::new(Self {
            node,
            key: RefCell::new(None),
            mode: Cell::new(SharedMode::Element),
            joined: Cell::new(false),
            rect: Cell::new(None),
            x: MotionValue::new(0.),
            y: MotionValue::new(0.),
            scale_x: MotionValue::new(1.),
            scale_y: MotionValue::new(1.),
            fade: MotionValue::new(1.),
        })
    }

    /// Sets the id (within `scope`); a changed id leaves the old entry.
    pub(crate) fn bind(&self, scope: u64, id: SharedId, mode: SharedMode) {
        self.mode.set(mode);
        let key = (scope, id);
        if self.key.borrow().as_ref() == Some(&key) {
            return;
        }
        self.leave();
        *self.key.borrow_mut() = Some(key);
        self.joined.set(false);
    }

    fn transition(&self) -> Transition {
        self.node
            .upgrade()
            .map_or_else(Transition::default, |node| node.layout_transition())
    }

    fn refresh(&self) {
        if let Some(node) = self.node.upgrade() {
            node.push();
        }
        incular_animation::schedule_frame();
    }

    /// The transform that shows this site's `own` rect at `shown`.
    fn flip(&self, shown: Rect, own: Rect) -> (Offset, f32, f32) {
        let origin = self
            .node
            .upgrade()
            .map_or(incular_config::Alignment::CENTER, |node| {
                node.transform_origin()
            });
        let ratio = |a: f32, b: f32| if b > 0. { a / b } else { 1. };
        let sx = ratio(shown.size.width, own.size.width);
        let sy = ratio(shown.size.height, own.size.height);
        // The point the scale keeps fixed.
        let pivot = Offset::new(
            own.origin.x + (origin.x + 1.) / 2. * own.size.width,
            own.origin.y + (origin.y + 1.) / 2. * own.size.height,
        );
        let x = shown.origin.x - pivot.x - sx * (own.origin.x - pivot.x);
        let y = shown.origin.y - pivot.y - sy * (own.origin.y - pivot.y);
        (Offset::new(x, y), sx, sy)
    }

    /// Jumps to showing `from` and animates home.
    fn start_from(&self, from: Rect, fade_in: bool) {
        let Some(own) = self.rect.get() else { return };
        let transition = self.transition();
        let (offset, sx, sy) = self.flip(from, own);
        for (value, start, rest) in [
            (&self.x, offset.x, 0.),
            (&self.y, offset.y, 0.),
            (&self.scale_x, sx, 1.),
            (&self.scale_y, sy, 1.),
        ] {
            value.set(start);
            value.animate_to(rest, &transition);
        }
        if fade_in {
            self.fade.set(0.);
            self.fade.animate_to(1., &transition);
        } else {
            self.fade.set(1.);
        }
        self.refresh();
    }

    /// A newer member took over in `Element` mode: this one hides.
    fn hide(&self) {
        self.fade.set(0.);
        self.refresh();
    }

    /// A newer member took over in `Bounds` mode: this one (still mounted,
    /// for example exiting) morphs to `to` and fades out.
    fn morph_out(&self, to: Rect) {
        let Some(own) = self.rect.get() else {
            return self.hide();
        };
        let transition = self.transition();
        let (offset, sx, sy) = self.flip(to, own);
        self.x.animate_to(offset.x, &transition);
        self.y.animate_to(offset.y, &transition);
        self.scale_x.animate_to(sx, &transition);
        self.scale_y.animate_to(sy, &transition);
        self.fade.animate_to(0., &transition);
        self.refresh();
    }

    /// First geometry: join the entry and take the element over.
    fn join(self: &Rc<Self>, visual: Rect) {
        let Some(key) = self.key.borrow().clone() else {
            return;
        };
        self.joined.set(true);
        let mode = self.mode.get();
        let now = Instant::now();
        let (previous, from) = REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            let entry = registry.entry(key).or_default();
            entry.members.retain(|member| member.strong_count() > 0);
            let previous = entry
                .holder
                .as_ref()
                .and_then(Weak::upgrade)
                .filter(|holder| !Rc::ptr_eq(holder, self));
            let from = if previous.is_some() {
                entry.last
            } else {
                entry
                    .left_at
                    .filter(|left| now.saturating_duration_since(*left) <= HANDOFF_WINDOW)
                    .and(entry.last)
            };
            entry.holder = Some(Rc::downgrade(self));
            entry.members.push(Rc::downgrade(self));
            entry.left_at = None;
            entry.last = Some(visual);
            (previous, from)
        });
        if let Some(previous) = previous {
            match mode {
                SharedMode::Element => previous.hide(),
                SharedMode::Bounds => {
                    if let Some(own) = self.rect.get() {
                        previous.morph_out(own);
                    }
                }
            }
        }
        if let Some(from) = from {
            self.start_from(from, mode == SharedMode::Bounds);
        }
    }

    /// Leaves the current entry, handing the element to the newest member.
    fn leave(&self) {
        let Some(key) = self.key.borrow_mut().take() else {
            return;
        };
        if !self.joined.replace(false) {
            return;
        }
        let next = REGISTRY.try_with(|registry| {
            let Ok(mut registry) = registry.try_borrow_mut() else {
                return None;
            };
            let entry = registry.get_mut(&key)?;
            entry
                .members
                .retain(|member| member.strong_count() > 0 && !is(member, self));
            let holding = entry.holder.as_ref().is_some_and(|holder| is(holder, self));
            if !holding {
                return None;
            }
            let next = entry.members.last().and_then(Weak::upgrade);
            match &next {
                Some(next) => entry.holder = Some(Rc::downgrade(next)),
                None => {
                    entry.holder = None;
                    entry.left_at = Some(Instant::now());
                }
            }
            if entry.members.is_empty() && entry.left_at.is_none() {
                registry.remove(&key);
                return None;
            }
            next.map(|next| (next, entry.last))
        });
        if let Ok(Some((next, Some(from)))) = next {
            if let Some(visual) = next.node.upgrade().and_then(|node| node.geometry.get()) {
                REGISTRY.with(|registry| {
                    if let Some(entry) = registry.borrow_mut().get_mut(&key) {
                        entry.last = Some(visual.visual_rect);
                    }
                });
            }
            next.start_from(from, next.mode.get() == SharedMode::Bounds);
        }
    }
}

impl Drop for SharedChannel {
    fn drop(&mut self) {
        self.leave();
    }
}

impl ChannelSource for SharedChannel {
    fn tick(&self, now: Instant) -> bool {
        let mut changed = self.x.tick(now);
        changed |= self.y.tick(now);
        changed |= self.scale_x.tick(now);
        changed |= self.scale_y.tick(now);
        changed |= self.fade.tick(now);
        changed
    }

    fn is_active(&self) -> bool {
        self.x.is_animating()
            || self.y.is_animating()
            || self.scale_x.is_animating()
            || self.scale_y.is_animating()
            || self.fade.is_animating()
    }

    fn apply(&self, values: &mut [f32; PROP_COUNT]) {
        values[Prop::X.index()] += self.x.get();
        values[Prop::Y.index()] += self.y.get();
        values[Prop::ScaleX.index()] *= self.scale_x.get();
        values[Prop::ScaleY.index()] *= self.scale_y.get();
        values[Prop::Opacity.index()] *= self.fade.get().clamp(0., 1.);
    }

    fn stages(&self, out: &mut [bool; 6]) {
        out[Stage::Transform as usize] = true;
        out[Stage::Opacity as usize] = true;
    }
}

/// Geometry reaches the channel through this listener (the channel itself
/// needs `Rc<Self>` to join).
pub(crate) struct SharedListener(pub(crate) Rc<SharedChannel>);

impl GeometryListener for SharedListener {
    fn geometry(&self, _node: &MotionNode, _previous: Option<FrameGeometry>, now: FrameGeometry) {
        let channel = &self.0;
        channel.rect.set(Some(now.layout_rect));
        if !channel.joined.get() {
            channel.join(now.visual_rect);
        } else if let Some(key) = channel.key.borrow().as_ref() {
            REGISTRY.with(|registry| {
                if let Some(entry) = registry.borrow_mut().get_mut(key)
                    && entry
                        .holder
                        .as_ref()
                        .is_some_and(|holder| is(holder, channel))
                {
                    entry.last = Some(now.visual_rect);
                }
            });
        }
    }
}
