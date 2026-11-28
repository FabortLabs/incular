use super::*;
use incular_config::TextDirection;
use std::{any::Any, cell::OnceCell};

/// Amount of offscreen content a scroll viewport caches on each side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollCacheExtent {
    Pixels(f32),
    Viewport(f32),
}

impl ScrollCacheExtent {
    #[must_use]
    pub const fn pixels(value: f32) -> Self {
        Self::Pixels(value)
    }

    #[must_use]
    pub const fn viewport(value: f32) -> Self {
        Self::Viewport(value)
    }

    #[must_use]
    pub const fn value(self) -> f32 {
        match self {
            Self::Pixels(value) | Self::Viewport(value) => value,
        }
    }

    #[must_use]
    pub fn resolve(self, viewport_extent: f32) -> f32 {
        match self {
            Self::Pixels(value) => value.max(0.),
            Self::Viewport(value) => (value * viewport_extent).max(0.),
        }
    }
}

/// Stable identity for a child materialized by a sliver.
///
/// The low 32 bits are owned by the leaf sliver. The high 32 bits contain a
/// compact stack of eight-bit group scopes, with the innermost scope in the
/// least-significant byte. Keeping the scope path in the ID means nested main
/// and cross-axis groups cannot accidentally reuse each other's child state.
/// The top 65,536 values of the low half are reserved for semantics anchors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SliverChildId(pub u64);

impl SliverChildId {
    pub(super) const fn list_item(index: usize) -> Self {
        Self(index as u64 + 1)
    }

    pub(crate) fn item_index(self) -> Option<usize> {
        if is_semantics_anchor(self) {
            return None;
        }
        // The scope bits must not borrow into a zero leaf ID. Box and
        // header slivers use local ID zero, which has no collection index.
        (self.0 & u32::MAX as u64)
            .checked_sub(1)
            .and_then(|index| usize::try_from(index).ok())
    }

    pub(super) fn scoped(sliver: usize, child: Self) -> Self {
        let path = child.0 >> 32;
        let scope = (sliver as u64).saturating_add(1).min(u8::MAX as u64);
        let path = (((path & 0x00ff_ffff) << 8) | scope) & u32::MAX as u64;
        Self((path << 32) | (child.0 & u32::MAX as u64))
    }

    pub(super) fn scope(self) -> usize {
        let scope = (self.0 >> 32) & u8::MAX as u64;
        scope
            .checked_sub(1)
            .map_or(usize::MAX, |value| value as usize)
    }

    pub(super) fn local(self) -> Self {
        Self((((self.0 >> 32) >> 8) << 32) | (self.0 & u32::MAX as u64))
    }
}

/// Which sliver of a viewport paints on top where slivers overlap (Flutter
/// `SliverPaintOrder`, `rendering/viewport.dart:51`). Hit testing runs in the
/// opposite order, so the sliver on top also receives the pointer first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SliverPaintOrder {
    /// The first sliver paints on top: slivers paint last to first and are
    /// hit-tested first to last. Flutter's default, which keeps a leading
    /// pinned or floating header above the content scrolling under it.
    #[default]
    FirstIsTop,
    /// The last sliver paints on top: slivers paint first to last and are
    /// hit-tested last to first.
    LastIsTop,
}

/// Paint order of a sliver viewport's retained children, as slots into
/// `ids` (the viewport's children, parallel to their sliver ids). Children
/// of one top-level sliver keep their own order; whole slivers are ordered by
/// `order` (Flutter `RenderViewport.childrenInPaintOrder`). `None` means the
/// children already paint in that order.
pub(crate) fn sliver_paint_slots(
    ids: &[SliverChildId],
    order: SliverPaintOrder,
) -> Option<Vec<usize>> {
    if order == SliverPaintOrder::LastIsTop {
        return None;
    }
    let first = ids.first()?.scope();
    if ids.iter().all(|id| id.scope() == first) {
        return None;
    }
    let mut slots: Vec<usize> = (0..ids.len()).collect();
    slots.sort_by_key(|slot| std::cmp::Reverse(ids[*slot].scope()));
    Some(slots)
}

