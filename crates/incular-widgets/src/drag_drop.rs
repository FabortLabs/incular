//! Typed, retained-local drag/drop bindings.
//!
//! A `DragDropContext` is generic over the payload and shared explicitly by
//! its [`Draggable`] sources and [`DragTarget`]s. There is no process-global
//! payload registry or untyped public `Any`: a target can only observe a
//! payload from the same typed context. A context exposes an optional
//! [`DragFeedback`] snapshot so an application can present it through its
//! existing `Overlay` while this crate remains navigation-independent.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Axis;
use incular_core::Offset;

use crate::Widget;
use crate::gestures::Velocity;

#[derive(Clone)]
pub struct DragFeedback {
    pub position: Offset,
    pub child: Widget,
}

struct DragState<T> {
    payload: Option<T>,
    position: Offset,
    feedback: Option<Widget>,
}

/// Typed local state shared by related drag sources and targets.
#[derive(Clone)]
pub struct DragDropContext<T> {
    state: Rc<RefCell<DragState<T>>>,
}
impl<T> Default for DragDropContext<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> DragDropContext<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(DragState {
                payload: None,
                position: Offset::ZERO,
                feedback: None,
            })),
        }
    }
    /// Returns an overlay-ready feedback snapshot during an active local drag.
    #[must_use]
    pub fn feedback(&self) -> Option<DragFeedback> {
        let state = self.state.borrow();
        state.feedback.clone().map(|child| DragFeedback {
            position: state.position,
            child,
        })
    }
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.state.borrow().payload.is_some()
    }
    fn id(&self) -> usize {
        Rc::as_ptr(&self.state) as usize
    }
}

/// Where a drag currently is, as the retained tree needs it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragPosition {
    /// The point hit-tested for targets: the axis-restricted pointer plus
    /// `feedback_offset` (Flutter `globalPosition + feedbackOffset`).
    pub hit_point: Offset,
    /// The feedback's top-left (Flutter `_lastOffset`).
    pub feedback_origin: Offset,
}

pub trait RetainedDragSource {
    fn context_id(&self) -> usize;
    /// Flutter `Draggable.affinity`: the axis the gesture must move along to
    /// start a drag, `None` for any direction.
    fn affinity(&self) -> Option<Axis> {
        None
    }
    /// False once `maxSimultaneousDrags` drags are active.
    fn can_start(&self) -> bool {
        true
    }
    /// `position` is the pointer, `origin` the draggable's global top-left.
    fn start(&self, position: Offset, origin: Offset);
    fn update(&self, position: Offset) -> DragPosition;
    /// The drag ended; `accepted` when a target took the payload.
    fn finish(&self, accepted: bool, velocity: Velocity);
    fn cancel(&self);
}
pub trait RetainedDragTarget {
    fn context_id(&self) -> usize;
    fn enter(&self) -> bool;
    /// `offset` is the feedback's top-left.
    fn update(&self, offset: Offset);
    fn leave(&self);
    fn drop_payload(&self, offset: Offset);
}

/// Flutter `DragAnchorStrategy`: where the feedback sits relative to the
/// pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DragAnchorStrategy {
    /// `childDragAnchorStrategy` (the default): the feedback starts where the
    /// child is and keeps the pointer's offset within it.
    #[default]
    Child,
    /// `pointerDragAnchorStrategy`: the feedback's top-left follows the
    /// pointer.
    Pointer,
}

/// Flutter `DraggableDetails`, reported when a drag ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DraggableDetails {
    /// Whether a [`DragTarget`] accepted the payload.
    pub was_accepted: bool,
    /// The release velocity, restricted to the draggable's axis.
    pub velocity: Velocity,
    /// The feedback's top-left when the drag ended.
    pub offset: Offset,
}

