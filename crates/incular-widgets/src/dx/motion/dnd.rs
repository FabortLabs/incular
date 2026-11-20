//! Typed drag and drop (§7.2): the dragged element itself lifts on the
//! compositor (no feedback widget), and drop targets style themselves with
//! ordinary conditions.
//!
//! ```ignore
//! let dnd = DragDrop::<FileId>::new();
//! file_row(f).draggable(&dnd, f.id)
//!     .motion(|m| m.dragging(|p| p.scale(1.03).lift(12.)));
//! folder(dir).drop_target(&dnd, |t| t.accepts(|id| ok(*id)).on_drop(|id| move_to(id)))
//!     .motion(|m| m.when(dnd.active(), |p| p.brightness(1.03)).when(dnd.over(), |p| p.scale(1.04)));
//! ```

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use incular_core::{Offset, reactivity::DependencySource};

use super::{
    Motion,
    behavior::SiteBehavior,
    cond::{Cond, Interaction, Interactions},
    node::{MotionNode, motion_widget},
};
use crate::dx::drag::Drag;
use crate::{BuildContext, Widget};

type Accepts<T> = Rc<dyn Fn(&T) -> bool>;
type OnDrop<T> = Rc<dyn Fn(T)>;
type OnHover<T> = Rc<dyn Fn(&T)>;

struct Target<T> {
    node: Weak<MotionNode>,
    accepts: RefCell<Option<Accepts<T>>>,
    on_drop: RefCell<Option<OnDrop<T>>>,
    on_enter: RefCell<Option<OnHover<T>>>,
    on_leave: RefCell<Option<OnHover<T>>>,
    over: Cell<bool>,
}

struct Shared<T> {
    active: RefCell<Option<T>>,
    pointer: Cell<Offset>,
    targets: RefCell<Rc<[Rc<Target<T>>]>>,
    changes: DependencySource,
}

/// A typed drag-and-drop context shared by draggables and drop targets.
pub struct DragDrop<T: Clone + 'static>(Rc<Shared<T>>);

impl<T: Clone + 'static> Clone for DragDrop<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Clone + 'static> Default for DragDrop<T> {
    fn default() -> Self {
        Self(Rc::new(Shared {
            active: RefCell::new(None),
            pointer: Cell::new(Offset::ZERO),
            targets: RefCell::new(Rc::from(Vec::new())),
            changes: DependencySource::default(),
        }))
    }
}

impl<T: Clone + 'static> DragDrop<T> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// On a drop target: a drag is running whose payload it accepts.
    #[must_use]
    pub fn active(&self) -> Cond {
        Cond::State(Interaction::DropActive)
    }

    /// On a drop target: an accepted payload is over it.
    #[must_use]
    pub fn over(&self) -> Cond {
        Cond::State(Interaction::DropOver)
    }

    /// Anywhere: any drag of this context is running.
    #[must_use]
    pub fn dragging(&self) -> Cond {
        let shared = self.0.clone();
        Cond::live(self.0.changes.clone(), move || {
            shared.active.borrow().is_some()
        })
    }

    /// The payload being dragged.
    #[must_use]
    pub fn payload(&self) -> Option<T> {
        self.0.active.borrow().clone()
    }

    fn refresh(&self) {
        let shared = &self.0;
        if shared
            .targets
            .borrow()
            .iter()
            .any(|target| target.node.strong_count() == 0)
        {
            let live: Rc<[_]> = shared
                .targets
                .borrow()
                .iter()
                .filter(|target| target.node.strong_count() > 0)
                .cloned()
                .collect();
            *shared.targets.borrow_mut() = live;
        }
        let targets = Rc::clone(&shared.targets.borrow());
        let payload = shared.active.borrow().clone();
        let pointer = shared.pointer.get();
        // The smallest accepting target under the pointer is "over".
        let mut best: Option<(usize, f32)> = None;
        for (index, target) in targets.iter().enumerate() {
            let Some(node) = target.node.upgrade() else {
                continue;
            };
            let accepts = payload.as_ref().is_some_and(|payload| {
                target
                    .accepts
                    .borrow()
                    .as_ref()
                    .is_none_or(|accepts| accepts(payload))
            });
            node.set_flag(Interaction::DropActive, accepts);
            if accepts
                && let Some(geometry) = node.geometry.get()
                && geometry.visual_rect.contains(pointer)
            {
                let area = geometry.visual_rect.size.width * geometry.visual_rect.size.height;
                if best.is_none_or(|(_, smallest)| area < smallest) {
                    best = Some((index, area));
                }
            }
        }
        for (index, target) in targets.iter().enumerate() {
            let over = best.is_some_and(|(best, _)| best == index);
            let was = target.over.replace(over);
            if let Some(payload) = payload.as_ref() {
                if over && !was {
                    if let Some(enter) = target.on_enter.borrow().clone() {
                        enter(payload);
                    }
                } else if !over
                    && was
                    && let Some(leave) = target.on_leave.borrow().clone()
                {
                    leave(payload);
                }
            }
            if let Some(node) = target.node.upgrade() {
                node.set_flag(Interaction::DropOver, over);
            }
        }
        shared.changes.notify();
    }

    fn drop_over(&self) {
        let payload = self.0.active.borrow_mut().take();
        let targets = Rc::clone(&self.0.targets.borrow());
        if let Some(payload) = payload
            && let Some(target) = targets.iter().find(|target| target.over.get())
            && let Some(on_drop) = target.on_drop.borrow().clone()
        {
            on_drop(payload);
        }
        for target in targets.iter() {
            target.over.set(false);
        }
    }
}

