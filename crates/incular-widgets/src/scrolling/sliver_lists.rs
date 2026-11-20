use super::reorderable;
use super::*;
use crate::advanced_slivers::{AnimatedItem, AnimatedItemBuilder, AnimatedItemPhase};

/// Retained state for a reorderable sliver. The order stores logical item
/// identities rather than visible slots, so moving an item preserves its
/// element state and measured extent.
#[derive(Clone)]
pub struct SliverReorderController {
    state: Rc<RefCell<SliverReorderState>>,
}

struct SliverReorderState {
    order: Vec<usize>,
    next_item: usize,
    revision: u64,
}

/// Flutter's reorder animations (widgets/reorderable_list.dart): the proxy's
/// lift and drop (`_DragInfo.startDrag`) and each item's gap move
/// (`_ReorderableItemState.updateForGap`) run 250 ms.
const REORDER_ANIMATION: Duration = Duration::from_millis(250);

/// The callbacks a drag start and release report.
#[derive(Clone, Default)]
struct ReorderCallbacks {
    on_reorder_start: Option<Rc<dyn Fn(usize)>>,
    on_reorder_end: Option<Rc<dyn Fn(usize)>>,
}

/// An item's gap translation along the content axis, Flutter
/// `_ReorderableItemState` `_startOffset`, `_targetOffset` and
/// `_offsetAnimation`.
struct GapOffset {
    start: f32,
    target: f32,
    animation: Option<AnimationController>,
}

impl GapOffset {
    /// `_ReorderableItemState.offset`.
    fn current(&self) -> f32 {
        match &self.animation {
            Some(animation) => {
                let t = Curve::EaseInOut.apply(animation.value());
                self.start + (self.target - self.start) * t
            }
            None => self.target,
        }
    }
}

/// The dragged item, Flutter `_DragInfo` plus the list's `_dragIndex`,
/// `_insertIndex` and `_finalDropPosition`, in content coordinates.
struct ActiveReorder {
    item: usize,
    index: usize,
    insert_index: usize,
    /// The item's slot offset when the drag started.
    origin: f32,
    extent: f32,
    /// `_proxyAnimation`: forward from the start, reversed at the release.
    proxy: AnimationController,
    /// Where the proxy lands, set at the release.
    drop: Option<f32>,
}

/// The in-flight reorder: how far its proxy travelled along the list's
/// content axis, plus the offsets (one past the end) of the items the last
/// layout built, starting at `first`. Flutter's index rule considers only
/// built items.
#[derive(Default)]
struct ReorderDrag {
    travel: f32,
    first: usize,
    offsets: Vec<f32>,
    active: Option<ActiveReorder>,
    /// Gap translations by position.
    gaps: HashMap<usize, GapOffset>,
    /// Set by a drag update or release: the next frame lays out again.
    dirty: bool,
}

impl ReorderDrag {
    /// The gap target of the built item at `index`; the dragged item has
    /// none.
    fn target(&self, index: usize) -> f32 {
        self.gaps.get(&index).map_or(0., |gap| gap.target)
    }

    /// The start and end of the built item at `index` with its gap target
    /// applied (`_ReorderableItemState.targetGeometry`).
    fn geometry(&self, index: usize) -> Option<(f32, f32)> {
        let slot = index.checked_sub(self.first)?;
        let (&start, &end) = (self.offsets.get(slot)?, self.offsets.get(slot + 1)?);
        let target = self.target(index);
        Some((start + target, end + target))
    }

    /// The content offset of the item at `position` with its gap animation.
    fn gap_offset(&self, position: usize) -> f32 {
        self.gaps.get(&position).map_or(0., GapOffset::current)
    }

    /// The proxy's leading edge (`_DragItemProxy` `effectivePosition`).
    fn proxy_offset(&self) -> Option<f32> {
        let active = self.active.as_ref()?;
        let position = active.origin + self.travel;
        Some(match active.drop {
            Some(drop) => {
                let t = Curve::EaseOut.apply(active.proxy.value());
                drop + (position - drop) * t
            }
            None => position,
        })
    }

    /// Flutter `_dragStart`: the item at `index` lifts into the proxy and
    /// every gap resets.
    fn start(&mut self, item: usize, index: usize) {
        let (origin, extent) = index
            .checked_sub(self.first)
            .and_then(|slot| Some((*self.offsets.get(slot)?, *self.offsets.get(slot + 1)?)))
            .map_or((0., 0.), |(start, end)| (start, end - start));
        let proxy = AnimationController::new(REORDER_ANIMATION);
        proxy.forward_on_next_tick();
        self.travel = 0.;
        self.gaps.clear();
        self.active = Some(ActiveReorder {
            item,
            index,
            insert_index: index,
            origin,
            extent,
            proxy,
            drop: None,
        });
        self.dirty = true;
    }

    /// Flutter `_dragUpdate` and `_dragUpdateItems`: the insertion index,
    /// counted with the dragged item still present, for the proxy's current
    /// geometry, then the gap moves toward it. An item claims the slot when
    /// the proxy's start lies in its leading half, or the proxy's end in its
    /// trailing half; otherwise the index follows the items the proxy passed
    /// entirely. Geometry is in content coordinates, which also covers a
    /// reversed list.
    fn update(&mut self, delta: f32) {
        self.travel += delta;
        let Some(active) = self.active.as_ref() else {
            return;
        };
        self.dirty = true;
        if active.drop.is_some() {
            return;
        }
        let (drag_index, gap_extent) = (active.index, active.extent);
        let proxy_start = active.origin + self.travel;
        let proxy_end = proxy_start + gap_extent;
        let mut new_index = active.insert_index;
        let count = self.offsets.len().saturating_sub(1);
        for index in self.first..self.first + count {
            let Some((start, end)) = self.geometry(index) else {
                continue;
            };
            let middle = start + (end - start) / 2.;
            if index == drag_index {
                // The dragged item's own slot is taken back only by the
                // proxy's end in its trailing half.
                if middle <= proxy_end && proxy_end <= end {
                    new_index = drag_index;
                }
            } else if start <= proxy_start && proxy_start <= middle {
                new_index = index;
                break;
            } else if middle <= proxy_end && proxy_end <= end {
                new_index = index + 1;
                break;
            } else if end < proxy_start && new_index < index + 1 {
                new_index = index + 1;
            } else if proxy_end < start && new_index > index {
                new_index = index;
            }
        }
        if new_index == active.insert_index {
            return;
        }
        if let Some(active) = self.active.as_mut() {
            active.insert_index = new_index;
        }
        for index in self.first..self.first + count {
            if index != drag_index {
                self.update_for_gap(index, drag_index, new_index, gap_extent);
            }
        }
    }

