use super::sliver_descriptors::CrossAxisGroupRenderSliver;
use super::sliver_fade::AnimatedOpacityRenderSliver;
use super::*;
use incular_core::finite_non_negative;
use std::any::Any;

pub(super) struct BoxRenderSliver {
    pub(super) child: Widget,
    pub(super) extent: Cell<f32>,
    pub(super) pinned: bool,
}

/// Geometry shared by the pinned persistent-header variants.
///
/// A pinned header has a normal scroll extent, but its paint extent continues
/// at the leading edge after its layout extent has collapsed.  Keeping that
/// distinction in the sliver geometry is what lets the viewport compute the
/// same obstruction/overlap values as Flutter's persistent-header render
/// objects instead of relying only on a post-layout position adjustment.
fn pinned_geometry(
    constraints: SliverConstraints,
    scroll_extent: f32,
    child_extent: f32,
    min_extent: f32,
) -> SliverGeometry {
    let scroll_extent = scroll_extent.max(0.);
    let child_extent = child_extent.max(0.);
    let effective_remaining_paint_extent =
        (constraints.remaining_paint_extent - constraints.overlap).max(0.);
    let paint_extent = child_extent.min(effective_remaining_paint_extent);
    let layout_extent =
        (scroll_extent - constraints.scroll_offset).clamp(0., effective_remaining_paint_extent);
    let cache_extent = if layout_extent > 0. {
        (-constraints.cache_origin + layout_extent).max(0.)
    } else {
        layout_extent
    };
    SliverGeometry {
        scroll_extent,
        paint_extent,
        layout_extent,
        max_paint_extent: scroll_extent,
        max_scroll_obstruction_extent: finite_non_negative(min_extent),
        hit_test_extent: paint_extent,
        paint_origin: constraints.overlap,
        cache_extent,
        visible: paint_extent > 0.,
        has_visual_overflow: true,
        scroll_offset_correction: None,
    }
}

/// Flutter's `RenderSliver.calculateCacheOffset`: the part of `[from, to]`
/// that lies inside the cache window, clamped to the remaining cache extent.
fn calculate_cache_offset(constraints: SliverConstraints, from: f32, to: f32) -> f32 {
    let start = constraints.scroll_offset + constraints.cache_origin;
    let end = constraints.scroll_offset + constraints.remaining_cache_extent;
    let clamp = |value: f32| value.max(start).min(end);
    (clamp(to) - clamp(from))
        .max(0.)
        .min(constraints.remaining_cache_extent)
}

impl BoxRenderSliver {
    pub(super) fn new(child: Widget) -> Self {
        Self {
            child,
            extent: Cell::new(DEFAULT_LAZY_ITEM_EXTENT),
            pinned: false,
        }
    }

    pub(super) fn pinned(child: Widget) -> Self {
        Self {
            pinned: true,
            ..Self::new(child)
        }
    }
}

impl RenderSliver for BoxRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let extent = self.extent.get().max(0.);
        let geometry = if self.pinned {
            // Flutter's PinnedHeaderSliver reports cacheExtent from the
            // child extent, not from the pinned persistent-header formula.
            let mut geometry = pinned_geometry(constraints, extent, extent, extent);
            geometry.cache_extent = calculate_cache_offset(constraints, 0., extent);
            geometry
        } else {
            SliverGeometry::from_scroll_extent(constraints, extent)
        };
        SliverLayout {
            geometry,
            children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: self.child.clone().with_sliver_scroll_exclusion(
                    self.pinned && geometry.layout_extent < extent,
                ),
                semantic_index: None,
                offset: 0.,
                cross_offset: 0.,
                constraints: sliver_child_constraints(
                    constraints.axis,
                    constraints.cross_axis_extent,
                    None,
                ),
                extent,
                placement: if self.pinned {
                    SliverChildPlacement::Pinned
                } else {
                    SliverChildPlacement::Flow
                },
            }],
            absorbed_overlap: (geometry.paint_extent - geometry.layout_extent).max(0.),
        }
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        if child.0 != 0 || !extent.is_finite() || extent < 0. {
            return false;
        }
        let extent = extent.max(0.);
        if (self.extent.get() - extent).abs() <= f32::EPSILON {
            return false;
        }
        self.extent.set(extent);
        true
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

/// Flutter snaps boundary indices when the reconstructed coordinate only
/// differs by float rounding. Layout coordinates here are f32, so use
/// one representable coordinate step rather than Dart's double epsilon.
fn fixed_extent_boundary(offset: f32, item_extent: f32) -> f64 {
    let extent = f64::from(item_extent);
    let coordinate = f64::from(offset);
    let ratio = coordinate / extent;
    let nearest = ratio.round();
    let tolerance = f64::from(offset.next_up()) - coordinate;
    if (coordinate - nearest * extent).abs() <= tolerance {
        nearest
    } else {
        ratio
    }
}

/// Exclusive materialization interval for a fixed-size child sequence.
fn fixed_extent_cache_range(
    cache_start: f32,
    cache_end: f32,
    item_extent: f32,
    count: usize,
) -> std::ops::Range<usize> {
    let (start, end) = if item_extent > 0. {
        (
            fixed_extent_boundary(cache_start, item_extent).floor() as usize,
            fixed_extent_boundary(cache_end, item_extent).ceil() as usize,
        )
    } else {
        // Layout the first zero-sized child without scanning the other
        // children that all share that same offset.
        (0, 1)
    };
    let start = start.min(count);
    start..end.min(count).max(start)
}

fn set_cached_keep_alive(
    widgets: &mut HashMap<usize, Widget>,
    index: usize,
    keep_alive: bool,
) -> bool {
    let Some(widget) = widgets.get_mut(&index) else {
        return false;
    };
    if widget.explicit_keep_alive_request() == keep_alive {
        return false;
    }
    *widget = widget.clone().with_keep_alive_request(keep_alive);
    true
}

fn set_cached_focus_keep_alive(
    widgets: &mut HashMap<usize, Widget>,
    index: usize,
    keep_alive: bool,
) -> bool {
    let Some(widget) = widgets.get_mut(&index) else {
        return false;
    };
    if widget.focus_keep_alive_request() == keep_alive {
        return false;
    }
    *widget = widget.clone().with_focus_keep_alive_request(keep_alive);
    true
}

/// Separates the live cache range from explicitly retained children. The
/// ordinary case allocates no parking buffer.
fn retain_fixed_cache(
    widgets: &mut HashMap<usize, Widget>,
    range: &std::ops::Range<usize>,
    count: usize,
) -> Vec<usize> {
    let mut parked = Vec::new();
    widgets.retain(|index, widget| {
        if range.contains(index) {
            true
        } else if *index < count && widget.keep_alive_request() {
            parked.push(*index);
            true
        } else {
            false
        }
    });
    parked.sort_unstable();
    parked
}

fn fixed_cache_slots(
    range: std::ops::Range<usize>,
    parked: &[usize],
) -> impl Iterator<Item = (usize, SliverChildPlacement)> + '_ {
    let split = parked.partition_point(|index| *index < range.start);
    // Preserve logical order on both sides of the active range.
    parked[..split]
        .iter()
        .map(|index| (*index, SliverChildPlacement::Parked))
        .chain(range.map(|index| (index, SliverChildPlacement::Flow)))
        .chain(
            parked[split..]
                .iter()
                .map(|index| (*index, SliverChildPlacement::Parked)),
        )
}

pub(super) struct FixedExtentRenderSliver {
    pub(super) item_count: usize,
    pub(super) item_extent: f32,
    pub(super) builder: Rc<dyn Fn(usize) -> Widget>,
    pub(super) widgets: HashMap<usize, Widget>,
    pub(super) find_child_index: Option<ListIndexLookup>,
}

impl FixedExtentRenderSliver {
    pub(super) fn new(
        item_count: usize,
        item_extent: f32,
        builder: Rc<dyn Fn(usize) -> Widget>,
    ) -> Self {
        Self {
            item_count,
            item_extent: finite_non_negative(item_extent),
            builder,
            widgets: HashMap::new(),
            find_child_index: None,
        }
    }

    fn child_widget(&mut self, index: usize) -> Widget {
        self.widgets
            .entry(index)
            .or_insert_with(|| (self.builder)(index))
            .clone()
    }
}

impl RenderSliver for FixedExtentRenderSliver {
    fn set_child_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        grid_tile.is_none()
            && child
                .item_index()
                .is_some_and(|index| set_cached_keep_alive(&mut self.widgets, index, keep_alive))
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        grid_tile.is_none()
            && child.item_index().is_some_and(|index| {
                set_cached_focus_keep_alive(&mut self.widgets, index, keep_alive)
            })
    }

    fn child_count(&self) -> Option<usize> {
        Some(self.item_count)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let total = self.item_count as f32 * self.item_extent;
        let cache_start = (constraints.scroll_offset + constraints.cache_origin).max(0.);
        let cache_end = (cache_start + constraints.remaining_cache_extent).max(cache_start);
        let range =
            fixed_extent_cache_range(cache_start, cache_end, self.item_extent, self.item_count);
        let parked = retain_fixed_cache(&mut self.widgets, &range, self.item_count);
        let children = fixed_cache_slots(range, &parked)
            .map(|(index, placement)| {
                let widget = self.child_widget(index);
                SliverChildLayout {
                    id: SliverChildId::list_item(index),
                    widget,
                    semantic_index: Some(index),
                    offset: index as f32 * self.item_extent,
                    cross_offset: 0.,
                    constraints: sliver_child_constraints(
                        constraints.axis,
                        constraints.cross_axis_extent,
                        Some(self.item_extent),
                    ),
                    extent: self.item_extent,
                    placement,
                }
            })
            .collect();
        SliverLayout {
            geometry: SliverGeometry::from_scroll_extent(constraints, total),
            children,
            absorbed_overlap: 0.,
        }
    }
}

/// Retained grid layout driven by a [`SliverGridDelegate`]. The grid only
/// materializes rows/columns intersecting the paint and cache intervals, and
/// recomputes tile dimensions from the current sliver constraints.
pub(super) struct GridRenderSliver {
    pub(super) item_count: usize,
    pub(super) delegate: SliverGridDelegate,
    pub(super) builder: Rc<dyn Fn(usize) -> Widget>,
    pub(super) axis: Axis,
    pub(super) widgets: HashMap<usize, Widget>,
}

impl GridRenderSliver {
    pub(super) fn new(
        item_count: usize,
        delegate: SliverGridDelegate,
        builder: Rc<dyn Fn(usize) -> Widget>,
        axis: Axis,
    ) -> Self {
        Self {
            item_count,
            delegate,
            builder,
            axis,
            widgets: HashMap::new(),
        }
    }

    fn child_widget(&mut self, index: usize) -> Widget {
        self.widgets
            .entry(index)
            .or_insert_with(|| (self.builder)(index))
            .clone()
    }
}

impl RenderSliver for GridRenderSliver {
    fn set_child_keep_alive(
        &mut self,
        _child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        grid_tile.is_some_and(|index| set_cached_keep_alive(&mut self.widgets, index, keep_alive))
    }

    fn set_child_focus_keep_alive(
        &mut self,
        _child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        grid_tile
            .is_some_and(|index| set_cached_focus_keep_alive(&mut self.widgets, index, keep_alive))
    }

    fn child_count(&self) -> Option<usize> {
        Some(self.item_count)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let cross_extent = constraints.cross_axis_extent.max(0.);
        let columns = self.delegate.cross_axis_count(cross_extent).max(1);
        let main_extent = self
            .delegate
            .resolve_main_axis_extent(cross_extent, columns);
        let main_spacing = self.delegate.resolved_main_axis_spacing();
        let cross_spacing = self.delegate.resolved_cross_axis_spacing();
        let rows = self.item_count.div_ceil(columns);
        let row_step = main_extent + main_spacing;
        let total = if rows == 0 {
            0.
        } else {
            rows as f32 * main_extent + rows.saturating_sub(1) as f32 * main_spacing
        };
        let cache_start = (constraints.scroll_offset + constraints.cache_origin).max(0.);
        let cache_end = (cache_start + constraints.remaining_cache_extent).max(cache_start);
        // Flutter's regular tile layout reports the first row for a zero stride
        // instead of dividing by it.
        let (start_row, end_row) = if row_step > 0. {
            (
                (cache_start / row_step).floor() as usize,
                (cache_end / row_step).ceil() as usize,
            )
        } else {
            (0, 1)
        };
        let row_range = start_row.min(rows)..end_row.min(rows).max(start_row.min(rows));
        let first_item = row_range.start.saturating_mul(columns).min(self.item_count);
        let last_item = row_range.end.saturating_mul(columns).min(self.item_count);
        let parked_tiles =
            retain_fixed_cache(&mut self.widgets, &(first_item..last_item), self.item_count);
        let mut parked_rows: Vec<_> = parked_tiles.iter().map(|index| index / columns).collect();
        parked_rows.dedup();

        let cell_cross_extent = if columns == 0 {
            0.
        } else {
            (cross_extent - cross_spacing * columns.saturating_sub(1) as f32).max(0.)
                / columns as f32
        };
        let axis = self.axis;
        let children = fixed_cache_slots(row_range, &parked_rows)
            .map(|(row, placement)| {
                let start = row * columns;
                let end = (start + columns).min(self.item_count);
                let mut keep_row = false;
                let cells: Vec<_> = (start..end)
                    .filter_map(|index| {
                        if placement == SliverChildPlacement::Parked
                            && !self.widgets.contains_key(&index)
                        {
                            return None;
                        }
                        let child = self.child_widget(index);
                        keep_row |= child.keep_alive_request();
                        let (width, height) = if axis == Axis::Vertical {
                            (cell_cross_extent, main_extent)
                        } else {
                            (main_extent, cell_cross_extent)
                        };
                        // The selected tile keeps its child slot even when earlier
                        // siblings leave the row while it is parked.
                        Some(
                            Widget::from(SizedBox::new().width(width).height(height).child(child))
                                .with_key(crate::tree::Key::GridCell(index)),
                        )
                    })
                    .collect();
                let widget = if axis == Axis::Vertical {
                    Widget::from(Row::new(cells).spacing(cross_spacing))
                } else {
                    Widget::from(Column::new(cells).spacing(cross_spacing))
                }
                .with_keep_alive_request(keep_row);
                SliverChildLayout {
                    id: SliverChildId::list_item(row),
                    widget,
                    semantic_index: Some(row),
                    offset: row as f32 * row_step,
                    cross_offset: 0.,
                    constraints: sliver_child_constraints(
                        constraints.axis,
                        constraints.cross_axis_extent,
                        Some(main_extent),
                    ),
                    extent: main_extent,
                    placement,
                }
            })
            .collect();

        let mut geometry = SliverGeometry::from_scroll_extent(constraints, total);
        geometry.has_visual_overflow |= constraints.overlap != 0.;
        SliverLayout {
            geometry,
            children,
            absorbed_overlap: 0.,
        }
    }
}

pub(super) struct VariableExtentRenderSliver {
    pub(super) index: MeasuredExtentIndex,
    pub(super) builder: Rc<dyn Fn(usize) -> Widget>,
    pub(super) widgets: HashMap<usize, Widget>,
    pub(super) find_child_index: Option<ListIndexLookup>,
    /// Slots the last compatible-state transfer wrote without
    /// established identity. Reconciliation reads this set to tell
    /// transferred guesses apart from explicit seeds; entries belong
    /// to this delegate generation only.
    pub(super) transferred: Rc<RefCell<HashSet<usize>>>,
    /// Lays each row out at exactly its indexed extent on the main axis, as
    /// Flutter's `SliverVariedExtentList` does. Plain list rows keep
    /// their intrinsic main-axis size.
    pub(super) tight_main_extent: bool,
    /// The Flutter SliverList descriptor publishes an estimate derived from
    /// this layout's child sample. Explicit Incular ListView extent indexes
    /// retain their own estimates and reconciliation behavior.
    pub(super) sample_scroll_extent: bool,
    last_sampled_extent: Option<f64>,
    /// Scratch list of kept-alive rows outside the window, reused per layout.
    kept: Vec<usize>,
    /// First child from the last materialized range, not a kept-alive child
    /// parked outside that range.
    leading_anchor: Option<VariableLeadingAnchor>,
    /// An overlapping range has reached the beginning again. Wait for the
    /// newly inserted leading children to be measured before comparing
    /// their total extent with the old anchor.
    reentered_leading_range: Option<ReenteredLeadingRange>,
}

#[derive(Clone)]
struct VariableLeadingAnchor {
    index: usize,
    offset: f32,
    key: Option<crate::tree::Key>,
    kind: std::mem::Discriminant<crate::tree::WidgetKind>,
}

struct ReenteredLeadingRange {
    anchor: VariableLeadingAnchor,
    measured: bool,
}

impl VariableExtentRenderSliver {
    pub(super) fn new(index: MeasuredExtentIndex, builder: Rc<dyn Fn(usize) -> Widget>) -> Self {
        Self {
            index,
            builder,
            widgets: HashMap::new(),
            find_child_index: None,
            transferred: Rc::new(RefCell::new(HashSet::new())),
            tight_main_extent: false,
            sample_scroll_extent: false,
            last_sampled_extent: None,
            kept: Vec::new(),
            leading_anchor: None,
            reentered_leading_range: None,
        }
    }

    fn child_widget(&mut self, index: usize) -> Widget {
        self.widgets
            .entry(index)
            .or_insert_with(|| (self.builder)(index))
            .clone()
    }

    fn variable_child_layout(
        &mut self,
        index: usize,
        start: f32,
        end: f32,
        constraints: SliverConstraints,
    ) -> SliverChildLayout {
        let extent = end - start;
        let widget = self.child_widget(index);
        SliverChildLayout {
            id: SliverChildId::list_item(index),
            widget,
            semantic_index: Some(index),
            offset: start,
            cross_offset: 0.,
            // Varied-extent rows take their indexed extent on the main axis;
            // plain list rows keep their intrinsic size.
            constraints: sliver_child_constraints(
                constraints.axis,
                constraints.cross_axis_extent,
                self.tight_main_extent.then_some(extent),
            ),
            extent,
            placement: SliverChildPlacement::Flow,
        }
    }

