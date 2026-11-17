//! Layout-agnostic reordering (§7.3): sortable lists, grids, table rows and
//! Kanban boards.
//!
//! Items measure their own rects, so any layout works. While dragging, the
//! lifted item follows the pointer and siblings shift on the compositor:
//! nothing rebuilds. On drop the item springs into its slot and `on_move`
//! runs once; when the app applies the move, every offset is absorbed into
//! the items' layout motion, so nothing jumps.
//!
//! ```ignore
//! Column::new(tasks.iter().map(|t| row(t).reorder_item(t.id)))
//!     .reorderable(move |mv: Move<TaskId>| tasks.update(|v| mv.apply(v, |t| t.id)));
//! ```

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use incular_animation::{MotionValue, Transition};
use incular_core::{Code, KeyboardKey, NamedKey, Offset, Rect, Size, time::Instant};
use incular_scroll::ScrollController;

use super::{
    Motion, Pose,
    behavior::SiteBehavior,
    cond::{Interaction, Interactions},
    dragging,
    node::{ChannelSource, MotionNode, motion_widget},
    pose::{PROP_COUNT, Prop, Stage},
};
use crate::dx::drag::{Drag, DragAxis, Release};
use crate::tree::{FrameDelegate, FrameGeometry, FrameTick};
use crate::{BuildContext, Widget};

// ---------------------------------------------------------------------------
// Public vocabulary

/// One reorder: `key` moved from `from` in `from_list` to `to` in `to_list`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Move<K, L = ()> {
    pub key: K,
    pub from: usize,
    pub to: usize,
    pub from_list: L,
    pub to_list: L,
}

impl<K: PartialEq, L> Move<K, L> {
    /// Applies a move within one list.
    pub fn apply<T>(&self, items: &mut Vec<T>, key: impl Fn(&T) -> K) {
        let Some(index) = items.iter().position(|item| key(item) == self.key) else {
            return;
        };
        let item = items.remove(index);
        items.insert(self.to.min(items.len()), item);
    }

    /// Applies a move between two lists (`from` and `to` may be the same
    /// list only through [`Move::apply`]).
    pub fn apply_between<T>(&self, from: &mut Vec<T>, to: &mut Vec<T>, key: impl Fn(&T) -> K) {
        let Some(index) = from.iter().position(|item| key(item) == self.key) else {
            return;
        };
        let item = from.remove(index);
        to.insert(self.to.min(to.len()), item);
    }
}

/// How the insertion slot is found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Collision {
    /// The item whose center is nearest the dragged item's center.
    #[default]
    ClosestCenter,
    /// The item under the pointer.
    PointerWithin,
}

/// What the dragged item's slot shows while it is lifted.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Placeholder {
    /// An empty gap.
    #[default]
    Gap,
    /// A translucent copy of the item, at this opacity, in its target slot.
    Ghost(f32),
}

#[derive(Clone, Copy)]
struct Options {
    collision: Collision,
    live: bool,
    axis_lock: bool,
    auto_scroll: bool,
    long_press_on_touch: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            collision: Collision::ClosestCenter,
            live: false,
            axis_lock: false,
            auto_scroll: true,
            long_press_on_touch: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Group state

/// The compositor shift a sibling shows while another item is dragged.
struct Shift {
    x: MotionValue,
    y: MotionValue,
    ghost_x: MotionValue,
    ghost_y: MotionValue,
    ghost_alpha: MotionValue,
}

impl Shift {
    fn new() -> Self {
        Self {
            x: MotionValue::new(0.),
            y: MotionValue::new(0.),
            ghost_x: MotionValue::new(0.),
            ghost_y: MotionValue::new(0.),
            ghost_alpha: MotionValue::new(0.),
        }
    }

    fn offset(&self) -> Offset {
        Offset::new(self.x.get(), self.y.get())
    }
}

struct Item<K> {
    focus: incular_gestures::FocusNode,
    ghost_affine: crate::AffineController,
    ghost_opacity: crate::OpacityController,
    key: RefCell<K>,
    list: Cell<usize>,
    node: Weak<MotionNode>,
    shift: Shift,
    label: RefCell<Option<String>>,
    scroll: RefCell<Option<ScrollController>>,
    announcement: RefCell<String>,
}

impl<K> Item<K> {
    /// Writes the ghost values into its compositor controllers.
    fn sync_ghost(&self) {
        self.ghost_affine.set_parts(crate::AffineParts {
            translation: Offset::new(self.shift.ghost_x.get(), self.shift.ghost_y.get()),
            ..crate::AffineParts::default()
        });
        self.ghost_opacity
            .set_opacity(self.shift.ghost_alpha.get().clamp(0., 1.));
    }

    fn rect(&self) -> Option<Rect> {
        self.node
            .upgrade()
            .and_then(|node| node.geometry.get())
            .map(|geometry| geometry.layout_rect)
    }
}

impl<K: 'static> ChannelSource for Item<K> {
    fn tick(&self, now: Instant) -> bool {
        let mut changed = self.shift.x.tick(now);
        changed |= self.shift.y.tick(now);
        changed |= self.shift.ghost_x.tick(now);
        changed |= self.shift.ghost_y.tick(now);
        changed |= self.shift.ghost_alpha.tick(now);
        if changed {
            self.sync_ghost();
        }
        changed
    }