    /// Flutter `_ReorderableItemState.updateForGap` with `animate: true`.
    fn update_for_gap(&mut self, index: usize, drag_index: usize, gap_index: usize, extent: f32) {
        let target = if gap_index < drag_index && index < drag_index && index >= gap_index {
            extent
        } else if gap_index > drag_index && index > drag_index && index < gap_index {
            -extent
        } else {
            0.
        };
        let gap = self.gaps.entry(index).or_insert(GapOffset {
            start: 0.,
            target: 0.,
            animation: None,
        });
        if gap.target == target {
            return;
        }
        let previous = gap.target;
        gap.target = target;
        match &gap.animation {
            None => {
                let animation = AnimationController::new(REORDER_ANIMATION);
                animation.forward_on_next_tick();
                gap.animation = Some(animation);
            }
            Some(animation) => {
                // Interrupted: continue from where the item is now.
                let t = Curve::EaseInOut.apply(animation.value());
                gap.start += (previous - gap.start) * t;
                animation.set_value(0.);
                animation.forward_on_next_tick();
            }
        }
    }

    /// Flutter `_dragEnd`: where the proxy lands, then its reverse run.
    /// Returns the insertion index for `onReorderEnd`.
    fn release(&mut self) -> Option<usize> {
        let active = self.active.as_ref()?;
        if active.drop.is_some() {
            return None;
        }
        let (index, insert) = (active.index, active.insert_index);
        let slot = |tree: &Self, at: usize| tree.geometry(at);
        let drop = if insert == index + 1 || insert == index {
            // Back to its own slot, from either side.
            active.origin
        } else if insert == 0 {
            slot(self, 0).map_or(active.origin, |(start, _)| start - active.extent)
        } else {
            slot(self, insert - 1).map_or(active.origin, |(_, end)| end)
        };
        let active = self.active.as_mut()?;
        active.drop = Some(drop);
        active.proxy.reverse_on_next_tick();
        self.dirty = true;
        Some(insert)
    }

    /// Flutter `_dragReset`.
    fn reset(&mut self) {
        self.active = None;
        self.gaps.clear();
        self.travel = 0.;
        self.dirty = true;
    }

    /// Advances the proxy and gap animations. Returns whether anything moved
    /// and, once the dropped proxy has landed (`_dropCompleted`), the
    /// reorder's `(from, to)` with `to` counted before removal.
    fn tick(&mut self, now: Instant) -> (bool, Option<(usize, usize)>) {
        let mut changed = std::mem::take(&mut self.dirty);
        for gap in self.gaps.values_mut() {
            if let Some(animation) = &gap.animation {
                changed |= animation.tick(now);
                if animation.status() == incular_animation::AnimationStatus::Completed {
                    gap.start = gap.target;
                    gap.animation = None;
                }
            }
        }
        let Some(active) = self.active.as_ref() else {
            return (changed, None);
        };
        changed |= active.proxy.tick(now);
        if active.drop.is_some()
            && !active.proxy.is_active()
            && active.proxy.status() == incular_animation::AnimationStatus::Dismissed
        {
            let completed = (active.index, active.insert_index);
            self.reset();
            return (true, Some(completed));
        }
        (changed, None)
    }

    fn is_animating(&self) -> bool {
        self.dirty
            || self.gaps.values().any(|gap| {
                gap.animation
                    .as_ref()
                    .is_some_and(AnimationController::is_active)
            })
            || self
                .active
                .as_ref()
                .is_some_and(|active| active.drop.is_some() || active.proxy.is_active())
    }
}

/// Follows the dragged proxy along the content axis (Flutter's proxy tracks
/// the pointer); a reversed list's content runs against the screen.
fn track_reorder_drag(
    draggable: Draggable<usize>,
    drag: &Rc<RefCell<ReorderDrag>>,
    axis: Axis,
    reverse: bool,
) -> Draggable<usize> {
    let update = drag.clone();
    draggable.on_drag_update(move |delta| {
        let along = match axis {
            Axis::Vertical => delta.y,
            Axis::Horizontal => delta.x,
        };
        update
            .borrow_mut()
            .update(if reverse { -along } else { along });
    })
}

/// Starts the reorder of `item` and reports `onReorderStart(index)`.
fn begin_reorder(
    drag: &Rc<RefCell<ReorderDrag>>,
    controller: &SliverReorderController,
    item: usize,
    callbacks: &ReorderCallbacks,
    reported: usize,
) {
    let index = controller.position_of(item).unwrap_or(reported);
    drag.borrow_mut().start(item, index);
    if let Some(callback) = &callbacks.on_reorder_start {
        callback(reported);
    }
}

/// Releases the reorder and reports `onReorderEnd(insertIndex)`.
fn end_reorder(drag: &Rc<RefCell<ReorderDrag>>, item: usize, callbacks: &ReorderCallbacks) {
    let insert = {
        let mut drag = drag.borrow_mut();
        if drag
            .active
            .as_ref()
            .is_none_or(|active| active.item != item)
        {
            return;
        }
        drag.release()
    };
    if let (Some(callback), Some(insert)) = (&callbacks.on_reorder_end, insert) {
        callback(insert);
    }
}

