//! Shared retained paint scopes attached to one or more sliver boxes.

use std::{
    any::Any,
    cell::RefCell,
    sync::atomic::{AtomicU64, Ordering},
};

use incular_core::{Offset, Rect};
use incular_rendering::flutter_layer_opacity;

use super::layout::SingleInnerSliver;
use super::sliver_clip::SliverClipPlacement;
use super::{
    RenderSliver, ShowOnScreen, SliverChildId, SliverConstraints, SliverLayout,
    SliverScrollDependency,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SliverLayerKey {
    Clip(u64),
    Opacity(u64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SliverOpacityPlacement {
    key: u64,
    pub(crate) alpha: f32,
    always_include_semantics: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SliverLayerPlacement {
    Clip(SliverClipPlacement),
    Opacity(SliverOpacityPlacement),
}

impl SliverLayerPlacement {
    pub(crate) fn key(&self) -> SliverLayerKey {
        match self {
            Self::Clip(clip) => SliverLayerKey::Clip(clip.key),
            Self::Opacity(opacity) => SliverLayerKey::Opacity(opacity.key),
        }
    }

    pub(crate) fn in_parent_space(&self, origin: Offset) -> Self {
        match self {
            Self::Clip(clip) => Self::Clip(clip.in_parent_space(origin)),
            Self::Opacity(opacity) => Self::Opacity(*opacity),
        }
    }

    pub(crate) fn dependency(&self) -> Option<incular_core::reactivity::DependencySource> {
        match self {
            Self::Clip(clip) => clip.dependency(),
            Self::Opacity(_) => None,
        }
    }

    pub(crate) fn approximate_clip(&self) -> Option<Rect> {
        match self {
            Self::Clip(clip) => Some(clip.approximate_rect()),
            Self::Opacity(_) => None,
        }
    }

    pub(crate) fn hit_clip(&self) -> Option<&SliverClipPlacement> {
        match self {
            Self::Clip(clip) => Some(clip),
            Self::Opacity(_) => None,
        }
    }

    pub(crate) fn contains(&self, point: Offset) -> bool {
        match self {
            Self::Clip(clip) => super::sliver_clip::shape_contains(&clip.resolved_shape(), point),
            Self::Opacity(_) => true,
        }
    }

    pub(crate) fn hides_paint(&self) -> bool {
        matches!(self, Self::Opacity(opacity) if opacity.alpha == 0.)
    }

    pub(crate) fn hides_semantics(&self) -> bool {
        matches!(self, Self::Opacity(opacity) if opacity.alpha == 0. && !opacity.always_include_semantics)
    }
}

pub(super) struct OpacityRenderSliver {
    inner: RefCell<Box<dyn RenderSliver>>,
    placement: SliverOpacityPlacement,
}

impl OpacityRenderSliver {
    pub(super) fn new(
        inner: Box<dyn RenderSliver>,
        opacity: f32,
        always_include_semantics: bool,
    ) -> Self {
        static KEYS: AtomicU64 = AtomicU64::new(1);
        Self {
            inner: RefCell::new(inner),
            placement: SliverOpacityPlacement {
                key: KEYS.fetch_add(1, Ordering::Relaxed),
                alpha: flutter_layer_opacity(opacity),
                always_include_semantics,
            },
        }
    }
}

impl RenderSliver for OpacityRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let mut layout = self.inner.get_mut().perform_layout(constraints);
        for child in &mut layout.children {
            child.widget = child
                .widget
                .clone()
                .with_sliver_layer(SliverLayerPlacement::Opacity(self.placement));
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

    fn child_count(&self) -> Option<usize> {
        self.inner.borrow().child_count()
    }

    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        self.inner.get_mut().reveal_child(child, rect, options)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.inner.borrow().scroll_layout_dependency()
    }

    fn invalidate_child_measurement(&mut self, child: SliverChildId) {
        self.inner.get_mut().invalidate_child_measurement(child);
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        self.inner.get_mut().set_child_extent(child, extent)
    }

    fn set_child_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.inner
            .get_mut()
            .set_child_keep_alive(child, grid_tile, keep_alive)
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        self.inner
            .get_mut()
            .set_child_focus_keep_alive(child, grid_tile, keep_alive)
    }

    fn revision(&self) -> u64 {
        self.inner.borrow().revision()
    }

    fn tick(&mut self, now: incular_core::time::Instant) -> bool {
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

impl SingleInnerSliver for OpacityRenderSliver {
    fn adopt_outer_state(&mut self, old: &mut Self) {
        self.placement.key = old.placement.key;
    }

    fn inner_cell(&mut self) -> &RefCell<Box<dyn RenderSliver>> {
        &self.inner
    }
}