    fn is_active(&self) -> bool {
        self.shift.x.is_animating()
            || self.shift.y.is_animating()
            || self.shift.ghost_x.is_animating()
            || self.shift.ghost_y.is_animating()
            || self.shift.ghost_alpha.is_animating()
    }

    fn apply(&self, values: &mut [f32; PROP_COUNT]) {
        values[Prop::X.index()] += self.shift.x.get();
        values[Prop::Y.index()] += self.shift.y.get();
    }

    fn stages(&self, out: &mut [bool; 6]) {
        out[Stage::Transform as usize] = true;
    }
}

#[derive(Clone)]
struct Session<K> {
    key: K,
    dragged: Rc<Item<K>>,
    from_list: usize,
    from: usize,
    to_list: usize,
    to: usize,
    pointer: Offset,
    keyboard: bool,
}

struct List {
    rect: Cell<Option<Rect>>,
}

/// The type-erased side of a group that items talk to.
trait GroupCore<K> {
    fn shared(&self) -> &Core<K>;
    fn list_index_for(&self, list: &dyn std::any::Any) -> Option<usize>;
    fn emit(&self, key: K, from_list: usize, from: usize, to_list: usize, to: usize);
}

struct Core<K> {
    items: RefCell<Vec<Rc<Item<K>>>>,
    lists: RefCell<Vec<Rc<List>>>,
    session: RefCell<Option<Session<K>>>,
    options: Cell<Options>,
    /// After a committed drop: frames left before leftover offsets the
    /// app's rebuild did not absorb drift home.
    pending_relax: RefCell<Option<(u8, Transition)>>,
}

impl<K> Default for Core<K> {
    fn default() -> Self {
        Self {
            items: RefCell::new(Vec::new()),
            lists: RefCell::new(Vec::new()),
            session: RefCell::new(None),
            options: Cell::new(Options::default()),
            pending_relax: RefCell::new(None),
        }
    }
}

type OnMove<K, L> = Rc<dyn Fn(Move<K, L>)>;

struct Group<K, L> {
    core: Core<K>,
    ids: RefCell<Vec<L>>,
    on_move: RefCell<Option<OnMove<K, L>>>,
}

impl<K: Clone + 'static, L: Clone + PartialEq + 'static> GroupCore<K> for Group<K, L> {
    fn shared(&self) -> &Core<K> {
        &self.core
    }

    fn list_index_for(&self, list: &dyn std::any::Any) -> Option<usize> {
        let list = list.downcast_ref::<L>()?;
        let mut ids = self.ids.borrow_mut();
        if let Some(index) = ids.iter().position(|id| id == list) {
            return Some(index);
        }
        ids.push(list.clone());
        self.core.lists.borrow_mut().push(Rc::new(List {
            rect: Cell::new(None),
        }));
        Some(ids.len() - 1)
    }

    fn emit(&self, key: K, from_list: usize, from: usize, to_list: usize, to: usize) {
        let ids = self.ids.borrow();
        let (Some(from_id), Some(to_id)) = (ids.get(from_list), ids.get(to_list)) else {
            return;
        };
        let mv = Move {
            key,
            from,
            to,
            from_list: from_id.clone(),
            to_list: to_id.clone(),
        };
        drop(ids);
        let on_move = self.on_move.borrow().clone();
        if let Some(on_move) = on_move {
            on_move(mv);
        }
    }
}

/// A reorder coordinator shared by several containers (a Kanban board).
pub struct ReorderGroup<K: Clone + 'static, L: Clone + PartialEq + 'static = ()>(Rc<Group<K, L>>);

impl<K: Clone + 'static, L: Clone + PartialEq + 'static> Clone for ReorderGroup<K, L> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K: Clone + 'static, L: Clone + PartialEq + 'static> Default for ReorderGroup<K, L> {
    fn default() -> Self {
        Self(Rc::new(Group {
            core: Core::default(),
            ids: RefCell::new(Vec::new()),
            on_move: RefCell::new(None),
        }))
    }
}

macro_rules! group_options {
    () => {
        /// How the insertion slot is chosen (closest center by default).
        #[must_use]
        pub fn collision(self, collision: Collision) -> Self {
            self.update(|options| options.collision = collision);
            self
        }

        /// Calls `on_move` on every slot change during the drag, not once on
        /// drop.
        #[must_use]
        pub fn live(self, live: bool) -> Self {
            self.update(|options| options.live = live);
            self
        }

        /// Restricts the drag to the list's axis.
        #[must_use]
        pub fn axis_lock(self) -> Self {
            self.update(|options| options.axis_lock = true);
            self
        }

        /// Scrolls the nearest scrollable near its edges (on by default).
        #[must_use]
        pub fn auto_scroll(self, enabled: bool) -> Self {
            self.update(|options| options.auto_scroll = enabled);
            self
        }

        /// Touch starts a drag with a long press (on by default), leaving
        /// quick swipes to scrolling.
        #[must_use]
        pub fn long_press_on_touch(self, enabled: bool) -> Self {
            self.update(|options| options.long_press_on_touch = enabled);
            self
        }
    };
}