    /// Flutter's RenderSliverList lays backwards from its first retained
    /// child. If inserting leading rows would push index zero before
    /// logical zero, it asks the viewport to correct by that distance.
    /// The extent index gives the same distance without walking all rows.
    fn leading_offset_correction(
        &mut self,
        range: &std::ops::Range<usize>,
        scroll_offset: f32,
    ) -> Option<f32> {
        if self.tight_main_extent || range.start != 0 || scroll_offset > f32::EPSILON {
            self.reentered_leading_range = None;
            return None;
        }
        if self
            .reentered_leading_range
            .as_ref()
            .is_some_and(|probe| probe.measured)
        {
            let probe = self
                .reentered_leading_range
                .take()
                .expect("measured leading range");
            if range.contains(&probe.anchor.index) {
                let delta = self.index.offset_for_index(probe.anchor.index) - probe.anchor.offset;
                if delta.is_finite() && delta > f32::EPSILON {
                    return Some(delta);
                }
            }
            return None;
        }
        if self.reentered_leading_range.is_none()
            && let Some(anchor) = self
                .leading_anchor
                .as_ref()
                .filter(|anchor| anchor.index != 0 && range.contains(&anchor.index))
                .cloned()
        {
            let current = self.child_widget(anchor.index);
            if anchor.key.as_ref() == current.key()
                && anchor.kind == std::mem::discriminant(current.kind())
            {
                self.reentered_leading_range = Some(ReenteredLeadingRange {
                    anchor,
                    measured: false,
                });
            }
        }
        None
    }
}

impl RenderSliver for VariableExtentRenderSliver {
    fn set_child_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        grid_tile.is_none()
            && child
                .item_index()
                .is_some_and(|index| set_cached_keep_alive(&mut self.widgets, index, keep_alive))
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        grid_tile.is_none()
            && child.item_index().is_some_and(|index| {
                set_cached_focus_keep_alive(&mut self.widgets, index, keep_alive)
            })
    }

    fn child_count(&self) -> Option<usize> {
        Some(self.index.len())
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let cache_start = (constraints.scroll_offset + constraints.cache_origin).max(0.);
        let cache_end = (cache_start + constraints.remaining_cache_extent).max(cache_start);
        let range =
            self.index
                .flutter_child_range(cache_start, (cache_end - cache_start).max(0.), 0.);
        if let Some(correction) = self.leading_offset_correction(&range, constraints.scroll_offset)
        {
            return SliverLayout::empty(SliverGeometry {
                scroll_offset_correction: Some(correction),
                ..SliverGeometry::ZERO
            });
        }
        // A row whose widget asks to be kept alive (Flutter's `KeepAlive`
        // parent data) stays materialized outside the window, so its state
        // survives scrolling away. Every other row leaves with the window.
        let count = self.index.len();
        self.widgets.retain(|index, widget| {
            *index < count && (range.contains(index) || widget.keep_alive_request())
        });
        let mut kept = std::mem::take(&mut self.kept);
        kept.clear();
        kept.extend(
            self.widgets
                .keys()
                .copied()
                .filter(|index| !range.contains(index)),
        );
        kept.sort_unstable();
        let split = kept.partition_point(|index| *index < range.start);
        let mut children = Vec::with_capacity(kept.len() + range.len());
        for &index in &kept[..split] {
            let edges = self.index.offsets_for_range(index..index + 1);
            let mut child = self.variable_child_layout(index, edges[0], edges[1], constraints);
            child.placement = SliverChildPlacement::Parked;
            children.push(child);
        }
        // Both edges of every materialized row, from one walk of the index.
        let offsets = self.index.offsets_for_range(range.clone());
        let first = range.start;
        if !range.is_empty() {
            let offset = offsets[0];
            if let Some(anchor) = &mut self.leading_anchor
                && anchor.index == first
            {
                anchor.offset = offset;
            } else {
                let widget = self.child_widget(first);
                self.leading_anchor = Some(VariableLeadingAnchor {
                    index: first,
                    offset,
                    key: widget.key().cloned(),
                    kind: std::mem::discriminant(widget.kind()),
                });
            }
        } else {
            self.leading_anchor = None;
        }
        let scroll_extent = if self.sample_scroll_extent && !range.is_empty() {
            // The viewport may publish this pass before its children have
            // been measured. Use the measured sample for its projection,
            // not the 48 px lookup seed for provisional rows. Otherwise
            // jumping to the last page can clamp to that provisional end
            // before the actual last rows have a chance to be measured.
            let transferred = self.transferred.borrow();
            let (sum, measured) = range
                .clone()
                .filter(|index| !transferred.contains(index))
                .filter_map(|index| self.index.measured_extent(index))
                .fold((0., 0usize), |(sum, count), extent| {
                    (sum + f64::from(extent), count + 1)
                });
            let sample = if measured != 0 {
                let average = sum / measured as f64;
                self.last_sampled_extent = Some(average);
                average
            } else {
                self.last_sampled_extent
                    .unwrap_or_else(|| f64::from(self.index.estimated_extent()))
            };
            let estimate =
                f64::from(offsets[0]) + sum + sample * (count - range.start - measured) as f64;
            estimate.min(f64::from(f32::MAX)) as f32
        } else if constraints.growth_direction == SliverGrowthDirection::Reverse
            && !self.tight_main_extent
            && !range.is_empty()
            && range.end < count
        {
            // For reverse-growth lists that use an Incular extent index,
            // preserve the current layout sample as the scroll estimate.
            let leading = f64::from(offsets[0]);
            let trailing = f64::from(offsets[range.len()]);
            (trailing + (trailing - leading) / range.len() as f64 * (count - range.end) as f64)
                as f32
        } else {
            self.index.total_extent()
        };
        for (index, edges) in range.zip(offsets.windows(2)) {
            children.push(self.variable_child_layout(index, edges[0], edges[1], constraints));
        }
        for &index in &kept[split..] {
            let edges = self.index.offsets_for_range(index..index + 1);
            let mut child = self.variable_child_layout(index, edges[0], edges[1], constraints);
            child.placement = SliverChildPlacement::Parked;
            children.push(child);
        }
        self.kept = kept;
        SliverLayout {
            geometry: SliverGeometry::from_scroll_extent(constraints, scroll_extent),
            children,
            absorbed_overlap: 0.,
        }
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        let Some(index) = child
            .0
            .checked_sub(1)
            .and_then(|index| usize::try_from(index).ok())
        else {
            return false;
        };
        if !extent.is_finite() || extent < 0. || index >= self.index.len() {
            return false;
        }
        // A valid child measurement consumes provisional provenance even when
        // its numeric value is unchanged, so later reconciliation cannot
        // overwrite established truth.
        self.transferred.borrow_mut().remove(&index);
        let changed = self.index.set_measured_extent(index, extent);
        // Even unchanged measurements get one retry to settle and retire
        // an outstanding leading probe; otherwise it could survive the
        // frame and react to an unrelated future edit of index zero.
        let first_probe_measurement = self.reentered_leading_range.as_mut().is_some_and(|probe| {
            if index < probe.anchor.index && !probe.measured {
                probe.measured = true;
                true
            } else {
                false
            }
        });
        changed || first_probe_measurement
    }

    fn revision(&self) -> u64 {
        self.index.revision()
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

pub(super) struct HeaderRenderSliver {
    pub(super) child: Widget,
    pub(super) extent: f32,
    pub(super) pinned: bool,
}

impl HeaderRenderSliver {
    pub(super) fn new(child: Widget, extent: f32, pinned: bool) -> Self {
        Self {
            child,
            extent: extent.max(0.),
            pinned,
        }
    }
}

/// Retained `SliverFloatingHeader` state, following Flutter's
/// `_RenderSliverFloatingHeader` (widgets/sliver_floating_header.dart). The
/// header reveals only while the user scrolls toward the leading edge
/// (`ScrollDirection.forward`); programmatic and idle moves only hide it.
/// When `snap` is set, the end of a scroll activity animates the effective
/// offset toward revealed (forward) or hidden (otherwise) presentation.
pub(super) struct FloatingHeaderRenderSliver {
    pub(super) child: Widget,
    pub(super) extent: Cell<f32>,
    pub(super) scroll_state: FloatingHeaderState,
    pub(super) snap: bool,
    /// Flutter's `snapMode`: whether the layout extent follows the visible
    /// paint extent while the header snaps (`Scroll`) or stays at its full
    /// scroll-driven extent (`Overlay`).
    pub(super) snap_mode: FloatingHeaderSnapMode,
    pub(super) snap_duration: Duration,
    pub(super) snap_curve: Curve,
    /// Last reduced-motion policy stamped by the owning delegate. The tree
    /// pushes the retained environment value before every tick, so fresh
    /// render slivers observe the current policy on their first decision
    /// without any rebuild or replacement.
    pub(super) reduced_motion: Cell<bool>,
    /// Scroll activity and user direction observed on the controller.
    pub(super) signal: Rc<Cell<FloatingScrollSignal>>,
    /// Owns the controller listener. Never read: dropping the subscription
    /// unsubscribes, so disposal stops signals without further cleanup.
    pub(super) _subscription: ScrollNotificationSubscription,
}

impl FloatingHeaderRenderSliver {
    /// Creates the render sliver and subscribes it to the controller's
    /// scroll activity, which drives both the reveal direction and the snap.
    pub(super) fn new(
        controller: &ScrollController,
        child: Widget,
        extent: f32,
        snap: bool,
        snap_mode: FloatingHeaderSnapMode,
        snap_duration: Duration,
        snap_curve: Curve,
    ) -> Self {
        let (signal, subscription) = floating_scroll_subscription(controller);
        Self {
            child,
            extent: Cell::new(extent),
            scroll_state: FloatingHeaderState::default(),
            snap,
            snap_mode,
            snap_duration,
            snap_curve,
            reduced_motion: Cell::new(false),
            signal,
            _subscription: subscription,
        }
    }

    /// Consumes an unconsumed activity transition. Flutter makes the snap
    /// decision in the `isScrollingNotifier` listener, before the next
    /// layout, so it sees the effective offset of the previous layout.
    fn consume_activity(&mut self) -> bool {
        let mut signal = self.signal.get();
        let pending = std::mem::take(&mut signal.pending);
        self.signal.set(signal);
        match pending {
            HeaderSnapActivity::None => false,
            HeaderSnapActivity::Started => {
                // `snapController?.stop()`: presentation stays where it is.
                self.scroll_state.snap = FloatingSnap::Idle;
                false
            }
            HeaderSnapActivity::Ended => {
                if !self.snap {
                    return false;
                }
                self.scroll_state.begin_snap(
                    signal.ended_direction,
                    self.extent.get().max(0.),
                    self.snap_duration,
                    self.snap_curve.clone(),
                    self.reduced_motion.get(),
                )
            }
        }
    }
}

/// Flutter's `ScrollDirection` as a scroll position reports it to slivers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum UserScrollDirection {
    /// No user scroll is in progress.
    #[default]
    Idle,
    /// The user moves toward the leading edge (offset decreasing).
    Forward,
    /// The user moves toward the trailing edge (offset increasing).
    Reverse,
}

impl UserScrollDirection {
    fn of_delta(delta: f32) -> Option<Self> {
        if delta < 0. {
            Some(Self::Forward)
        } else if delta > 0. {
            Some(Self::Reverse)
        } else {
            None
        }
    }
}

/// What a floating header has observed of its controller. Flutter's
/// position resets `userScrollDirection` to idle whenever its activity stops
/// scrolling, so the direction a layout sees is idle outside an activity;
/// the direction at the end of an activity decides the snap.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FloatingScrollSignal {
    pending: HeaderSnapActivity,
    active: bool,
    direction: UserScrollDirection,
    /// A user-scroll sample set the direction during this activity.
    explicit: bool,
    /// Inside the notifications of a `jump_to`.
    jumping: bool,
    ended_direction: UserScrollDirection,
}

impl FloatingScrollSignal {
    fn layout_direction(self) -> UserScrollDirection {
        if self.active {
            self.direction
        } else {
            UserScrollDirection::Idle
        }
    }
}

fn floating_scroll_subscription(
    controller: &ScrollController,
) -> (
    Rc<Cell<FloatingScrollSignal>>,
    ScrollNotificationSubscription,
) {
    let signal = Rc::new(Cell::new(FloatingScrollSignal::default()));
    let subscription = controller.add_notification_listener({
        let signal = signal.clone();
        // The listener lives in the controller and the subscription removes
        // it, so this clone never outlives the render sliver.
        let observed = controller.clone();
        move |notification| {
            let mut state = signal.get();
            match notification.kind {
                // A `jump_to` reports Start and End without opening an
                // activity. Flutter's `jumpTo` leaves `isScrollingNotifier`
                // untouched and runs with an idle direction, so the header
                // neither snaps nor reveals for it.
                ScrollNotificationType::Start if observed.current_activity_id().is_none() => {
                    state.jumping = true;
                }
                ScrollNotificationType::End if state.jumping => {
                    state.jumping = false;
                }
                ScrollNotificationType::Start => {
                    state.active = true;
                    state.direction = UserScrollDirection::Idle;
                    state.explicit = false;
                    state.pending = HeaderSnapActivity::Started;
                }
                ScrollNotificationType::UserScroll => {
                    if let Some(direction) = UserScrollDirection::of_delta(notification.delta) {
                        state.direction = direction;
                        state.explicit = true;
                    }
                }
                // Drags report no user-scroll samples; their updates carry
                // the user's direction (Flutter `applyUserOffset`).
                ScrollNotificationType::Update if state.active && !state.explicit => {
                    if let Some(direction) = UserScrollDirection::of_delta(notification.delta) {
                        state.direction = direction;
                    }
                }
                ScrollNotificationType::End => {
                    state.active = false;
                    state.ended_direction = state.direction;
                    state.direction = UserScrollDirection::Idle;
                    state.pending = HeaderSnapActivity::Ended;
                }
                _ => {}
            }
            signal.set(state);
            false
        }
    });
    (signal, subscription)
}

/// Flutter `_RenderSliverFloatingHeader` presentation state.
#[derive(Clone, Debug, Default)]
pub(super) struct FloatingHeaderState {
    last_scroll_offset: Option<f32>,
    /// Distance from the header's start to the viewport's start: zero when
    /// fully revealed, the extent (or more) when hidden.
    effective_scroll_offset: f32,
    snap: FloatingSnap,
}

#[derive(Clone, Debug, Default, PartialEq)]
enum FloatingSnap {
    #[default]
    Idle,
    /// The ticker starts on the first tick after the decision, as an
    /// `AnimationController.forward` does.
    Running {
        from: f32,
        target: f32,
        started_at: Option<Instant>,
        duration: Duration,
        curve: Curve,
    },
    /// The endpoint was applied; one presenting frame stays scheduled.
    Settling,
}

impl FloatingHeaderState {
    /// Drops a running snap, keeping the current presentation.
    pub(super) fn cancel_snap(&mut self) {
        self.snap = FloatingSnap::Idle;
    }

    fn is_snapping(&self) -> bool {
        !matches!(self.snap, FloatingSnap::Idle)
    }

    /// Flutter `isScrollingUpdate` when scrolling stops.
    fn begin_snap(
        &mut self,
        direction: UserScrollDirection,
        extent: f32,
        duration: Duration,
        curve: Curve,
        reduced_motion: bool,
    ) -> bool {
        let effective = self.effective_scroll_offset;
        let partially_visible = match direction {
            UserScrollDirection::Forward => effective > 0.,
            UserScrollDirection::Reverse => effective < extent,
            UserScrollDirection::Idle => true,
        };
        if !partially_visible {
            return false;
        }
        let target = if direction == UserScrollDirection::Forward {
            0.
        } else {
            extent
        };
        if reduced_motion {
            let changed = target != effective;
            self.effective_scroll_offset = target;
            self.snap = FloatingSnap::Settling;
            return changed;
        }
        self.snap = FloatingSnap::Running {
            from: effective,
            target,
            started_at: None,
            duration,
            curve,
        };
        false
    }

    /// Advances a running snap; the animation value replaces the effective
    /// offset, as the snap controller's listener does.
    fn tick(&mut self, now: Instant) -> bool {
        match &mut self.snap {
            FloatingSnap::Idle => false,
            FloatingSnap::Settling => {
                self.snap = FloatingSnap::Idle;
                false
            }
            FloatingSnap::Running {
                from,
                target,
                started_at,
                duration,
                curve,
            } => {
                let started = *started_at.get_or_insert(now);
                let elapsed = now.saturating_duration_since(started).as_secs_f32();
                let total = duration.as_secs_f32();
                let progress = if total > 0. {
                    (elapsed / total).clamp(0., 1.)
                } else {
                    1.
                };
                let value = if progress >= 1. {
                    *target
                } else {
                    f32::interpolate(*from, *target, curve.apply(progress))
                };
                if progress >= 1. {
                    self.snap = FloatingSnap::Settling;
                }
                let changed = value != self.effective_scroll_offset;
                self.effective_scroll_offset = value;
                changed
            }
        }
    }

    /// Flutter `_RenderSliverFloatingHeader.performLayout` effective offset.
    fn update(&mut self, scroll_offset: f32, extent: f32, direction: UserScrollDirection) -> f32 {
        let scroll_offset = scroll_offset.max(0.);
        let needs_update = self
            .last_scroll_offset
            .is_some_and(|last| scroll_offset < last || self.effective_scroll_offset < extent);
        if needs_update {
            let last = self.last_scroll_offset.unwrap_or(scroll_offset);
            // Positive while the header grows.
            let mut delta = last - scroll_offset;
            if direction == UserScrollDirection::Forward {
                if self.effective_scroll_offset > extent {
                    self.effective_scroll_offset = extent;
                }
            } else {
                delta = delta.min(0.);
            }
            self.effective_scroll_offset =
                (self.effective_scroll_offset - delta).clamp(0., scroll_offset);
        } else {
            self.effective_scroll_offset = scroll_offset;
        }
        self.last_scroll_offset = Some(scroll_offset);
        self.effective_scroll_offset
    }
}

#[derive(Clone, Default)]
pub(super) struct HeaderScrollState {
    last_scroll_offset: Option<f32>,
    effective_scroll_offset: f32,
    snap: HeaderSnapProgress,
}

impl HeaderScrollState {
    fn update(&mut self, scroll_offset: f32, extent: f32) -> f32 {
        self.update_with_floating_expansion(scroll_offset, extent, true)
    }

