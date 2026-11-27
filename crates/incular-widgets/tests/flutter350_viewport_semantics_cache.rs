use incular_config::{Axis, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size, time::Instant};
use incular_semantics::SemanticRole;
use incular_widgets::{
    Column, ScrollCacheExtent, SizedBox, Sliver, SliverToBoxAdapter, Viewport, Widget,
    internal::{ElementId, ExplicitSemantics, WidgetTree},
};

fn viewport(child: Widget, axis: Axis, cache: Option<ScrollCacheExtent>) -> Widget {
    let child: Box<dyn Sliver> = Box::new(SliverToBoxAdapter::new(child));
    let viewport = Viewport::new([child]).axis_direction(axis);
    match cache {
        Some(extent) => viewport.scroll_cache_extent(extent).into(),
        None => viewport.into(),
    }
}

fn frame(tree: &mut WidgetTree, size: Size) {
    tree.layout(Constraints::tight(size)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    tree.update_semantics();
}

fn cache_clip(tree: &WidgetTree, current: ElementId) -> Option<Rect> {
    tree.viewport_semantics_cache_clip(current).or_else(|| {
        tree.children(current)?
            .iter()
            .find_map(|&child| cache_clip(tree, child))
    })
}

fn vertical_clip(extension: f32) -> Rect {
    Rect::from_origin_size(
        Offset::new(0., -extension),
        Size::new(800., 600. + 2. * extension),
    )
}

#[test]
fn default_and_explicit_pixel_extents_expand_only_the_main_axis() {
    for (extent, expected) in [(None, 250.), (Some(0.), 0.), (Some(500.), 500.)] {
        let mut tree = WidgetTree::new();
        let root = tree
            .mount(viewport(
                SizedBox::square(400.).into(),
                Axis::Vertical,
                extent.map(ScrollCacheExtent::pixels),
            ))
            .unwrap();
        frame(&mut tree, Size::new(800., 600.));
        assert_eq!(cache_clip(&tree, root), Some(vertical_clip(expected)));
    }
}

#[test]
fn viewport_multiples_use_the_actual_main_axis_extent() {
    for (factor, expected) in [(0., 0.), (0.5, 300.), (1., 600.), (2.5, 1500.)] {
        let mut tree = WidgetTree::new();
        let root = tree
            .mount(viewport(
                SizedBox::square(400.).into(),
                Axis::Vertical,
                Some(ScrollCacheExtent::viewport(factor)),
            ))
            .unwrap();
        frame(&mut tree, Size::new(800., 600.));
        assert_eq!(
            cache_clip(&tree, root),
            Some(vertical_clip(expected)),
            "factor {factor}"
        );
    }
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(viewport(
            SizedBox::square(400.).into(),
            Axis::Horizontal,
            Some(ScrollCacheExtent::viewport(0.5)),
        ))
        .unwrap();
    frame(&mut tree, Size::new(800., 600.));
    assert_eq!(
        cache_clip(&tree, root),
        Some(Rect::from_origin_size(
            Offset::new(-400., 0.),
            Size::new(1600., 600.)
        ))
    );
    frame(&mut tree, Size::new(600., 600.));
    assert_eq!(
        cache_clip(&tree, root),
        Some(Rect::from_origin_size(
            Offset::new(-300., 0.),
            Size::new(1200., 600.)
        ))
    );
}

#[test]
fn the_cached_semantic_window_really_controls_offscreen_node_retention() {
    let labeled = |label| {
        Widget::box_(Size::new(200., 20.), Color::WHITE)
            .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label(label))
    };
    let child = Widget::from(
        Column::new([
            Widget::from(SizedBox::new().height(650.)),
            labeled("within cache"),
            Widget::from(SizedBox::new().height(270.)),
            labeled("beyond cache"),
        ])
        .text_direction(TextDirection::Ltr),
    );
    let scene = |factor| {
        viewport(
            child.clone(),
            Axis::Vertical,
            Some(ScrollCacheExtent::viewport(factor)),
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(0.5)).unwrap();
    frame(&mut tree, Size::new(800., 600.));
    let contains = |tree: &WidgetTree, label| {
        tree.semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some(label))
    };
    assert!(contains(&tree, "within cache"));
    assert!(!contains(&tree, "beyond cache"));

    tree.update(root, scene(0.)).unwrap();
    frame(&mut tree, Size::new(800., 600.));
    assert!(!contains(&tree, "within cache"));
}
