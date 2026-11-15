//! Viewport paint order (slivers-012): Flutter `SliverPaintOrder`
//! (`rendering/viewport.dart:51`), `RenderViewport.childrenInPaintOrder` and
//! `childrenInHitTestOrder` (`rendering/viewport.dart:1243-1260`). The
//! default `firstIsTop` paints slivers last to first and hit-tests them first
//! to last; `lastIsTop` is the reverse.

use std::time::Instant;

use incular_config::Constraints;
use incular_core::{Color, Offset, Size};
use incular_rendering::PaintCommand;
use incular_scroll::ScrollController;
use incular_widgets::{
    CustomScrollView, SliverPaintOrder, Widget,
    internal::{ElementId, PinnedHeaderSliver, Sliver, SliverToBoxAdapter, WidgetTree},
};

const HEADER: Color = Color::WHITE;
const CONTENT: Color = Color::BLACK;
const SECOND: Color = Color::rgba(255, 0, 0, 255);

/// A pinned header over two content slivers, scrolled so the first content
/// sliver runs under the header and the second shows below it.
fn scrolled(order: Option<SliverPaintOrder>) -> (WidgetTree, ElementId, ElementId) {
    let controller = ScrollController::new();
    let slivers: Vec<Box<dyn Sliver>> = vec![
        Box::new(PinnedHeaderSliver::new(Widget::box_(
            Size::new(100., 20.),
            HEADER,
        ))),
        Box::new(SliverToBoxAdapter::new(Widget::box_(
            Size::new(100., 220.),
            CONTENT,
        ))),
        Box::new(SliverToBoxAdapter::new(Widget::box_(
            Size::new(100., 220.),
            SECOND,
        ))),
    ];
    let mut view = CustomScrollView::new(slivers).controller(controller.clone());
    if let Some(order) = order {
        view = view.paint_order(order);
    }
    let mut tree = WidgetTree::new();
    let root = tree.mount(view.into()).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    assert!(controller.jump_to(180.));
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("scrolled layout");
    tree.update_compositor(Instant::now()).expect("compositor");
    let children = tree.children(root).expect("sliver children");
    let (header, content) = (children[0], children[1]);
    (tree, header, content)
}

fn rect_colors(tree: &mut WidgetTree) -> Vec<Color> {
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

#[test]
fn first_is_top_is_the_default_and_paints_slivers_last_to_first() {
    let (mut tree, header, _) = scrolled(None);
    // The last sliver paints first; the first (the header) paints last, on top.
    let colors = rect_colors(&mut tree);
    let position = |color| colors.iter().position(|painted| *painted == color);
    assert!(colors.contains(&SECOND), "{colors:?}");
    assert!(position(SECOND) < position(CONTENT), "{colors:?}");
    assert!(position(CONTENT) < position(HEADER), "{colors:?}");
    // Hit testing runs first to last: the header claims the overlap.
    let header_render = tree.render_id(header).expect("header render");
    assert_eq!(tree.hit_test(Offset::new(10., 5.)), Some(header_render));
}

#[test]
fn last_is_top_paints_slivers_first_to_last() {
    let (mut tree, _, content) = scrolled(Some(SliverPaintOrder::LastIsTop));
    let colors = rect_colors(&mut tree);
    let position = |color| colors.iter().position(|painted| *painted == color);
    assert!(colors.contains(&SECOND), "{colors:?}");
    assert!(position(HEADER) < position(CONTENT), "{colors:?}");
    assert!(position(CONTENT) < position(SECOND), "{colors:?}");
    // The later content sliver is on top of the pinned header, and is hit.
    let content_render = tree.render_id(content).expect("content render");
    assert_eq!(tree.hit_test(Offset::new(10., 5.)), Some(content_render));
}
