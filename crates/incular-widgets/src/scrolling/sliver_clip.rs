use incular_config::Axis;
use incular_core::{Offset, Rect};
use incular_rendering::RRect;
use incular_scroll::{SliverConstraints, SliverGeometry};

use super::sliver_paint_geometry::{SliverPaintBounds, is_reversed};
use crate::{BorderRadiusGeometry, ClipShape};

/// Controls how a sliver's clip reacts to overlapping slivers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClipOverlapBehavior {
    /// Preserve the full clip underneath an overlapping sliver.
    None,
    /// Trim the leading edge to avoid the overlap.
    #[default]
    FollowEdge,
    /// Shift rounded corners using their straight middle span.
    PreserveShape,
}

pub(super) fn sliver_clip_shape(
    constraints: SliverConstraints,
    geometry: SliverGeometry,
    bounds: &SliverPaintBounds,
    shape: ClipShape,
    overlap: ClipOverlapBehavior,
) -> ClipShape {
    let translate =
        |rect: Rect| Rect::from_origin_size(rect.origin + bounds.rect.origin, rect.size);
    match shape {
        ClipShape::Rect(rect) => {
            let rect = translate(rect);
            let inside = constraints.axis.main_extent(rect.size);
            ClipShape::Rect(intersect_leading(
                rect,
                inside,
                constraints,
                geometry,
                overlap,
            ))
        }
        ClipShape::RRect(rrect) => {
            let rect = translate(rrect.rect);
            let r = rrect.radii;
            let main = constraints.axis.main_extent(rect.size);
            let inside = if overlap == ClipOverlapBehavior::PreserveShape {
                match constraints.axis {
                    Axis::Horizontal => {
                        main - r.top_left.max(r.bottom_left) - r.top_right.max(r.bottom_right)
                    }
                    Axis::Vertical => {
                        main - r.top_left.max(r.top_right) - r.bottom_left.max(r.bottom_right)
                    }
                }
            } else {
                main
            };
            // Copy the actual radii, don't reinterpret rounded corners
            // as new insets when the leading edge moves.
            ClipShape::RRect(RRect {
                rect: intersect_leading(rect, inside, constraints, geometry, overlap),
                radii: r,
            })
        }
        // A rectangular or rounded sliver clipper may return only those
        // matching shapes; leave fallback selection to its descriptor.
        other => other,
    }
}

fn intersect_leading(
    rect: Rect,
    inside: f32,
    constraints: SliverConstraints,
    geometry: SliverGeometry,
    overlap: ClipOverlapBehavior,
) -> Rect {
    if overlap == ClipOverlapBehavior::None {
        return rect;
    }
    let flexible = (inside - geometry.max_scroll_obstruction_extent).max(0.);
    let shift = (flexible - constraints.scroll_offset).clamp(
        -flexible.min(constraints.scroll_offset),
        constraints.overlap.max(0.),
    );
    let axis = constraints.axis;
    let start = if axis.is_vertical() {
        rect.origin.y
    } else {
        rect.origin.x
    };
    let cross = if axis.is_vertical() {
        rect.origin.x
    } else {
        rect.origin.y
    };
    let end = start + axis.main_extent(rect.size);
    let (start, end) = if is_reversed(constraints) {
        (start, end.min(geometry.paint_extent - shift))
    } else {
        (start.max(shift), end)
    };
    Rect::from_origin_size(
        axis.offset(start, cross),
        axis.size((end - start).max(0.), axis.cross_extent(rect.size)),
    )
}

pub(crate) fn shape_contains(shape: &ClipShape, point: Offset) -> bool {
    let rect = shape.bounds();
    // Flutter's Rect and RRect reject right and bottom boundaries.
    if point.x < rect.origin.x
        || point.x >= rect.origin.x + rect.size.width
        || point.y < rect.origin.y
        || point.y >= rect.origin.y + rect.size.height
    {
        return false;
    }
    match shape {
        ClipShape::Rect(_) => true,
        ClipShape::RRect(rrect) => RRect::new(rrect.rect, rrect.radii).contains(point),
        ClipShape::Path(_) => false,
    }
}