/// Flutter `_dragCancel`: the list returns to its pre-drag state.
fn cancel_reorder(drag: &Rc<RefCell<ReorderDrag>>, item: usize) {
    let mut drag = drag.borrow_mut();
    if drag
        .active
        .as_ref()
        .is_some_and(|active| active.item == item && active.drop.is_none())
    {
        drag.reset();
    }
}

impl SliverReorderController {
    #[must_use]
    pub fn new(item_count: usize) -> Self {
        Self {
            state: Rc::new(RefCell::new(SliverReorderState {
                order: (0..item_count).collect(),
                next_item: item_count,
                revision: 0,
            })),
        }
    }

    #[must_use]
    pub fn item_count(&self) -> usize {
        self.state.borrow().order.len()
    }

    /// Returns the logical item IDs in their current visual order.
    #[must_use]
    pub fn order(&self) -> Vec<usize> {
        self.state.borrow().order.clone()
    }

    #[must_use]
    pub fn item_at(&self, position: usize) -> Option<usize> {
        self.state.borrow().order.get(position).copied()
    }

    #[must_use]
    pub fn position_of(&self, item: usize) -> Option<usize> {
        self.state
            .borrow()
            .order
            .iter()
            .position(|candidate| *candidate == item)
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.state.borrow().revision
    }

    /// Moves `from` to `to`, where `to` is the final position after removal.
    /// This is the same operation used by the retained sliver drop target.
    pub fn move_item(&self, from: usize, to: usize) -> bool {
        let mut state = self.state.borrow_mut();
        if from >= state.order.len() || to >= state.order.len() || from == to {
            return false;
        }
        let item = state.order.remove(from);
        state.order.insert(to, item);
        state.revision = state.revision.checked_add(1).expect("revision exhausted");
        true
    }

    /// Applies Flutter's `onReorder(oldIndex, newIndex)` convention. Flutter
    /// reports `newIndex` before the old item is removed, so destinations after
    /// the source are shifted back by one.
    pub fn reorder(&self, old_index: usize, new_index: usize) -> bool {
        let len = self.item_count();
        if old_index >= len || new_index > len {
            return false;
        }
        let destination = if old_index < new_index {
            new_index.saturating_sub(1)
        } else {
            new_index
        };
        self.move_item(old_index, destination.min(len.saturating_sub(1)))
    }

    /// Changes the number of logical items. Newly appended items receive
    /// fresh identities; existing identities and their retained state remain.
    pub fn set_item_count(&self, item_count: usize) {
        let mut state = self.state.borrow_mut();
        if item_count == state.order.len() {
            return;
        }
        if item_count < state.order.len() {
            state.order.truncate(item_count);
        } else {
            while state.order.len() < item_count {
                let item = state.next_item;
                state.next_item = state
                    .next_item
                    .checked_add(1)
                    .expect("sliver item identity space exhausted");
                state.order.push(item);
            }
        }
        state.revision = state.revision.checked_add(1).expect("revision exhausted");
    }
}

struct ReorderableRenderSliver {
    index: MeasuredExtentIndex,
    builder: Rc<dyn Fn(usize) -> Widget>,
    controller: SliverReorderController,
    drag_context: DragDropContext<usize>,
    on_reorder: Option<Rc<dyn Fn(usize, usize)>>,
    on_reorder_item: Option<Rc<dyn Fn(usize, usize)>>,
    on_reorder_start: Option<Rc<dyn Fn(usize)>>,
    on_reorder_end: Option<Rc<dyn Fn(usize)>>,
    require_drag_listener: bool,
    delayed_states: Rc<RefCell<HashMap<usize, Rc<Cell<bool>>>>>,
    widgets: HashMap<usize, Widget>,
    order: Vec<usize>,
    item_positions: HashMap<usize, usize>,
    controller_revision: u64,
    axis: Axis,
    reverse: bool,
    drag: Rc<RefCell<ReorderDrag>>,
}

struct ReorderableRenderSliverParams {
    controller: SliverReorderController,
    builder: Rc<dyn Fn(usize) -> Widget>,
    drag_context: DragDropContext<usize>,
    on_reorder: Option<Rc<dyn Fn(usize, usize)>>,
    on_reorder_item: Option<Rc<dyn Fn(usize, usize)>>,
    on_reorder_start: Option<Rc<dyn Fn(usize)>>,
    on_reorder_end: Option<Rc<dyn Fn(usize)>>,
    item_extent: Option<f32>,
    item_extent_builder: Option<Rc<dyn Fn(usize) -> f32>>,
    prototype_item: Option<Widget>,
    axis: Axis,
    reverse: bool,
    require_drag_listener: bool,
}

/// Maps each logical item to its first visual position.
fn item_position_map(order: &[usize]) -> HashMap<usize, usize> {
    let mut positions = HashMap::with_capacity(order.len());
    for (position, item) in order.iter().enumerate() {
        positions.entry(*item).or_insert(position);
    }
    positions
}