/// Typed retained drag source. Its payload stays within its supplied
/// `DragDropContext`, and may be previewed by an application overlay through
/// `DragDropContext::feedback`.
///
/// The drag starts once the pointer moves past the touch slop in any
/// direction (Flutter `ImmediateMultiDragGestureRecognizer`), or along
/// `affinity` when set.
#[derive(Clone)]
pub struct Draggable<T: Clone + 'static> {
    context: DragDropContext<T>,
    payload: T,
    child: Widget,
    child_when_dragging: Option<Widget>,
    feedback: Option<Rc<dyn Fn(T) -> Widget>>,
    axis: Option<Axis>,
    affinity: Option<Axis>,
    feedback_offset: Offset,
    drag_anchor_strategy: DragAnchorStrategy,
    max_simultaneous_drags: Option<usize>,
    on_start: Option<Rc<dyn Fn(T)>>,
    on_end: Option<Rc<dyn Fn(T)>>,
    on_cancel: Option<Rc<dyn Fn(T)>>,
    on_drag_update: Option<Rc<dyn Fn(Offset)>>,
    on_drag_end: Option<Rc<dyn Fn(DraggableDetails)>>,
    on_drag_completed: Option<Rc<dyn Fn()>>,
    on_draggable_canceled: Option<Rc<dyn Fn(Velocity, Offset)>>,
}
impl<T: Clone + 'static> Draggable<T> {
    #[must_use]
    pub fn new(context: DragDropContext<T>, payload: T, child: impl Into<Widget>) -> Self {
        Self {
            context,
            payload,
            child: child.into(),
            child_when_dragging: None,
            feedback: None,
            axis: None,
            affinity: None,
            feedback_offset: Offset::ZERO,
            drag_anchor_strategy: DragAnchorStrategy::Child,
            max_simultaneous_drags: None,
            on_start: None,
            on_end: None,
            on_cancel: None,
            on_drag_update: None,
            on_drag_end: None,
            on_drag_completed: None,
            on_draggable_canceled: None,
        }
    }
    #[must_use]
    pub fn feedback(mut self, builder: impl Fn(T) -> Widget + 'static) -> Self {
        self.feedback = Some(Rc::new(builder));
        self
    }
    /// Shown in place of the child while a drag is active.
    #[must_use]
    pub fn child_when_dragging(mut self, child: impl Into<Widget>) -> Self {
        self.child_when_dragging = Some(child.into());
        self
    }
    /// Restricts the feedback's movement to one axis (Flutter `axis`).
    #[must_use]
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = Some(axis);
        self
    }
    /// Starts the drag only for movement along this axis (Flutter
    /// `affinity`), leaving the other axis to enclosing scrollables.
    #[must_use]
    pub fn affinity(mut self, axis: Axis) -> Self {
        self.affinity = Some(axis);
        self
    }
    /// Offsets the point hit-tested for targets from the pointer.
    #[must_use]
    pub fn feedback_offset(mut self, offset: Offset) -> Self {
        self.feedback_offset = offset;
        self
    }
    #[must_use]
    pub fn drag_anchor_strategy(mut self, strategy: DragAnchorStrategy) -> Self {
        self.drag_anchor_strategy = strategy;
        self
    }
    /// Limits concurrent drags; `0` disables dragging. Unlimited by default.
    #[must_use]
    pub fn max_simultaneous_drags(mut self, max: usize) -> Self {
        self.max_simultaneous_drags = Some(max);
        self
    }
    /// Flutter `onDragStarted`, with the payload.
    #[must_use]
    pub fn on_start(mut self, callback: impl Fn(T) + 'static) -> Self {
        self.on_start = Some(Rc::new(callback));
        self
    }
    /// Called when the pointer is released, whether or not a target
    /// accepted the payload; not called when the pointer is cancelled.
    #[must_use]
    pub fn on_end(mut self, callback: impl Fn(T) + 'static) -> Self {
        self.on_end = Some(Rc::new(callback));
        self
    }
    /// Called when the platform cancels the pointer during a drag.
    #[must_use]
    pub fn on_cancel(mut self, callback: impl Fn(T) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(callback));
        self
    }
    /// Flutter `onDragUpdate`: the restricted movement since the last update.
    #[must_use]
    pub fn on_drag_update(mut self, callback: impl Fn(Offset) + 'static) -> Self {
        self.on_drag_update = Some(Rc::new(callback));
        self
    }
    /// Flutter `onDragEnd`: every finished drag, including cancelled ones.
    #[must_use]
    pub fn on_drag_end(mut self, callback: impl Fn(DraggableDetails) + 'static) -> Self {
        self.on_drag_end = Some(Rc::new(callback));
        self
    }
    /// Flutter `onDragCompleted`: a target accepted the payload.
    #[must_use]
    pub fn on_drag_completed(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_drag_completed = Some(Rc::new(callback));
        self
    }
    /// Flutter `onDraggableCanceled`: the drag ended without a target
    /// accepting it, with the release velocity and feedback top-left.
    #[must_use]
    pub fn on_draggable_canceled(mut self, callback: impl Fn(Velocity, Offset) + 'static) -> Self {
        self.on_draggable_canceled = Some(Rc::new(callback));
        self
    }
}
struct DragSourceBinding<T: Clone + 'static> {
    context: DragDropContext<T>,
    payload: T,
    feedback: Option<Rc<dyn Fn(T) -> Widget>>,
    axis: Option<Axis>,
    affinity: Option<Axis>,
    feedback_offset: Offset,
    drag_anchor_strategy: DragAnchorStrategy,
    max_simultaneous_drags: Option<usize>,
    on_start: Option<Rc<dyn Fn(T)>>,
    on_end: Option<Rc<dyn Fn(T)>>,
    on_cancel: Option<Rc<dyn Fn(T)>>,
    on_drag_update: Option<Rc<dyn Fn(Offset)>>,
    on_drag_end: Option<Rc<dyn Fn(DraggableDetails)>>,
    on_drag_completed: Option<Rc<dyn Fn()>>,
    on_draggable_canceled: Option<Rc<dyn Fn(Velocity, Offset)>>,
    /// `_activeCount`.
    active: Rc<Cell<usize>>,
    /// Rebuilds the child for `childWhenDragging`.
    revision: Rc<Cell<u64>>,
    /// `dragStartPoint`, the axis-restricted position and the last pointer.
    anchor: Cell<Offset>,
    position: Cell<Offset>,
    pointer: Cell<Offset>,
}
impl<T: Clone + 'static> DragSourceBinding<T> {
    /// Flutter `_restrictAxis`.
    fn restrict(&self, offset: Offset) -> Offset {
        match self.axis {
            None => offset,
            Some(Axis::Horizontal) => Offset::new(offset.x, 0.),
            Some(Axis::Vertical) => Offset::new(0., offset.y),
        }
    }
    fn set_active(&self, active: usize) {
        self.active.set(active);
        self.revision.set(self.revision.get().wrapping_add(1));
    }
    fn end(&self, accepted: bool, velocity: Velocity) {
        let offset = self.position.get() - self.anchor.get();
        self.set_active(self.active.get().saturating_sub(1));
        {
            let mut state = self.context.state.borrow_mut();
            state.payload = None;
            state.feedback = None;
        }
        if let Some(callback) = &self.on_drag_end {
            callback(DraggableDetails {
                was_accepted: accepted,
                velocity,
                offset,
            });
        }
        if accepted {
            if let Some(callback) = &self.on_drag_completed {
                callback();
            }
        } else if let Some(callback) = &self.on_draggable_canceled {
            callback(velocity, offset);
        }
    }
}
impl<T: Clone + 'static> RetainedDragSource for DragSourceBinding<T> {
    fn context_id(&self) -> usize {
        self.context.id()
    }
    fn affinity(&self) -> Option<Axis> {
        self.affinity
    }
    fn can_start(&self) -> bool {
        self.max_simultaneous_drags
            .is_none_or(|max| self.active.get() < max)
    }
    fn start(&self, position: Offset, origin: Offset) {
        // Flutter `childDragAnchorStrategy` is the pointer's position within
        // the child; `pointerDragAnchorStrategy` is zero.
        self.anchor.set(match self.drag_anchor_strategy {
            DragAnchorStrategy::Child => position - origin,
            DragAnchorStrategy::Pointer => Offset::ZERO,
        });
        self.position.set(position);
        self.pointer.set(position);
        self.set_active(self.active.get() + 1);
        let feedback = self
            .feedback
            .as_ref()
            .map(|builder| builder(self.payload.clone()));
        let mut state = self.context.state.borrow_mut();
        state.payload = Some(self.payload.clone());
        state.position = position - self.anchor.get();
        state.feedback = feedback;
        drop(state);
        if let Some(callback) = &self.on_start {
            callback(self.payload.clone());
        }
    }
    fn update(&self, pointer: Offset) -> DragPosition {
        let delta = self.restrict(pointer - self.pointer.get());
        self.pointer.set(pointer);
        let old = self.position.get();
        let position = old + delta;
        self.position.set(position);
        let feedback_origin = position - self.anchor.get();
        self.context.state.borrow_mut().position = feedback_origin;
        if position != old
            && let Some(callback) = &self.on_drag_update
        {
            callback(delta);
        }
        DragPosition {
            hit_point: position + self.feedback_offset,
            feedback_origin,
        }
    }
    fn finish(&self, accepted: bool, velocity: Velocity) {
        let payload = self.payload.clone();
        let velocity = Velocity::new(self.restrict(velocity.pixels_per_second));
        self.end(accepted, velocity);
        if let Some(callback) = &self.on_end {
            callback(payload);
        }
    }
    fn cancel(&self) {
        let payload = self.payload.clone();
        self.end(false, Velocity::ZERO);
        if let Some(callback) = &self.on_cancel {
            callback(payload);
        }
    }
}
impl<T: Clone + 'static> From<Draggable<T>> for Widget {
    fn from(value: Draggable<T>) -> Self {
        let active = Rc::new(Cell::new(0));
        let revision = Rc::new(Cell::new(0_u64));
        let child = match value.child_when_dragging {
            None => value.child,
            Some(when_dragging) => {
                let active = active.clone();
                let child = value.child;
                Widget::stateful_builder(revision.clone(), move |_| {
                    if active.get() == 0 {
                        child.clone()
                    } else {
                        when_dragging.clone()
                    }
                })
            }
        };
        Widget::draggable(
            Rc::new(DragSourceBinding {
                context: value.context,
                payload: value.payload,
                feedback: value.feedback,
                axis: value.axis,
                affinity: value.affinity,
                feedback_offset: value.feedback_offset,
                drag_anchor_strategy: value.drag_anchor_strategy,
                max_simultaneous_drags: value.max_simultaneous_drags,
                on_start: value.on_start,
                on_end: value.on_end,
                on_cancel: value.on_cancel,
                on_drag_update: value.on_drag_update,
                on_drag_end: value.on_drag_end,
                on_drag_completed: value.on_drag_completed,
                on_draggable_canceled: value.on_draggable_canceled,
                active,
                revision,
                anchor: Cell::new(Offset::ZERO),
                position: Cell::new(Offset::ZERO),
                pointer: Cell::new(Offset::ZERO),
            }),
            child,
        )
    }
}

