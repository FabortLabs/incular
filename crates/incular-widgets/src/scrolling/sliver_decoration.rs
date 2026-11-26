//! One retained paint layer for the sliver, not a decoration on each box row.

use super::{layout::SingleInnerSliver, sliver_paint_geometry::SliverPaintBounds, *};
use crate::{
    CustomPaint, CustomPainter, DecorationPosition, ExcludeSemantics, IgnorePointer, SizedBox,
};
use incular_config::TextDirection;
use incular_rendering::{Canvas, DisplayList};
use std::any::Any;
use std::rc::Weak;

/// The paint-time configuration of one retained sliver decoration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliverBoxPaintConfiguration {
    pub size: Size,
    pub text_direction: Option<TextDirection>,
}

/// Custom painter for an entire decorated sliver. The canvas is already
/// in the decoration's local coordinates, with its top left at zero.
pub trait SliverBoxPainter {
    fn paint(&mut self, canvas: &mut Canvas, configuration: SliverBoxPaintConfiguration);

    /// Releases painter resources when the decoration changes or detaches.
    fn dispose(&mut self) {}
}

/// Creates a retained sliver painter. Reuse the same factory handle across
/// rebuilds to keep that painter's state.
pub type SliverBoxPainterFactory = Rc<dyn Fn() -> Box<dyn SliverBoxPainter>>;

struct PainterLease(RefCell<Box<dyn SliverBoxPainter>>);

impl Drop for PainterLease {
    fn drop(&mut self) {
        self.0.get_mut().dispose();
    }
}

struct DecorationPicture {
    painter: Weak<PainterLease>,
    position: DecorationPosition,
    direction: Option<TextDirection>,
}

impl CustomPainter for DecorationPicture {
    fn paint(&self, size: Size) -> DisplayList {
        let Some(painter) = self.painter.upgrade() else {
            return DisplayList::default();
        };
        let mut canvas = Canvas::default();
        painter.0.borrow_mut().paint(
            &mut canvas,
            SliverBoxPaintConfiguration {
                size,
                text_direction: self.direction,
            },
        );
        canvas.finish()
    }

    fn should_repaint(&self, old: &Self) -> bool {
        !Weak::ptr_eq(&self.painter, &old.painter)
            || self.direction != old.direction
            || self.position != old.position
    }
}

pub(super) struct DecoratedRenderSliver {
    pub inner: RefCell<Box<dyn RenderSliver>>,
    pub decorate: Rc<dyn Fn(Widget) -> Widget>,
    pub position: DecorationPosition,
    scroll_dependent: bool,
    factory: Option<SliverBoxPainterFactory>,
    painter: Option<Rc<PainterLease>>,
}

impl DecoratedRenderSliver {
    pub fn new(
        inner: Box<dyn RenderSliver>,
        decorate: Rc<dyn Fn(Widget) -> Widget>,
        position: DecorationPosition,
        factory: Option<SliverBoxPainterFactory>,
    ) -> Self {
        Self {
            inner: RefCell::new(inner),
            decorate,
            position,
            scroll_dependent: false,
            factory,
            painter: None,
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
        if self.painter.is_none()
            && let Some(factory) = &self.factory
        {
            self.painter = Some(Rc::new(PainterLease(RefCell::new(factory()))));
        }
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
        let Some(bounds) =
            SliverPaintBounds::new(constraints, geometry, self.reported_cross_axis_extent())
        else {
            return layout;
        };
        let size = bounds.size();
        let decoration = if let Some(painter) = &self.painter {
            CustomPaint::from_painter(
                size,
                DecorationPicture {
                    painter: Rc::downgrade(painter),
                    position: self.position,
                    direction: constraints.text_direction,
                },
            )
            .into()
        } else {
            let empty = SizedBox::new().width(size.width).height(size.height).into();
            (self.decorate)(empty)
        };
        let widget = IgnorePointer::new(ExcludeSemantics::new(decoration)).into();
        let layer = SliverChildLayout {
            id: SliverChildId::scoped(1, SliverChildId(0)),
            widget,
            semantic_index: None,
            offset: bounds.content_offset(constraints),
            cross_offset: 0.,
            constraints: Constraints::tight(size),
            extent: constraints.axis.main_extent(size),
            placement: SliverChildPlacement::Flow,
        };
        match self.position {
            DecorationPosition::Background => layout.children.insert(0, layer),
            DecorationPosition::Foreground => layout.children.push(layer),
        }
        layout
    }

    fn reported_cross_axis_extent(&self) -> Option<f32> {
        self.inner.borrow().reported_cross_axis_extent()
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
    fn adopt_outer_state(&mut self, old: &mut Self) {
        match (&self.factory, &old.factory) {
            (Some(factory), Some(previous)) if Rc::ptr_eq(factory, previous) => {
                self.painter = old.painter.take();
            }
            _ => {
                // A descriptor replacement releases old resources before
                // the next painter is created. Previously mounted paint
                // widgets hold only a weak reference to that lease.
                old.painter = None;
            }
        }
    }

    fn inner_cell(&mut self) -> &RefCell<Box<dyn RenderSliver>> {
        &self.inner
    }
}