use super::{Sliver, SliverChildId, SliverLayout, SliverScrollDependency, geometry::RenderSliver};
use crate::{ClipReclip, CustomClipper, Widget, layout::basic::ClipDelegate};
use incular_config::Clip;
use incular_rendering::ClipCoverage;
use incular_scroll::ScrollController;
use std::{
    any::Any,
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

/// A rectangular clip around an entire scrollable sliver.
pub struct SliverClipRect {
    sliver: Box<dyn Sliver>,
    clip_behavior: Clip,
    clip_overlap: ClipOverlapBehavior,
    clipper: Option<ClipDelegate>,
}

impl SliverClipRect {
    #[must_use]
    pub fn new(sliver: impl Sliver + 'static) -> Self {
        Self {
            sliver: Box::new(sliver),
            clip_behavior: Clip::HardEdge,
            clip_overlap: ClipOverlapBehavior::FollowEdge,
            clipper: None,
        }
    }

    #[must_use]
    pub fn clip_behavior(mut self, behavior: Clip) -> Self {
        self.clip_behavior = behavior;
        self
    }

    #[must_use]
    pub fn clip_overlap(mut self, overlap: ClipOverlapBehavior) -> Self {
        self.clip_overlap = overlap;
        self
    }

    #[must_use]
    pub fn clipper(mut self, clipper: impl CustomClipper) -> Self {
        self.clipper = Some(ClipDelegate::new(clipper));
        self
    }
}

impl Sliver for SliverClipRect {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        super::single_sliver_viewport(
            controller.clone(),
            axis,
            reverse,
            incular_scroll::ScrollPhysics::default(),
            super::DEFAULT_SLIVER_CACHE_EXTENT,
            Clip::HardEdge,
            self.create_render_sliver(controller, axis, reverse),
        )
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(ClipRenderSliver::new(
            self.sliver.create_render_sliver(controller, axis, reverse),
            self.clip_behavior,
            self.clip_overlap,
            self.clipper.clone(),
            None,
        ))
    }

    fn cross_axis_flex(&self) -> Option<usize> {
        self.sliver.cross_axis_flex()
    }
}

/// A rounded clip around an entire scrollable sliver.
pub struct SliverClipRRect {
    sliver: Box<dyn Sliver>,
    clip_behavior: Clip,
    clip_overlap: ClipOverlapBehavior,
    radii: BorderRadiusGeometry,
    clipper: Option<ClipDelegate>,
}

impl SliverClipRRect {
    #[must_use]
    pub fn new(sliver: impl Sliver + 'static) -> Self {
        Self {
            sliver: Box::new(sliver),
            clip_behavior: Clip::AntiAlias,
            clip_overlap: ClipOverlapBehavior::FollowEdge,
            radii: BorderRadiusGeometry::ZERO,
            clipper: None,
        }
    }

    #[must_use]
    pub fn border_radius(mut self, radius: impl Into<BorderRadiusGeometry>) -> Self {
        self.radii = radius.into();
        self
    }

    #[must_use]
    pub fn clip_behavior(mut self, behavior: Clip) -> Self {
        self.clip_behavior = behavior;
        self
    }

    #[must_use]
    pub fn clip_overlap(mut self, overlap: ClipOverlapBehavior) -> Self {
        self.clip_overlap = overlap;
        self
    }

    #[must_use]
    pub fn clipper(mut self, clipper: impl CustomClipper) -> Self {
        self.clipper = Some(ClipDelegate::new(clipper));
        self
    }
}

impl Sliver for SliverClipRRect {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        super::single_sliver_viewport(
            controller.clone(),
            axis,
            reverse,
            incular_scroll::ScrollPhysics::default(),
            super::DEFAULT_SLIVER_CACHE_EXTENT,
            Clip::HardEdge,
            self.create_render_sliver(controller, axis, reverse),
        )
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(ClipRenderSliver::new(
            self.sliver.create_render_sliver(controller, axis, reverse),
            self.clip_behavior,
            self.clip_overlap,
            self.clipper.clone(),
            Some(self.radii),
        ))
    }

    fn cross_axis_flex(&self) -> Option<usize> {
        self.sliver.cross_axis_flex()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ClipShapeResolver {
    constraints: SliverConstraints,
    geometry: SliverGeometry,
    bounds: SliverPaintBounds,
    radii: Option<BorderRadiusGeometry>,
    overlap: ClipOverlapBehavior,
}

impl ClipShapeResolver {
    fn resolve(self, clipper: Option<&ClipDelegate>) -> ClipShape {
        let local_rect = Rect::from_origin_size(Offset::ZERO, self.bounds.size());
        let explicit = clipper.map(|clipper| clipper.clip(self.bounds.size()));
        let shape = match (explicit, self.radii) {
            (Some(ClipShape::Rect(rect)), None) => ClipShape::Rect(rect),
            (Some(ClipShape::RRect(rect)), Some(_)) => ClipShape::RRect(rect),
            (_, None) => ClipShape::Rect(local_rect),
            (_, Some(radii)) => ClipShape::RRect(RRect {
                rect: local_rect,
                radii: radii
                    .resolve(
                        self.constraints
                            .text_direction
                            .unwrap_or(incular_config::TextDirection::Ltr),
                    )
                    .to_corner_radii(),
            }),
        };
        sliver_clip_shape(
            self.constraints,
            self.geometry,
            &self.bounds,
            shape,
            self.overlap,
        )
    }
}

/// The same lazy shape snapshot is shared by every materialized row and
/// used by both painting and input. Reclip does not need the sliver to lay
/// its children out again.
struct LiveClipFrame {
    resolver: ClipShapeResolver,
    clipper: ClipDelegate,
    signal: ClipReclip,
    cached: RefCell<(u64, ClipShape)>,
}

impl LiveClipFrame {
    fn shape(&self) -> ClipShape {
        let revision = self.signal.revision();
        let mut cached = self.cached.borrow_mut();
        if revision != cached.0 {
            *cached = (revision, self.resolver.resolve(Some(&self.clipper)));
        }
        cached.1.clone()
    }
}

#[derive(Clone)]
pub(crate) struct SliverClipPlacement {
    pub key: u64,
    pub shape: ClipShape,
    pub coverage: ClipCoverage,
    live: Option<Rc<LiveClipFrame>>,
    translation: Offset,
}

impl std::fmt::Debug for SliverClipPlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SliverClipPlacement")
            .field("key", &self.key)
            .field("shape", &self.shape)
            .field("coverage", &self.coverage)
            .finish_non_exhaustive()
    }
}