/// One child placement returned by a render sliver.
#[derive(Clone, Debug)]
pub struct SliverChildLayout {
    pub id: SliverChildId,
    pub widget: Widget,
    /// Logical accessibility position. This is independent from `id` because
    /// rows, reorderable keys, and animated entries may use a different
    /// retained identity than their collection position.
    pub semantic_index: Option<usize>,
    /// Main-axis content offset before the viewport scroll transform.
    pub offset: f32,
    /// Cross-axis content offset.
    pub cross_offset: f32,
    /// Box constraints used when the retained child is laid out.
    pub constraints: Constraints,
    /// Measured/estimated main-axis extent used for anchor and pinning math.
    pub extent: f32,
    /// Placement and stacking policy within the viewport.
    pub placement: SliverChildPlacement,
}

/// Retained placement of a sliver child relative to ordinary content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliverChildPlacement {
    /// Translate with the content and use normal paint order.
    Flow,
    /// Keep the element alive outside the sliver cache without displaying it.
    Parked,
    /// Pin to the viewport edge and paint above flowing content.
    Pinned,
    /// Use sliver-computed placement and paint above flowing content.
    Floating,
}

/// Result of laying out one render sliver.
#[derive(Clone, Debug)]
pub struct SliverLayout {
    pub geometry: SliverGeometry,
    pub children: Vec<SliverChildLayout>,
    /// Overlap absorbed for a following nested viewport.
    pub absorbed_overlap: f32,
}

impl SliverLayout {
    pub(super) fn empty(geometry: SliverGeometry) -> Self {
        Self {
            geometry,
            children: Vec::new(),
            absorbed_overlap: 0.,
        }
    }
}

/// Retained sliver protocol. Implementations receive viewport constraints and
/// return geometry plus only the children needed for the current paint/cache
/// interval. This is the analogue of Flutter's `RenderSliver` contract.
pub trait RenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout;

    /// An explicit cross-axis extent reported by the most recent layout.
    /// When absent, consumers keep the incoming cross-axis extent.
    fn reported_cross_axis_extent(&self) -> Option<f32> {
        None
    }

    /// Annotation metadata from the most recent layout. Unannotated
    /// slivers and slivers that have never laid out return None.
    fn semantic_annotation_metadata(&self) -> Option<incular_semantics::SemanticAnnotationMetadata> {
        None
    }

    /// Lets a sliver adjust a reveal request in its mounted child's
    /// coordinate space and schedule any required presentation change.
    /// The default leaves the target rectangle unchanged.
    #[doc(hidden)]
    fn reveal_child(
        &mut self,
        _child: SliverChildId,
        _rect: Rect,
        _options: &ShowOnScreen,
    ) -> Option<Rect> {
        None
    }

    /// Identifies when scrolling invalidates retained child geometry.
    /// Override for headers or custom slivers whose placement changes inside
    /// an already materialized cache window.
    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        SliverScrollDependency::CacheWindow
    }

    /// Returns the logical child count when this sliver is backed by an
    /// indexed child delegate. Non-indexed slivers leave it unknown.
    fn child_count(&self) -> Option<usize> {
        None
    }

    /// Records the exact extent measured by the retained child tree.
    fn set_child_extent(&mut self, _child: SliverChildId, _extent: f32) -> bool {
        false
    }

    /// Structural/measurement revision used to invalidate a viewport's
    /// materialized child range without rebuilding the application.
    fn revision(&self) -> u64 {
        0
    }

    /// Advances retained sliver-local animation state without rebuilding the
    /// application widget description. The viewport calls this from the
    /// compositor phase, and schedules layout only when geometry changed.
    fn tick(&mut self, _now: Instant) -> bool {
        false
    }

    /// Whether another frame is required for sliver-local animation.
    fn is_animating(&self) -> bool {
        false
    }

    /// Receives the ambient reduced-motion policy. The owning viewport pushes
    /// it before ticking; wrappers forward it to their inner sliver. The
    /// default ignores it so slivers without motion policy keep existing
    /// behavior exactly.
    fn set_reduced_motion(&mut self, _reduced: bool) {}

    /// Downcast hook for compatible-state transfer across descriptor updates.
    /// Only slivers with retained measurement override this; the default
    /// keeps every other sliver replaceable without coupling them together.
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }

    /// Invalidates retained measurement for one materialized child after its
    /// subtree rebuilt with new content. Routing mirrors
    /// [`RenderSliver::set_child_extent`]: sequences descend by scope and
    /// transparent wrappers forward; measured slivers demote back to an
    /// unverified estimate for unbounded revalidation. The default is a
    /// no-op so stateless slivers ignore content rebuilds.
    fn invalidate_child_measurement(&mut self, _child: SliverChildId) {}

    /// Updates parent data on a live child without rerunning the delegate.
    /// Grid lanes pass the tile's own index inside the materialized row.
    #[doc(hidden)]
    fn set_child_keep_alive(
        &mut self,
        _child: SliverChildId,
        _grid_tile: Option<usize>,
        _keep_alive: bool,
    ) -> bool {
        false
    }

    /// Retains an active editor independently of explicit KeepAlive.
    #[doc(hidden)]
    fn set_child_focus_keep_alive(
        &mut self,
        _child: SliverChildId,
        _grid_tile: Option<usize>,
        _keep_alive: bool,
    ) -> bool {
        false
    }

    /// Routes one source without clearing another source's retention.
    #[doc(hidden)]
    fn update_child_retention(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        source: SliverKeepAliveSource,
        keep_alive: bool,
    ) -> bool {
        match source {
            SliverKeepAliveSource::Explicit => self.set_child_keep_alive(child, grid_tile, keep_alive),
            SliverKeepAliveSource::Focus => self.set_child_focus_keep_alive(child, grid_tile, keep_alive),
            SliverKeepAliveSource::Automatic => false,
        }
    }

}