    fn update_with_floating_expansion(
        &mut self,
        scroll_offset: f32,
        extent: f32,
        allow_floating_expansion: bool,
    ) -> f32 {
        if let Some(previous) = self.last_scroll_offset {
            if scroll_offset != previous {
                // New scroll movement interrupts a running snap; the
                // animation's current presentation becomes the baseline the
                // delta continues from, so interruption never jumps.
                self.snap = HeaderSnapProgress::Idle;
            }
            // Hidden distance stops at the header extent. Accumulating the
            // rest of the document would delay revealing it on reversal.
            let delta = scroll_offset - previous;
            let delta = if allow_floating_expansion {
                delta
            } else {
                delta.max(0.)
            };
            self.effective_scroll_offset = (self.effective_scroll_offset + delta)
                .clamp(0., extent)
                .min(scroll_offset);
        } else {
            self.effective_scroll_offset = scroll_offset.min(extent);
        }
        self.last_scroll_offset = Some(scroll_offset);
        self.effective_scroll_offset
    }

    /// Drops a running snap without touching the effective offset, so
    /// presentation falls back to whatever the scroll offset determines.
    pub(super) fn cancel_snap(&mut self) {
        self.snap = HeaderSnapProgress::Idle;
    }

    /// Applies the running snap's own target immediately and schedules the
    /// single presenting frame through the shared completion path. Used when
    /// reduced motion is enabled mid-flight; the endpoint selection is
    /// unchanged, only the interpolation is skipped.
    fn resolve_snap_immediately(&mut self, range: f32) -> bool {
        let HeaderSnapProgress::Running(run) = &self.snap else {
            return false;
        };
        let target = run.target.clamp(0., range);
        let changed = target != self.effective_scroll_offset;
        self.effective_scroll_offset = target;
        self.snap = HeaderSnapProgress::Settling;
        changed
    }

    /// Whether snap work remains: a running animation, or a completed one
    /// whose exact endpoint still needs one presenting frame.
    pub(super) fn is_snapping(&self) -> bool {
        matches!(
            self.snap,
            HeaderSnapProgress::Running(_) | HeaderSnapProgress::Settling
        )
    }

    /// Expands a floating header toward a reveal target. Flutter uses the
    /// same presentation animation for snapping and revealing; an in-flight
    /// animation is not retargeted by another show-on-screen request.
    fn request_reveal(&mut self, target: f32, options: &ShowOnScreen) -> bool {
        if target >= self.effective_scroll_offset
            || matches!(self.snap, HeaderSnapProgress::Running(_))
        {
            return false;
        }
        self.snap = HeaderSnapProgress::Running(HeaderSnapRun {
            from: self.effective_scroll_offset,
            target,
            started_at: None,
            duration: options.duration,
            curve: options.curve.clone(),
        });
        true
    }

    /// Consumes pending scroll activity and advances snap presentation.
    ///
    /// `spec` carries the header's snap configuration and live snap range;
    /// `frame` is the last layout's paint/overlap/scroll snapshot. A new
    /// activity cancels running and settling work first — even when the
    /// offset has not moved — while preserving the current presentation as
    /// the baseline later movement continues from. A run starts only from an
    /// actual scroll-end signal while the header is partially revealed and
    /// free of leading overscroll — never from an unchanged offset. The
    /// revealed half (or more) animates to fully revealed; the hidden half
    /// animates to fully hidden, clamped to the scrolled distance the scroll
    /// state itself enforces. Under reduced motion the same endpoint
    /// resolves immediately through the shared completion path instead of
    /// interpolating: no animation runs, but the endpoint is still presented
    /// exactly before scheduling goes idle. The logical scroll extent and
    /// controller offset are untouched: only the effective (presentation)
    /// offset moves, and on completion it equals the endpoint exactly so
    /// later scrolls continue coherently. Returns whether the effective
    /// offset moved.
    pub(super) fn advance_snap(
        &mut self,
        now: Instant,
        activity: HeaderSnapActivity,
        spec: HeaderSnapSpec,
        frame: HeaderSnapFrame,
        reduced_motion: bool,
    ) -> bool {
        if activity == HeaderSnapActivity::Started {
            self.snap = HeaderSnapProgress::Idle;
            return false;
        }
        let range = spec.range.max(0.);
        // Enabling reduced motion mid-flight resolves the run's own target
        // on this tick instead of interpolating toward it.
        if reduced_motion && matches!(self.snap, HeaderSnapProgress::Running(_)) {
            return self.resolve_snap_immediately(range);
        }
        if activity == HeaderSnapActivity::Ended
            && spec.enabled
            && spec.floating
            && range > 0.
            && frame.overlap >= 0.
            && frame.scroll.is_finite()
        {
            let visible = (frame.paint - spec.pinned).clamp(0., range);
            // The hidden edge can never lie past the scrolled distance:
            // scroll-driven presentation clamps hidden distance to it, so a
            // target past it would fight every layout back. When the header
            // is already maximally hidden for its offset, the target equals
            // the current presentation and no run starts.
            let target = if visible >= range / 2. {
                0.
            } else {
                range.min(frame.scroll).max(0.)
            };
            let from = self.effective_scroll_offset.clamp(0., range);
            if target == from {
                self.snap = HeaderSnapProgress::Idle;
            } else if reduced_motion {
                self.effective_scroll_offset = target;
                self.snap = HeaderSnapProgress::Settling;
                return true;
            } else {
                self.snap = HeaderSnapProgress::Running(HeaderSnapRun {
                    from,
                    target,
                    started_at: Some(now),
                    duration: spec.duration,
                    curve: spec.curve,
                });
            }
        }
        let HeaderSnapProgress::Running(run) = &mut self.snap else {
            if matches!(self.snap, HeaderSnapProgress::Settling) {
                self.snap = HeaderSnapProgress::Idle;
            }
            return false;
        };
        let target = run.target.clamp(0., range);
        let started_at = *run.started_at.get_or_insert(now);
        let elapsed = now.saturating_duration_since(started_at);
        let progress = if run.duration.is_zero() {
            1.
        } else {
            (elapsed.as_secs_f32() / run.duration.as_secs_f32()).clamp(0., 1.)
        };
        let value = if progress >= 1. {
            target
        } else {
            f32::interpolate(run.from, target, run.curve.apply(progress))
        };
        let changed = value != self.effective_scroll_offset;
        self.effective_scroll_offset = value;
        if progress >= 1. {
            self.snap = HeaderSnapProgress::Settling;
        }
        changed
    }
}

/// Neutral snap timing: one fixed duration and curve shared by every floating
/// header, so snapping stays a presentation policy rather than per-header
/// configuration.
const HEADER_SNAP_DURATION: Duration = Duration::from_millis(300);

/// Unconsumed scroll-activity signal for snap decisions. Each event
/// overwrites the previous one, which preserves ordering across frames: End
/// followed by Start leaves `Started` (no snap may begin inside the new
/// activity), while Start followed by End leaves `Ended` (a fresh endpoint
/// decision runs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum HeaderSnapActivity {
    /// No unconsumed activity transition.
    #[default]
    None,
    /// A scroll activity began: cancel snap work, start nothing.
    Started,
    /// A scroll activity ended: a partially revealed header may snap.
    Ended,
}

/// Mutually exclusive snap presentation states. A running animation carries
/// its start presentation, resolved target, and timing together; there are no
/// partially valid flag combinations.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum HeaderSnapProgress {
    /// No snap animation; presentation follows scroll deltas.
    #[default]
    Idle,
    /// A snap animation is driving the effective offset.
    Running(HeaderSnapRun),
    /// The animation reached its endpoint exactly on the last tick. One more
    /// frame stays scheduled so the exact endpoint is laid out and presented
    /// before the state returns to idle; without it the completing tick would
    /// mark layout dirty while reporting no active animation, and no frame
    /// would present the endpoint.
    Settling,
}

/// One snap animation: where presentation started, the resolved absolute
/// effective target, and when/how fast it gets there. The target is resolved
/// against the live range every tick, so geometry changes retarget the run
/// instead of stranding it past a new edge.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct HeaderSnapRun {
    from: f32,
    target: f32,
    started_at: Option<Instant>,
    duration: Duration,
    curve: Curve,
}

/// A header's snap configuration plus its live snap range for one tick.
#[derive(Clone, Debug)]
pub(super) struct HeaderSnapSpec {
    /// Snap animation timing for this header.
    pub(super) duration: Duration,
    /// Snap animation curve for this header.
    pub(super) curve: Curve,
    /// The descriptor enabled snapping.
    pub(super) enabled: bool,
    /// The behavior presents a floating range (Floating or FloatingPinned).
    pub(super) floating: bool,
    /// Snap travel: the full extent for Floating, the collapse range for
    /// FloatingPinned.
    pub(super) range: f32,
    /// The minimum that stays visible and is excluded from snap travel (the
    /// pinned minimum for FloatingPinned, zero for Floating).
    pub(super) pinned: f32,
}

/// The last layout's paint/overlap snapshot a snap tick decides from.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct HeaderSnapFrame {
    /// Painted extent of the last layout.
    pub(super) paint: f32,
    /// Constraint overlap of the last layout; negative means leading
    /// overscroll is presenting instead.
    pub(super) overlap: f32,
    /// Scroll offset the last layout ran under; hidden targets can never
    /// exceed it, since scroll-driven presentation clamps hidden distance to
    /// the scrolled distance.
    pub(super) scroll: f32,
}

/// Creates the shared scroll-activity signal and its controller subscription
/// for a header that can snap. The listener records actual scroll-activity
/// `Start` and `End` transitions, each overwriting the previous one, so an
/// `End` stranded inside a newer activity can never start a snap; starting
/// from unchanged offsets is never inferred. Pass `false` unless snapping
/// can engage, so inert headers keep their existing behavior exactly. The
/// returned subscription must be retained by the render sliver: dropping it
/// unsubscribes, so disposal stops snap signals without further cleanup.
pub(super) fn snap_activity_subscription(
    controller: &ScrollController,
    enabled: bool,
) -> (
    Rc<Cell<HeaderSnapActivity>>,
    Option<ScrollNotificationSubscription>,
) {
    let activity = Rc::new(Cell::new(HeaderSnapActivity::None));
    let subscription = enabled.then(|| {
        let activity = activity.clone();
        controller.add_notification_listener(move |notification| {
            match notification.kind {
                ScrollNotificationType::Start => activity.set(HeaderSnapActivity::Started),
                ScrollNotificationType::End => activity.set(HeaderSnapActivity::Ended),
                _ => {}
            }
            false
        })
    });
    (activity, subscription)
}

fn floating_geometry(
    constraints: SliverConstraints,
    extent: f32,
    effective_offset: f32,
    layout_follows_paint: bool,
) -> SliverGeometry {
    let scroll_offset = constraints.scroll_offset;
    let effective_remaining_paint_extent =
        (constraints.remaining_paint_extent - constraints.overlap).max(0.);
    let paint_extent = (extent - effective_offset)
        .max(0.)
        .min(effective_remaining_paint_extent);
    // Flutter: `Overlay` keeps the scroll-driven layout extent, `Scroll` lays
    // out the header at its visible paint extent (widgets/sliver_floating_header.dart:209-249).
    let layout_extent = if layout_follows_paint {
        paint_extent
    } else {
        (extent - scroll_offset)
            .clamp(0., effective_remaining_paint_extent)
            .min(paint_extent)
    };
    SliverGeometry {
        scroll_extent: extent,
        paint_extent,
        layout_extent,
        max_paint_extent: extent,
        max_scroll_obstruction_extent: 0.,
        hit_test_extent: paint_extent,
        paint_origin: constraints.overlap.min(0.),
        cache_extent: if layout_extent > 0. {
            (-constraints.cache_origin + layout_extent).max(0.)
        } else {
            0.
        },
        visible: paint_extent > 0.,
        has_visual_overflow: true,
        scroll_offset_correction: None,
    }
}

impl RenderSliver for FloatingHeaderRenderSliver {
    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        SliverScrollDependency::ScrollOffset
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let extent = self.extent.get().max(0.);
        let scroll_offset = constraints.scroll_offset;
        // A scroll end observed since the last frame decides its snap from
        // the previous layout's presentation, as Flutter's listener does.
        self.consume_activity();
        let direction = self.signal.get().layout_direction();
        let effective_offset = self.scroll_state.update(scroll_offset, extent, direction);
        let geometry = floating_geometry(
            constraints,
            extent,
            effective_offset,
            self.snap_mode == FloatingHeaderSnapMode::Scroll,
        );
        // Flutter places the child at `min(0, paintExtent - childExtent)`.
        let effective_offset = effective_offset.clamp(0., extent);
        SliverLayout {
            geometry,
            // The sequence converts this back through the viewport transform.
            // `extent - effective` is not a normal flow offset: it exposes
            // the child at the leading edge while the header is floating.
            children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: self.child.clone(),
                semantic_index: None,
                offset: scroll_offset - effective_offset,
                cross_offset: 0.,
                constraints: sliver_child_constraints(
                    constraints.axis,
                    constraints.cross_axis_extent,
                    None,
                ),
                extent,
                placement: SliverChildPlacement::Floating,
            }],
            absorbed_overlap: (geometry.paint_extent - geometry.layout_extent).max(0.),
        }
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        if child.0 != 0 || !extent.is_finite() || extent < 0. {
            return false;
        }
        let extent = extent.max(0.);
        if (self.extent.get() - extent).abs() <= f32::EPSILON {
            return false;
        }
        self.extent.set(extent);
        true
    }

    fn tick(&mut self, now: Instant) -> bool {
        let decided = self.consume_activity();
        self.scroll_state.tick(now) || decided
    }

    fn is_animating(&self) -> bool {
        self.scroll_state.is_snapping() || self.signal.get().pending != HeaderSnapActivity::None
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion.set(reduced);
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

impl RenderSliver for HeaderRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let geometry = if self.pinned {
            pinned_geometry(constraints, self.extent, self.extent, self.extent)
        } else {
            // Flutter's scrolling persistent header always reports visual
            // overflow ("conservatively say we do have overflow").
            SliverGeometry {
                has_visual_overflow: true,
                ..SliverGeometry::from_scroll_extent(constraints, self.extent)
            }
        };
        SliverLayout {
            geometry,
            children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: self.child.clone().with_sliver_scroll_exclusion(self.pinned),
                semantic_index: None,
                offset: 0.,
                cross_offset: 0.,
                constraints: sliver_child_constraints(
                    constraints.axis,
                    constraints.cross_axis_extent,
                    Some(self.extent),
                ),
                extent: self.extent,
                placement: if self.pinned {
                    SliverChildPlacement::Pinned
                } else {
                    SliverChildPlacement::Flow
                },
            }],
            absorbed_overlap: (geometry.paint_extent - geometry.layout_extent).max(0.),
        }
    }
}

/// Last mounted child size and orientation at the header's paint edge.
#[derive(Clone, Copy, Debug)]
pub(super) struct HeaderRevealFrame {
    axis: Axis,
    reverse: bool,
    extent: f32,
}

pub(super) struct ResizingHeaderRenderSliver {
    pub(super) child: Widget,
    pub(super) delegate: Option<crate::scrolling::PersistentHeaderBuilder>,
    pub(super) min_extent: f32,
    pub(super) max_extent: f32,
    pub(super) scroll_behavior: SliverHeaderScrollBehavior,
    pub(super) overscroll_behavior: SliverHeaderOverscrollBehavior,
    pub(super) scroll_state: HeaderScrollState,
    pub(super) snap: bool,
    /// Flutter persistent headers float during user scrolling, not just
    /// because a programmatic jump moves closer to the header.
    pub(super) persistent_controller: Option<ScrollController>,
    pub(super) reveal_configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
    pub(super) reveal_frame: Option<HeaderRevealFrame>,
    /// Last reduced-motion policy stamped by the owning delegate. The tree
    /// pushes the retained environment value before every tick, so fresh
    /// render slivers observe the current policy on their first decision
    /// without any rebuild or replacement.
    pub(super) reduced_motion: Cell<bool>,
    pub(super) snap_frame: Cell<HeaderSnapFrame>,
    pub(super) snap_activity: Rc<Cell<HeaderSnapActivity>>,
    /// Owns the scroll-activity listener while snapping can engage. Never read:
    /// dropping the subscription unsubscribes, so disposal stops snap
    /// triggers without further cleanup. `None` when snapping cannot engage.
    pub(super) _snap_subscription: Option<ScrollNotificationSubscription>,
}

/// Lifecycle of a naturally measured header's logical extent.
///
/// Validity depends on the actual measurement inputs, not on position or
/// type correspondence:
///
/// - An unverified estimate seeds the first frame so a new header has
///   geometry before its child is measured. It never authorizes stretched
///   presentation: estimates always measure unbounded first, so a header
///   created during overscroll establishes its true natural size instead of
///   mistaking stretched visuals (or a blind default) for logical extent.
/// - A validated measurement is tied to the cross-axis extent it was recorded
///   under (wrapping content re-flows when the viewport width changes). A
///   cross change demotes it back to an estimate for unbounded revalidation,
///   including mid-overscroll; any genuine range change then settles through
///   Scroll's documented extent policy.
/// - Transferred measurements arrive as estimate seeds, never as validity:
///   matching positions and types prove nothing about current content, so
///   every seed revalidates unbounded before stretched presentation applies.
///   Seeding with the predecessor's validated value (rather than a blind
///   hint) keeps equivalent replacements range-stable through validation,
///   while changed content authoritatively re-establishes itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum NaturalHeaderExtent {
    /// Unverified first estimate: a transferred predecessor value or a child
    /// hint / lazy default. Always measures unbounded next.
    Estimate(f32),
    /// Validated by retained child measurement under the recorded cross-axis
    /// extent.
    Measured { natural: f32, cross: f32 },
}

/// Whether the most recent layout presented stretched visuals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum HeaderPresentation {
    /// Normal presentation; child measurements describe logical extent.
    #[default]
    Settled,
    /// Leading overscroll presentation; child measurements are transient.
    Stretched,
}