impl ReorderableRenderSliver {
    fn new(params: ReorderableRenderSliverParams) -> Self {
        let ReorderableRenderSliverParams {
            controller,
            builder,
            drag_context,
            on_reorder,
            on_reorder_item,
            on_reorder_start,
            on_reorder_end,
            item_extent,
            item_extent_builder,
            prototype_item,
            axis,
            reverse,
            require_drag_listener,
        } = params;
        let order = controller.order();
        let index = if let Some(extent) = item_extent {
            MeasuredExtentIndex::with_estimates(order.len(), extent, |_| extent)
        } else if let Some(extent_builder) = item_extent_builder {
            MeasuredExtentIndex::with_estimates(
                order.len(),
                DEFAULT_LAZY_ITEM_EXTENT,
                move |position| extent_builder(position),
            )
        } else if let Some(prototype) = prototype_item {
            let extent = widget_main_extent_hint(&prototype, axis)
                .unwrap_or(DEFAULT_LAZY_ITEM_EXTENT)
                .max(1.);
            MeasuredExtentIndex::with_estimates(order.len(), extent, |_| extent)
        } else {
            MeasuredExtentIndex::new(order.len(), DEFAULT_LAZY_ITEM_EXTENT)
        };
        Self {
            index,
            builder,
            controller_revision: controller.revision(),
            controller,
            drag_context,
            on_reorder,
            on_reorder_item,
            on_reorder_start,
            on_reorder_end,
            require_drag_listener,
            delayed_states: Rc::new(RefCell::new(HashMap::new())),
            widgets: HashMap::new(),
            item_positions: item_position_map(&order),
            order,
            axis,
            reverse,
            drag: Rc::new(RefCell::new(ReorderDrag::default())),
        }
    }

    fn sync_controller(&mut self) {
        let revision = self.controller.revision();
        if revision == self.controller_revision {
            return;
        }
        let next_order = self.controller.order();
        if self.order.len() == next_order.len() {
            let mut current = self.order.clone();
            for (destination, item) in next_order.iter().copied().enumerate() {
                let Some(source) = current.iter().position(|candidate| *candidate == item) else {
                    continue;
                };
                if source != destination {
                    let moved = self.index.move_item(source, destination);
                    debug_assert!(moved, "reorder source and destination are in range");
                    let moved = current.remove(source);
                    current.insert(destination, moved);
                }
            }
        } else {
            self.index.set_len(next_order.len());
        }
        // A cached target captures its old destination position. Recreate the
        // lightweight drag wrappers after an order mutation while retaining
        // the underlying element by its stable logical item ID.
        if self.order != next_order {
            self.widgets.clear();
        }
        self.item_positions = item_position_map(&next_order);
        self.widgets
            .retain(|item, _| self.item_positions.contains_key(item));
        self.order = next_order;
        self.controller_revision = revision;
    }

    fn callbacks(&self) -> ReorderCallbacks {
        ReorderCallbacks {
            on_reorder_start: self.on_reorder_start.clone(),
            on_reorder_end: self.on_reorder_end.clone(),
        }
    }

    /// Wraps an item in its drag source. As in Flutter, the list (not a drop
    /// target) owns the reorder: the drag lifts the item into a proxy, the
    /// other items move aside, and the release animates the proxy into its
    /// slot before `onReorder` reports the move.
    fn child_widget(&mut self, position: usize, item: usize) -> Widget {
        if let Some(widget) = self.widgets.get(&item) {
            return widget.clone();
        }
        let mut child = (self.builder)(item);
        let context = self.drag_context.clone();
        let controller = self.controller.clone();
        let callbacks = self.callbacks();
        let delayed_states = self.delayed_states.clone();
        let drag = self.drag.clone();
        let (axis, reverse) = (self.axis, self.reverse);
        let mut install_source = |child: Widget, spec: reorderable::ReorderableListenerSpec| {
            let delayed = if spec.delayed {
                let delayed = delayed_states
                    .borrow_mut()
                    .entry(item)
                    .or_insert_with(|| Rc::new(Cell::new(false)))
                    .clone();
                Some(delayed)
            } else {
                delayed_states.borrow_mut().remove(&item);
                None
            };
            let can_start = delayed.clone();
            let start_index = spec.index;
            let draggable = Draggable::new(context.clone(), item, child)
                .on_start({
                    let drag = drag.clone();
                    let controller = controller.clone();
                    let callbacks = callbacks.clone();
                    move |_| {
                        // A delayed listener's drag starts only after its long
                        // press, so the start itself marks the listener started.
                        if let Some(started) = &can_start {
                            started.set(true);
                        }
                        begin_reorder(&drag, &controller, item, &callbacks, start_index);
                    }
                })
                .on_end({
                    let drag = drag.clone();
                    let callbacks = callbacks.clone();
                    let delayed = delayed.clone();
                    move |_| {
                        if delayed.as_ref().is_some_and(|started| !started.get()) {
                            return;
                        }
                        end_reorder(&drag, item, &callbacks);
                        if let Some(delayed) = &delayed {
                            delayed.set(false);
                        }
                    }
                })
                .on_cancel({
                    let drag = drag.clone();
                    let delayed = delayed.clone();
                    move |_| {
                        cancel_reorder(&drag, item);
                        if let Some(delayed) = &delayed {
                            delayed.set(false);
                        }
                    }
                });
            // Flutter `ReorderableDelayedDragStartListener`: one delayed drag
            // recognizer, not a long-press detector around an immediate drag.
            let draggable = if spec.delayed {
                draggable
                    .start_delay(incular_gestures::PointerGestureRecognizer::LONG_PRESS_TIMEOUT)
            } else {
                draggable
            };
            track_reorder_drag(draggable, &drag, axis, reverse).into()
        };
        let found_listener = reorderable::install_listener(&mut child, &mut install_source);
        let source_enabled = found_listener.is_some_and(|spec| spec.enabled);
        if found_listener.is_none() {
            delayed_states.borrow_mut().remove(&item);
        }
        if !source_enabled && !self.require_drag_listener && found_listener.is_none() {
            let draggable = Draggable::new(context, item, child)
                .on_start({
                    let drag = drag.clone();
                    let controller = controller.clone();
                    let callbacks = callbacks.clone();
                    move |_| begin_reorder(&drag, &controller, item, &callbacks, position)
                })
                .on_end({
                    let drag = drag.clone();
                    let callbacks = callbacks.clone();
                    move |_| end_reorder(&drag, item, &callbacks)
                })
                .on_cancel({
                    let drag = drag.clone();
                    move |_| {
                        cancel_reorder(&drag, item);
                        delayed_states.borrow_mut().remove(&item);
                    }
                });
            child = track_reorder_drag(draggable, &drag, axis, reverse).into();
        } else if found_listener.is_some() && !source_enabled {
            delayed_states.borrow_mut().remove(&item);
        }
        self.widgets.insert(item, child.clone());
        child
    }

