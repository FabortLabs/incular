//! Flutter's `SliverSemantics` and `SliverEnsureSemantics`
//! (`widgets/sliver.dart`, `widgets/basic.dart`).
//!
//! A viewport's sliver children are siblings of one retained element, so a
//! sliver has no element of its own to carry a semantic node. A semantics
//! sliver therefore adds an anchor child that covers the sliver's scroll
//! extent and carries the annotations; the semantics pass places the
//! sliver's other children under the anchor's node.

use std::any::Any;

use super::*;
use crate::Semantics;

/// The low half of an anchor's child id; no list reaches this index.
const ANCHOR: u64 = u32::MAX as u64;

/// Whether `id` is the anchor a [`SliverSemantics`] adds.
pub(crate) fn is_semantics_anchor(id: SliverChildId) -> bool {
    id.0 & u32::MAX as u64 == ANCHOR
}

/// Whether `child` was produced by the sliver whose anchor is `anchor`: the
/// anchor's scope path, outermost scope in the lowest byte, is a suffix of
/// the child's.
pub(crate) fn in_semantics_group(anchor: SliverChildId, child: SliverChildId) -> bool {
    let path = anchor.0 >> 32;
    let bits = 64 - path.leading_zeros();
    let width = bits.div_ceil(8) * 8;
    let mask = if width >= 64 {
        u64::MAX
    } else {
        (1_u64 << width) - 1
    };
    child != anchor && (child.0 >> 32) & mask == path
}

/// Flutter's `SliverSemantics`: annotates a sliver with one semantic node
/// whose children are the sliver's children. The node takes the
/// `semantics` properties; its child is ignored.
pub struct SliverSemantics {
    semantics: Rc<dyn Fn() -> Semantics>,
    sliver: Box<dyn Sliver>,
}

impl SliverSemantics {
    #[must_use]
    pub fn new(semantics: impl Fn() -> Semantics + 'static, sliver: impl Sliver + 'static) -> Self {
        Self {
            semantics: Rc::new(semantics),
            sliver: Box::new(sliver),
        }
    }
}

impl Sliver for SliverSemantics {
    fn build(&self, controller: &ScrollController) -> Widget {
        (self.semantics)()
            .with_child(self.sliver.build(controller))
            .into()
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(SemanticsRenderSliver {
            inner: RefCell::new(self.sliver.create_render_sliver(controller, axis, reverse)),
            semantics: self.semantics.clone(),
        })
    }
}

pub(super) struct SemanticsRenderSliver {
    inner: RefCell<Box<dyn RenderSliver>>,
    semantics: Rc<dyn Fn() -> Semantics>,
}

impl super::layout::SingleInnerSliver for SemanticsRenderSliver {
    fn inner_cell(&mut self) -> &RefCell<Box<dyn RenderSliver>> {
        &self.inner
    }
}

impl RenderSliver for SemanticsRenderSliver {
    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.inner.borrow().scroll_layout_dependency()
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let mut layout = self.inner.borrow_mut().perform_layout(constraints);
        let extent = layout.geometry.scroll_extent.max(0.);
        // The anchor covers the whole sliver behind its children, so its
        // bounds are the sliver's and it neither paints nor takes hits.
        layout.children.insert(
            0,
            SliverChildLayout {
                id: SliverChildId(ANCHOR),
                widget: (self.semantics)()
                    .with_child(crate::SizedBox::expand())
                    .into(),
                semantic_index: None,
                offset: 0.,
                cross_offset: 0.,
                constraints: sliver_child_constraints(
                    constraints.axis,
                    constraints.cross_axis_extent,
                    Some(extent),
                ),
                extent,
                placement: SliverChildPlacement::Flow,
            },
        );
        layout
    }

    fn child_count(&self) -> Option<usize> {
        self.inner.borrow().child_count()
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        if !is_semantics_anchor(child) {
            self.inner.borrow_mut().invalidate_child_measurement(child);
        }
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        !is_semantics_anchor(child) && self.inner.borrow_mut().set_child_extent(child, extent)
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

/// Flutter's `SliverEnsureSemantics`: keeps a sliver in the semantics tree
/// while it is outside the viewport and its cache extent. Incular's
/// viewport already keeps every child its slivers build in the semantics
/// tree, wherever it is, so this passes the sliver through unchanged.
pub struct SliverEnsureSemantics {
    sliver: Box<dyn Sliver>,
}

impl SliverEnsureSemantics {
    #[must_use]
    pub fn new(sliver: impl Sliver + 'static) -> Self {
        Self {
            sliver: Box::new(sliver),
        }
    }
}

impl Sliver for SliverEnsureSemantics {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.sliver.build(controller)
    }

    fn cross_axis_flex(&self) -> Option<usize> {
        self.sliver.cross_axis_flex()
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        self.sliver.build_with_config(controller, axis, reverse)
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        self.sliver.create_render_sliver(controller, axis, reverse)
    }
}