impl<K: Clone + 'static, L: Clone + PartialEq + 'static> ReorderGroup<K, L> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Called with each committed move.
    #[must_use]
    pub fn on_move(self, on_move: impl Fn(Move<K, L>) + 'static) -> Self {
        *self.0.on_move.borrow_mut() = Some(Rc::new(on_move));
        self
    }

    fn update(&self, change: impl FnOnce(&mut Options)) {
        let mut options = self.0.core.options.get();
        change(&mut options);
        self.0.core.options.set(options);
    }

    group_options!();

    fn scope(&self, list: L) -> ListScope<K> {
        let group: Rc<dyn GroupCore<K>> = self.0.clone();
        let index = group.list_index_for(&list).unwrap_or(0);
        ListScope { group, list: index }
    }
}

/// Published by a reorderable container to its items.
pub(crate) struct ListScope<K> {
    group: Rc<dyn GroupCore<K>>,
    list: usize,
}

impl<K> Clone for ListScope<K> {
    fn clone(&self) -> Self {
        Self {
            group: self.group.clone(),
            list: self.list,
        }
    }
}

// ---------------------------------------------------------------------------
// Planning: where every item goes for a session

/// The axis a list's items run along.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    Vertical,
    Horizontal,
    Grid,
}

fn flow_of(rects: &[Rect]) -> Flow {
    if rects.len() < 2 {
        return Flow::Vertical;
    }
    let first = rects[0].origin;
    if rects
        .iter()
        .all(|rect| (rect.origin.x - first.x).abs() < 1.)
    {
        Flow::Vertical
    } else if rects
        .iter()
        .all(|rect| (rect.origin.y - first.y).abs() < 1.)
    {
        Flow::Horizontal
    } else {
        Flow::Grid
    }
}

fn center(rect: Rect) -> Offset {
    Offset::new(
        rect.origin.x + rect.size.width / 2.,
        rect.origin.y + rect.size.height / 2.,
    )
}

/// Reading order: rows top to bottom, left to right within a row.
fn reading_order(a: Rect, b: Rect) -> std::cmp::Ordering {
    let tolerance = a.size.height.min(b.size.height) / 2.;
    if (a.origin.y - b.origin.y).abs() < tolerance {
        a.origin.x.total_cmp(&b.origin.x)
    } else {
        a.origin.y.total_cmp(&b.origin.y)
    }
}

/// Whether `point` lies after `rect`'s center in the list's order.
fn is_past(point: Offset, rect: Rect, flow: Flow) -> bool {
    let middle = center(rect);
    match flow {
        Flow::Vertical => point.y > middle.y,
        Flow::Horizontal => point.x > middle.x,
        Flow::Grid => {
            if (point.y - middle.y).abs() < rect.size.height / 2. {
                point.x > middle.x
            } else {
                point.y > middle.y
            }
        }
    }
}

/// Lays `sizes` out the way `original` (the list's measured rects, in
/// order) is laid out, returning each origin.
fn restack(original: &[Rect], sizes: &[Size], list_rect: Option<Rect>) -> Vec<Offset> {
    let flow = flow_of(original);
    let start = original
        .first()
        .map(|rect| rect.origin)
        .or_else(|| list_rect.map(|rect| rect.origin))
        .unwrap_or(Offset::ZERO);
    let gap = |axis: Flow| -> f32 {
        let gaps: Vec<f32> = original
            .windows(2)
            .map(|pair| match axis {
                Flow::Horizontal => pair[1].origin.x - (pair[0].origin.x + pair[0].size.width),
                _ => pair[1].origin.y - (pair[0].origin.y + pair[0].size.height),
            })
            .collect();
        if gaps.is_empty() {
            0.
        } else {
            gaps.iter().sum::<f32>() / gaps.len() as f32
        }
    };
    match flow {
        Flow::Vertical => {
            let gap = gap(Flow::Vertical);
            let mut y = start.y;
            sizes
                .iter()
                .map(|size| {
                    let origin = Offset::new(start.x, y);
                    y += size.height + gap;
                    origin
                })
                .collect()
        }
        Flow::Horizontal => {
            let gap = gap(Flow::Horizontal);
            let mut x = start.x;
            sizes
                .iter()
                .map(|size| {
                    let origin = Offset::new(x, start.y);
                    x += size.width + gap;
                    origin
                })
                .collect()
        }
        Flow::Grid => {
            let step = match original {
                [.., a, b] => b.origin - a.origin,
                _ => Offset::ZERO,
            };
            (0..sizes.len())
                .map(|index| match original.get(index) {
                    Some(rect) => rect.origin,
                    None => {
                        let last = original.last().map_or(start, |rect| rect.origin);
                        last + Offset::new(
                            step.x * (index + 1 - original.len()) as f32,
                            step.y * (index + 1 - original.len()) as f32,
                        )
                    }
                })
                .collect()
        }
    }
}