impl PartialEq for SliverClipPlacement {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.shape == other.shape
            && self.coverage == other.coverage
            && match (&self.live, &other.live) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b) && self.translation == other.translation,
                (None, None) => true,
                _ => false,
            }
    }
}

impl SliverClipPlacement {
    pub(crate) fn resolved_shape(&self) -> ClipShape {
        self.live.as_ref().map_or_else(
            || self.shape.clone(),
            |live| translate_shape(&live.shape(), self.translation),
        )
    }

    pub(crate) fn in_parent_space(&self, origin: Offset) -> Self {
        Self {
            shape: translate_shape(&self.resolved_shape(), origin),
            live: None,
            translation: Offset::ZERO,
            ..self.clone()
        }
    }

    pub(crate) fn dependency(&self) -> Option<incular_core::reactivity::DependencySource> {
        Some(self.live.as_ref()?.signal.source())
    }
}

struct CustomClipSnapshot {
    resolver: ClipShapeResolver,
    shape: ClipShape,
    live: Option<Rc<LiveClipFrame>>,
}

impl CustomClipSnapshot {
    fn new(resolver: ClipShapeResolver, clipper: &ClipDelegate, shape: ClipShape) -> Self {
        let live = clipper.reclip_signal().map(|signal| {
            Rc::new(LiveClipFrame {
                resolver,
                clipper: clipper.clone(),
                signal: signal.clone(),
                cached: RefCell::new((signal.revision(), shape.clone())),
            })
        });
        Self {
            resolver,
            shape,
            live,
        }
    }

    fn shape(&self) -> ClipShape {
        self.live
            .as_ref()
            .map_or_else(|| self.shape.clone(), |live| live.shape())
    }
}

pub(super) struct ClipRenderSliver {
    pub inner: std::cell::RefCell<Box<dyn RenderSliver>>,
    key: u64,
    behavior: Clip,
    overlap: ClipOverlapBehavior,
    clipper: Option<ClipDelegate>,
    radii: Option<BorderRadiusGeometry>,
    cached_custom: Option<Box<CustomClipSnapshot>>,
}

impl ClipRenderSliver {
    fn new(
        inner: Box<dyn RenderSliver>,
        behavior: Clip,
        overlap: ClipOverlapBehavior,
        clipper: Option<ClipDelegate>,
        radii: Option<BorderRadiusGeometry>,
    ) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            inner: std::cell::RefCell::new(inner),
            key: NEXT.fetch_add(1, Ordering::Relaxed),
            behavior,
            overlap,
            clipper,
            radii,
            cached_custom: None,
        }
    }
}