    /// Flutter `_dropCompleted`: `onReorder(from, to)` when they differ,
    /// with `to` counted before the dragged item is removed.
    fn complete_reorder(&self, from: usize, to: usize) {
        if from == to {
            return;
        }
        let destination = if to > from { to - 1 } else { to };
        let moved = self.controller.move_item(from, destination);
        if let Some(callback) = &self.on_reorder {
            callback(from, to);
        } else if let Some(callback) = &self.on_reorder_item
            && moved
        {
            callback(from, destination);
        }
    }
}

impl RenderSliver for ReorderableRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        self.sync_controller();
        let cache_start = (constraints.scroll_offset + constraints.cache_origin).max(0.);
        let cache_end = (cache_start + constraints.remaining_cache_extent).max(cache_start);
        let range =
            self.index
                .materialized_range(cache_start, (cache_end - cache_start).max(0.), 0.);
        // The built items' geometry, for the drop-index rule of a drag.
        {
            let mut drag = self.drag.borrow_mut();
            drag.first = range.start;
            drag.offsets = self.index.offsets_for_range(range.clone());
        }
        self.widgets
            .retain(|item, _| self.item_positions.contains_key(item));
        // Taken so the children can be built with `&mut self`; `child_widget`
        // never reads `order`.
        let order = std::mem::take(&mut self.order);
        // While a reorder runs, the dragged item is the proxy: it follows the
        // drag (then its drop) and paints above the others, which move aside
        // by their gap translations (Flutter `_DragItemProxy` and
        // `_ReorderableItemState.offset`).
        let (proxy, gaps): (Option<(usize, f32)>, Vec<f32>) = {
            let drag = self.drag.borrow();
            (
                drag.active
                    .as_ref()
                    .zip(drag.proxy_offset())
                    .map(|(active, offset)| (active.index, offset)),
                range
                    .clone()
                    .map(|position| drag.gap_offset(position))
                    .collect(),
            )
        };
        let mut children: Vec<SliverChildLayout> = range
            .zip(gaps)
            .map(|(position, gap)| {
                let item = order[position];
                let slot = self.index.offset_for_index(position);
                let extent = self.index.offset_for_index(position + 1) - slot;
                let (offset, placement) = match proxy {
                    Some((index, offset)) if index == position => {
                        (offset, crate::scrolling::SliverChildPlacement::Floating)
                    }
                    _ => (slot + gap, crate::scrolling::SliverChildPlacement::Flow),
                };
                SliverChildLayout {
                    id: SliverChildId::list_item(item),
                    widget: self.child_widget(position, item),
                    semantic_index: Some(position),
                    offset,
                    cross_offset: 0.,
                    constraints: sliver_child_constraints(
                        constraints.axis,
                        constraints.cross_axis_extent,
                        Some(extent),
                    ),
                    extent,
                    placement,
                }
            })
            .collect();
        self.order = order;
        // The proxy paints last, above the list.
        if let Some(at) = children
            .iter()
            .position(|child| child.placement != crate::scrolling::SliverChildPlacement::Flow)
        {
            let proxy = children.remove(at);
            children.push(proxy);
        }
        SliverLayout {
            geometry: SliverGeometry::from_scroll_extent(constraints, self.index.total_extent()),
            children,
            absorbed_overlap: 0.,
        }
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        let Some(item) = child
            .0
            .checked_sub(1)
            .and_then(|id| usize::try_from(id).ok())
        else {
            return false;
        };
        self.item_positions
            .get(&item)
            .copied()
            .is_some_and(|position| self.index.set_measured_extent(position, extent))
    }

    fn revision(&self) -> u64 {
        self.controller
            .revision()
            .checked_add(self.index.revision())
            .expect("sliver list composite revision exhausted")
    }

    /// Advances the reorder proxy and gap animations before layout, and
    /// reports a landed drop (`_dropCompleted`).
    fn tick(&mut self, now: Instant) -> bool {
        let (changed, completed) = self.drag.borrow_mut().tick(now);
        if let Some((from, to)) = completed {
            self.complete_reorder(from, to);
        }
        changed
    }

    fn is_animating(&self) -> bool {
        self.drag.borrow().is_animating()
    }
}

/// Reorderable sliver list. Items are wrapped in retained drag sources and
/// targets, so a normal pointer drag performs the same logical operation as a
/// Flutter reorderable list without OS-level input injection.
pub struct SliverReorderableList {
    controller: SliverReorderController,
    builder: Rc<dyn Fn(usize) -> Widget>,
    drag_context: DragDropContext<usize>,
    on_reorder: Option<Rc<dyn Fn(usize, usize)>>,
    on_reorder_item: Option<Rc<dyn Fn(usize, usize)>>,
    on_reorder_start: Option<Rc<dyn Fn(usize)>>,
    on_reorder_end: Option<Rc<dyn Fn(usize)>>,
    item_extent: Option<f32>,
    item_extent_builder: Option<Rc<dyn Fn(usize) -> f32>>,
    prototype_item: Option<Widget>,
    require_drag_listener: bool,
}