/// Retained naturally measured header with optional leading stretch.
///
/// Only unbounded child measurements may establish or update the measured
/// extent. Stretched (tight) samples are presentation-only and can never
/// accumulate into the logical scroll range: tight totals conflate natural
/// size with transient overscroll, so they carry no validity signal. A
/// revision-driven descendant rebuild demotes back to an estimate instead,
/// so in-place content changes revalidate unbounded even mid-overscroll;
/// the tight presentation still resolves the live-measured bottom with the
/// toolbar taking the remainder every pass.
pub(super) struct NaturalHeaderRenderSliver {
    pub(super) child: Widget,
    pub(super) scroll_behavior: SliverHeaderScrollBehavior,
    pub(super) overscroll_behavior: SliverHeaderOverscrollBehavior,
    pub(super) extent: Cell<NaturalHeaderExtent>,
    pub(super) presentation: Cell<HeaderPresentation>,
    pub(super) last_cross: Cell<f32>,
    pub(super) sample_unbounded: Cell<bool>,
    pub(super) scroll_state: HeaderScrollState,
    pub(super) snap: bool,
    /// Last reduced-motion policy stamped by the owning delegate. The tree
    /// pushes the retained environment value before every tick, so fresh
    /// render slivers observe the current policy on their first decision
    /// without any rebuild or replacement.
    pub(super) reduced_motion: Cell<bool>,
    pub(super) snap_frame: Cell<HeaderSnapFrame>,
    pub(super) snap_activity: Rc<Cell<HeaderSnapActivity>>,
    /// Owns the scroll-activity listener while snapping can engage. Never read:
    /// dropping the subscription unsubscribes, so disposal stops snap
    /// triggers without further cleanup. `None` when snapping cannot engage.
    pub(super) _snap_subscription: Option<ScrollNotificationSubscription>,
}

impl NaturalHeaderRenderSliver {
    /// Compatibility and invalidation rules for transfer across descriptor
    /// updates (applied pairwise by sliver position). Returns whether any
    /// state was adopted.
    ///
    /// - A predecessor's latest number is adopted only as an unverified
    ///   estimate seed, never as validity — even for identical positions and
    ///   types. The seed revalidates unbounded, so equivalent replacements
    ///   stay range-stable while changed content cannot retain a stale
    ///   measurement. Seeding from the predecessor (rather than a blind
    ///   hint) also survives a drain demotion that ran moments earlier on
    ///   the old delegate during the same update.
    /// - Reversal tracking transfers only when `scroll_behavior` is identical,
    ///   since it defines the effective-offset range semantics.
    /// - Stretched presentation is never inherited; it recomputes from live
    ///   overlap on the next layout.
    pub(super) fn adopt_compatible_state(&mut self, previous: &Self) -> bool {
        let mut adopted = false;
        let seed = match previous.extent.get() {
            NaturalHeaderExtent::Estimate(estimate) => estimate,
            NaturalHeaderExtent::Measured { natural, .. } => natural,
        }
        .max(0.);
        // A never-measured predecessor carries no information beyond the
        // fresh hint seed unless invalidation demoted it — but a demoted
        // seed is exactly as good, so adopt uniformly and revalidate.
        if !matches!(self.extent.get(), NaturalHeaderExtent::Estimate(current) if (current - seed).abs() <= f32::EPSILON)
        {
            self.extent.set(NaturalHeaderExtent::Estimate(seed));
            adopted = true;
        }
        if self.scroll_behavior == previous.scroll_behavior {
            self.scroll_state = previous.scroll_state.clone();
            adopted = true;
        }
        // A running snap never transfers: replacement always re-seeds the
        // extent to an unverified estimate, so the snap range basis is
        // uncertain until revalidation. Presentation falls back to whatever
        // the scroll offset determines, which is always coherent.
        self.scroll_state.cancel_snap();
        adopted
    }
}

pub(super) struct FillRemainingRenderSliver {
    pub(super) child: Widget,
    pub(super) has_scroll_body: bool,
    pub(super) fill_overscroll: bool,
}

impl FillRemainingRenderSliver {
    /// Flutter's `RenderSliverFillRemainingAndOverscroll`: the child fills the
    /// remaining space plus any overscroll area, with a minimum of the
    /// remaining extent and a maximum that includes overscroll.
    fn overscroll_layout(&self, constraints: SliverConstraints, extent: f32) -> SliverLayout {
        // The viewport caps the paint budget at its own extent; past this
        // sliver's leading edge, the clipped part of the child is carried in
        // scroll_offset. Keep that part in the child's layout budget so its
        // visible trailing edge can still reach the end of the viewport.
        let max_extent = (constraints.remaining_paint_extent - constraints.overlap.min(0.)
            + constraints.scroll_offset)
            .max(extent);
        let paint_extent = max_extent.min(constraints.remaining_paint_extent).max(0.);
        let mut geometry = SliverGeometry::from_scroll_extent(constraints, extent);
        geometry.paint_extent = paint_extent;
        geometry.layout_extent = paint_extent;
        geometry.hit_test_extent = paint_extent;
        geometry.max_paint_extent = max_extent;
        geometry.visible = paint_extent > 0.;
        let cross = constraints.cross_axis_extent.max(0.);
        let child_constraints = match constraints.axis {
            Axis::Vertical => Constraints::new(cross, cross, extent, max_extent),
            Axis::Horizontal => Constraints::new(extent, max_extent, cross, cross),
        };
        SliverLayout {
            geometry,
            children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: self.child.clone(),
                semantic_index: None,
                offset: 0.,
                cross_offset: 0.,
                constraints: child_constraints,
                extent: max_extent,
                placement: SliverChildPlacement::Flow,
            }],
            absorbed_overlap: 0.,
        }
    }
}

impl RenderSliver for FillRemainingRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let child_hint = widget_main_extent_hint(&self.child, constraints.axis).unwrap_or(0.);
        let remaining_extent =
            (constraints.viewport_main_axis_extent - constraints.preceding_scroll_extent).max(0.);
        if !self.has_scroll_body && self.fill_overscroll {
            return self.overscroll_layout(constraints, remaining_extent.max(child_hint));
        }
        let extent = if self.has_scroll_body {
            (constraints.remaining_paint_extent - constraints.overlap.min(0.)).max(0.)
        } else {
            // Flutter's RenderSliverFillRemaining recomputes the extent from the
            // current remaining space and the child's intrinsic size on every
            // layout (rendering/sliver_fill.dart:154-166). Carrying the previous
            // extent forward would keep a stale maximum after content above shrinks.
            remaining_extent.max(child_hint)
        };
        let (geometry, child_constraints) = if self.has_scroll_body {
            // A scroll body claims one viewport of scroll range, independently
            // of the part that can paint in this frame. At the trailing cache
            // boundary, lay out the hidden child loosely in the cache budget
            // rather than collapsing it until it is on screen.
            let scroll_extent = constraints.viewport_main_axis_extent;
            let cache_extent = calculate_cache_offset(constraints, 0., scroll_extent);
            let (start, end) = constraints.visible_interval(extent);
            let paint_extent = end - start;
            let max_extent = if extent == 0. { cache_extent } else { extent };
            let cross = constraints.cross_axis_extent;
            let child_constraints = match constraints.axis {
                Axis::Vertical => Constraints::new(cross, cross, extent, max_extent),
                Axis::Horizontal => Constraints::new(extent, max_extent, cross, cross),
            };
            let geometry = SliverGeometry {
                scroll_extent,
                paint_extent,
                layout_extent: paint_extent,
                max_paint_extent: paint_extent,
                hit_test_extent: paint_extent,
                cache_extent,
                visible: paint_extent > 0.,
                has_visual_overflow: extent > constraints.remaining_paint_extent
                    || constraints.scroll_offset > 0.,
                ..SliverGeometry::ZERO
            };
            (geometry, child_constraints)
        } else {
            (
                SliverGeometry::from_scroll_extent(constraints, extent),
                sliver_child_constraints(
                    constraints.axis,
                    constraints.cross_axis_extent,
                    Some(extent),
                ),
            )
        };
        SliverLayout {
            geometry,
            children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: self.child.clone(),
                semantic_index: None,
                // The parent sequence contributes the preceding scroll
                // extent; a sliver child is always positioned in this
                // sliver's local coordinate space.
                offset: 0.,
                cross_offset: 0.,
                constraints: child_constraints,
                extent,
                placement: SliverChildPlacement::Flow,
            }],
            absorbed_overlap: 0.,
        }
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        // A scroll body consumes the current paint budget, and a stretching
        // child follows that budget through overscroll. Rematerializing the
        // cache is not required for either size to change. Ordinary static
        // fill depends on the preceding scroll extent, not the current offset.
        if self.has_scroll_body || self.fill_overscroll {
            SliverScrollDependency::ScrollOffset
        } else {
            SliverScrollDependency::CacheWindow
        }
    }
}

pub(super) struct ViewportExtentRenderSliver {
    pub(super) item_count: usize,
    pub(super) children: Option<Rc<Vec<Widget>>>,
    pub(super) builder: Option<Rc<dyn Fn(usize) -> Widget>>,
    pub(super) find_child_index: Option<FindChildIndex>,
    pub(super) widgets: HashMap<usize, Widget>,
    pub(super) viewport_fraction: f32,
    pub(super) fallback_extent: f32,
    /// Flutter `SliverFillViewport.padEnds`.
    pub(super) pad_ends: bool,
    pub(super) allow_implicit_scrolling: bool,
}

impl ViewportExtentRenderSliver {
    fn build_child(&self, index: usize) -> Widget {
        self.children
            .as_ref()
            .and_then(|children| children.get(index).cloned())
            .or_else(|| self.builder.as_ref().map(|builder| builder(index)))
            .unwrap_or_else(|| SizedBox::shrink().into())
    }

    fn child_widget(&mut self, index: usize) -> Widget {
        if let Some(widget) = self.widgets.get(&index) {
            return widget.clone();
        }
        let widget = self.build_child(index);
        self.widgets.insert(index, widget.clone());
        widget
    }

    fn adopt_keyed_keep_alives(&mut self, previous: &Self, probes: &mut u64) {
        let Some(find_index) = self.find_child_index.clone() else {
            return;
        };
        // Only children materialized by the outgoing delegate are queried,
        // in original index order. A scroll in the same delegate never
        // makes the reverse lookup, even when it materializes fresh pages.
        let mut indices: Vec<_> = previous.widgets.keys().copied().collect();
        indices.sort_unstable();
        for index in indices {
            let old = &previous.widgets[&index];
            let destination = old.key().and_then(&*find_index).unwrap_or(index);
            if !old.keep_alive_request()
                || destination >= self.item_count
                || self.widgets.contains_key(&destination)
            {
                continue;
            }
            *probes += 1;
            let widget = self.build_child(destination);
            if widget.key() == old.key()
                && std::mem::discriminant(widget.kind()) == std::mem::discriminant(old.kind())
            {
                self.widgets.insert(
                    destination,
                    widget.with_focus_keep_alive_request(old.focus_keep_alive_request()),
                );
            }
        }
    }
}

impl RenderSliver for ViewportExtentRenderSliver {
    fn set_child_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        grid_tile.is_none()
            && child
                .item_index()
                .is_some_and(|index| set_cached_keep_alive(&mut self.widgets, index, keep_alive))
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        grid_tile.is_none()
            && child.item_index().is_some_and(|index| {
                set_cached_focus_keep_alive(&mut self.widgets, index, keep_alive)
            })
    }

    fn child_count(&self) -> Option<usize> {
        Some(self.item_count)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        if self.allow_implicit_scrolling {
            SliverScrollDependency::CacheWindow
        } else {
            // The visiting interval can cross a page boundary without
            // invalidating the cached materialization range.
            SliverScrollDependency::ScrollOffset
        }
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let viewport_extent = if constraints.viewport_main_axis_extent.is_finite() {
            constraints.viewport_main_axis_extent.max(0.)
        } else {
            self.fallback_extent
        };
        let extent = (viewport_extent * self.viewport_fraction).max(0.);
        // Flutter `_SliverFractionalPadding`: with `padEnds`, the first and
        // last pages are inset by half the space a page leaves unused, so
        // they can be centered.
        let pad = if self.pad_ends {
            viewport_extent * ((1. - self.viewport_fraction).clamp(0., 1.) / 2.)
        } else {
            0.
        };
        let total = self.item_count as f32 * extent + 2. * pad;
        let visible_start = (constraints.scroll_offset - pad).max(0.);
        let visible_end = visible_start + viewport_extent;
        let cache_start = (constraints.scroll_offset + constraints.cache_origin - pad).max(0.);
        let cache_end = (constraints.scroll_offset
            + constraints.cache_origin
            + constraints.remaining_cache_extent
            - pad)
            .max(cache_start);
        let range = fixed_extent_cache_range(cache_start, cache_end, extent, self.item_count);
        let parked = retain_fixed_cache(&mut self.widgets, &range, self.item_count);
        let children = fixed_cache_slots(range, &parked)
            .map(|(index, placement)| {
                let start = index as f32 * extent;
                let mut widget = self.child_widget(index);
                if !self.allow_implicit_scrolling
                    && (start >= visible_end || start + extent <= visible_start)
                {
                    // Do not park or replace the page: its identity, paint,
                    // focus and retained cache membership stay unchanged.
                    widget = widget.exclude_semantics();
                }
                SliverChildLayout {
                    id: SliverChildId::list_item(index),
                    widget,
                    semantic_index: Some(index),
                    offset: pad + start,
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
        SliverLayout {
            geometry: SliverGeometry::from_scroll_extent(constraints, total),
            children,
            absorbed_overlap: 0.,
        }
    }
}

impl RenderSliver for ResizingHeaderRenderSliver {
    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        let config = self.reveal_configuration?;
        let frame = self.reveal_frame?;
        if child != SliverChildId(0) {
            return None;
        }
        let (start, cross) = match frame.axis {
            Axis::Vertical => (rect.origin.y, rect.origin.x),
            Axis::Horizontal => (rect.origin.x, rect.origin.y),
        };
        let end = start + frame.axis.main_extent(rect.size);
        if !start.is_finite() || !end.is_finite() || !cross.is_finite() {
            return None;
        }
        // Ignore the portion outside the header's growth-leading edge.
        // An explicit rectangle may still extend further into the header
        // than it currently measures; that is what requests its expansion.
        let (start, end, requested) = if frame.reverse {
            (start, end.min(frame.extent), frame.extent - start)
        } else {
            (start.max(0.), end, end)
        };
        let extent = config
            .limit(requested)
            .clamp(frame.extent, self.max_extent.max(frame.extent));
        let target_offset = self.max_extent - extent;
        if extent > frame.extent && self.scroll_state.request_reveal(target_offset, options) {
            // An activity notification already queued before this request
            // must not overwrite the newer reveal.
            let _ = self.snap_activity.take();
            incular_animation::schedule_frame();
        }
        Some(Rect::from_origin_size(
            frame.axis.offset(start, cross),
            frame
                .axis
                .size((end - start).max(0.), frame.axis.cross_extent(rect.size)),
        ))
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        SliverScrollDependency::ScrollOffset
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let range = self.max_extent - self.min_extent;
        let scroll_offset = constraints.scroll_offset;
        let allow_floating_expansion =
            self.persistent_controller
                .as_ref()
                .is_none_or(|controller| {
                    let leading = if constraints.growth_direction == SliverGrowthDirection::Reverse
                    {
                        1
                    } else {
                        -1
                    };
                    controller.user_scroll_direction_sign() == leading
                });
        let effective_offset = match self.scroll_behavior {
            SliverHeaderScrollBehavior::Floating => {
                self.scroll_state.update_with_floating_expansion(
                    scroll_offset,
                    self.max_extent,
                    allow_floating_expansion,
                )
            }
            SliverHeaderScrollBehavior::FloatingPinned => {
                // Flutter's persistent delegate tracks how far the entire
                // header has scrolled, even while minExtent remains pinned.
                // The separate Incular resizing widget reveals from its
                // collapsed span without that initial dead distance.
                let reveal_extent = if self.delegate.is_some() {
                    self.max_extent
                } else {
                    range
                };
                self.scroll_state.update_with_floating_expansion(
                    scroll_offset,
                    reveal_extent,
                    allow_floating_expansion,
                )
            }
            _ => scroll_offset,
        };
        let stretch = match self.overscroll_behavior {
            SliverHeaderOverscrollBehavior::Stretch
                if scroll_offset == 0. && constraints.preceding_scroll_extent == 0. =>
            {
                -constraints.overlap.min(0.)
            }
            _ => 0.,
        };
        let current = ((self.max_extent - effective_offset)
            .clamp(self.min_extent, self.max_extent)
            + stretch)
            .min(f32::MAX);
        if self.reveal_configuration.is_some() {
            self.reveal_frame = Some(HeaderRevealFrame {
                axis: constraints.axis,
                reverse: constraints.reverse
                    ^ (constraints.growth_direction == SliverGrowthDirection::Reverse),
                extent: current,
            });
        }
        if stretch > 0. {
            // Stretch owns presentation during leading overscroll; a running
            // snap yields to it and the next scroll end decides again.
            self.scroll_state.cancel_snap();
        }
        let (mut geometry, offset, mut placement) = match self.scroll_behavior {
            SliverHeaderScrollBehavior::Pinned | SliverHeaderScrollBehavior::FloatingPinned => (
                // Flutter's SliverResizingHeader reports the cache covered by its
                // current extent, `calculateCacheOffset(0..childExtent)`
                // (widgets/sliver_resizing_header.dart:223), not the pinned
                // persistent-header formula.
                SliverGeometry {
                    cache_extent: calculate_cache_offset(constraints, 0., current),
                    ..pinned_geometry(constraints, self.max_extent, current, self.min_extent)
                },
                0.,
                SliverChildPlacement::Pinned,
            ),
            SliverHeaderScrollBehavior::Scroll => (
                SliverGeometry {
                    // Flutter's resizing header always reports visual overflow.
                    has_visual_overflow: true,
                    ..SliverGeometry::from_scroll_extent(constraints, self.max_extent)
                },
                scroll_offset.min(range),
                SliverChildPlacement::Flow,
            ),
            SliverHeaderScrollBehavior::Floating => (
                floating_geometry(constraints, self.max_extent, effective_offset, false),
                scroll_offset - effective_offset + effective_offset.min(range),
                SliverChildPlacement::Floating,
            ),
        };
        if stretch > 0. {
            let available = (constraints.remaining_paint_extent + stretch).min(f32::MAX);
            geometry.paint_extent = current.min(available);
            geometry.hit_test_extent = geometry.paint_extent;
            geometry.max_paint_extent = current;
            geometry.paint_origin = -stretch;
            geometry.visible = geometry.paint_extent > 0.;
            // This placement cancels the viewport's overscroll translation.
            // Automatic pinning would move it back down and leave a gap.
            placement = SliverChildPlacement::Floating;
        }
        self.snap_frame.set(HeaderSnapFrame {
            paint: geometry.paint_extent,
            overlap: constraints.overlap,
            scroll: constraints.scroll_offset,
        });
        // Flutter `RenderSliverPersistentHeader.layoutChild`: the delegate
        // builds with `min(scrollOffset, maxExtent)`, and a pinned header
        // overlaps content once something scrolls under it.
        let widget = match &self.delegate {
            Some(build) => {
                let shrink = match self.scroll_behavior {
                    SliverHeaderScrollBehavior::Floating
                    | SliverHeaderScrollBehavior::FloatingPinned => effective_offset,
                    _ => scroll_offset,
                }
                .clamp(0., self.max_extent);
                let overlaps = matches!(
                    self.scroll_behavior,
                    SliverHeaderScrollBehavior::Pinned | SliverHeaderScrollBehavior::FloatingPinned
                ) && constraints.overlap > 0.;
                build(shrink, overlaps)
            }
            None => self.child.clone(),
        };
        // A pinned persistent delegate is always outside the scroll pane.
        // SliverResizingHeader itself is excluded only once pinned paint
        // outlives its scrolling layout extent.
        let excluded = self.scroll_behavior == SliverHeaderScrollBehavior::Pinned
            && (self.delegate.is_some() || geometry.layout_extent < current);
        let widget = widget.with_sliver_scroll_exclusion(excluded);
        SliverLayout {
            geometry,
            children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget,
                semantic_index: None,
                offset,
                cross_offset: 0.,
                constraints: sliver_child_constraints(
                    constraints.axis,
                    constraints.cross_axis_extent,
                    Some(current),
                ),
                extent: current,
                placement,
            }],
            absorbed_overlap: if stretch > 0. {
                0.
            } else {
                (geometry.paint_extent - geometry.layout_extent).max(0.)
            },
        }
    }

    fn tick(&mut self, now: Instant) -> bool {
        // Consuming here (before advancing) applies the ordering rule: a
        // newer Start already overwrote any stranded End.
        let activity = self.snap_activity.take();
        let (range, pinned) = match self.scroll_behavior {
            SliverHeaderScrollBehavior::Floating => (self.max_extent, 0.),
            SliverHeaderScrollBehavior::FloatingPinned => {
                ((self.max_extent - self.min_extent).max(0.), self.min_extent)
            }
            _ => (0., 0.),
        };
        self.scroll_state.advance_snap(
            now,
            activity,
            HeaderSnapSpec {
                enabled: self.snap,
                floating: matches!(
                    self.scroll_behavior,
                    SliverHeaderScrollBehavior::Floating
                        | SliverHeaderScrollBehavior::FloatingPinned
                ),
                range,
                pinned,
                duration: HEADER_SNAP_DURATION,
                curve: Curve::EaseInOut,
            },
            self.snap_frame.get(),
            self.reduced_motion.get(),
        )
    }

    fn is_animating(&self) -> bool {
        self.scroll_state.is_snapping()
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion.set(reduced);
    }
}

