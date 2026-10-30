//! Flutter's `DragBoundary` (`widgets/drag_boundary.dart`): a region that
//! dragged content can be kept inside.

use std::{cell::Cell, rc::Rc};

use incular_core::{Offset, Rect};
use incular_rendering::LayerLink;

use crate::tree::RetainedState;
use crate::{BuildContext, CompositedTransformTarget, Widget};

/// Flutter's `DragBoundaryDelegate`: decides where a dragged object may go.
pub trait DragBoundaryDelegate<T> {
    fn is_within_boundary(&self, dragged: &T) -> bool;

    fn nearest_position_within_boundary(&self, dragged: T) -> T;
}

/// The rectangle delegate of a [`DragBoundary`],
/// `_DragBoundaryDelegateForRect`. The boundary is the region's rectangle
/// when the delegate is asked, which lets it outlive the build that
/// obtained it, for example in a drag callback.
#[derive(Clone, Debug)]
pub struct RectDragBoundary {
    link: Option<LayerLink>,
    use_global_position: bool,
}

impl RectDragBoundary {
    /// The boundary rectangle, or `None` for no boundary: no
    /// [`DragBoundary`], or one not yet laid out.
    #[must_use]
    pub fn boundary(&self) -> Option<Rect> {
        let link = self.link.as_ref()?;
        let size = link.leader_size()?;
        let local = Rect::from_origin_size(Offset::ZERO, size);
        if !self.use_global_position {
            return Some(local);
        }
        Some(link.leader_transform()?.transform_rect_bbox(local))
    }
}

impl DragBoundaryDelegate<Rect> for RectDragBoundary {
    fn is_within_boundary(&self, dragged: &Rect) -> bool {
        let Some(boundary) = self.boundary() else {
            return true;
        };
        let contains = |point: Offset| {
            point.x >= boundary.origin.x
                && point.y >= boundary.origin.y
                && point.x < boundary.origin.x + boundary.size.width
                && point.y < boundary.origin.y + boundary.size.height
        };
        contains(dragged.origin)
            && contains(Offset::new(
                dragged.origin.x + dragged.size.width,
                dragged.origin.y + dragged.size.height,
            ))
    }

    /// # Panics
    /// Panics when `dragged` is larger than the boundary, where Flutter
    /// throws.
    fn nearest_position_within_boundary(&self, dragged: Rect) -> Rect {
        let Some(boundary) = self.boundary() else {
            return dragged;
        };
        let right = boundary.origin.x + boundary.size.width;
        let bottom = boundary.origin.y + boundary.size.height;
        assert!(
            right - dragged.size.width >= boundary.origin.x
                && bottom - dragged.size.height >= boundary.origin.y,
            "The rect is larger than the boundary. The rect width must be less than the \
             boundary width, and the rect height must be less than the boundary height."
        );
        Rect::from_origin_size(
            Offset::new(
                dragged
                    .origin
                    .x
                    .clamp(boundary.origin.x, right - dragged.size.width),
                dragged
                    .origin
                    .y
                    .clamp(boundary.origin.y, bottom - dragged.size.height),
            ),
            dragged.size,
        )
    }
}

#[derive(Clone)]
struct DragBoundaryScope(LayerLink);

struct DragBoundaryLink(LayerLink);

impl RetainedState for DragBoundaryLink {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

/// Flutter's `DragBoundary`: descendants keep dragged content inside this
/// widget's box through [`DragBoundary::for_rect_of`].
pub struct DragBoundary {
    child: Widget,
}

impl DragBoundary {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }

    /// `DragBoundary.forRectOf`: the nearest boundary, in view coordinates
    /// or, without `use_global_position`, the boundary's own. Without a
    /// `DragBoundary` it allows everything.
    #[must_use]
    pub fn for_rect_of(context: &BuildContext<'_>, use_global_position: bool) -> RectDragBoundary {
        Self::for_rect_maybe_of(context, use_global_position).unwrap_or(RectDragBoundary {
            link: None,
            use_global_position,
        })
    }

    /// `DragBoundary.forRectMaybeOf`: `None` without a `DragBoundary`.
    #[must_use]
    pub fn for_rect_maybe_of(
        context: &BuildContext<'_>,
        use_global_position: bool,
    ) -> Option<RectDragBoundary> {
        context
            .find::<DragBoundaryScope>()
            .map(|scope| RectDragBoundary {
                link: Some(scope.0),
                use_global_position,
            })
    }
}

impl From<DragBoundary> for Widget {
    fn from(value: DragBoundary) -> Self {
        Widget::retained_builder(
            |_| DragBoundaryLink(LayerLink::new()),
            move |link, _, _| {
                Widget::environment_scope(
                    DragBoundaryScope(link.0.clone()),
                    CompositedTransformTarget::new(link.0.clone(), value.child.clone()).into(),
                )
            },
            false,
        )
    }
}