impl SliverReorderableList {
    #[must_use]
    pub fn new<W>(item_count: usize, builder: impl Fn(usize) -> W + 'static) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            controller: SliverReorderController::new(item_count),
            builder: Rc::new(move |i| builder(i).into()),
            drag_context: DragDropContext::new(),
            on_reorder: None,
            on_reorder_item: None,
            on_reorder_start: None,
            on_reorder_end: None,
            item_extent: None,
            item_extent_builder: None,
            prototype_item: None,
            require_drag_listener: false,
        }
    }

    /// Replaces the retained order controller.
    #[must_use]
    pub fn controller(mut self, controller: SliverReorderController) -> Self {
        self.controller = controller;
        self
    }

    #[must_use]
    pub fn reorder_controller(&self) -> SliverReorderController {
        self.controller.clone()
    }

    /// Receives `(old_position, new_position)` after a successful drop.
    #[must_use]
    pub fn on_reorder(mut self, callback: impl Fn(usize, usize) + 'static) -> Self {
        self.on_reorder = Some(Rc::new(callback));
        self.on_reorder_item = None;
        self
    }

    /// Receives `(old_position, final_position)` after the old item is
    /// removed. This is the adjusted Flutter `onReorderItem` convention.
    #[must_use]
    pub fn on_reorder_item(mut self, callback: impl Fn(usize, usize) + 'static) -> Self {
        self.on_reorder_item = Some(Rc::new(callback));
        self.on_reorder = None;
        self
    }

    /// Receives the visual index once the drag source has actually started.
    #[must_use]
    pub fn on_reorder_start(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_reorder_start = Some(Rc::new(callback));
        self
    }

    /// Receives the final visual index when a started drag ends, including a
    /// drop at its original location.
    #[must_use]
    pub fn on_reorder_end(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_reorder_end = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub(crate) fn require_drag_listener(mut self, required: bool) -> Self {
        self.require_drag_listener = required;
        self
    }

    #[must_use]
    pub fn item_extent(mut self, extent: f32) -> Self {
        self.item_extent = extent.is_finite().then_some(extent.max(1.));
        self.item_extent_builder = None;
        self.prototype_item = None;
        self
    }

    #[must_use]
    pub fn item_extent_builder(mut self, builder: impl Fn(usize) -> f32 + 'static) -> Self {
        self.item_extent_builder = Some(Rc::new(builder));
        self.item_extent = None;
        self.prototype_item = None;
        self
    }

    #[must_use]
    pub fn prototype_item(mut self, prototype: impl Into<Widget>) -> Self {
        self.prototype_item = Some(prototype.into());
        self.item_extent = None;
        self.item_extent_builder = None;
        self
    }
}

impl Sliver for SliverReorderableList {
    fn build(&self, controller: &ScrollController) -> Widget {
        single_sliver_viewport(
            controller.clone(),
            Axis::Vertical,
            false,
            ScrollPhysics::default(),
            DEFAULT_SLIVER_CACHE_EXTENT,
            Clip::HardEdge,
            self.create_render_sliver(controller, Axis::Vertical, false),
        )
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(ReorderableRenderSliver::new(
            ReorderableRenderSliverParams {
                controller: self.controller.clone(),
                builder: self.builder.clone(),
                drag_context: self.drag_context.clone(),
                on_reorder: self.on_reorder.clone(),
                on_reorder_item: self.on_reorder_item.clone(),
                on_reorder_start: self.on_reorder_start.clone(),
                on_reorder_end: self.on_reorder_end.clone(),
                item_extent: self.item_extent,
                item_extent_builder: self.item_extent_builder.clone(),
                prototype_item: self.prototype_item.clone(),
                axis: _axis,
                reverse: _reverse,
                require_drag_listener: self.require_drag_listener,
            },
        ))
    }
}

struct AnimatedListEntry {
    id: u64,
    animation: AnimationController,
    removing: bool,
}

#[derive(Clone, Copy)]
struct AnimatedListEntrySnapshot {
    id: u64,
    value: f32,
    removing: bool,
    incoming: bool,
}

struct SliverAnimatedListState {
    entries: Vec<AnimatedListEntry>,
    next_id: u64,
    duration: Duration,
    revision: u64,
    structure_revision: u64,
}

/// Retained insertion/removal state for [`SliverAnimatedList`]. Calling
/// `insert` or `remove` changes only this small state object; the viewport
/// then drives the size transition from its normal frame clock.
#[derive(Clone)]
pub struct SliverAnimatedListController {
    state: Rc<RefCell<SliverAnimatedListState>>,
}

impl SliverAnimatedListController {
    #[must_use]
    pub fn new(item_count: usize) -> Self {
        Self::with_duration(item_count, Duration::from_millis(250))
    }

    #[must_use]
    pub fn with_duration(item_count: usize, duration: Duration) -> Self {
        let entries = (0..item_count)
            .map(|id| {
                let animation = AnimationController::new(duration);
                animation.set_value(1.);
                AnimatedListEntry {
                    id: id as u64,
                    animation,
                    removing: false,
                }
            })
            .collect();
        Self {
            state: Rc::new(RefCell::new(SliverAnimatedListState {
                entries,
                next_id: item_count as u64,
                duration,
                revision: 0,
                structure_revision: 0,
            })),
        }
    }

    #[must_use]
    pub fn item_count(&self) -> usize {
        self.state.borrow().entries.len()
    }

    #[must_use]
    pub fn duration(&self) -> Duration {
        self.state.borrow().duration
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.state.borrow().revision
    }

    /// Inserts one logical item and starts its expansion from zero extent.
    pub fn insert(&self, index: usize) -> bool {
        self.insert_at(index, Instant::now())
    }

    /// Deterministic-clock form of [`Self::insert`], useful for tests.
    pub fn insert_at(&self, index: usize, now: Instant) -> bool {
        let mut state = self.state.borrow_mut();
        let duration = state.duration;
        let id = state.next_id;
        state.next_id = state
            .next_id
            .checked_add(1)
            .expect("animated list identity space exhausted");
        let animation = AnimationController::new(duration);
        animation.forward(now);
        let index = index.min(state.entries.len());
        state.entries.insert(
            index,
            AnimatedListEntry {
                id,
                animation,
                removing: false,
            },
        );
        state.revision = state.revision.checked_add(1).expect("revision exhausted");
        state.structure_revision = state
            .structure_revision
            .checked_add(1)
            .expect("structure revision exhausted");
        true
    }