impl RenderSliver for NaturalHeaderRenderSliver {
    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        SliverScrollDependency::ScrollOffset
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let cross = constraints.cross_axis_extent.max(0.);
        self.last_cross.set(cross);
        // A validated measurement only covers the cross extent recorded with
        // it: wrapping content re-flows when the viewport width changes, so a
        // cross change demotes back to an estimate for unbounded revalidation
        // — including mid-overscroll, where any genuine range change then
        // settles through Scroll's documented extent policy.
        if let NaturalHeaderExtent::Measured {
            natural,
            cross: validated,
        } = self.extent.get()
            && (validated - cross).abs() > f32::EPSILON
        {
            self.extent
                .set(NaturalHeaderExtent::Estimate(natural.max(0.)));
        }
        let (natural, unverified) = match self.extent.get() {
            NaturalHeaderExtent::Estimate(estimate) => (estimate.max(0.), true),
            NaturalHeaderExtent::Measured { natural, .. } => (natural.max(0.), false),
        };
        let scroll_offset = constraints.scroll_offset;
        let effective_offset = match self.scroll_behavior {
            SliverHeaderScrollBehavior::Floating => {
                self.scroll_state.update(scroll_offset, natural)
            }
            SliverHeaderScrollBehavior::FloatingPinned => {
                // A fixed natural header has no collapse range; keep the
                // collapsed-equivalent extent pinned while still tracking the
                // reversal state for a consistent lifecycle.
                self.scroll_state.update(scroll_offset, 0.)
            }
            _ => scroll_offset,
        };
        let stretch = match self.overscroll_behavior {
            SliverHeaderOverscrollBehavior::Stretch
                if scroll_offset == 0. && constraints.preceding_scroll_extent == 0. =>
            {
                -constraints.overlap.min(0.)
            }
            _ => 0.,
        };
        let stretched_now = stretch > 0.;
        self.presentation.set(if stretched_now {
            HeaderPresentation::Stretched
        } else {
            HeaderPresentation::Settled
        });
        if stretched_now {
            // Stretch owns presentation during leading overscroll; a running
            // snap yields to it and the next scroll end decides again.
            self.scroll_state.cancel_snap();
        }
        let current = (natural + stretch).min(f32::MAX);
        let (mut geometry, offset, mut placement) = match self.scroll_behavior {
            SliverHeaderScrollBehavior::Pinned | SliverHeaderScrollBehavior::FloatingPinned => (
                pinned_geometry(constraints, natural, current, natural),
                0.,
                SliverChildPlacement::Pinned,
            ),
            SliverHeaderScrollBehavior::Scroll => (
                SliverGeometry::from_scroll_extent(constraints, natural),
                0.,
                SliverChildPlacement::Flow,
            ),
            SliverHeaderScrollBehavior::Floating => (
                floating_geometry(constraints, natural, effective_offset, false),
                scroll_offset - effective_offset,
                SliverChildPlacement::Floating,
            ),
        };
        if stretch > 0. {
            let available = (constraints.remaining_paint_extent + stretch).min(f32::MAX);
            geometry.paint_extent = current.min(available);
            geometry.hit_test_extent = geometry.paint_extent;
            geometry.max_paint_extent = current;
            geometry.paint_origin = -stretch;
            geometry.visible = geometry.paint_extent > 0.;
            // This placement cancels the viewport's overscroll translation.
            // Automatic pinning would move it back down and leave a gap.
            placement = SliverChildPlacement::Floating;
        }
        // Stretched presentation uses tight constraints so a toolbar can fill
        // the extra extent. Settled layouts stay unbounded on the main axis
        // so later content changes are measured instead of being clamped to
        // a previously cached extent. Unverified estimates always measure
        // unbounded first, even while stretched, so a header created during
        // overscroll learns its true size; the estimate only seeds one
        // transient presentation frame, which the viewport's bounded
        // re-layout pass then corrects.
        let unbounded_sample = unverified || !stretched_now;
        self.sample_unbounded.set(unbounded_sample);
        let child_constraints = if unbounded_sample {
            sliver_child_constraints(constraints.axis, constraints.cross_axis_extent, None)
        } else {
            sliver_child_constraints(
                constraints.axis,
                constraints.cross_axis_extent,
                Some(current),
            )
        };
        self.snap_frame.set(HeaderSnapFrame {
            paint: geometry.paint_extent,
            overlap: constraints.overlap,
            scroll: constraints.scroll_offset,
        });
        SliverLayout {
            geometry,
            children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: self.child.clone(),
                semantic_index: None,
                offset,
                cross_offset: 0.,
                constraints: child_constraints,
                extent: current,
                placement,
            }],
            absorbed_overlap: if stretch > 0. {
                0.
            } else {
                (geometry.paint_extent - geometry.layout_extent).max(0.)
            },
        }
    }

    fn tick(&mut self, now: Instant) -> bool {
        // Consuming here (before advancing) applies the ordering rule: a
        // newer Start already overwrote any stranded End.
        let activity = self.snap_activity.take();
        let natural = match self.extent.get() {
            NaturalHeaderExtent::Estimate(estimate) => estimate.max(0.),
            NaturalHeaderExtent::Measured { natural, .. } => natural.max(0.),
        };
        // A floating-pinned natural header has no collapse range, so its snap
        // endpoints coincide and the run below stays inert by construction.
        let (range, pinned) = match self.scroll_behavior {
            SliverHeaderScrollBehavior::Floating => (natural, 0.),
            SliverHeaderScrollBehavior::FloatingPinned => (0., natural),
            _ => (0., 0.),
        };
        self.scroll_state.advance_snap(
            now,
            activity,
            HeaderSnapSpec {
                enabled: self.snap,
                floating: matches!(
                    self.scroll_behavior,
                    SliverHeaderScrollBehavior::Floating
                        | SliverHeaderScrollBehavior::FloatingPinned
                ),
                range,
                pinned,
                duration: HEADER_SNAP_DURATION,
                curve: Curve::EaseInOut,
            },
            self.snap_frame.get(),
            self.reduced_motion.get(),
        )
    }

    fn is_animating(&self) -> bool {
        self.scroll_state.is_snapping()
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion.set(reduced);
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        if child.0 != 0 || !extent.is_finite() || extent < 0. {
            return false;
        }
        let extent = extent.max(0.);
        match self.extent.get() {
            // Unbounded learning samples are the true natural size by
            // construction. Adopting one can never mistake stretched visuals
            // for logical extent. The transition always requests another
            // layout — even when the value equals the estimate — because
            // presentation constraints must still switch (unbounded learning
            // to settled or tight stretch). A tight sample against an
            // estimate only happens when content was invalidated mid-pass
            // after tight constraints went out; it carries no validity, so
            // the estimate stands while the follow-up pass revalidates.
            NaturalHeaderExtent::Estimate(_) if self.sample_unbounded.get() => {
                self.extent.set(NaturalHeaderExtent::Measured {
                    natural: extent,
                    cross: self.last_cross.get(),
                });
                true
            }
            NaturalHeaderExtent::Estimate(_) => true,
            // Tight samples conflate natural size with transient overscroll
            // and carry no validity signal, so they are ignored: ignoring
            // them keeps overscroll from drifting the range across repeated
            // stretch/recovery cycles.
            NaturalHeaderExtent::Measured { .. }
                if self.presentation.get() == HeaderPresentation::Stretched =>
            {
                false
            }
            NaturalHeaderExtent::Measured { natural, cross } => {
                if (natural - extent).abs() <= f32::EPSILON {
                    return false;
                }
                self.extent.set(NaturalHeaderExtent::Measured {
                    natural: extent,
                    cross,
                });
                true
            }
        }
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }

    fn invalidate_child_measurement(&mut self, _child: SliverChildId) {
        // A rebuilt descendant means new content: drop validity back to an
        // estimate seeded with the last validated value. The next layout
        // revalidates unbounded (even mid-overscroll), so equivalent content
        // reconverges without range movement while changed content cannot
        // retain staleness.
        if let NaturalHeaderExtent::Measured { natural, .. } = self.extent.get() {
            self.extent
                .set(NaturalHeaderExtent::Estimate(natural.max(0.)));
        }
    }
}

pub(super) struct EmptyRenderSliver;

impl RenderSliver for EmptyRenderSliver {
    fn perform_layout(&mut self, _: SliverConstraints) -> SliverLayout {
        SliverLayout::empty(SliverGeometry::ZERO)
    }

    fn child_count(&self) -> Option<usize> {
        Some(0)
    }
}

pub(super) struct PaddingRenderSliver {
    pub(super) inner: RefCell<Box<dyn RenderSliver>>,
    pub(super) padding: crate::EdgeInsetsGeometry,
    pub(super) safe_padding: Option<(EdgeInsets, incular_config::MediaQueryEdges)>,
}

impl RenderSliver for PaddingRenderSliver {
    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        self.inner.get_mut().reveal_child(child, rect, options)
    }

    fn set_child_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.inner
            .borrow_mut()
            .set_child_keep_alive(child, grid_tile, keep_alive)
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.inner
            .borrow_mut()
            .set_child_focus_keep_alive(child, grid_tile, keep_alive)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.inner.borrow().scroll_layout_dependency()
    }

    fn child_count(&self) -> Option<usize> {
        self.inner.borrow().child_count()
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let padding = if let Some((minimum, edges)) = self.safe_padding {
            let safe = constraints.safe_insets.normalized();
            EdgeInsets::only(
                if edges.left { safe.left } else { 0. }.max(minimum.left),
                if edges.top { safe.top } else { 0. }.max(minimum.top),
                if edges.right { safe.right } else { 0. }.max(minimum.right),
                if edges.bottom { safe.bottom } else { 0. }.max(minimum.bottom),
            )
        } else {
            self.padding.try_resolve(constraints.text_direction)
                .expect("directional SliverPadding requires Directionality")
        };
        debug_assert!(
            [padding.left, padding.top, padding.right, padding.bottom]
                .into_iter()
                .all(|inset| inset >= 0.),
            "SliverPadding requires nonnegative padding",
        );
        let physical_start = main_before(constraints.axis, padding);
        let physical_end = main_after(constraints.axis, padding);
        // Flutter's applyGrowthDirectionToAxisDirection composes the
        // viewport direction with the side of the center this sliver grows on.
        let reverse_growth = constraints.growth_direction == SliverGrowthDirection::Reverse;
        let (before, after) = if constraints.reverse ^ reverse_growth {
            (physical_end, physical_start)
        } else {
            (physical_start, physical_end)
        };
        let total_padding = before + after;
        let paint_offset = |from: f32, to: f32| {
            let start = from.max(constraints.scroll_offset);
            let end = to.min(constraints.scroll_offset + constraints.remaining_paint_extent);
            (end - start).max(0.)
        };
        let cache_offset = |from: f32, to: f32| {
            let cache_start = (constraints.scroll_offset + constraints.cache_origin).max(0.);
            let start = from.max(cache_start);
            let end = to.min(cache_start + constraints.remaining_cache_extent);
            (end - start).max(0.)
        };
        let reduced_cross = (constraints.cross_axis_extent
            - cross_before(constraints.axis, padding)
            - cross_after(constraints.axis, padding))
        .max(0.);
        let before_paint_extent = paint_offset(0., before);
        let before_cache_extent = cache_offset(0., before);
        let inner_scroll = (constraints.scroll_offset - before).max(0.);
        let mut inner_constraints = SliverConstraints::new(
            constraints.axis,
            constraints.reverse,
            inner_scroll,
            constraints.preceding_scroll_extent + before,
            if constraints.overlap > 0. {
                (constraints.overlap - before_paint_extent).max(0.)
            } else {
                constraints.overlap
            },
            (constraints.remaining_paint_extent - before_paint_extent).max(0.),
            reduced_cross,
            constraints.viewport_main_axis_extent,
            (constraints.remaining_cache_extent - before_cache_extent).max(0.),
            (constraints.cache_origin + before).min(0.),
        );
        inner_constraints.text_direction = constraints.text_direction;
        inner_constraints.growth_direction = constraints.growth_direction;
        let safe = constraints.safe_insets;
        inner_constraints.safe_insets = match self.safe_padding {
            Some((_, edges)) => EdgeInsets::only(
                if edges.left { 0. } else { safe.left },
                if edges.top { 0. } else { safe.top },
                if edges.right { 0. } else { safe.right },
                if edges.bottom { 0. } else { safe.bottom },
            ),
            None => safe,
        };
        let mut layout = self.inner.borrow_mut().perform_layout(inner_constraints);
        let inner_geometry = layout.geometry.normalized();
        if let Some(correction) = inner_geometry.scroll_offset_correction {
            let mut geometry = SliverGeometry::ZERO;
            geometry.scroll_offset_correction = Some(correction);
            return SliverLayout::empty(geometry);
        }
        for child in &mut layout.children {
            child.offset += before;
            child.cross_offset += cross_before(constraints.axis, padding);
            if let Some((_, edges)) = self.safe_padding {
                child.widget = crate::MediaQuery::remove_padding(child.widget.clone(), edges);
            }
        }
        let scroll_extent = total_padding + inner_geometry.scroll_extent;
        let after_paint_extent = paint_offset(
            before + inner_geometry.scroll_extent,
            total_padding + inner_geometry.scroll_extent,
        );
        let after_cache_extent = cache_offset(
            before + inner_geometry.scroll_extent,
            total_padding + inner_geometry.scroll_extent,
        );
        let paint_extent = (before_paint_extent
            + inner_geometry
                .paint_extent
                .max(inner_geometry.layout_extent + after_paint_extent))
        .min(constraints.remaining_paint_extent)
        .max(0.);
        let layout_extent =
            (before_paint_extent + after_paint_extent + inner_geometry.layout_extent)
                .min(paint_extent)
                .max(0.);
        layout.geometry = SliverGeometry {
            paint_origin: inner_geometry.paint_origin,
            scroll_extent,
            paint_extent,
            layout_extent,
            cache_extent: (before_cache_extent + after_cache_extent + inner_geometry.cache_extent)
                .min(constraints.remaining_cache_extent)
                .max(0.),
            max_paint_extent: total_padding + inner_geometry.max_paint_extent,
            // Flutter's SliverPadding reports padding/layout of the child,
            // not the child's pinned obstruction as its own.
            max_scroll_obstruction_extent: 0.,
            hit_test_extent: (before_paint_extent
                + after_paint_extent
                + inner_geometry.paint_extent)
                .max(before_paint_extent + inner_geometry.hit_test_extent),
            visible: paint_extent > 0.,
            has_visual_overflow: inner_geometry.has_visual_overflow,
            scroll_offset_correction: None,
        };
        layout
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        self.inner.borrow_mut().set_child_extent(child, extent)
    }

    fn revision(&self) -> u64 {
        self.inner.borrow().revision()
    }

    fn tick(&mut self, now: Instant) -> bool {
        self.inner.borrow_mut().tick(now)
    }

    fn is_animating(&self) -> bool {
        self.inner.borrow().is_animating()
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        self.inner.borrow_mut().set_reduced_motion(reduced);
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        self.inner.borrow_mut().invalidate_child_measurement(child);
    }
}

/// Changes the sliver constraints themselves, before tight box children
/// receive cross-axis constraints. A box wrapper after layout is too late.
pub(super) struct ConstrainedCrossAxisRenderSliver {
    pub(super) inner: RefCell<Box<dyn RenderSliver>>,
    max_extent: f32,
    last_extent: Option<f32>,
}

impl ConstrainedCrossAxisRenderSliver {
    pub(super) fn new(inner: Box<dyn RenderSliver>, max_extent: f32) -> Self {
        Self { inner: RefCell::new(inner), max_extent, last_extent: None }
    }
}

impl RenderSliver for ConstrainedCrossAxisRenderSliver {
    fn perform_layout(&mut self, mut constraints: SliverConstraints) -> SliverLayout {
        constraints.cross_axis_extent = constraints.cross_axis_extent.min(self.max_extent);
        self.last_extent = Some(constraints.cross_axis_extent);
        self.inner.get_mut().perform_layout(constraints)
    }