impl RenderSliver for ClipRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let inner = self.inner.get_mut();
        let mut layout = inner.perform_layout(constraints);
        if self.behavior == Clip::None
            || layout.children.is_empty()
            || layout.geometry.scroll_offset_correction.is_some()
        {
            return layout;
        }
        let geometry = layout.geometry;
        let Some(bounds) =
            SliverPaintBounds::new(constraints, geometry, inner.reported_cross_axis_extent())
        else {
            return layout;
        };
        let resolver = ClipShapeResolver {
            constraints,
            geometry,
            bounds,
            radii: self.radii,
            overlap: self.overlap,
        };
        let (shape, live) = if let Some(clipper) = self.clipper.as_ref() {
            if self
                .cached_custom
                .as_ref()
                .is_none_or(|cached| cached.resolver != resolver)
            {
                let shape = resolver.resolve(Some(clipper));
                self.cached_custom =
                    Some(Box::new(CustomClipSnapshot::new(resolver, clipper, shape)));
            }
            let cached = self.cached_custom.as_ref().expect("custom clip resolved");
            (cached.shape(), cached.live.clone())
        } else {
            (resolver.resolve(None), None)
        };
        let coverage = match self.behavior {
            Clip::None => unreachable!(),
            Clip::HardEdge => ClipCoverage::HardEdge,
            Clip::AntiAlias => ClipCoverage::AntiAlias,
            Clip::AntiAliasWithSaveLayer => ClipCoverage::AntiAliasWithSaveLayer,
        };
        for child in &mut layout.children {
            let mut main = child.offset - constraints.scroll_offset;
            if child.placement == super::SliverChildPlacement::Pinned {
                main = main.max(0.);
            }
            if is_reversed(constraints) {
                main = geometry.paint_extent - main - child.extent;
            }
            let origin = constraints.axis.offset(main, child.cross_offset);
            let translation = Offset::ZERO - origin;
            let placement = SliverClipPlacement {
                key: self.key,
                shape: translate_shape(&shape, translation),
                coverage,
                live: live.clone(),
                translation,
            };
            child.widget = child.widget.clone().with_sliver_clip(placement);
        }
        layout
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        if self.behavior == Clip::None {
            self.inner.borrow().scroll_layout_dependency()
        } else {
            SliverScrollDependency::ScrollOffset
        }
    }

    fn reported_cross_axis_extent(&self) -> Option<f32> {
        self.inner.borrow().reported_cross_axis_extent()
    }

    fn child_count(&self) -> Option<usize> {
        self.inner.borrow().child_count()
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        self.inner.get_mut().set_child_extent(child, extent)
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        self.inner.get_mut().invalidate_child_measurement(child);
    }

    fn set_child_keep_alive(
        &mut self,
        child: SliverChildId,
        tile: Option<usize>,
        keep: bool,
    ) -> bool {
        self.inner.get_mut().set_child_keep_alive(child, tile, keep)
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        tile: Option<usize>,
        keep: bool,
    ) -> bool {
        self.inner
            .get_mut()
            .set_child_focus_keep_alive(child, tile, keep)
    }

    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &crate::ShowOnScreen,
    ) -> Option<Rect> {
        self.inner.get_mut().reveal_child(child, rect, options)
    }

    fn revision(&self) -> u64 {
        self.inner.borrow().revision()
    }

    fn tick(&mut self, now: Instant) -> bool {
        self.inner.get_mut().tick(now)
    }

    fn is_animating(&self) -> bool {
        self.inner.borrow().is_animating()
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        self.inner.get_mut().set_reduced_motion(reduced);
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

impl super::layout::SingleInnerSliver for ClipRenderSliver {
    fn adopt_outer_state(&mut self, previous: &mut Self) {
        if self.radii.is_some() == previous.radii.is_some() {
            self.key = previous.key;
        }
        if self.behavior == previous.behavior
            && self.overlap == previous.overlap
            && self.radii == previous.radii
            && let (Some(next), Some(old), Some(cached)) = (
                self.clipper.as_ref(),
                previous.clipper.as_ref(),
                previous.cached_custom.as_ref(),
            )
            && !next.reclips_from(old)
        {
            self.cached_custom = Some(Box::new(CustomClipSnapshot::new(
                cached.resolver,
                next,
                cached.shape(),
            )));
        }
    }

    fn inner_cell(&mut self) -> &std::cell::RefCell<Box<dyn RenderSliver>> {
        &self.inner
    }
}

pub(crate) fn translate_shape(shape: &ClipShape, delta: Offset) -> ClipShape {
    let translate = |rect: Rect| Rect::from_origin_size(rect.origin + delta, rect.size);
    match shape {
        ClipShape::Rect(rect) => ClipShape::Rect(translate(*rect)),
        ClipShape::RRect(rrect) => ClipShape::RRect(RRect {
            rect: translate(rrect.rect),
            radii: rrect.radii,
        }),
        ClipShape::Path(_) => unreachable!("sliver rectangle clips only carry rect or rrect"),
    }
}