/// Independent owners of retained sliver parent data.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliverKeepAliveSource {
    Explicit,
    Focus,
    Automatic,
}

/// Scroll input that invalidates a sliver's retained layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SliverScrollDependency {
    /// Existing children can be translated until the cache window is exhausted.
    CacheWindow,
    /// Every scroll offset change can alter child size or placement.
    ScrollOffset,
}

/// Obstruction in the laid-out growth run before a viewport child.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SliverRevealGeometry {
    pub(crate) pins_at_edge: bool,
    pub(crate) obstruction_before: f32,
    pub(crate) reverse_growth: bool,
    /// Leading-edge adjustment to this sliver's approximate paint clip.
    /// Independent of the cache bounds used to retain offscreen semantics.
    pub(crate) semantic_clip_inset: f32,
    pub(crate) outside_paint_and_cache: bool,
}

impl SliverRevealGeometry {
    /// Reserves the covered strip in logical scroll-direction coordinates.
    pub(crate) fn reserve(self, leading: f32, trailing: f32) -> (f32, f32) {
        let obstruction = self.obstruction_before.min((trailing - leading).max(0.));
        if self.reverse_growth {
            (leading, trailing - obstruction)
        } else {
            (leading + obstruction, trailing)
        }
    }
}

/// Private bridge for widget materialization and viewport-scoped child IDs.
pub(crate) trait SliverViewportDelegate {
    fn scroll_layout_dependency(&self) -> SliverScrollDependency;
    fn perform_layout(&self, constraints: SliverConstraints) -> SliverViewportLayout;
    fn set_child_extent(&self, child: SliverChildId, extent: f32) -> bool;
    fn reveal_geometry(&self, _child: SliverChildId) -> SliverRevealGeometry {
        SliverRevealGeometry::default()
    }
    fn reveal_child(
        &self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect>;
    fn revision(&self) -> u64;
    fn sliver_count(&self) -> usize;
    fn child_count(&self) -> Option<usize>;
    fn tick(&self, now: Instant) -> bool;
    fn is_animating(&self) -> bool;
    /// Downcast hook for compatible-state transfer. Only sequence delegates
    /// override this; every other delegate keeps the replaceable default.
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }
    /// Inherits compatible retained sliver state (validated measurements,
    /// reversal tracking) from the delegate this one replaces. The default is
    /// a no-op so custom delegates keep existing replacement semantics.
    /// `probes` counts rows examined while adopting, so callers can
    /// prove the work stays proportional to measured rows.
    fn adopt_compatible_state(
        &self,
        _previous: &dyn SliverViewportDelegate,
        _probes: &mut u64,
        _preserve_anchor: bool,
    ) {
    }
    /// Invalidates one viewport-scoped child's retained measurement after a
    /// content rebuild below it. Returns whether a sliver was found to
    /// notify; the default reports none.
    fn invalidate_sliver_child(&self, _child: SliverChildId) -> bool {
        false
    }
    fn update_child_retention(
        &self,
        _child: SliverChildId,
        _grid_tile: Option<usize>,
        _source: SliverKeepAliveSource,
        _keep_alive: bool,
    ) -> bool {
        false
    }