    fn reported_cross_axis_extent(&self) -> Option<f32> { self.last_extent }

    fn semantic_annotation_metadata(&self) -> Option<incular_semantics::SemanticAnnotationMetadata> {
        self.inner.borrow().semantic_annotation_metadata()
    }

    fn reveal_child(&mut self, child: SliverChildId, rect: Rect, options: &ShowOnScreen) -> Option<Rect> {
        self.inner.get_mut().reveal_child(child, rect, options)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.inner.borrow().scroll_layout_dependency()
    }

    fn child_count(&self) -> Option<usize> { self.inner.borrow().child_count() }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        self.inner.get_mut().set_child_extent(child, extent)
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        self.inner.get_mut().invalidate_child_measurement(child);
    }

    fn set_child_keep_alive(&mut self, child: SliverChildId, tile: Option<usize>, keep: bool) -> bool {
        self.inner.get_mut().set_child_keep_alive(child, tile, keep)
    }

    fn set_child_focus_keep_alive(&mut self, child: SliverChildId, tile: Option<usize>, keep: bool) -> bool {
        self.inner.get_mut().set_child_focus_keep_alive(child, tile, keep)
    }

    fn revision(&self) -> u64 { self.inner.borrow().revision() }
    fn tick(&mut self, now: Instant) -> bool { self.inner.get_mut().tick(now) }
    fn is_animating(&self) -> bool { self.inner.borrow().is_animating() }
    fn set_reduced_motion(&mut self, reduced: bool) { self.inner.get_mut().set_reduced_motion(reduced); }
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> { Some(self) }
}

pub(super) struct WidgetWrapRenderSliver {
    pub(super) inner: RefCell<Box<dyn RenderSliver>>,
    pub(super) wrap: Rc<dyn Fn(Widget) -> Widget>,
    /// Hidden wrappers keep the inner children laid out but take no scroll
    /// or paint extent, matching Flutter's offstage and hidden slivers.
    pub(super) hidden: bool,
}

pub(super) struct LayoutBuilderRenderSliver {
    pub(super) builder: SliverLayoutBuilderFn,
    pub(super) child: Widget,
    pub(super) extent: Cell<f32>,
    pub(super) last_constraints: Option<SliverConstraints>,
    pub(super) revision: u64,
}

impl RenderSliver for LayoutBuilderRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        if self.last_constraints != Some(constraints) {
            self.child = (self.builder)(constraints);
            self.last_constraints = Some(constraints);
            self.revision = self.revision.checked_add(1).expect("revision exhausted");
        }
        let extent = self.extent.get().max(0.);
        SliverLayout {
            geometry: SliverGeometry::from_scroll_extent(constraints, extent),
            children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: self.child.clone(),
                semantic_index: None,
                offset: 0.,
                cross_offset: 0.,
                constraints: sliver_child_constraints(
                    constraints.axis,
                    constraints.cross_axis_extent,
                    None,
                ),
                extent,
                placement: SliverChildPlacement::Flow,
            }],
            absorbed_overlap: 0.,
        }
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        if child.0 != 0 || !extent.is_finite() || extent < 0. {
            return false;
        }
        let changed = (self.extent.get() - extent).abs() > f32::EPSILON;
        self.extent.set(extent.max(0.));
        changed
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

/// Shared overlap state between an outer absorber and an inner injector.
#[derive(Clone, Default)]
pub struct SliverOverlapHandle {
    extent: Rc<Cell<f32>>,
}

impl SliverOverlapHandle {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn extent(&self) -> f32 {
        self.extent.get()
    }

    pub(super) fn set_extent(&self, extent: f32) {
        self.extent.set(extent.max(0.));
    }
}

pub(super) struct OverlapAbsorberRenderSliver {
    pub(super) inner: RefCell<Box<dyn RenderSliver>>,
    pub(super) handle: SliverOverlapHandle,
}

impl RenderSliver for OverlapAbsorberRenderSliver {
    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        self.inner.get_mut().reveal_child(child, rect, options)
    }

    fn set_child_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.inner
            .borrow_mut()
            .set_child_keep_alive(child, grid_tile, keep_alive)
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.inner
            .borrow_mut()
            .set_child_focus_keep_alive(child, grid_tile, keep_alive)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.inner.borrow().scroll_layout_dependency()
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let layout = self.inner.borrow_mut().perform_layout(constraints);
        self.handle.set_extent(layout.absorbed_overlap);
        layout
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        self.inner.borrow_mut().set_child_extent(child, extent)
    }

    fn revision(&self) -> u64 {
        self.inner.borrow().revision()
    }

    fn tick(&mut self, now: Instant) -> bool {
        self.inner.borrow_mut().tick(now)
    }

    fn is_animating(&self) -> bool {
        self.inner.borrow().is_animating()
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        self.inner.borrow_mut().set_reduced_motion(reduced);
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        self.inner.borrow_mut().invalidate_child_measurement(child);
    }
}

pub(super) struct OverlapInjectorRenderSliver {
    pub(super) handle: SliverOverlapHandle,
}

impl RenderSliver for OverlapInjectorRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let extent = self.handle.extent();
        SliverLayout::empty(SliverGeometry::from_scroll_extent(constraints, extent))
    }
}

impl RenderSliver for WidgetWrapRenderSliver {
    fn reported_cross_axis_extent(&self) -> Option<f32> {
        (!self.hidden).then(|| self.inner.borrow().reported_cross_axis_extent()).flatten()
    }

    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        self.inner.get_mut().reveal_child(child, rect, options)
    }

    fn set_child_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.inner
            .borrow_mut()
            .set_child_keep_alive(child, grid_tile, keep_alive)
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.inner
            .borrow_mut()
            .set_child_focus_keep_alive(child, grid_tile, keep_alive)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.inner.borrow().scroll_layout_dependency()
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let mut layout = self.inner.borrow_mut().perform_layout(constraints);
        if self.hidden {
            layout.geometry = SliverGeometry::ZERO;
            layout.absorbed_overlap = 0.;
        }
        for child in &mut layout.children {
            let mut inner = child.widget.clone();
            let clips = inner.take_sliver_layers();
            child.widget = (self.wrap)(inner).restore_sliver_layers(clips);
        }
        layout
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        self.inner.borrow_mut().invalidate_child_measurement(child);
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        self.inner.borrow_mut().set_child_extent(child, extent)
    }

    fn revision(&self) -> u64 {
        self.inner.borrow().revision()
    }

    fn tick(&mut self, now: Instant) -> bool {
        self.inner.borrow_mut().tick(now)
    }

    fn is_animating(&self) -> bool {
        self.inner.borrow().is_animating()
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        self.inner.borrow_mut().set_reduced_motion(reduced);
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

/// Sequential viewport implementation shared by `CustomScrollView` and
/// sliver groups. It computes the same per-sliver constraint fields used by a
/// real viewport and scopes child identity at each boundary.
pub(super) struct SequenceRenderSliver {
    pub(super) children: Vec<RefCell<Box<dyn RenderSliver>>>,
    reveal_geometry: RefCell<Vec<SliverRevealGeometry>>,
}

impl SequenceRenderSliver {
    pub(super) fn new(children: Vec<Box<dyn RenderSliver>>) -> Self {
        let reveal_geometry = RefCell::new(vec![SliverRevealGeometry::default(); children.len()]);
        Self {
            children: children.into_iter().map(RefCell::new).collect(),
            reveal_geometry,
        }
    }

    fn layout_sequence(&self, constraints: SliverConstraints) -> SliverLayout {
        self.layout_run(constraints, 0..self.children.len(), 0.)
    }

    /// Lays out `slivers` (indices into the children) one after another in
    /// that order, as Flutter `RenderViewportBase.layoutChildSequence` walks
    /// from one child with `advance`. Child identities stay scoped by each
    /// sliver's own index, whatever the walking order.
    ///
    /// `parked` shifts where slivers that are not visible and not scrolled
    /// past are placed, for a run whose Flutter `initialLayoutOffset` is
    /// clamped to the viewport while its start lies beyond it.
    fn layout_run(
        &self,
        constraints: SliverConstraints,
        slivers: impl Iterator<Item = usize>,
        parked: f32,
    ) -> SliverLayout {
        // These are the same independent cursors used by Flutter's
        // RenderViewport.layoutChildSequence. `preceding_scroll_extent` is
        // the logical scroll range consumed by previous slivers, while
        // `layout_offset` is the paint/layout cursor. They intentionally
        // diverge when a sliver is pinned, overlaps content, or has already
        // moved past the trailing edge of the viewport.
        let initial_layout_offset = 0.;
        let mut layout_offset = initial_layout_offset;
        let mut remaining_cache_extent = constraints.remaining_cache_extent;
        let mut cache_origin = constraints.cache_origin;
        let mut scroll_offset = constraints.scroll_offset;
        let mut max_paint_offset = layout_offset + constraints.overlap;
        let mut preceding = 0.;
        let mut preceding_obstruction = 0.;
        let mut children = Vec::new();
        let mut total_correction = None;
        let mut visual_overflow = false;
        for sliver_index in slivers {
            let sliver = &self.children[sliver_index];
            let sliver_scroll_offset = scroll_offset.max(0.);
            // A sliver must not be asked to cache content before its local
            // scroll offset. This is the same corrected cache-origin rule
            // used by Flutter's viewport and is important when a viewport is
            // partially scrolled into a preceding sliver.
            let corrected_cache_origin = cache_origin.max(-sliver_scroll_offset);
            let cache_extent_correction = cache_origin - corrected_cache_origin;
            let mut sliver_constraints = SliverConstraints::new(
                constraints.axis,
                constraints.reverse,
                sliver_scroll_offset,
                preceding,
                max_paint_offset - layout_offset,
                (constraints.remaining_paint_extent - layout_offset + initial_layout_offset)
                    .max(0.),
                constraints.cross_axis_extent,
                constraints.viewport_main_axis_extent,
                (remaining_cache_extent + cache_extent_correction).max(0.),
                corrected_cache_origin,
            );
            sliver_constraints.text_direction = constraints.text_direction;
            sliver_constraints.growth_direction = constraints.growth_direction;
            sliver_constraints.safe_insets = constraints.safe_insets;
            let layout = sliver.borrow_mut().perform_layout(sliver_constraints);
            let geometry = layout.geometry.normalized();
            if total_correction.is_none() {
                total_correction = geometry.scroll_offset_correction;
            }

            // Flutter restarts the sequence at the first correction. Keep the
            // already laid out prefix so the retained tree remains coherent;
            // the viewport will apply the correction and run this sequence
            // again before painting.
            if geometry.scroll_offset_correction.is_some() {
                break;
            }

            // Flutter's describeApproximatePaintClip adjusts each sliver's
            // leading edge by its own overlap, not by the viewport-wide
            // obstruction total. The cache still includes the clipped rows.
            let semantic_clip_inset = if sliver_constraints.overlap != 0.
                && sliver_constraints.viewport_main_axis_extent.is_finite()
            {
                sliver_constraints.viewport_main_axis_extent
                    - sliver_constraints.remaining_paint_extent
                    + sliver_constraints.overlap
            } else {
                0.
            };
            self.reveal_geometry.borrow_mut()[sliver_index] = SliverRevealGeometry {
                pins_at_edge: geometry.max_scroll_obstruction_extent > 0.,
                obstruction_before: preceding_obstruction,
                reverse_growth: constraints.growth_direction == SliverGrowthDirection::Reverse,
                semantic_clip_inset,
                outside_paint_and_cache: !geometry.visible && geometry.cache_extent <= 0.,
            };
            preceding_obstruction = (preceding_obstruction + geometry.max_scroll_obstruction_extent)
                .min(f32::MAX);
            visual_overflow |= geometry.has_visual_overflow;
            let effective_layout_offset = layout_offset + geometry.paint_origin;
            // Once a sliver is past the trailing edge its effective paint
            // offset is no longer meaningful. Its increasing scroll cursor is
            // still useful for retaining a stable content ordering, matching
            // RenderViewport's fallback placement for invisible slivers.
            let sliver_paint_offset = if geometry.visible || scroll_offset > 0. {
                effective_layout_offset
            } else {
                -scroll_offset + initial_layout_offset + parked
            };
            children.reserve(layout.children.len());
            for mut child in layout.children {
                child.id = SliverChildId::scoped(sliver_index, child.id);
                // The retained tree applies one viewport-level transform.
                // Convert Flutter's paint-space child position back into the
                // sequence's content space so that transform produces the
                // same result for normal, overlapping, and pinned slivers.
                child.offset = sliver_paint_offset + child.offset - sliver_scroll_offset
                    + constraints.scroll_offset;
                children.push(child);
            }

            max_paint_offset =
                max_paint_offset.max(effective_layout_offset + geometry.paint_extent);
            preceding += geometry.scroll_extent;
            scroll_offset -= geometry.scroll_extent;
            layout_offset += geometry.layout_extent;
            if geometry.cache_extent != 0. {
                remaining_cache_extent -= geometry.cache_extent - cache_extent_correction;
                cache_origin = (corrected_cache_origin + geometry.cache_extent).min(0.);
            }
            // Custom slivers may expose an overlap that is not represented by
            // their paint extent. Preserve it as an explicit obstruction;
            // built-in pinned headers report exactly paintExtent-layoutExtent
            // here, so this does not double-count them.
            max_paint_offset =
                max_paint_offset.max(layout_offset + layout.absorbed_overlap.max(0.));
        }
        apply_pinned_offsets(
            &mut children,
            constraints.scroll_offset,
            constraints.viewport_main_axis_extent,
        );
        let mut geometry = SliverGeometry::from_scroll_extent(constraints, preceding);
        geometry.has_visual_overflow |= visual_overflow;
        geometry.scroll_offset_correction = total_correction;
        SliverLayout {
            geometry,
            children,
            absorbed_overlap: (max_paint_offset - layout_offset).max(0.),
        }
    }
}

impl RenderSliver for SequenceRenderSliver {
    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        self.children
            .get_mut(child.scope())?
            .get_mut()
            .reveal_child(child.local(), rect, options)
    }

    fn set_child_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.children.get(child.scope()).is_some_and(|sliver| {
            sliver
                .borrow_mut()
                .set_child_keep_alive(child.local(), grid_tile, keep_alive)
        })
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.children.get(child.scope()).is_some_and(|sliver| {
            sliver
                .borrow_mut()
                .set_child_focus_keep_alive(child.local(), grid_tile, keep_alive)
        })
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.children
            .iter()
            .map(|child| child.borrow().scroll_layout_dependency())
            .max()
            .unwrap_or(SliverScrollDependency::CacheWindow)
    }

    fn child_count(&self) -> Option<usize> {
        self.children.iter().try_fold(0usize, |count, child| {
            child
                .borrow()
                .child_count()
                .map(|child_count| count + child_count)
        })
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        self.layout_sequence(constraints)
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        let index = child.scope();
        self.children
            .get(index)
            .is_some_and(|sliver| sliver.borrow_mut().set_child_extent(child.local(), extent))
    }

    fn revision(&self) -> u64 {
        self.children
            .iter()
            .map(|child| child.borrow().revision())
            .fold(0, u64::wrapping_add)
    }

    fn tick(&mut self, now: Instant) -> bool {
        // Every sibling observes this frame, even when an earlier sliver
        // has already changed. A short-circuiting any() starves later tickers.
        self.children.iter().fold(false, |changed, child| {
            child.borrow_mut().tick(now) | changed
        })
    }

    fn is_animating(&self) -> bool {
        self.children
            .iter()
            .any(|child| child.borrow().is_animating())
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        for child in &self.children {
            child.borrow_mut().set_reduced_motion(reduced);
        }
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        // Route exactly like set_child_extent so invalidation reaches the
        // sliver that owns the rebuilt descendant and no other.
        let index = child.scope();
        if let Some(sliver) = self.children.get(index) {
            sliver
                .borrow_mut()
                .invalidate_child_measurement(child.local());
        }
    }
}

/// Transfers compatible header state and provisional variable-row extents,
/// recursing by position through sequences and transparent wrappers.
pub(crate) fn transfer_retained_sliver_state(
    fresh: &mut dyn RenderSliver,
    retained: &mut dyn RenderSliver,
    probes: &mut u64,
    preserve_anchor: bool,
) {
    if let (Some(fresh), Some(retained)) = (
        fresh
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<AnimatedOpacityRenderSliver>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<AnimatedOpacityRenderSliver>()),
    ) {
        fresh.adopt(retained);
        let mut fresh_inner = fresh.inner_cell().borrow_mut();
        let mut retained_inner = retained.inner_cell().borrow_mut();
        transfer_retained_sliver_state(
            &mut **fresh_inner,
            &mut **retained_inner,
            probes,
            preserve_anchor,
        );
        return;
    }
    if adopt_indexed_keep_alives(fresh, retained, probes)
        || adopt_natural_header_state(fresh, retained)
        || adopt_box_extent(fresh, retained)
        || adopt_floating_header_state(fresh, retained)
        || adopt_variable_extent_measurements(fresh, retained, probes, preserve_anchor)
    {
        return;
    }
    if let (Some(fresh), Some(retained)) = (
        fresh
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<SequenceRenderSliver>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<SequenceRenderSliver>()),
    ) {
        for (fresh_child, retained_child) in fresh.children.iter().zip(retained.children.iter()) {
            let mut fresh_sliver = fresh_child.borrow_mut();
            let mut retained_sliver = retained_child.borrow_mut();
            transfer_retained_sliver_state(
                &mut **fresh_sliver,
                &mut **retained_sliver,
                probes,
                preserve_anchor,
            );
        }
        return;
    }
    if let (Some(fresh), Some(retained)) = (
        fresh
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<CrossAxisGroupRenderSliver>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<CrossAxisGroupRenderSliver>()),
    ) {
        fresh.adopt_compatible_state(retained, probes, preserve_anchor);
        return;
    }
    transfer_single_inner_sliver_state::<super::sliver_clip::ClipRenderSliver>(
        fresh,
        retained,
        probes,
        preserve_anchor,
    );
    transfer_single_inner_sliver_state::<super::sliver_layer::OpacityRenderSliver>(
        fresh,
        retained,
        probes,
        preserve_anchor,
    );
    transfer_single_inner_sliver_state::<PaddingRenderSliver>(
        fresh,
        retained,
        probes,
        preserve_anchor,
    );
    transfer_single_inner_sliver_state::<super::sliver_decoration::DecoratedRenderSliver>(
        fresh, retained, probes, preserve_anchor,
    );
    transfer_single_inner_sliver_state::<ConstrainedCrossAxisRenderSliver>(
        fresh, retained, probes, preserve_anchor,
    );
    transfer_single_inner_sliver_state::<WidgetWrapRenderSliver>(
        fresh,
        retained,
        probes,
        preserve_anchor,
    );
    transfer_single_inner_sliver_state::<OverlapAbsorberRenderSliver>(
        fresh,
        retained,
        probes,
        preserve_anchor,
    );
    transfer_single_inner_sliver_state::<super::sliver_semantics::SemanticsRenderSliver>(
        fresh,
        retained,
        probes,
        preserve_anchor,
    );
}