impl<K: Clone + PartialEq + 'static> Core<K> {
    fn live_items(&self) -> Vec<Rc<Item<K>>> {
        self.items
            .borrow_mut()
            .retain(|item| item.node.strong_count() > 0);
        self.items.borrow().clone()
    }

    /// The items of `list` with rects, in reading order.
    fn ordered(&self, list: usize) -> Vec<(Rc<Item<K>>, Rect)> {
        let mut items: Vec<(Rc<Item<K>>, Rect)> = self
            .live_items()
            .into_iter()
            .filter(|item| item.list.get() == list)
            .filter_map(|item| item.rect().map(|rect| (item, rect)))
            .collect();
        items.sort_by(|a, b| reading_order(a.1, b.1));
        items
    }

    fn index_of(&self, item: &Rc<Item<K>>) -> Option<(usize, usize)> {
        let list = item.list.get();
        self.ordered(list)
            .iter()
            .position(|(other, _)| Rc::ptr_eq(other, item))
            .map(|index| (list, index))
    }

    /// Each affected item's target origin for the session.
    fn plan(&self, session: &Session<K>) -> Vec<(Rc<Item<K>>, Rect, Offset)> {
        let mut out = Vec::new();
        let dragged_rect = session.dragged.rect().unwrap_or_default();
        let lists = if session.from_list == session.to_list {
            vec![session.to_list]
        } else {
            vec![session.from_list, session.to_list]
        };
        for list in lists {
            let ordered = self.ordered(list);
            let original: Vec<Rect> = ordered.iter().map(|(_, rect)| *rect).collect();
            let mut arranged: Vec<(Rc<Item<K>>, Rect)> = ordered
                .into_iter()
                .filter(|(item, _)| !Rc::ptr_eq(item, &session.dragged))
                .collect();
            if list == session.to_list {
                let index = session.to.min(arranged.len());
                arranged.insert(index, (session.dragged.clone(), dragged_rect));
            }
            let sizes: Vec<Size> = arranged.iter().map(|(_, rect)| rect.size).collect();
            let list_rect = self
                .lists
                .borrow()
                .get(list)
                .and_then(|list| list.rect.get());
            let origins = restack(&original, &sizes, list_rect);
            for ((item, rect), origin) in arranged.into_iter().zip(origins) {
                out.push((item, rect, origin));
            }
        }
        out
    }

    /// The list and index the dragged item would drop at.
    fn locate(&self, session: &Session<K>) -> (usize, usize) {
        let options = self.options.get();
        let Some(node) = session.dragged.node.upgrade() else {
            return (session.to_list, session.to);
        };
        let Some(rect) = session.dragged.rect() else {
            return (session.to_list, session.to);
        };
        let offset = node
            .drag_channel
            .borrow()
            .as_ref()
            .map_or(Offset::ZERO, |channel| channel.offset());
        let probe = match options.collision {
            Collision::ClosestCenter => center(rect) + offset,
            Collision::PointerWithin => session.pointer,
        };
        // The list under the probe, else the current one.
        let list = self
            .lists
            .borrow()
            .iter()
            .position(|list| list.rect.get().is_some_and(|rect| rect.contains(probe)))
            .unwrap_or(session.to_list);
        let others: Vec<(Rc<Item<K>>, Rect)> = self
            .ordered(list)
            .into_iter()
            .filter(|(item, _)| !Rc::ptr_eq(item, &session.dragged))
            .collect();
        if others.is_empty() {
            return (list, 0);
        }
        let flow = flow_of(&others.iter().map(|(_, rect)| *rect).collect::<Vec<_>>());
        let chosen = match options.collision {
            Collision::PointerWithin => others.iter().position(|(_, rect)| rect.contains(probe)),
            Collision::ClosestCenter => None,
        }
        .unwrap_or_else(|| {
            let mut best = (0, f32::INFINITY);
            for (index, (_, rect)) in others.iter().enumerate() {
                let middle = center(*rect);
                let distance = (middle.x - probe.x).powi(2) + (middle.y - probe.y).powi(2);
                if distance < best.1 {
                    best = (index, distance);
                }
            }
            best.0
        });
        let past = is_past(probe, others[chosen].1, flow);
        (list, chosen + usize::from(past))
    }

    /// Moves every sibling toward its planned slot.
    fn arrange(&self, session: &Session<K>, transition: &Transition, ghost: Option<f32>) {
        let plan = self.plan(session);
        for item in self.live_items() {
            let target = plan
                .iter()
                .find(|(planned, _, _)| Rc::ptr_eq(planned, &item))
                .map_or(Offset::ZERO, |(_, rect, origin)| *origin - rect.origin);
            if Rc::ptr_eq(&item, &session.dragged) {
                if let Some(alpha) = ghost {
                    item.shift.ghost_x.animate_to(target.x, transition);
                    item.shift.ghost_y.animate_to(target.y, transition);
                    item.shift.ghost_alpha.set(alpha);
                    item.sync_ghost();
                }
                if session.keyboard
                    && let Some(node) = item.node.upgrade()
                {
                    dragging::settle_to(&node, target, transition);
                }
                continue;
            }
            item.shift.x.animate_to(target.x, transition);
            item.shift.y.animate_to(target.y, transition);
        }
        incular_animation::schedule_frame();
    }

    /// Counts down a pending post-drop relax.
    fn tick_relax(&self) {
        if self.session.borrow().is_some() {
            self.pending_relax.borrow_mut().take();
            return;
        }
        let pending = self.pending_relax.borrow_mut().take();
        match pending {
            Some((0, transition)) => self.relax(&transition),
            Some((frames, transition)) => {
                *self.pending_relax.borrow_mut() = Some((frames - 1, transition));
                incular_animation::schedule_frame();
            }
            None => {}
        }
    }

    /// Animates every leftover offset home (after a drop the app ignored).
    fn relax(&self, transition: &Transition) {
        for item in self.live_items() {
            item.shift.x.animate_to(0., transition);
            item.shift.y.animate_to(0., transition);
            item.shift.ghost_alpha.set(0.);
            item.sync_ghost();
            if let Some(node) = item.node.upgrade() {
                let offset = node
                    .drag_channel
                    .borrow()
                    .as_ref()
                    .map_or(Offset::ZERO, |channel| channel.offset());
                if offset != Offset::ZERO {
                    dragging::settle_to(&node, Offset::ZERO, transition);
                }
            }
        }
        incular_animation::schedule_frame();
    }
}

