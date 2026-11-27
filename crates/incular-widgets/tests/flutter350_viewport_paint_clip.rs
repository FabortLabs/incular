use incular_config::{Axis, Clip, Constraints};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::PaintCommand;
use incular_scroll::{SliverConstraints, SliverGeometry};
use incular_widgets::{
    CustomScrollView, ScrollController, ShrinkWrappingViewport, SizedBox, SliverEnsureSemantics,
    Widget,
    internal::{
        ElementId, PinnedHeaderSliver, RenderSliver, Sliver, SliverLayout, SliverToBoxAdapter,
        WidgetTree,
    },
};

fn mount(view: CustomScrollView) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(view.into()).unwrap();
    layout(&mut tree);
    (tree, root)
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
}

fn sliver(axis: Axis, extent: f32) -> SliverToBoxAdapter {
    SliverToBoxAdapter::new(Widget::box_(axis.size(extent, 20.), Color::WHITE))
}

#[test]
fn fitting_sliver_reports_clip_by_behavior_without_a_redundant_paint_layer() {
    let full = Rect::from_origin_size(incular_core::Offset::ZERO, Size::new(100., 100.));
    for clip in [
        None,
        Some(Clip::None),
        Some(Clip::AntiAlias),
        Some(Clip::AntiAliasWithSaveLayer),
    ] {
        let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(sliver(Axis::Vertical, 20.))];
        let view = CustomScrollView::new(slivers);
        let view = match clip {
            Some(clip) => view.clip_behavior(clip),
            None => view,
        };
        let (mut tree, root) = mount(view);
        let child = tree.children(root).unwrap()[0];
        assert_eq!(
            tree.viewport_approximate_paint_clip(root, child),
            (clip != Some(Clip::None)).then_some(full),
            "{clip:?}"
        );
        assert_eq!(tree.viewport_approximate_paint_clip(root, root), None);
        assert_eq!(tree.viewport_approximate_paint_clip(child, root), None);
        assert!(!tree.paint().commands().iter().any(|command| matches!(
            command,
            PaintCommand::PushClip { .. } | PaintCommand::PushClipRRect { .. }
        )));
    }
}

#[test]
fn pinned_overlap_adjusts_only_the_child_clip_at_the_physical_leading_edge() {
    let full = Rect::from_origin_size(incular_core::Offset::ZERO, Size::new(100., 100.));
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            for clip in [Clip::HardEdge, Clip::None] {
                let controller = ScrollController::new();
                let slivers: Vec<Box<dyn Sliver>> = vec![
                    Box::new(PinnedHeaderSliver::new(Widget::box_(
                        axis.size(20., 20.),
                        Color::BLACK,
                    ))),
                    Box::new(sliver(axis, 220.)),
                ];
                let (mut tree, root) = mount(
                    CustomScrollView::new(slivers)
                        .scroll_direction(axis)
                        .reverse(reverse)
                        .clip_behavior(clip)
                        .controller(controller.clone()),
                );
                assert!(controller.jump_to(60.));
                layout(&mut tree);
                let children = tree.children(root).unwrap();
                let origin = axis.offset(if reverse { 0. } else { 20. }, 0.);
                let under_header = Rect::from_origin_size(origin, axis.size(80., 100.));
                assert_eq!(
                    tree.viewport_approximate_paint_clip(root, children[0]),
                    (clip != Clip::None).then_some(full),
                );
                assert_eq!(
                    tree.viewport_approximate_paint_clip(root, children[1]),
                    (clip != Clip::None).then_some(under_header),
                    "{axis:?}, reverse={reverse}, clip={clip:?}"
                );
            }
        }
    }
}

#[test]
fn ensured_sliver_outside_the_cache_is_not_clipped_by_the_query() {
    for (cache, should_clip) in [(0., false), (150., true)] {
        let slivers: Vec<Box<dyn Sliver>> = vec![
            Box::new(sliver(Axis::Vertical, 220.)),
            Box::new(SliverEnsureSemantics::new(sliver(Axis::Vertical, 20.))),
        ];
        let (tree, root) = mount(CustomScrollView::new(slivers).cache_extent(cache));
        let children = tree.children(root).unwrap();
        assert_eq!(
            tree.viewport_approximate_paint_clip(root, children[1])
                .is_some(),
            should_clip,
            "cache={cache}"
        );
    }
}

#[derive(Clone, Copy)]
struct OnePixelOverlap;

impl Sliver for OnePixelOverlap {
    fn build(&self, _controller: &ScrollController) -> Widget {
        SizedBox::shrink().into()
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(*self)
    }
}

impl RenderSliver for OnePixelOverlap {
    fn perform_layout(&mut self, _constraints: SliverConstraints) -> SliverLayout {
        SliverLayout {
            geometry: SliverGeometry {
                paint_extent: 1.,
                max_paint_extent: 1.,
                ..SliverGeometry::ZERO
            },
            children: Vec::new(),
            absorbed_overlap: 0.,
        }
    }
}

#[test]
fn unbounded_shrink_wrapping_ignores_overlap_in_the_finite_paint_clip() {
    let size = Size::new(800., 600.);
    let full = Rect::from_origin_size(Offset::ZERO, size);
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            let slivers: Vec<Box<dyn Sliver>> =
                vec![Box::new(OnePixelOverlap), Box::new(sliver(axis, 40.))];
            let mut tree = WidgetTree::new();
            let root = tree
                .mount(
                    ShrinkWrappingViewport::new(slivers)
                        .axis_direction(axis)
                        .reverse(reverse)
                        .into(),
                )
                .unwrap();
            let bounded = Constraints::tight(size);
            let unbounded = match axis {
                Axis::Vertical => Constraints::new(800., 800., 600., f32::INFINITY),
                Axis::Horizontal => Constraints::new(800., f32::INFINITY, 600., 600.),
            };
            let inset_clip = Rect::from_origin_size(
                axis.offset(if reverse { 0. } else { 1. }, 0.),
                axis.size(axis.main_extent(size) - 1., axis.cross_extent(size)),
            );
            for (constraints, expected) in
                [(bounded, inset_clip), (unbounded, full), (bounded, inset_clip)]
            {
                tree.layout(constraints).unwrap();
                tree.update_compositor(incular_core::time::Instant::now())
                    .unwrap();
                let child = tree.children(root).unwrap()[0];
                assert_eq!(
                    tree.viewport_approximate_paint_clip(root, child),
                    Some(expected),
                    "{axis:?}, reverse={reverse}, {constraints:?}"
                );
            }
        }
    }
}