    /// Starts shrinking the item at `index`. It remains in the sliver until
    /// the reverse animation reaches zero, so the retained element and drag
    /// state are not destroyed halfway through the transition.
    pub fn remove(&self, index: usize) -> bool {
        self.remove_at(index, Instant::now())
    }

    /// Deterministic-clock form of [`Self::remove`], useful for tests.
    pub fn remove_at(&self, index: usize, now: Instant) -> bool {
        let state = self.state.borrow_mut();
        let Some(entry) = state.entries.get(index) else {
            return false;
        };
        if entry.removing {
            return false;
        }
        entry.animation.reverse(now);
        drop(state);
        let mut state = self.state.borrow_mut();
        if let Some(entry) = state.entries.get_mut(index) {
            entry.removing = true;
            state.revision = state.revision.checked_add(1).expect("revision exhausted");
            state.structure_revision = state
                .structure_revision
                .checked_add(1)
                .expect("structure revision exhausted");
            true
        } else {
            false
        }
    }

    /// Advances all active insert/remove transitions and removes entries that
    /// have completed their reverse animation.
    pub fn tick(&self, now: Instant) -> bool {
        let mut state = self.state.borrow_mut();
        let mut changed = false;
        for entry in &state.entries {
            changed |= entry.animation.tick(now);
        }
        let before = state.entries.len();
        state.entries.retain(|entry| {
            !(entry.removing && !entry.animation.is_active() && entry.animation.value() <= 0.)
        });
        if state.entries.len() != before {
            changed = true;
            state.structure_revision = state
                .structure_revision
                .checked_add(1)
                .expect("structure revision exhausted");
        }
        if changed {
            state.revision = state.revision.checked_add(1).expect("revision exhausted");
        }
        changed
    }

    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.state
            .borrow()
            .entries
            .iter()
            .any(|entry| entry.animation.is_active())
    }

    fn structure_revision(&self) -> u64 {
        self.state.borrow().structure_revision
    }

    fn snapshot_into(&self, out: &mut Vec<AnimatedListEntrySnapshot>) {
        out.clear();
        out.extend(
            self.state
                .borrow()
                .entries
                .iter()
                .map(|entry| AnimatedListEntrySnapshot {
                    id: entry.id,
                    value: entry.animation.value().clamp(0., 1.),
                    removing: entry.removing,
                    incoming: !entry.removing && entry.animation.is_active(),
                }),
        );
    }

    /// The animation driving the entry with stable identity `id`.
    #[must_use]
    pub fn animation_for_id(&self, id: u64) -> Option<AnimationController> {
        self.state
            .borrow()
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.animation.clone())
    }

    /// The animation driving the entry currently at `index`.
    #[must_use]
    pub fn animation_for_index(&self, index: usize) -> Option<AnimationController> {
        self.state
            .borrow()
            .entries
            .get(index)
            .map(|entry| entry.animation.clone())
    }
}

struct AnimatedExtentRenderSliver {
    controller: SliverAnimatedListController,
    builder: SliverAnimatedItemBuilder,
    index: MeasuredExtentIndex,
    entries: Vec<AnimatedListEntrySnapshot>,
    spare_entries: Vec<AnimatedListEntrySnapshot>,
    position_by_id: HashMap<u64, usize>,
    positions: Vec<usize>,
    base_extents: HashMap<u64, f32>,
    widgets: HashMap<u64, (AnimatedItem, Widget)>,
    structure_revision: u64,
    estimated_extent: f32,
}

impl AnimatedExtentRenderSliver {
    fn new(
        controller: SliverAnimatedListController,
        builder: SliverAnimatedItemBuilder,
        estimated_extent: f32,
    ) -> Self {
        let mut entries = Vec::new();
        controller.snapshot_into(&mut entries);
        Self {
            index: MeasuredExtentIndex::new(entries.len(), estimated_extent),
            position_by_id: entries
                .iter()
                .enumerate()
                .map(|(position, entry)| (entry.id, position))
                .collect(),
            controller,
            builder,
            entries,
            spare_entries: Vec::new(),
            positions: Vec::new(),
            base_extents: HashMap::new(),
            widgets: HashMap::new(),
            structure_revision: u64::MAX,
            estimated_extent: estimated_extent.max(1.),
        }
    }

    fn sync_entries(&mut self) {
        let mut next = std::mem::take(&mut self.spare_entries);
        self.controller.snapshot_into(&mut next);
        let structure_revision = self.controller.structure_revision();
        if structure_revision != self.structure_revision {
            self.index = MeasuredExtentIndex::new(next.len(), self.estimated_extent);
            let live: std::collections::HashSet<u64> = next.iter().map(|entry| entry.id).collect();
            self.base_extents.retain(|id, _| live.contains(id));
            self.widgets.retain(|id, _| live.contains(id));
            self.position_by_id.clear();
            self.position_by_id.extend(
                next.iter()
                    .enumerate()
                    .map(|(position, entry)| (entry.id, position)),
            );
            for (position, entry) in next.iter().enumerate() {
                let base = self
                    .base_extents
                    .get(&entry.id)
                    .copied()
                    .unwrap_or(self.estimated_extent);
                let _ = self.index.set_measured_extent(position, base * entry.value);
            }
            self.structure_revision = structure_revision;
        } else {
            for (position, (old, next)) in self.entries.iter().zip(&next).enumerate() {
                if (old.value - next.value).abs() > f32::EPSILON {
                    let base = self
                        .base_extents
                        .get(&next.id)
                        .copied()
                        .unwrap_or(self.estimated_extent);
                    let _ = self.index.set_measured_extent(position, base * next.value);
                }
            }
        }
        self.spare_entries = std::mem::replace(&mut self.entries, next);
    }