/// Typed drop target for the same local `DragDropContext` as its sources.
#[derive(Clone)]
pub struct DragTarget<T: Clone + 'static> {
    context: DragDropContext<T>,
    child: Widget,
    on_will_accept: Option<Rc<dyn Fn(T) -> bool>>,
    on_enter: Option<Rc<dyn Fn(T)>>,
    on_leave: Option<Rc<dyn Fn(T)>>,
    on_update: Option<Rc<dyn Fn(T, Offset)>>,
    on_drop: Option<Rc<dyn Fn(T)>>,
    on_accept_with_details: Option<Rc<dyn Fn(T, Offset)>>,
}
impl<T: Clone + 'static> DragTarget<T> {
    #[must_use]
    pub fn new(context: DragDropContext<T>, child: impl Into<Widget>) -> Self {
        Self {
            context,
            child: child.into(),
            on_will_accept: None,
            on_enter: None,
            on_leave: None,
            on_update: None,
            on_drop: None,
            on_accept_with_details: None,
        }
    }
    #[must_use]
    pub fn on_will_accept(mut self, callback: impl Fn(T) -> bool + 'static) -> Self {
        self.on_will_accept = Some(Rc::new(callback));
        self
    }
    #[must_use]
    pub fn on_enter(mut self, callback: impl Fn(T) + 'static) -> Self {
        self.on_enter = Some(Rc::new(callback));
        self
    }
    #[must_use]
    pub fn on_leave(mut self, callback: impl Fn(T) + 'static) -> Self {
        self.on_leave = Some(Rc::new(callback));
        self
    }
    /// Flutter `onMove`: the payload and the feedback's top-left.
    #[must_use]
    pub fn on_update(mut self, callback: impl Fn(T, Offset) + 'static) -> Self {
        self.on_update = Some(Rc::new(callback));
        self
    }
    #[must_use]
    pub fn on_drop(mut self, callback: impl Fn(T) + 'static) -> Self {
        self.on_drop = Some(Rc::new(callback));
        self
    }
    /// Flutter `onAcceptWithDetails`: the payload and the feedback's
    /// top-left where it was dropped.
    #[must_use]
    pub fn on_accept_with_details(mut self, callback: impl Fn(T, Offset) + 'static) -> Self {
        self.on_accept_with_details = Some(Rc::new(callback));
        self
    }
}
struct DragTargetBinding<T: Clone + 'static> {
    context: DragDropContext<T>,
    on_will_accept: Option<Rc<dyn Fn(T) -> bool>>,
    on_enter: Option<Rc<dyn Fn(T)>>,
    on_leave: Option<Rc<dyn Fn(T)>>,
    on_update: Option<Rc<dyn Fn(T, Offset)>>,
    on_drop: Option<Rc<dyn Fn(T)>>,
    on_accept_with_details: Option<Rc<dyn Fn(T, Offset)>>,
}
impl<T: Clone + 'static> DragTargetBinding<T> {
    fn payload(&self) -> Option<T> {
        self.context.state.borrow().payload.clone()
    }
}
impl<T: Clone + 'static> RetainedDragTarget for DragTargetBinding<T> {
    fn context_id(&self) -> usize {
        self.context.id()
    }
    fn enter(&self) -> bool {
        let Some(payload) = self.payload() else {
            return false;
        };
        if !self
            .on_will_accept
            .as_ref()
            .is_none_or(|callback| callback(payload.clone()))
        {
            return false;
        }
        if let Some(callback) = &self.on_enter {
            callback(payload);
        }
        true
    }
    fn update(&self, offset: Offset) {
        if let (Some(payload), Some(callback)) = (self.payload(), &self.on_update) {
            callback(payload, offset);
        }
    }
    fn leave(&self) {
        if let (Some(payload), Some(callback)) = (self.payload(), &self.on_leave) {
            callback(payload);
        }
    }
    fn drop_payload(&self, offset: Offset) {
        let Some(payload) = self.payload() else {
            return;
        };
        if let Some(callback) = &self.on_drop {
            callback(payload.clone());
        }
        if let Some(callback) = &self.on_accept_with_details {
            callback(payload, offset);
        }
    }
}
impl<T: Clone + 'static> From<DragTarget<T>> for Widget {
    fn from(value: DragTarget<T>) -> Self {
        Widget::drag_target(
            Rc::new(DragTargetBinding {
                context: value.context,
                on_will_accept: value.on_will_accept,
                on_enter: value.on_enter,
                on_leave: value.on_leave,
                on_update: value.on_update,
                on_drop: value.on_drop,
                on_accept_with_details: value.on_accept_with_details,
            }),
            value.child,
        )
    }
}

/// A draggable widget that initiates drag operations exclusively on long-press.
#[derive(Clone)]
pub struct LongPressDraggable<T: Clone + 'static> {
    context: DragDropContext<T>,
    payload: T,
    child: Widget,
}

impl<T: Clone + 'static> LongPressDraggable<T> {
    #[must_use]
    pub fn new(context: DragDropContext<T>, payload: T, child: impl Into<Widget>) -> Self {
        Self {
            context,
            payload,
            child: child.into(),
        }
    }
}

impl<T: Clone + 'static> From<LongPressDraggable<T>> for Widget {
    fn from(value: LongPressDraggable<T>) -> Self {
        Draggable::new(value.context, value.payload, value.child).into()
    }
}