// ---------------------------------------------------------------------------
// The item behavior

struct ItemBehavior<K> {
    key: K,
    label: Option<String>,
    placeholder: Placeholder,
}

struct ItemSlot<K> {
    item: RefCell<Option<Rc<Item<K>>>>,
    scope: RefCell<Option<ListScope<K>>>,
}

fn slot<K: 'static>(node: &MotionNode) -> Rc<ItemSlot<K>> {
    node.behavior_state(|| ItemSlot {
        item: RefCell::new(None),
        scope: RefCell::new(None),
    })
}

impl<K: Clone + PartialEq + 'static> ItemBehavior<K> {
    fn parts(node: &MotionNode) -> Option<(Rc<Item<K>>, ListScope<K>)> {
        let slot = slot::<K>(node);
        let item = slot.item.borrow().clone()?;
        let scope = slot.scope.borrow().clone()?;
        Some((item, scope))
    }

    fn begin(node: &Rc<MotionNode>, global: Offset, keyboard: bool) -> bool {
        let Some((item, scope)) = Self::parts(node) else {
            return false;
        };
        let core = scope.group.shared();
        let Some((list, index)) = core.index_of(&item) else {
            return false;
        };
        if core.options.get().axis_lock {
            let rects: Vec<Rect> = core.ordered(list).iter().map(|(_, rect)| *rect).collect();
            node.drag_axis.set(match flow_of(&rects) {
                Flow::Vertical => Some(DragAxis::Y),
                Flow::Horizontal => Some(DragAxis::X),
                Flow::Grid => None,
            });
        }
        let key = item.key.borrow().clone();
        *core.session.borrow_mut() = Some(Session {
            key,
            dragged: item,
            from_list: list,
            from: index,
            to_list: list,
            to: index,
            pointer: global,
            keyboard,
        });
        true
    }

    fn step(node: &Rc<MotionNode>, placeholder: Placeholder) {
        let Some((_, scope)) = Self::parts(node) else {
            return;
        };
        let core = scope.group.shared();
        let Some(mut session) = core.session.borrow().clone() else {
            return;
        };
        let (list, to) = core.locate(&session);
        if (list, to) == (session.to_list, session.to) {
            return;
        }
        session.to_list = list;
        session.to = to;
        let transition = node.layout_transition();
        core.arrange(&session, &transition, ghost(placeholder));
        if core.options.get().live {
            let committed = session.clone();
            session.from_list = list;
            session.from = to;
            *core.session.borrow_mut() = Some(session);
            if (committed.from_list, committed.from) != (committed.to_list, committed.to) {
                scope.group.emit(
                    committed.key,
                    committed.from_list,
                    committed.from,
                    committed.to_list,
                    committed.to,
                );
            }
        } else {
            *core.session.borrow_mut() = Some(session);
        }
    }

    /// Commits (or cancels) the session: the dragged item springs into its
    /// slot and `on_move` runs once.
    fn finish(node: &Rc<MotionNode>, cancelled: bool, placeholder: Placeholder) {
        node.drag_axis.set(None);
        let Some((item, scope)) = Self::parts(node) else {
            return;
        };
        let core = scope.group.shared();
        let Some(mut session) = core.session.borrow_mut().take() else {
            return;
        };
        let transition = node.layout_transition();
        if cancelled {
            session.to_list = session.from_list;
            session.to = session.from;
        }
        let plan = core.plan(&session);
        let target = plan
            .iter()
            .find(|(planned, _, _)| Rc::ptr_eq(planned, &item))
            .map_or(Offset::ZERO, |(_, rect, origin)| *origin - rect.origin);
        dragging::settle_to(node, target, &transition);
        let _ = placeholder;
        item.shift.ghost_alpha.set(0.);
        item.sync_ghost();
        let moved = (session.from_list, session.from) != (session.to_list, session.to);
        let total = core.ordered(session.to_list).len();
        let label = item
            .label
            .borrow()
            .clone()
            .unwrap_or_else(|| "Item".to_owned());
        *item.announcement.borrow_mut() = if cancelled {
            format!(
                "{label} returned to position {} of {total}",
                session.from + 1
            )
        } else {
            format!("Moved {label} to position {} of {total}", session.to + 1)
        };
        node.request_rebuild();
        if moved && !cancelled && !core.options.get().live {
            scope.group.emit(
                session.key.clone(),
                session.from_list,
                session.from,
                session.to_list,
                session.to,
            );
        }
        if !moved || cancelled {
            core.relax(&transition);
            return;
        }
        // Whatever the app's rebuild did not absorb drifts home.
        *core.pending_relax.borrow_mut() = Some((2, transition));
        incular_animation::schedule_frame();
    }

    /// Scrolls the nearest scrollable while the pointer is near its edge.
    fn auto_scroll(node: &Rc<MotionNode>) {
        let Some((item, scope)) = Self::parts(node) else {
            return;
        };
        let core = scope.group.shared();
        if !core.options.get().auto_scroll {
            return;
        }
        let Some(controller) = item.scroll.borrow().clone() else {
            return;
        };
        let (Some(session), Some(geometry)) = (core.session.borrow().clone(), node.geometry.get())
        else {
            return;
        };
        const EDGE: f32 = 48.;
        const SPEED: f32 = 12.;
        let viewport = geometry.viewport;
        let top = session.pointer.y - viewport.origin.y;
        let bottom = viewport.origin.y + viewport.size.height - session.pointer.y;
        let delta = if top < EDGE {
            -SPEED * (1. - top.max(0.) / EDGE)
        } else if bottom < EDGE {
            SPEED * (1. - bottom.max(0.) / EDGE)
        } else {
            return;
        };
        let before = controller.offset();
        controller.scroll_by(delta);
        let scrolled = controller.offset() - before;
        if scrolled.abs() < 0.01 {
            return;
        }
        dragging::nudge(node, Offset::new(0., scrolled));
        let weak = Rc::downgrade(node);
        incular_animation::schedule_frame_callback(move |_| {
            if let Some(node) = weak.upgrade()
                && node.interactions().contains(Interaction::Dragging)
            {
                Self::auto_scroll(&node);
            }
        });
    }

    fn keyboard(
        node: &Rc<MotionNode>,
        event: incular_core::KeyboardEvent,
        placeholder: Placeholder,
    ) -> bool {
        if !event.state.is_down() {
            return false;
        }
        let Some((_, scope)) = Self::parts(node) else {
            return false;
        };
        let core = scope.group.shared();
        let lifted = core
            .session
            .borrow()
            .as_ref()
            .is_some_and(|session| session.keyboard);
        let toggles = event.code == Code::Space || event.key == KeyboardKey::Named(NamedKey::Enter);
        if toggles {
            if lifted {
                node.set_flag(Interaction::Dragging, false);
                Self::finish(node, false, placeholder);
            } else if Self::begin(node, Offset::ZERO, true) {
                node.set_flag(Interaction::Dragging, true);
            }
            return true;
        }
        if !lifted {
            return false;
        }
        let step: isize = match event.key {
            KeyboardKey::Named(NamedKey::ArrowUp | NamedKey::ArrowLeft) => -1,
            KeyboardKey::Named(NamedKey::ArrowDown | NamedKey::ArrowRight) => 1,
            KeyboardKey::Named(NamedKey::Escape) => {
                node.set_flag(Interaction::Dragging, false);
                Self::finish(node, true, placeholder);
                return true;
            }
            _ => return false,
        };
        let Some(mut session) = core.session.borrow().clone() else {
            return false;
        };
        let count = core.ordered(session.to_list).len();
        let to = session.to as isize + step;
        if to < 0 || to as usize >= count {
            return true;
        }
        session.to = to as usize;
        core.arrange(&session, &node.layout_transition(), ghost(placeholder));
        *core.session.borrow_mut() = Some(session);
        true
    }

    /// A screen-reader move by one slot, committed at once.
    fn nudge_by(node: &Rc<MotionNode>, step: isize) {
        let Some((item, scope)) = Self::parts(node) else {
            return;
        };
        let core = scope.group.shared();
        let Some((list, index)) = core.index_of(&item) else {
            return;
        };
        let count = core.ordered(list).len();
        let to = index as isize + step;
        if to < 0 || to as usize >= count {
            return;
        }
        let label = item
            .label
            .borrow()
            .clone()
            .unwrap_or_else(|| "Item".to_owned());
        *item.announcement.borrow_mut() =
            format!("Moved {label} to position {} of {count}", to + 1);
        node.request_rebuild();
        scope
            .group
            .emit(item.key.borrow().clone(), list, index, list, to as usize);
    }
}

