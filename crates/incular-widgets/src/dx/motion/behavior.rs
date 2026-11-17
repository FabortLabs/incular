//! Site behaviors: drag sources, drop targets and reorder items plug into a
//! motion site through these hooks, sharing its drag channel, geometry and
//! interaction flags instead of nesting a second site.

use std::rc::Rc;

use incular_core::Offset;

use super::{cond::Interactions, node::MotionNode};
use crate::{BuildContext, Widget};

pub(crate) trait SiteBehavior {
    /// Each build, before the site assembles.
    fn bind(&self, _node: &Rc<MotionNode>, _context: &BuildContext<'_>) {}

    /// Wraps the assembled site; `child` is the site's own child.
    fn wrap(&self, _node: &Rc<MotionNode>, widget: Widget, _child: &Widget) -> Widget {
        widget
    }

    /// The interaction states this behavior sets.
    fn interactions(&self) -> Interactions {
        Interactions::NONE
    }

    /// Whether the node must report its geometry.
    fn wants_geometry(&self) -> bool {
        false
    }

    /// A drag began at `global`.
    fn drag_start(&self, _node: &Rc<MotionNode>, _global: Offset) {}

    /// The pointer moved to `global`.
    fn drag_update(&self, _node: &Rc<MotionNode>, _global: Offset) {}

    /// The drag ended. Returns `true` when the behavior settles the element
    /// itself, skipping the drag's `Release`.
    fn drag_end(
        &self,
        _node: &Rc<MotionNode>,
        _global: Offset,
        _velocity: Offset,
        _cancelled: bool,
    ) -> bool {
        false
    }

    /// Hands over the visual offset this behavior holds, zeroing it, when a
    /// layout change takes the element where the offset was showing it.
    /// `moved` is how far the layout moved the element.
    fn take_offset(&self, _node: &MotionNode, _moved: Offset) -> Offset {
        Offset::ZERO
    }
}