    /// Pushes the ambient reduced-motion policy toward snap-capable
    /// descendants. The default ignores it; sequence delegates override it to
    /// reach headers nested behind transparent wrappers.
    fn set_reduced_motion(&self, _reduced: bool) {}
}

type SliverDelegateFactory = Box<dyn FnOnce() -> Rc<dyn SliverViewportDelegate>>;

/// Builds the opposite horizontal sliver sequence only if directionality needs it.
/// Custom slivers receive the same physical axis at construction and at layout.
pub(crate) struct AlternateSliverDelegate {
    value: OnceCell<Rc<dyn SliverViewportDelegate>>,
    factory: RefCell<Option<SliverDelegateFactory>>,
}

impl AlternateSliverDelegate {
    pub(super) fn new(factory: impl FnOnce() -> Rc<dyn SliverViewportDelegate> + 'static) -> Self {
        Self {
            value: OnceCell::new(),
            factory: RefCell::new(Some(Box::new(factory))),
        }
    }

    pub(crate) fn get(&self) -> Rc<dyn SliverViewportDelegate> {
        self.value
            .get_or_init(|| {
                let factory = self
                    .factory
                    .borrow_mut()
                    .take()
                    .expect("sliver delegate factory");
                factory()
            })
            .clone()
    }
}

#[derive(Clone, Default)]
pub(crate) enum SliverAxisResolution {
    #[default]
    Physical,
    Inherited,
    /// Custom slivers can use the resolved direction at creation time.
    InheritedWithDelegate(Rc<AlternateSliverDelegate>),
}

#[derive(Clone)]
pub struct SliverViewportConfig {
    pub(crate) controller: ScrollController,
    /// Preserve the current position when rebuilding a scroll view that
    /// has no caller-supplied controller or PageStorage key.
    pub(crate) preserve_implicit_scroll_position: bool,
    pub(crate) axis: Axis,
    pub(crate) reverse: bool,
    pub(crate) text_direction: Option<TextDirection>,
    pub(crate) safe_insets: EdgeInsets,
    /// Raw viewports supply a physical axis; high-level scroll views
    /// resolve a horizontal axis against Directionality.
    pub(crate) axis_resolution: SliverAxisResolution,
    pub(crate) physics: ScrollPhysics,
    pub(crate) increment_calculator: Option<crate::ScrollIncrementCalculator>,
    pub(crate) cache_extent: ScrollCacheExtent,
    /// Pages leave the retained tree when their whole extent leaves the cache.
    pub(crate) evict_outside_cache: bool,
    pub(crate) shrink_wrap: bool,
    pub(crate) clip_behavior: Clip,
    pub(crate) drag_start_behavior: DragStartBehavior,
    pub(crate) hit_test_behavior: crate::gestures::HitTestBehavior,
    /// Flutter `Viewport.paintOrder`; [`SliverPaintOrder::FirstIsTop`] by default.
    pub(crate) paint_order: SliverPaintOrder,
    pub(crate) delegate: Rc<dyn SliverViewportDelegate>,
    pub(crate) semantic_child_count: Option<usize>,
    /// `Some(page)` when the view set no physics: lowering takes the ambient
    /// `ScrollBehavior`'s, with page snapping on top for `page`.
    pub(crate) inherit_physics: Option<bool>,
    pub(crate) keyboard_dismissal: SliverKeyboardDismissal,
    /// The ambient scroll behavior, resolved at lowering.
    pub(crate) behavior: crate::scroll_behavior::ResolvedScrollBehavior,
}