fn ghost(placeholder: Placeholder) -> Option<f32> {
    match placeholder {
        Placeholder::Ghost(alpha) => Some(alpha.clamp(0., 1.)),
        Placeholder::Gap => None,
    }
}

impl<K: Clone + PartialEq + 'static> SiteBehavior for ItemBehavior<K> {
    fn bind(&self, node: &Rc<MotionNode>, context: &BuildContext<'_>) {
        let slot = slot::<K>(node);
        let Some(scope) = context.find::<ListScope<K>>() else {
            return;
        };
        let item = slot.item.borrow().clone();
        let item = match item {
            Some(item) => item,
            None => {
                let item = Rc::new(Item {
                    focus: incular_gestures::FocusNode::new(),
                    ghost_affine: crate::AffineController::new(),
                    ghost_opacity: crate::OpacityController::new(),
                    key: RefCell::new(self.key.clone()),
                    list: Cell::new(scope.list),
                    node: Rc::downgrade(node),
                    shift: Shift::new(),
                    label: RefCell::new(None),
                    scroll: RefCell::new(None),
                    announcement: RefCell::new(String::new()),
                });
                node.add_extra(item.clone());
                scope.group.shared().items.borrow_mut().push(item.clone());
                *slot.item.borrow_mut() = Some(item.clone());
                item
            }
        };
        node.long_press_touch
            .set(scope.group.shared().options.get().long_press_on_touch);
        *item.key.borrow_mut() = self.key.clone();
        item.list.set(scope.list);
        item.label.borrow_mut().clone_from(&self.label);
        *item.scroll.borrow_mut() = context
            .find::<crate::scroll_aware_image_provider::ScrollableScope>()
            .map(|scope| scope.controller().clone());
        *slot.scope.borrow_mut() = Some(scope);
    }

    fn wrap(&self, node: &Rc<MotionNode>, widget: Widget, child: &Widget) -> Widget {
        let slot = slot::<K>(node);
        let item = slot.item.borrow().clone();
        let mut widget = widget;
        // The ghost sits in the target slot, under the lifted item.
        if let (Placeholder::Ghost(_), Some(item)) = (self.placeholder, item.as_ref()) {
            let ghost = crate::Opacity::new(
                0.,
                Widget::controlled_affine(item.ghost_affine.clone(), child.clone()),
            )
            .controller(item.ghost_opacity.clone())
            .into();
            widget = crate::Stack::new([ghost, widget])
                .fit(incular_config::StackFit::Passthrough)
                .into();
        }
        // Keyboard: Space lifts and drops, arrows move, Escape cancels.
        let placeholder = self.placeholder;
        let keys = Rc::downgrade(node);
        let mut listener = crate::KeyboardListener::new(widget);
        if let Some(item) = item.as_ref() {
            listener = listener.focus_node(item.focus.clone());
        }
        widget = listener
            .on_key(move |event| {
                keys.upgrade()
                    .is_some_and(|node| Self::keyboard(&node, event, placeholder))
            })
            .into();
        // Screen readers: increase / decrease move by one slot, and the
        // last move is announced politely.
        let (down, up) = (Rc::downgrade(node), Rc::downgrade(node));
        let announcement = item
            .as_ref()
            .map(|item| item.announcement.borrow().clone())
            .unwrap_or_default();
        let mut semantics = crate::Semantics::new(widget)
            .on_increase(move || {
                if let Some(node) = down.upgrade() {
                    Self::nudge_by(&node, 1);
                }
            })
            .on_decrease(move || {
                if let Some(node) = up.upgrade() {
                    Self::nudge_by(&node, -1);
                }
            })
            .increased_value("Move down")
            .decreased_value("Move up");
        if !announcement.is_empty() {
            semantics = semantics.live_region(true).value(announcement);
        }
        if let Some(label) = &self.label {
            semantics = semantics.label(label.clone());
        }
        semantics.into()
    }

    fn interactions(&self) -> Interactions {
        Interactions::NONE.with(Interaction::Dragging, true)
    }

    fn wants_geometry(&self) -> bool {
        true
    }

    fn drag_start(&self, node: &Rc<MotionNode>, global: Offset) {
        Self::begin(node, global, false);
    }

    fn drag_update(&self, node: &Rc<MotionNode>, global: Offset) {
        if let Some((_, scope)) = Self::parts(node)
            && let Some(session) = scope.group.shared().session.borrow_mut().as_mut()
        {
            session.pointer = global;
        }
        Self::step(node, self.placeholder);
        Self::auto_scroll(node);
    }

    fn drag_end(
        &self,
        node: &Rc<MotionNode>,
        _global: Offset,
        _velocity: Offset,
        cancelled: bool,
    ) -> bool {
        Self::finish(node, cancelled, self.placeholder);
        true
    }

    fn take_offset(&self, node: &MotionNode, moved: Offset) -> Offset {
        let Some((item, scope)) = Self::parts(node) else {
            return Offset::ZERO;
        };
        let core = scope.group.shared();
        let dragging = core
            .session
            .borrow()
            .as_ref()
            .is_some_and(|session| Rc::ptr_eq(&session.dragged, &item));
        if dragging {
            // Live mode moved the dragged item under the pointer: keep it
            // there instead of animating the jump.
            if let Some(node) = item.node.upgrade() {
                dragging::nudge(&node, Offset::new(-moved.x, -moved.y));
            }
            return moved;
        }
        let shift = item.shift.offset();
        item.shift.x.set(0.);
        item.shift.y.set(0.);
        shift + dragging::take_offset(node)
    }
}