/// What a drop target accepts and does.
pub struct DropRules<T> {
    accepts: Option<Accepts<T>>,
    on_drop: Option<OnDrop<T>>,
    on_enter: Option<OnHover<T>>,
    on_leave: Option<OnHover<T>>,
}

impl<T> Default for DropRules<T> {
    fn default() -> Self {
        Self {
            accepts: None,
            on_drop: None,
            on_enter: None,
            on_leave: None,
        }
    }
}

impl<T> DropRules<T> {
    /// Which payloads this target takes (all by default).
    #[must_use]
    pub fn accepts(mut self, accepts: impl Fn(&T) -> bool + 'static) -> Self {
        self.accepts = Some(Rc::new(accepts));
        self
    }

    /// Called once with the payload dropped here.
    #[must_use]
    pub fn on_drop(mut self, on_drop: impl Fn(T) + 'static) -> Self {
        self.on_drop = Some(Rc::new(on_drop));
        self
    }

    #[must_use]
    pub fn on_enter(mut self, on_enter: impl Fn(&T) + 'static) -> Self {
        self.on_enter = Some(Rc::new(on_enter));
        self
    }

    #[must_use]
    pub fn on_leave(mut self, on_leave: impl Fn(&T) + 'static) -> Self {
        self.on_leave = Some(Rc::new(on_leave));
        self
    }
}

struct Source<T: Clone + 'static> {
    dnd: DragDrop<T>,
    payload: T,
}

impl<T: Clone + 'static> SiteBehavior for Source<T> {
    fn drag_start(&self, _node: &Rc<MotionNode>, global: Offset) {
        *self.dnd.0.active.borrow_mut() = Some(self.payload.clone());
        self.dnd.0.pointer.set(global);
        self.dnd.refresh();
    }

    fn drag_update(&self, _node: &Rc<MotionNode>, global: Offset) {
        self.dnd.0.pointer.set(global);
        self.dnd.refresh();
    }

    fn drag_end(
        &self,
        _node: &Rc<MotionNode>,
        global: Offset,
        _velocity: Offset,
        cancelled: bool,
    ) -> bool {
        self.dnd.0.pointer.set(global);
        if cancelled {
            self.dnd.0.active.borrow_mut().take();
        } else {
            self.dnd.refresh();
            self.dnd.drop_over();
        }
        self.dnd.refresh();
        false
    }
}

struct TargetSlot<T>(Rc<Target<T>>);

struct Sink<T: Clone + 'static> {
    dnd: DragDrop<T>,
    rules: Rc<DropRules<T>>,
}

