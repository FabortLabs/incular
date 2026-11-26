//! Renderer-independent custom painting contract.

use std::any::Any;
use std::rc::Rc;

use incular_core::{Offset, Rect, Size};
use incular_rendering::DisplayList;

use crate::Semantics;
use crate::tree::Key;

/// Flutter's `CustomPainterSemantics` (`rendering/custom_paint.dart`): one
/// region of a painting that assistive technology sees as its own node.
///
/// `semantics` carries the region's properties; build it with
/// `Semantics::builder()` and leave its child unset, since the region has
/// no widget content.
#[derive(Clone)]
pub struct CustomPainterSemantics {
    rect: Rect,
    key: Option<Key>,
    semantics: Semantics,
}

impl CustomPainterSemantics {
    /// A region covering `rect`, in the painter's coordinates.
    #[must_use]
    pub fn new(rect: Rect, semantics: Semantics) -> Self {
        Self {
            rect,
            key: None,
            semantics: semantics.as_custom_painter_region(),
        }
    }

    /// Flutter `key`: keeps the region's node across rebuilds when the
    /// region list changes order.
    #[must_use]
    pub fn key(mut self, key: impl Into<Key>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// The region, in the painter's coordinates.
    #[must_use]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub(crate) fn into_parts(self) -> (Rect, Option<Key>, Semantics) {
        (self.rect, self.key, self.semantics)
    }
}

impl std::fmt::Debug for CustomPainterSemantics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomPainterSemantics")
            .field("rect", &self.rect)
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

/// Flutter's `SemanticsBuilderCallback`: the semantic regions of a painting
/// at its laid-out size.
pub type SemanticsBuilder = Rc<dyn Fn(Size) -> Vec<CustomPainterSemantics>>;

/// A delegate that produces custom display-list drawing commands.
pub trait CustomPainter {
    /// Draws visual content into `canvas` bounds.
    fn paint(&self, size: incular_core::Size) -> DisplayList;
    /// Decides whether a repaint is required when compared to a previous painter instance.
    fn should_repaint(&self, old: &Self) -> bool
    where
        Self: Sized;
    /// Tests a point in the painter's local coordinates. Returning `None`
    /// uses Flutter's default: a background painter accepts it, while a
    /// foreground painter leaves it to the child.
    fn hit_test(&self, _position: Offset) -> Option<bool> {
        None
    }

    /// An optional repaint source, like Flutter's painter Listenable.
    /// A notification repaints the mounted picture without rebuilding
    /// its widget, layout, or accessibility regions.
    fn repaint_source(&self) -> Option<incular_core::reactivity::DependencySource> {
        None
    }

    /// Whether this replacement changes its accessibility regions. By
    /// default this follows `should_repaint`, but either can be independent.
    fn should_rebuild_semantics(&self, old: &Self) -> bool
    where
        Self: Sized,
    {
        self.should_repaint(old)
    }

    /// Flutter `semanticsBuilder`: the painting's semantic regions, or `None`
    /// (the default) when the painting has none. `CustomPaint` rebuilds the
    /// regions whenever it rebuilds or its size changes.
    fn semantics_builder(&self) -> Option<SemanticsBuilder> {
        None
    }
}

/// A type-erased painter retained by a `CustomPaint` widget.
///
/// Painting runs at the laid-out size, so the display list is produced at
/// paint time. Equality is identity; a changed delegate asks its own
/// `should_repaint` before the render object repaints.
#[derive(Clone)]
pub struct PainterDelegate {
    painter: Rc<dyn Any>,
    paint: fn(&dyn Any, incular_core::Size) -> DisplayList,
    should_repaint: fn(&dyn Any, &dyn Any) -> bool,
    hit_test: fn(&dyn Any, Offset) -> Option<bool>,
    should_rebuild_semantics: fn(&dyn Any, &dyn Any) -> bool,
    semantics: Option<SemanticsBuilder>,
    repaint_source: Option<incular_core::reactivity::DependencySource>,
}

fn paint_erased<P: CustomPainter + 'static>(
    painter: &dyn Any,
    size: incular_core::Size,
) -> DisplayList {
    painter
        .downcast_ref::<P>()
        .map_or_else(DisplayList::default, |painter| painter.paint(size))
}

fn hit_test_erased<P: CustomPainter + 'static>(painter: &dyn Any, position: Offset) -> Option<bool> {
    painter.downcast_ref::<P>()?.hit_test(position)
}

fn should_repaint_erased<P: CustomPainter + 'static>(new: &dyn Any, old: &dyn Any) -> bool {
    match (new.downcast_ref::<P>(), old.downcast_ref::<P>()) {
        (Some(new), Some(old)) => new.should_repaint(old),
        _ => true,
    }
}

fn should_rebuild_semantics_erased<P: CustomPainter + 'static>(new: &dyn Any, old: &dyn Any) -> bool {
    match (new.downcast_ref::<P>(), old.downcast_ref::<P>()) {
        (Some(new), Some(old)) => new.should_rebuild_semantics(old),
        _ => true,
    }
}

impl PainterDelegate {
    /// Retains a painter delegate.
    #[must_use]
    pub fn new<P: CustomPainter + 'static>(painter: P) -> Self {
        let semantics = painter.semantics_builder();
        let repaint_source = painter.repaint_source();
        Self {
            semantics,
            repaint_source,
            painter: Rc::new(painter),
            paint: paint_erased::<P>,
            should_repaint: should_repaint_erased::<P>,
            hit_test: hit_test_erased::<P>,
            should_rebuild_semantics: should_rebuild_semantics_erased::<P>,
        }
    }

    /// Produces the display list for a laid-out size.
    #[must_use]
    pub(crate) fn paint(&self, size: incular_core::Size) -> DisplayList {
        (self.paint)(&*self.painter, size)
    }

    pub(crate) fn repaint_source(&self) -> Option<incular_core::reactivity::DependencySource> {
        self.repaint_source.clone()
    }

    pub(crate) fn hit_test(&self, position: Offset) -> Option<bool> {
        (self.hit_test)(&*self.painter, position)
    }

    /// The painter's semantics builder, read once when it was retained.
    #[must_use]
    pub(crate) fn semantics_builder(&self) -> Option<SemanticsBuilder> {
        self.semantics.clone()
    }

    pub(crate) fn should_rebuild_semantics_over(&self, old: &Self) -> bool {
        (self.should_rebuild_semantics)(&*self.painter, &*old.painter)
    }

    /// Flutter's `shouldRepaint` gate: whether this painter requires a repaint
    /// over `old`. A changed painter type always repaints.
    #[must_use]
    pub(crate) fn should_repaint_over(&self, old: &Self) -> bool {
        (self.should_repaint)(&*self.painter, &*old.painter)
    }
}

impl std::fmt::Debug for PainterDelegate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PainterDelegate").finish_non_exhaustive()
    }
}

impl PartialEq for PainterDelegate {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.painter, &other.painter)
    }
}