// ---------------------------------------------------------------------------
// Builders

/// A reorderable item; `.motion(..)` styles it as the same site.
pub struct ReorderItem<K: Clone + PartialEq + 'static> {
    child: Widget,
    key: K,
    motion: Motion,
    label: Option<String>,
    placeholder: Placeholder,
    handle: bool,
}

impl<K: Clone + PartialEq + 'static> ReorderItem<K> {
    /// Styles the item on the same site (the default lift is
    /// `dragging(|p| p.scale(1.03).lift(16.))`).
    #[must_use]
    pub fn motion(mut self, build: impl FnOnce(Motion) -> Motion) -> Widget {
        self.motion = build(self.motion);
        self.into()
    }

    /// The pose while lifted.
    #[must_use]
    pub fn lift(mut self, pose: impl FnOnce(Pose) -> Pose) -> Self {
        self.motion = self.motion.dragging(pose);
        self
    }

    /// What the slot shows while the item is lifted.
    #[must_use]
    pub fn placeholder(mut self, placeholder: Placeholder) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// The name screen readers use in move announcements.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Only a descendant `.reorder_handle()` starts the drag.
    #[must_use]
    pub fn with_handle(mut self) -> Self {
        self.handle = true;
        self
    }
}

impl<K: Clone + PartialEq + 'static> From<ReorderItem<K>> for Widget {
    fn from(item: ReorderItem<K>) -> Self {
        let mut drag = Drag::both().release(Release::Stay);
        drag.handle_only = item.handle;
        let mut motion = item.motion.drag(drag);
        if motion.layout.is_none() {
            motion = motion.layout_position();
        }
        motion.behaviors.push(Rc::new(ItemBehavior {
            key: item.key,
            label: item.label,
            placeholder: item.placeholder,
        }));
        motion_widget(item.child, motion)
    }
}