    fn child_widget(&mut self, position: usize, entry: AnimatedListEntrySnapshot) -> Widget {
        // Flutter `AnimatedItemBuilder(context, index, animation)`: an
        // animated builder sees the entry's progress, so it is rebuilt when
        // that changes; an index-only builder is built once per entry.
        let item = AnimatedItem {
            id: entry.id,
            index: position,
            value: entry.value,
            phase: if entry.removing {
                AnimatedItemPhase::Outgoing
            } else if entry.incoming {
                AnimatedItemPhase::Incoming
            } else {
                AnimatedItemPhase::Stable
            },
        };
        if let Some((built, widget)) = self.widgets.get(&entry.id)
            && (!self.builder.animated || *built == item)
        {
            return widget.clone();
        }
        let widget = (self.builder.build)(position, item);
        self.widgets.insert(entry.id, (item, widget.clone()));
        widget
    }
}

impl RenderSliver for AnimatedExtentRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        self.sync_entries();
        let cache_start = (constraints.scroll_offset + constraints.cache_origin).max(0.);
        let cache_end = (cache_start + constraints.remaining_cache_extent).max(cache_start);
        let mut positions = std::mem::take(&mut self.positions);
        positions.clear();
        positions.extend(self.index.materialized_range(
            cache_start,
            (cache_end - cache_start).max(0.),
            0.,
        ));
        // A freshly inserted zero-size entry does not intersect an offset
        // interval mathematically, but it must still be retained so its size
        // transition can paint on the next frame.
        positions.extend(
            self.entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.value < 1. || entry.removing)
                .map(|(position, _)| position),
        );
        positions.sort_unstable();
        positions.dedup();
        let children = positions
            .iter()
            .copied()
            .map(|position| {
                let entry = self.entries[position];
                let offset = self.index.offset_for_index(position);
                let extent = (self.index.offset_for_index(position + 1) - offset).max(0.);
                SliverChildLayout {
                    id: SliverChildId::list_item(entry.id as usize),
                    widget: self.child_widget(position, entry),
                    semantic_index: Some(position),
                    offset,
                    cross_offset: 0.,
                    constraints: sliver_child_constraints(
                        constraints.axis,
                        constraints.cross_axis_extent,
                        Some(extent),
                    ),
                    extent,
                    placement: crate::scrolling::SliverChildPlacement::Flow,
                }
            })
            .collect();
        self.positions = positions;
        SliverLayout {
            geometry: SliverGeometry::from_scroll_extent(constraints, self.index.total_extent()),
            children,
            absorbed_overlap: 0.,
        }
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        let Some(id) = child.0.checked_sub(1) else {
            return false;
        };
        let Some(position) = self.position_by_id.get(&id).copied() else {
            return false;
        };
        let entry = self.entries[position];
        if entry.value <= f32::EPSILON || !extent.is_finite() || extent < 0. {
            return false;
        }
        let base = (extent / entry.value).max(0.);
        self.base_extents.insert(id, base);
        self.index.set_measured_extent(position, extent)
    }

    fn revision(&self) -> u64 {
        self.controller
            .revision()
            .checked_add(self.index.revision())
            .expect("animated list composite revision exhausted")
    }

    fn tick(&mut self, now: Instant) -> bool {
        let changed = self.controller.tick(now);
        if changed {
            self.sync_entries();
        }
        changed
    }

    fn is_animating(&self) -> bool {
        self.controller.is_animating()
    }
}

/// Animated list sliver with retained insertion/removal transitions.
/// The item builder of a [`SliverAnimatedList`].
#[derive(Clone)]
struct SliverAnimatedItemBuilder {
    build: AnimatedItemBuilder,
    /// Whether the builder reads the item's animation.
    animated: bool,
}

pub struct SliverAnimatedList {
    builder: SliverAnimatedItemBuilder,
    controller: SliverAnimatedListController,
}

impl SliverAnimatedList {
    #[must_use]
    pub fn new<W>(item_count: usize, builder: impl Fn(usize) -> W + 'static) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            controller: SliverAnimatedListController::new(item_count),
            builder: SliverAnimatedItemBuilder {
                build: Rc::new(move |i, _| builder(i).into()),
                animated: false,
            },
        }
    }

    /// Flutter `SliverAnimatedList(itemBuilder: (context, index, animation))`:
    /// the builder also receives the item's animation snapshot, so it can
    /// fade or slide the item while it is inserted or removed. The item is
    /// rebuilt whenever its animation value or phase changes; use
    /// [`SliverAnimatedListController::animation_for_id`] for the live
    /// controller.
    #[must_use]
    pub fn animated<W>(
        item_count: usize,
        builder: impl Fn(usize, AnimatedItem) -> W + 'static,
    ) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            controller: SliverAnimatedListController::new(item_count),
            builder: SliverAnimatedItemBuilder {
                build: Rc::new(move |i, item| builder(i, item).into()),
                animated: true,
            },
        }
    }

    #[must_use]
    pub fn controller(mut self, controller: SliverAnimatedListController) -> Self {
        self.controller = controller;
        self
    }

    #[must_use]
    pub fn animation_controller(&self) -> SliverAnimatedListController {
        self.controller.clone()
    }
}

impl Sliver for SliverAnimatedList {
    fn build(&self, controller: &ScrollController) -> Widget {
        single_sliver_viewport(
            controller.clone(),
            Axis::Vertical,
            false,
            ScrollPhysics::default(),
            DEFAULT_SLIVER_CACHE_EXTENT,
            Clip::HardEdge,
            self.create_render_sliver(controller, Axis::Vertical, false),
        )
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(AnimatedExtentRenderSliver::new(
            self.controller.clone(),
            self.builder.clone(),
            DEFAULT_LAZY_ITEM_EXTENT,
        ))
    }
}