fn adopt_natural_header_state(
    fresh: &mut dyn RenderSliver,
    retained: &mut dyn RenderSliver,
) -> bool {
    let (Some(fresh), Some(retained)) = (
        fresh
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<NaturalHeaderRenderSliver>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<NaturalHeaderRenderSliver>()),
    ) else {
        return false;
    };
    fresh.adopt_compatible_state(retained)
}

/// Carries positional estimates to avoid resetting unchanged lazy content to
/// the global fallback. No row correspondence is implied: explicit seeds win,
/// reused elements establish identity, and fresh elements invalidate guesses.
/// Offscreen totals remain estimates until those rows are materialized.
/// Rebuild only the formerly kept entries with the new delegate. A
/// matching descriptor keeps its focus request, but its explicit request
/// always comes from the new widget. If the static list can resolve keys,
/// remap a parked child by key instead of pinning it to its prior index.
/// Unrelated off-cache widgets are never built.
fn transfer_keep_alives(
    retained: &HashMap<usize, Widget>,
    fresh: &mut HashMap<usize, Widget>,
    count: usize,
    build: impl Fn(usize) -> Widget,
    find_child_index: Option<&ListIndexLookup>,
    probes: &mut u64,
) {
    let mut indices: Vec<_> = retained
        .iter()
        .filter_map(|(&index, widget)| {
            (widget.keep_alive_request()
                || (widget.key().is_some() && find_child_index.is_some_and(|lookup| lookup.include_active)))
                .then_some(index)
        })
        .collect();
    indices.sort_unstable();
    for index in indices {
        let old = &retained[&index];
        let destination = match (old.key(), find_child_index) {
            (Some(key), Some(lookup)) => (lookup.index)(key).unwrap_or(index),
            _ => index,
        };
        if !old.keep_alive_request() || destination >= count || fresh.contains_key(&destination) {
            continue;
        }
        *probes += 1;
        let new = build(destination);
        let same_child = old.key() == new.key()
            && std::mem::discriminant(old.kind()) == std::mem::discriminant(new.kind());
        if !new.keep_alive_request() || same_child {
            fresh.insert(
                destination,
                new.with_focus_keep_alive_request(same_child && old.focus_keep_alive_request()),
            );
        }
    }
}

fn adopt_indexed_keep_alives(
    fresh: &mut dyn RenderSliver,
    retained: &mut dyn RenderSliver,
    probes: &mut u64,
) -> bool {
    if let (Some(fresh), Some(retained)) = (
        fresh
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<FixedExtentRenderSliver>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<FixedExtentRenderSliver>()),
    ) {
        transfer_keep_alives(
            &retained.widgets,
            &mut fresh.widgets,
            fresh.item_count,
            |index| (fresh.builder)(index),
            fresh.find_child_index.as_ref(),
            probes,
        );
        return true;
    }
    if let (Some(fresh), Some(retained)) = (
        fresh
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<ViewportExtentRenderSliver>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<ViewportExtentRenderSliver>()),
    ) {
        if fresh.find_child_index.is_some() {
            fresh.adopt_keyed_keep_alives(retained, probes);
            return true;
        }
        let children = fresh.children.as_ref();
        let builder = fresh.builder.as_ref();
        transfer_keep_alives(
            &retained.widgets,
            &mut fresh.widgets,
            fresh.item_count,
            |index| {
                children
                    .and_then(|children| children.get(index))
                    .cloned()
                    .or_else(|| builder.map(|builder| builder(index)))
                    .unwrap_or_else(|| SizedBox::shrink().into())
            },
            None,
            probes,
        );
        return true;
    }
    if let (Some(fresh), Some(retained)) = (
        fresh
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<GridRenderSliver>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<GridRenderSliver>()),
    ) {
        transfer_keep_alives(
            &retained.widgets,
            &mut fresh.widgets,
            fresh.item_count,
            |index| (fresh.builder)(index),
            None,
            probes,
        );
        return true;
    }
    false
}

fn adopt_variable_extent_measurements(
    fresh: &mut dyn RenderSliver,
    retained: &mut dyn RenderSliver,
    probes: &mut u64,
    preserve_anchor: bool,
) -> bool {
    let (Some(fresh), Some(retained)) = (
        fresh
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<VariableExtentRenderSliver>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<VariableExtentRenderSliver>()),
    ) else {
        return false;
    };
    // Iterate retained-measured rows only: unmeasured slots carry
    // nothing to adopt. Fresh seeds always win; only bare fallbacks
    // accept a carried measurement, recorded so reconciliation can
    // tell transferred guesses apart from explicit seeds. This
    // transfer only shields totals and first paint; per-row truth is
    // established post-mapping below.
    transfer_keep_alives(
        &retained.widgets,
        &mut fresh.widgets,
        fresh.index.len(),
        |index| (fresh.builder)(index),
        fresh.find_child_index.as_ref(),
        probes,
    );
    let fresh_len = fresh.index.len();
    if preserve_anchor && fresh.sample_scroll_extent && retained.sample_scroll_extent {
        fresh.last_sampled_extent = retained.last_sampled_extent;
    }
    if preserve_anchor && !fresh.tight_main_extent && !retained.tight_main_extent {
        fresh.leading_anchor = retained
            .leading_anchor
            .as_ref()
            .filter(|anchor| anchor.index < fresh_len)
            .cloned();
    }
    let mut adopted = false;
    let before = retained.index.metrics();
    let indices = retained.index.measured_indices();
    let after = retained.index.metrics();
    *probes += after.enumeration_chunks - before.enumeration_chunks + after.enumeration_slots
        - before.enumeration_slots;
    for index in indices {
        if index >= fresh_len {
            break;
        }
        *probes += 1;
        if fresh.index.measured_extent(index).is_some() {
            continue;
        }
        if let Some(extent) = retained.index.measured_extent(index)
            && fresh.index.set_measured_extent(index, extent)
        {
            adopted = true;
            fresh.transferred.borrow_mut().insert(index);
        }
    }
    adopted
}

struct VariableSlot {
    index: MeasuredExtentIndex,
    local: usize,
    transferred: Rc<RefCell<HashSet<usize>>>,
}

/// Resolves a viewport-scoped child to the shared index, local row, and
/// transfer-provenance set from the same variable sliver. Descends sequences
/// by scope and through the transparent wrappers used by state transfer.
/// Non-variable shapes return `None` and manage their own measurements.
fn variable_slot_for_child(
    delegate: &dyn SliverViewportDelegate,
    child: SliverChildId,
) -> Option<VariableSlot> {
    fn slot_in_sliver(sliver: &mut dyn RenderSliver, child: SliverChildId) -> Option<VariableSlot> {
        if let Some(variable) = sliver
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<VariableExtentRenderSliver>())
        {
            return Some(VariableSlot {
                index: variable.index.clone(),
                local: usize::try_from(child.0.checked_sub(1)?).ok()?,
                transferred: variable.transferred.clone(),
            });
        }
        if let Some(wrapper) = sliver.as_any_mut()
            .and_then(|any| any.downcast_mut::<ConstrainedCrossAxisRenderSliver>())
        {
            return slot_in_sliver(&mut **wrapper.inner.borrow_mut(), child);
        }
        if let Some(wrapper) = sliver.as_any_mut()
            .and_then(|any| any.downcast_mut::<super::sliver_decoration::DecoratedRenderSliver>())
        {
            let child = super::sliver_decoration::DecoratedRenderSliver::content_id(child)?;
            return slot_in_sliver(&mut **wrapper.inner.borrow_mut(), child);
        }
        if let Some(wrapper) = sliver.as_any_mut()
            .and_then(|any| any.downcast_mut::<PaddingRenderSliver>())
        {
            return slot_in_sliver(&mut **wrapper.inner.borrow_mut(), child);
        }
        if let Some(wrapper) = sliver.as_any_mut()
            .and_then(|any| any.downcast_mut::<WidgetWrapRenderSliver>())
        {
            return slot_in_sliver(&mut **wrapper.inner.borrow_mut(), child);
        }
        if let Some(wrapper) = sliver.as_any_mut()
            .and_then(|any| any.downcast_mut::<OverlapAbsorberRenderSliver>())
        {
            return slot_in_sliver(&mut **wrapper.inner.borrow_mut(), child);
        }
        None
    }
    let sequence = delegate
        .as_any()?
        .downcast_ref::<SequenceViewportDelegate>()?;
    let sequence = sequence.sequence.borrow();
    let sliver = sequence.children.get(child.scope())?;
    slot_in_sliver(&mut **sliver.borrow_mut(), child.local())
}

/// Records an identity-established measurement after element
/// reconciliation: `extent` is the retained size of the reused old
/// element now sitting at `child`. Applies only to previously
/// unseeded slots — fresh explicit seeds always stand — and reports
/// whether the index changed. Non-variable shapes report false.
pub(crate) fn adopt_reconciled_measurement(
    delegate: &dyn SliverViewportDelegate,
    child: SliverChildId,
    extent: f32,
) -> bool {
    let Some(slot) = variable_slot_for_child(delegate, child) else {
        return false;
    };
    if !extent.is_finite() || extent < 0. {
        return false;
    }
    let provisional = slot.transferred.borrow_mut().remove(&slot.local);
    if !provisional && slot.index.measured_extent(slot.local).is_some() {
        return false;
    }
    slot.index.set_measured_extent(slot.local, extent)
}

/// Demotes a transferred guess for a fresh-mounted child back to an
/// estimate. Fires only for slots the last transfer wrote without
/// established identity, so explicit seeds and untouched fallbacks
/// are never disturbed. Non-variable shapes report false.
pub(crate) fn invalidate_transferred_measurement(
    delegate: &dyn SliverViewportDelegate,
    child: SliverChildId,
) -> bool {
    let Some(slot) = variable_slot_for_child(delegate, child) else {
        return false;
    };
    if !slot.transferred.borrow_mut().remove(&slot.local) {
        return false;
    }
    slot.index.invalidate_extent(slot.local)
}

/// Adopts a measured box extent so replacing a viewport descriptor does not
/// flash the lazy default and spuriously move the scroll range. Pinning is
/// presentation-only; the recorded `measured_extent` transfers regardless.
fn adopt_box_extent(fresh: &mut dyn RenderSliver, retained: &mut dyn RenderSliver) -> bool {
    let (Some(fresh), Some(retained)) = (
        fresh
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<BoxRenderSliver>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<BoxRenderSliver>()),
    ) else {
        return false;
    };
    let measured = retained.extent.get();
    if (fresh.extent.get() - measured).abs() <= f32::EPSILON {
        return false;
    }
    fresh.extent.set(measured);
    true
}

/// Adopts a measured floating extent with its reversal tracking so a
/// replacement keeps revealing without replaying hidden distance.
fn adopt_floating_header_state(
    fresh: &mut dyn RenderSliver,
    retained: &mut dyn RenderSliver,
) -> bool {
    let (Some(fresh), Some(retained)) = (
        fresh
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<FloatingHeaderRenderSliver>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<FloatingHeaderRenderSliver>()),
    ) else {
        return false;
    };
    if (fresh.extent.get() - retained.extent.get()).abs() > f32::EPSILON {
        fresh.extent.set(retained.extent.get());
    }
    fresh.scroll_state = retained.scroll_state.clone();
    // The fresh subscription has seen nothing yet: carry the observed
    // activity, direction and any unconsumed transition across.
    fresh.signal.set(retained.signal.get());
    if fresh.snap != retained.snap {
        // A running snap belongs to the retained snap configuration. Without
        // an identical flag the fresh header settles to scroll-determined
        // presentation instead of inheriting a foreign animation.
        fresh.scroll_state.cancel_snap();
    }
    true
}

pub(super) trait SingleInnerSliver {
    fn adopt_outer_state(&mut self, _old: &mut Self) {}

    fn inner_cell(&mut self) -> &RefCell<Box<dyn RenderSliver>>;
}

impl SingleInnerSliver for ConstrainedCrossAxisRenderSliver {
    fn inner_cell(&mut self) -> &RefCell<Box<dyn RenderSliver>> {
        &self.inner
    }
}

impl SingleInnerSliver for PaddingRenderSliver {
    fn inner_cell(&mut self) -> &RefCell<Box<dyn RenderSliver>> {
        &self.inner
    }
}

impl SingleInnerSliver for WidgetWrapRenderSliver {
    fn inner_cell(&mut self) -> &RefCell<Box<dyn RenderSliver>> {
        &self.inner
    }
}

impl SingleInnerSliver for OverlapAbsorberRenderSliver {
    fn inner_cell(&mut self) -> &RefCell<Box<dyn RenderSliver>> {
        &self.inner
    }
}

fn transfer_single_inner_sliver_state<T: SingleInnerSliver + 'static>(
    fresh: &mut dyn RenderSliver,
    retained: &mut dyn RenderSliver,
    probes: &mut u64,
    preserve_anchor: bool,
) {
    if let (Some(fresh), Some(retained)) = (
        fresh.as_any_mut().and_then(|any| any.downcast_mut::<T>()),
        retained
            .as_any_mut()
            .and_then(|any| any.downcast_mut::<T>()),
    ) {
        fresh.adopt_outer_state(retained);
        let mut fresh_inner = fresh.inner_cell().borrow_mut();
        let mut retained_inner = retained.inner_cell().borrow_mut();
        transfer_retained_sliver_state(
            &mut **fresh_inner,
            &mut **retained_inner,
            probes,
            preserve_anchor,
        );
    }
}

pub(super) struct SequenceViewportDelegate {
    pub(super) sequence: RefCell<SequenceRenderSliver>,
    /// Last reduced-motion policy stamped into snap-capable descendants.
    /// `None` forces the first push, so freshly built delegates (including
    /// replacements) observe the current policy on their first frame without
    /// any rebuild. Steady-state pushes early-out on equality.
    snap_policy: Cell<Option<bool>>,
    /// Where scroll offset zero sits (Flutter `Viewport.center` and `anchor`).
    center: ViewportCenter,
}

/// Flutter `Viewport.center` and `anchor`: the sliver at `index` starts at
/// scroll offset zero, which sits `anchor` of the way down the viewport.
/// Slivers before `index` grow in reverse from there.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct ViewportCenter {
    pub(super) index: usize,
    pub(super) anchor: f32,
}

impl ViewportCenter {
    /// Whether the viewport lays out like a plain forward sequence.
    fn is_leading(self) -> bool {
        self.index == 0 && self.anchor == 0.
    }
}

impl SequenceViewportDelegate {
    pub(super) fn new(slivers: Vec<Box<dyn RenderSliver>>) -> Self {
        Self::centered(slivers, ViewportCenter::default())
    }

    pub(super) fn centered(slivers: Vec<Box<dyn RenderSliver>>, center: ViewportCenter) -> Self {
        let count = slivers.len();
        Self {
            sequence: RefCell::new(SequenceRenderSliver::new(slivers)),
            snap_policy: Cell::new(None),
            center: ViewportCenter {
                index: center.index.min(count.saturating_sub(1)),
                anchor: if center.anchor.is_finite() {
                    center.anchor.clamp(0., 1.)
                } else {
                    0.
                },
            },
        }
    }

    /// Flutter `RenderViewport._attemptLayout` around a center: the slivers
    /// before the center lay out in reverse from it, nearest first, and the
    /// center and the slivers after it lay out forward. Each side gets the
    /// paint and cache budget on its side of the center line.
    ///
    /// `constraints` are the viewport's forward constraints, whose scroll
    /// offset and overlap together carry the unclamped position. Offset zero
    /// is content position `anchor * extent`: forward children sit after it
    /// and reverse children mirror before it.
    fn layout_centered(&self, constraints: SliverConstraints) -> SliverViewportLayout {
        let sequence = self.sequence.borrow();
        let extent = constraints.viewport_main_axis_extent;
        let cache = (-constraints.cache_origin).max(0.);
        let pixels = constraints.scroll_offset + constraints.overlap;
        let zero = extent * self.center.anchor;
        // `centerOffset`: the center line's distance from the leading edge.
        let center_offset = zero - pixels;
        let has_reverse = self.center.index > 0;
        // One side of the center, scrolled `scroll` past its own start: the
        // reverse side starts at the trailing edge and grows toward the
        // leading one.
        let side = |scroll: f32, overlap: f32| {
            let mut side = SliverConstraints::new(
                constraints.axis,
                constraints.reverse,
                scroll.max(0.),
                0.,
                overlap,
                (extent + scroll).clamp(0., extent),
                constraints.cross_axis_extent,
                extent,
                (extent + cache + scroll).clamp(0., extent + 2. * cache),
                (-scroll).clamp(-cache, 0.),
            );
            side.text_direction = constraints.text_direction;
            side.safe_insets = constraints.safe_insets;
            side
        };

        let mut children = Vec::new();
        let mut leading_extent = 0.;
        let mut correction = None;
        let mut reverse_overflow = false;
        if has_reverse {
            let mut reverse_constraints = side(center_offset - extent, 0.);
            reverse_constraints.growth_direction = SliverGrowthDirection::Reverse;
            let reverse = sequence.layout_run(
                reverse_constraints,
                (0..self.center.index).rev(),
                // Flutter starts the reverse run at the viewport's leading
                // edge while the center line is above it.
                center_offset.min(0.),
            );
            // Reverse content that moved shifts the position the other way.
            // Like a forward sequence, the laid out children stay so the
            // retained tree remains coherent until the corrected layout.
            correction = reverse
                .geometry
                .scroll_offset_correction
                .map(|delta| -delta);
            reverse_overflow = reverse.geometry.has_visual_overflow;
            leading_extent = (reverse.geometry.scroll_extent - zero).max(0.);
            children = reverse.children;
            // Mirror about the center line; the viewport keeps the
            // trailing edge when the measured extent differs.
            for child in &mut children {
                child.offset = zero - child.offset - child.extent;
            }
            // Keep each sliver's children together in sliver order, as the
            // paint and semantics walks expect.
            children.sort_by_key(|child: &SliverChildLayout| child.id.scope());
        }
        let forward_overlap = if has_reverse {
            0.
        } else {
            (-center_offset).min(0.)
        };
        let forward = sequence.layout_run(
            side(-center_offset, forward_overlap),
            self.center.index..sequence.children.len(),
            0.,
        );
        children.reserve(forward.children.len());
        children.extend(forward.children.into_iter().map(|mut child| {
            child.offset += zero;
            child
        }));
        // Flutter `applyContentDimensions(min(0, min + extent * anchor),
        // max(0, max - extent * (1 - anchor)))`: the published content is
        // the leading extent plus everything from offset zero to the end.
        let mut geometry =
            SliverGeometry::from_scroll_extent(constraints, forward.geometry.scroll_extent + zero);
        geometry.has_visual_overflow |= reverse_overflow || forward.geometry.has_visual_overflow;
        geometry.scroll_offset_correction =
            correction.or(forward.geometry.scroll_offset_correction);
        SliverViewportLayout {
            geometry,
            children,
            leading_extent,
            reverse_slivers: self.center.index,
        }
    }
}