/// A raw Viewport paints slivers but does not install the ScrollView's
/// keyboard dismissal listener.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SliverKeyboardDismissal {
    #[default]
    Disabled,
    Inherited,
    Explicit(crate::ScrollViewKeyboardDismissBehavior),
}

impl SliverViewportConfig {
    /// The retained viewport's configured clip, even if its content fits.
    #[must_use]
    pub const fn clip_behavior(&self) -> Clip {
        self.clip_behavior
    }

    /// The sliver's overlap-adjusted clip, if it differs from the viewport bounds.
    pub(crate) fn overlap_paint_clip(
        &self,
        incoming: Constraints,
        viewport: Size,
        geometry: SliverRevealGeometry,
    ) -> Option<Rect> {
        let inset = geometry.semantic_clip_inset;
        // Although shrink-wrap measures its retained children with a finite
        // working size, Flutter has no finite start of overlap when the
        // incoming maximum along the scroll axis is infinite.
        let bounded = match self.axis {
            Axis::Horizontal => incoming.is_width_bounded(),
            Axis::Vertical => incoming.is_height_bounded(),
        };
        if inset == 0. || (self.shrink_wrap && !bounded) {
            return None;
        }
        let origin = self.axis.offset(
            if self.reverse ^ geometry.reverse_growth {
                0.
            } else {
                inset
            },
            0.,
        );
        let main = (self.axis.main_extent(viewport) - inset).max(0.);
        Some(Rect::from_origin_size(
            origin,
            self.axis.size(main, self.axis.cross_extent(viewport)),
        ))
    }
}

impl std::fmt::Debug for SliverViewportConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SliverViewportConfig")
            .field("axis", &self.axis)
            .field("reverse", &self.reverse)
            .field("physics", &self.physics)
            .field("cache_extent", &self.cache_extent)
            .field("shrink_wrap", &self.shrink_wrap)
            .field("clip_behavior", &self.clip_behavior)
            .field("drag_start_behavior", &self.drag_start_behavior)
            .field("hit_test_behavior", &self.hit_test_behavior)
            .field("paint_order", &self.paint_order)
            .field("sliver_count", &self.delegate.sliver_count())
            .finish()
    }
}

impl PartialEq for SliverViewportConfig {
    fn eq(&self, other: &Self) -> bool {
        self.controller == other.controller
            && self.preserve_implicit_scroll_position == other.preserve_implicit_scroll_position
            && self.axis == other.axis
            && self.reverse == other.reverse
            && self.text_direction == other.text_direction
            && self.safe_insets == other.safe_insets
            && self.physics == other.physics
            && self.increment_calculator == other.increment_calculator
            && self.cache_extent == other.cache_extent
            && self.evict_outside_cache == other.evict_outside_cache
            && self.shrink_wrap == other.shrink_wrap
            && self.clip_behavior == other.clip_behavior
            && self.drag_start_behavior == other.drag_start_behavior
            && self.hit_test_behavior == other.hit_test_behavior
            && self.paint_order == other.paint_order
            && self.semantic_child_count == other.semantic_child_count
            && self.inherit_physics == other.inherit_physics
            && self.keyboard_dismissal == other.keyboard_dismissal
            && self.behavior == other.behavior
            && Rc::ptr_eq(&self.delegate, &other.delegate)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SliverViewportLayout {
    pub geometry: SliverGeometry,
    pub children: Vec<SliverChildLayout>,
    /// Scroll extent before offset zero: the position can scroll back to
    /// `-leading_extent` (Flutter `minScrollExtent`). Zero unless slivers
    /// before a center reach above the anchor.
    pub leading_extent: f32,
    /// The slivers before this index grow in reverse from the center
    /// (Flutter `GrowthDirection.reverse`).
    pub reverse_slivers: usize,
}

impl SliverViewportLayout {
    /// Whether `child` belongs to a sliver that grows in reverse. Such a
    /// child is placed by its trailing edge, the edge nearer the center, so
    /// a measured extent that differs from its estimate moves its leading
    /// edge instead.
    pub(crate) fn grows_in_reverse(&self, child: &SliverChildLayout) -> bool {
        child.id.scope() < self.reverse_slivers
    }
}
