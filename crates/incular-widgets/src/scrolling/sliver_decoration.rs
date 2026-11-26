//! One retained paint layer for the sliver, not a decoration on each box row.

use super::{layout::SingleInnerSliver, *};
use crate::{DecorationPosition, ExcludeSemantics, IgnorePointer, SizedBox};
use std::any::Any;

pub(super) struct DecoratedRenderSliver {
    pub inner: RefCell<Box<dyn RenderSliver>>,
    pub decorate: Rc<dyn Fn(Widget) -> Widget>,
    pub position: DecorationPosition,
    scroll_dependent: bool,
}

impl DecoratedRenderSliver {
    pub fn new(
        inner: Box<dyn RenderSliver>,
        decorate: Rc<dyn Fn(Widget) -> Widget>,
        position: DecorationPosition,
    ) -> Self {
        Self {
            inner: RefCell::new(inner),
            decorate,
            position,
            scroll_dependent: false,
        }
    }

    /// Inner content occupies child scope zero; scope one is the inert
    /// paint box. This prevents both nested decorators and arbitrary child
    /// IDs from colliding, without using collection indices as paint IDs.
    pub(super) fn content_id(child: SliverChildId) -> Option<SliverChildId> {
        (child.scope() == 0).then(|| child.local())
    }
}

impl RenderSliver for DecoratedRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let mut layout = self.inner.get_mut().perform_layout(constraints);
        let geometry = layout.geometry;
        for child in &mut layout.children {
            child.id = SliverChildId::scoped(0, child.id);
        }
        self.scroll_dependent =
            !geometry.max_paint_extent.is_finite() || geometry.max_scroll_obstruction_extent > 0.;
        // A cached sliver can enter the clipped viewport by compositor
        // translation alone; keep its one decoration in that same cache.
        if (!geometry.visible && geometry.cache_extent <= 0.)
            || geometry.scroll_offset_correction.is_some()
        {
            return layout;
        }
        // Flutter RenderSliver.getMaxPaintRect, expressed in the retained
        // content coordinates used by the viewport. Reversed viewports
        // mirror this box just as they mirror each real child.
        let extent = if geometry.max_paint_extent.is_infinite() {
            constraints.scroll_offset + geometry.cache_extent + constraints.cache_origin
        } else {
            geometry.max_paint_extent
        };
        if !extent.is_finite() || extent <= 0. {
            return layout;
        }
        let max_leading = (geometry.scroll_extent - geometry.max_scroll_obstruction_extent).max(0.);
        let leading = constraints.scroll_offset.min(max_leading).max(0.);
        let cross = constraints.cross_axis_extent;
        let size = if constraints.axis.is_vertical() {
            Size::new(cross, extent)
        } else {
            Size::new(extent, cross)
        };
        let empty = SizedBox::new().width(size.width).height(size.height).into();
        let widget = IgnorePointer::new(ExcludeSemantics::new((self.decorate)(empty))).into();
        let layer = SliverChildLayout {
            id: SliverChildId::scoped(1, SliverChildId(0)),
            widget,
            semantic_index: None,
            offset: constraints.scroll_offset - leading,
            cross_offset: 0.,
            constraints: Constraints::tight(size),
            extent,
            placement: SliverChildPlacement::Flow,
        };
        match self.position {
            DecorationPosition::Background => layout.children.insert(0, layer),
            DecorationPosition::Foreground => layout.children.push(layer),
        }
        layout
    }

    fn semantic_annotation_metadata(
        &self,
    ) -> Option<incular_semantics::SemanticAnnotationMetadata> {
        self.inner.borrow().semantic_annotation_metadata()
    }

    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        self.inner
            .get_mut()
            .reveal_child(Self::content_id(child)?, rect, options)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        if self.scroll_dependent {
            SliverScrollDependency::ScrollOffset
        } else {
            self.inner.borrow().scroll_layout_dependency()
        }
    }

    fn child_count(&self) -> Option<usize> {
        self.inner.borrow().child_count()
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        Self::content_id(child)
            .is_some_and(|child| self.inner.get_mut().set_child_extent(child, extent))
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        if let Some(child) = Self::content_id(child) {
            self.inner.get_mut().invalidate_child_measurement(child);
        }
    }

    fn set_child_keep_alive(
        &mut self,
        child: SliverChildId,
        tile: Option<usize>,
        keep: bool,
    ) -> bool {
        Self::content_id(child)
            .is_some_and(|child| self.inner.get_mut().set_child_keep_alive(child, tile, keep))
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        tile: Option<usize>,
        keep: bool,
    ) -> bool {
        Self::content_id(child).is_some_and(|child| {
            self.inner
                .get_mut()
                .set_child_focus_keep_alive(child, tile, keep)
        })
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

impl SingleInnerSliver for DecoratedRenderSliver {
    fn inner_cell(&mut self) -> &RefCell<Box<dyn RenderSliver>> {
        &self.inner
    }
}