/// A reorderable container with its own (single-list) group.
pub struct Reorderable<K: Clone + 'static> {
    child: Widget,
    group: ReorderGroup<K, ()>,
}

impl<K: Clone + PartialEq + 'static> Reorderable<K> {
    fn update(&self, change: impl FnOnce(&mut Options)) {
        self.group.update(change);
    }

    group_options!();
}

impl<K: Clone + PartialEq + 'static> From<Reorderable<K>> for Widget {
    fn from(value: Reorderable<K>) -> Self {
        list_host(value.group.scope(()), value.child)
    }
}

/// Records a list's own rect (for drops into empty lists) and publishes
/// its scope.
struct ListTicker<K> {
    list: Weak<List>,
    group: Weak<dyn GroupCore<K>>,
}

impl<K: Clone + PartialEq + 'static> FrameTick for ListTicker<K> {
    fn tick(&self, _now: Instant) -> bool {
        if let Some(group) = self.group.upgrade() {
            group.shared().tick_relax();
        }
        false
    }
    fn is_active(&self) -> bool {
        self.group
            .upgrade()
            .is_some_and(|group| group.shared().pending_relax.borrow().is_some())
    }
    fn wants_geometry(&self) -> bool {
        true
    }
    fn geometry(&self, geometry: FrameGeometry) {
        if let Some(list) = self.list.upgrade() {
            list.rect.set(Some(geometry.layout_rect));
        }
    }
}

fn list_host<K: Clone + PartialEq + 'static>(scope: ListScope<K>, child: Widget) -> Widget {
    let list = scope.group.shared().lists.borrow().get(scope.list).cloned();
    let child = match list {
        Some(list) => Widget::frame_ticker(
            FrameDelegate::new(Rc::new(ListTicker {
                list: Rc::downgrade(&list),
                group: Rc::downgrade(&scope.group),
            })),
            child,
        ),
        None => child,
    };
    Widget::environment_scope(scope, child)
}

/// `.reorder_item()`, `.reorderable()`, `.reorderable_in()` and
/// `.reorder_handle()` for anything that becomes a widget.
pub trait ReorderExt: Into<Widget> + Sized {
    /// Makes this element a reorderable item identified by `key`.
    #[must_use]
    fn reorder_item<K: Clone + PartialEq + 'static>(self, key: K) -> ReorderItem<K> {
        ReorderItem {
            child: self.into(),
            key,
            motion: Motion::new().dragging(|p| p.scale(1.03).lift(16.)),
            label: None,
            placeholder: Placeholder::Gap,
            handle: false,
        }
    }

    /// Makes this container's reorder items sortable; `on_move` runs once
    /// per committed move.
    #[must_use]
    fn reorderable<K: Clone + PartialEq + 'static>(
        self,
        on_move: impl Fn(Move<K>) + 'static,
    ) -> Reorderable<K> {
        Reorderable {
            child: self.into(),
            group: ReorderGroup::new().on_move(on_move),
        }
    }

    /// Makes this container list `list` of a shared `group` (Kanban).
    #[must_use]
    fn reorderable_in<K: Clone + PartialEq + 'static, L: Clone + PartialEq + 'static>(
        self,
        group: &ReorderGroup<K, L>,
        list: L,
    ) -> Widget {
        list_host(group.scope(list), self.into())
    }

    /// Makes this element the drag handle of the enclosing reorder item
    /// (declared with `ReorderItem::with_handle`).
    #[must_use]
    fn reorder_handle(self) -> Widget {
        let child = self.into();
        crate::Builder::new(move |context| {
            match context
                .find::<dragging::HandleScope>()
                .and_then(|scope| scope.0.upgrade())
            {
                Some(node) => dragging::pan(&node, child.clone()),
                None => child.clone(),
            }
        })
        .into()
    }
}

impl<T: Into<Widget>> ReorderExt for T {}