impl SliverViewportDelegate for SequenceViewportDelegate {
    fn set_child_keep_alive(
        &self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.sequence
            .borrow_mut()
            .set_child_keep_alive(child, grid_tile, keep_alive)
    }

    fn set_child_focus_keep_alive(
        &self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.sequence
            .borrow_mut()
            .set_child_focus_keep_alive(child, grid_tile, keep_alive)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        // Parked reverse slivers move with the position (see
        // `layout_centered`), so a viewport with slivers before its center
        // lays out again on every scroll.
        if self.center.index > 0 {
            return SliverScrollDependency::ScrollOffset;
        }
        self.sequence.borrow().scroll_layout_dependency()
    }

    fn perform_layout(&self, constraints: SliverConstraints) -> SliverViewportLayout {
        if !self.center.is_leading() {
            return self.layout_centered(constraints);
        }
        let layout = self.sequence.borrow_mut().perform_layout(constraints);
        SliverViewportLayout {
            geometry: layout.geometry,
            children: layout.children,
            leading_extent: 0.,
            reverse_slivers: 0,
        }
    }

    fn set_child_extent(&self, child: SliverChildId, extent: f32) -> bool {
        self.sequence.borrow_mut().set_child_extent(child, extent)
    }

    fn reveal_geometry(&self, child: SliverChildId) -> SliverRevealGeometry {
        self.sequence
            .borrow()
            .reveal_geometry
            .borrow()
            .get(child.scope())
            .copied()
            .unwrap_or_default()
    }

    fn reveal_child(
        &self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        self.sequence
            .try_borrow_mut()
            .ok()?
            .reveal_child(child, rect, options)
    }

    fn revision(&self) -> u64 {
        self.sequence.borrow().revision()
    }

    fn sliver_count(&self) -> usize {
        self.sequence.borrow().children.len()
    }

    fn child_count(&self) -> Option<usize> {
        self.sequence.borrow().child_count()
    }

    fn tick(&self, now: Instant) -> bool {
        self.sequence.borrow_mut().tick(now)
    }

    fn is_animating(&self) -> bool {
        self.sequence.borrow().is_animating()
    }

    fn set_reduced_motion(&self, reduced: bool) {
        if self.snap_policy.get() == Some(reduced) {
            return;
        }
        self.sequence.borrow_mut().set_reduced_motion(reduced);
        self.snap_policy.set(Some(reduced));
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn invalidate_sliver_child(&self, child: SliverChildId) -> bool {
        let sequence = self.sequence.borrow();
        let Some(sliver) = sequence.children.get(child.scope()) else {
            return false;
        };
        sliver
            .borrow_mut()
            .invalidate_child_measurement(child.local());
        true
    }

    fn adopt_compatible_state(
        &self,
        previous: &dyn SliverViewportDelegate,
        probes: &mut u64,
        preserve_anchor: bool,
    ) {
        let Some(previous) = previous
            .as_any()
            .and_then(|any| any.downcast_ref::<SequenceViewportDelegate>())
        else {
            return;
        };
        let preserve_anchor = preserve_anchor && self.center == previous.center;
        let fresh = self.sequence.borrow_mut();
        let retained = previous.sequence.borrow_mut();
        for (fresh_child, retained_child) in fresh.children.iter().zip(retained.children.iter()) {
            let mut fresh_sliver = fresh_child.borrow_mut();
            let mut retained_sliver = retained_child.borrow_mut();
            transfer_retained_sliver_state(
                &mut **fresh_sliver,
                &mut **retained_sliver,
                probes,
                preserve_anchor,
            );
        }
    }
}

/// Builds a viewport for one sliver. Box scrollables such as `ListView` and
/// `PageView` use this same retained sliver protocol as `CustomScrollView`
/// instead of maintaining a second lazy-list implementation.
pub(super) struct SliverViewportOptions {
    pub(super) controller: ScrollController,
    pub(super) axis: Axis,
    pub(super) reverse: bool,
    pub(super) physics: ScrollPhysics,
    pub(super) cache_extent: f32,
    pub(super) clip_behavior: Clip,
    pub(super) drag_start_behavior: DragStartBehavior,
    pub(super) hit_test_behavior: crate::gestures::HitTestBehavior,
    pub(super) shrink_wrap: bool,
}

pub(super) fn single_sliver_viewport(
    controller: ScrollController,
    axis: Axis,
    reverse: bool,
    physics: ScrollPhysics,
    cache_extent: f32,
    clip_behavior: Clip,
    sliver: Box<dyn RenderSliver>,
) -> Widget {
    single_sliver_viewport_with_options(
        SliverViewportOptions {
            controller,
            axis,
            reverse,
            physics,
            cache_extent,
            clip_behavior,
            drag_start_behavior: DragStartBehavior::Start,
            hit_test_behavior: crate::gestures::HitTestBehavior::Opaque,
            shrink_wrap: false,
        },
        sliver,
    )
}

pub(super) fn single_sliver_viewport_with_options(
    options: SliverViewportOptions,
    sliver: Box<dyn RenderSliver>,
) -> Widget {
    Widget::sliver_viewport_with_delegate_options(
        options.controller,
        options.axis,
        options.reverse,
        options.physics,
        options.cache_extent,
        options.shrink_wrap,
        options.clip_behavior,
        options.drag_start_behavior,
        options.hit_test_behavior,
        Rc::new(SequenceViewportDelegate::new(vec![sliver])),
    )
}

pub(super) fn sliver_child_constraints(axis: Axis, cross: f32, extent: Option<f32>) -> Constraints {
    let cross = cross.max(0.);
    match (axis, extent) {
        (Axis::Vertical, Some(extent)) => Constraints::new(cross, cross, extent, extent),
        (Axis::Horizontal, Some(extent)) => Constraints::new(extent, extent, cross, cross),
        (Axis::Vertical, None) => Constraints::new(cross, cross, 0., f32::INFINITY),
        (Axis::Horizontal, None) => Constraints::new(0., f32::INFINITY, cross, cross),
    }
}

/// Returns a conservative main-axis size for a widget that is used as a
/// sliver prototype. This is deliberately limited to dimensions that are
/// independent of the eventual viewport; widgets whose size depends on
/// ambient constraints return `None` and are measured by the retained child
/// pass instead.
pub(super) fn widget_main_extent_hint(widget: &Widget, axis: Axis) -> Option<f32> {
    fn finite(value: f32) -> Option<f32> {
        value.is_finite().then_some(value.max(0.))
    }

    fn dimension(size: Size, axis: Axis) -> Option<f32> {
        finite(axis.main_extent(size))
    }

    fn constrained(child: &Widget, axis: Axis, min: f32, max: f32) -> Option<f32> {
        let min = finite(min).unwrap_or(0.);
        let child = widget_main_extent_hint(child, axis);
        if max.is_finite() {
            let max = max.max(min);
            child
                .map(|value| value.clamp(min, max))
                .or_else(|| finite(max))
        } else {
            child.or_else(|| (min > 0.).then_some(min))
        }
    }

    let result = match widget.kind() {
        WidgetKind::Box { size, .. } => dimension(*size, axis),
        WidgetKind::Shape { size, path, .. } => {
            size.and_then(|size| dimension(size, axis)).or_else(|| {
                path.tight_bounds()
                    .and_then(|bounds| dimension(bounds.size, axis))
            })
        }
        WidgetKind::CustomPaint { size, .. } => size.and_then(|size| dimension(size, axis)),
        WidgetKind::Colored { child, .. } => child
            .as_ref()
            .map_or(Some(0.), |child| widget_main_extent_hint(child, axis)),
        WidgetKind::Decorated { size, child, .. } => size
            .and_then(|size| dimension(size, axis))
            .or_else(|| widget_main_extent_hint(child, axis)),
        WidgetKind::Banner { child, .. } => child
            .as_ref()
            .and_then(|child| widget_main_extent_hint(child, axis)),
        WidgetKind::Button(spec) => dimension(spec.size, axis)
            .filter(|extent| *extent > 0.)
            .or_else(|| {
                spec.child
                    .as_ref()
                    .and_then(|child| widget_main_extent_hint(child, axis))
            })
            .or_else(|| dimension(spec.size, axis)),
        WidgetKind::Text { style, .. } | WidgetKind::SelectableText { style, .. } => {
            let font_size = style.size.max(1.);
            let line_height = style
                .line_height
                .map_or(font_size * 1.2, |height| match height {
                    incular_text::LineHeight::Normal => font_size * 1.2,
                    incular_text::LineHeight::Multiplier(multiplier) => {
                        font_size * multiplier.max(0.)
                    }
                    incular_text::LineHeight::Absolute(pixels) => pixels.max(0.),
                });
            finite(line_height)
        }
        WidgetKind::Image { width, height, .. } => finite(match axis {
            Axis::Horizontal => width.unwrap_or(0.),
            Axis::Vertical => height.unwrap_or(0.),
        })
        .filter(|extent| *extent > 0.),
        WidgetKind::TextField(spec) => dimension(spec.size, axis),
        WidgetKind::Padding { padding, child } => {
            widget_main_extent_hint(child, axis).map(|extent| {
                extent
                    + if axis.is_vertical() {
                        padding.top + padding.bottom
                    } else {
                        padding.left + padding.right
                    }
            })
        }
        WidgetKind::Constrained { constraints, child } => constrained(
            child,
            axis,
            if axis.is_vertical() {
                constraints.min_height()
            } else {
                constraints.min_width()
            },
            if axis.is_vertical() {
                constraints.max_height()
            } else {
                constraints.max_width()
            },
        ),
        WidgetKind::Limited {
            max_width,
            max_height,
            child,
        } => {
            let max = if axis.is_vertical() {
                *max_height
            } else {
                *max_width
            };
            match child {
                Some(child) => widget_main_extent_hint(child, axis)
                    .map(|extent| extent.min(max))
                    .or_else(|| finite(max)),
                None => Some(0.),
            }
        }
        WidgetKind::SizedOverflow { size, .. } => finite(axis.main_extent(*size)),
        WidgetKind::Overflow {
            min_width,
            max_width,
            min_height,
            max_height,
            child,
            ..
        } => {
            let min = if axis.is_vertical() {
                min_height.unwrap_or(0.)
            } else {
                min_width.unwrap_or(0.)
            };
            let max = if axis.is_vertical() {
                max_height.unwrap_or(f32::INFINITY)
            } else {
                max_width.unwrap_or(f32::INFINITY)
            };
            child
                .as_ref()
                .and_then(|child| constrained(child, axis, min, max))
        }
        WidgetKind::Positioned {
            width,
            height,
            child,
            ..
        } => {
            let explicit = if axis.is_vertical() { *height } else { *width };
            explicit
                .and_then(finite)
                .or_else(|| widget_main_extent_hint(child, axis))
        }
        WidgetKind::Visibility {
            visible,
            hidden,
            child,
            ..
        } => {
            if *visible || hidden.layout == crate::tree::HiddenLayout::PreserveSpace {
                widget_main_extent_hint(child, axis)
            } else {
                Some(0.)
            }
        }
        WidgetKind::Unconstrained { child, transform, .. } => child
            .as_ref()
            .map_or(Some(0.0), |child| {
                if transform.is_some() { None } else { widget_main_extent_hint(child, axis) }
            }),
        WidgetKind::Align { child, .. }
        | WidgetKind::ClipOval { child, .. }
        | WidgetKind::Gesture { child, .. }
        | WidgetKind::Draggable { child, .. }
        | WidgetKind::DragTarget { child, .. }
        | WidgetKind::IgnorePointer { child, .. }
        | WidgetKind::AbsorbPointer { child, .. }
        | WidgetKind::ProxyLayout { child, .. }
        | WidgetKind::RepaintBoundary { child }
        | WidgetKind::AnimationTicker { child, .. }
        | WidgetKind::Opacity { child, .. }
        | WidgetKind::Blur { child, .. }
        | WidgetKind::DropShadow { child, .. }
        | WidgetKind::Blend { child, .. }
        | WidgetKind::Translate { child, .. }
        | WidgetKind::Transform { child, .. }
        | WidgetKind::Scale { child, .. }
        | WidgetKind::Affine { child, .. }
        | WidgetKind::Rotation { child, .. } => widget_main_extent_hint(child, axis),
        WidgetKind::ShaderMask { child, .. }
        | WidgetKind::BackdropFilter { child, .. }
        | WidgetKind::AnnotatedRegion { child, .. }
        | WidgetKind::CompositedTransformTarget { child, .. }
        | WidgetKind::CompositedTransformFollower { child, .. } => {
            widget_main_extent_hint(child, axis)
        }
        WidgetKind::RawInput { child, .. } => child
            .as_ref()
            .and_then(|child| widget_main_extent_hint(child, axis)),
        WidgetKind::ColorFiltered { child, .. }
        | WidgetKind::ClipRect { child, .. }
        | WidgetKind::ClipRRect { child, .. }
        | WidgetKind::FittedBox { child, .. }
        | WidgetKind::ClipPath { child, .. } => child
            .as_ref()
            .map_or(Some(0.), |child| widget_main_extent_hint(child, axis)),
        WidgetKind::Baseline { child, .. } => widget_main_extent_hint(child, axis),
        WidgetKind::Flexible { child, .. } => widget_main_extent_hint(child, axis),
        WidgetKind::Flex {
            axis: flex_axis,
            children,
            spacing,
            ..
        } => {
            if *flex_axis == axis {
                let mut total = 0.;
                for child in children {
                    total += widget_main_extent_hint(child, axis)?;
                }
                let spacing = spacing.max(0.) * children.len().saturating_sub(1) as f32;
                finite(total + spacing)
            } else {
                let mut max: Option<f32> = None;
                for child in children {
                    let hint = widget_main_extent_hint(child, axis)?;
                    max = Some(max.map_or(hint, |m| m.max(hint)));
                }
                max.or(Some(0.))
            }
        }
        WidgetKind::Stack { children, .. } | WidgetKind::IndexedStack { children, .. } => children
            .iter()
            .filter_map(|child| widget_main_extent_hint(child, axis))
            .reduce(f32::max),
        // The delegate decides the size.
        WidgetKind::CustomLayout { .. } => None,
        WidgetKind::LayoutId { child, .. } => widget_main_extent_hint(child, axis),
        WidgetKind::SelectionArea { child, .. }
        | WidgetKind::SelectionContainer { child, .. }
        | WidgetKind::SelectionListener { child, .. }
        | WidgetKind::IndexedSemantics { child, .. }
        | WidgetKind::SemanticsDebugger { child, .. } => widget_main_extent_hint(child, axis),
        WidgetKind::PersistentHeader { child, .. } => widget_main_extent_hint(child, axis),
        WidgetKind::NotificationListener { child, .. } => widget_main_extent_hint(child, axis),
        WidgetKind::RawScrollbar { child, .. }
        | WidgetKind::DraggableScrollableActuator { child, .. } => {
            widget_main_extent_hint(child, axis)
        }
        // A scrollable or a layout builder obtains its main-axis extent from
        // its parent; guessing it from the child would make a prototype list
        // report a different extent from the actual viewport.
        WidgetKind::Scroll { .. }
        | WidgetKind::ListWheelScrollView { .. }
        | WidgetKind::ListWheelViewport { .. }
        | WidgetKind::DraggableScrollableSheet { .. }
        | WidgetKind::TwoDimensionalScrollView { .. }
        | WidgetKind::TwoDimensionalViewport { .. }
        | WidgetKind::SliverViewport { .. }
        | WidgetKind::LayoutBuilder { .. }
        | WidgetKind::AspectRatio { .. }
        | WidgetKind::SizeObserver { .. }
        | WidgetKind::Fractional { .. }
        | WidgetKind::ListBody { .. }
        | WidgetKind::OverflowBar { .. }
        | WidgetKind::Wrap { .. }
        | WidgetKind::Table { .. } => None,
    };
    result.and_then(finite)
}

fn main_before(axis: Axis, padding: EdgeInsets) -> f32 {
    if axis.is_vertical() {
        padding.top
    } else {
        padding.left
    }
}

fn main_after(axis: Axis, padding: EdgeInsets) -> f32 {
    if axis.is_vertical() {
        padding.bottom
    } else {
        padding.right
    }
}

fn cross_before(axis: Axis, padding: EdgeInsets) -> f32 {
    if axis.is_vertical() {
        padding.left
    } else {
        padding.top
    }
}

fn cross_after(axis: Axis, padding: EdgeInsets) -> f32 {
    if axis.is_vertical() {
        padding.right
    } else {
        padding.bottom
    }
}

fn apply_pinned_offsets(children: &mut [SliverChildLayout], scroll: f32, viewport: f32) {
    let viewport = viewport.max(0.);
    // Pinned headers reserve a slot at the leading edge once their normal
    // position reaches that slot. Every subsequent pinned header uses the end
    // of the previous slot, preventing overlap while preserving the normal
    // flow position before it reaches the stack. Slivers lay out in scroll
    // space, so a reversed axis needs no separate rule: the viewport mirrors
    // the result when it positions the children.
    let mut stack = 0.;
    for child in children
        .iter_mut()
        .filter(|child| child.placement == SliverChildPlacement::Pinned)
    {
        let normal = child.offset;
        let extent = child.extent.max(0.).min(viewport);
        let current = normal - scroll;
        if current <= stack {
            child.offset = normal + stack - current;
            stack += extent;
        }
    }
}