impl<T: Clone + 'static> SiteBehavior for Sink<T> {
    fn bind(&self, node: &Rc<MotionNode>, _context: &BuildContext<'_>) {
        let slot = node.behavior_state(|| {
            let target = Rc::new(Target {
                node: Rc::downgrade(node),
                accepts: RefCell::new(None),
                on_drop: RefCell::new(None),
                on_enter: RefCell::new(None),
                on_leave: RefCell::new(None),
                over: Cell::new(false),
            });
            let targets: Rc<[_]> = self
                .dnd
                .0
                .targets
                .borrow()
                .iter()
                .cloned()
                .chain(std::iter::once(target.clone()))
                .collect();
            *self.dnd.0.targets.borrow_mut() = targets;
            TargetSlot(target)
        });
        let target = &slot.0;
        target.accepts.borrow_mut().clone_from(&self.rules.accepts);
        target.on_drop.borrow_mut().clone_from(&self.rules.on_drop);
        target
            .on_enter
            .borrow_mut()
            .clone_from(&self.rules.on_enter);
        target
            .on_leave
            .borrow_mut()
            .clone_from(&self.rules.on_leave);
    }

    fn interactions(&self) -> Interactions {
        Interactions::NONE
            .with(Interaction::DropActive, true)
            .with(Interaction::DropOver, true)
    }

    fn wants_geometry(&self) -> bool {
        true
    }
}

/// A draggable element; `.motion(..)` styles it as the same site.
pub struct DragSite<T: Clone + 'static> {
    child: Widget,
    dnd: DragDrop<T>,
    payload: T,
    motion: Motion,
}

impl<T: Clone + 'static> DragSite<T> {
    /// Styles the draggable (for example `dragging(..)`) on the same site.
    #[must_use]
    pub fn motion(mut self, build: impl FnOnce(Motion) -> Motion) -> Widget {
        self.motion = build(self.motion);
        self.into()
    }
}

impl<T: Clone + 'static> From<DragSite<T>> for Widget {
    fn from(site: DragSite<T>) -> Self {
        let mut motion = site.motion;
        if motion.drag.is_none() {
            motion = motion.drag(Drag::both());
        }
        motion.behaviors.push(Rc::new(Source {
            dnd: site.dnd,
            payload: site.payload,
        }));
        motion_widget(site.child, motion)
    }
}

/// A drop target; `.motion(..)` styles it as the same site.
pub struct DropSite<T: Clone + 'static> {
    child: Widget,
    dnd: DragDrop<T>,
    rules: DropRules<T>,
    motion: Motion,
}

impl<T: Clone + 'static> DropSite<T> {
    /// Styles the target (for example with `dnd.active()` / `dnd.over()`).
    #[must_use]
    pub fn motion(mut self, build: impl FnOnce(Motion) -> Motion) -> Widget {
        self.motion = build(self.motion);
        self.into()
    }
}

impl<T: Clone + 'static> From<DropSite<T>> for Widget {
    fn from(site: DropSite<T>) -> Self {
        let mut motion = site.motion;
        motion.behaviors.push(Rc::new(Sink {
            dnd: site.dnd,
            rules: Rc::new(site.rules),
        }));
        motion_widget(site.child, motion)
    }
}

/// `.draggable()` and `.drop_target()` for anything that becomes a widget.
pub trait DndExt: Into<Widget> + Sized {
    /// Makes this element draggable with `payload`; it lifts itself.
    #[must_use]
    fn draggable<T: Clone + 'static>(self, dnd: &DragDrop<T>, payload: T) -> DragSite<T> {
        DragSite {
            child: self.into(),
            dnd: dnd.clone(),
            payload,
            motion: Motion::new(),
        }
    }

    /// Makes this element a drop target for `dnd`.
    #[must_use]
    fn drop_target<T: Clone + 'static>(
        self,
        dnd: &DragDrop<T>,
        rules: impl FnOnce(DropRules<T>) -> DropRules<T>,
    ) -> DropSite<T> {
        DropSite {
            child: self.into(),
            dnd: dnd.clone(),
            rules: rules(DropRules::default()),
            motion: Motion::new(),
        }
    }
}

impl<T: Into<Widget>> DndExt for T {}
