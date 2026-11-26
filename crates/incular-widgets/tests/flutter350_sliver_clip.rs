//! Shields content with one retained rectangular or rounded sliver clip.
use std::time::Instant;

use incular_config::{Axis, Clip, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{ClipCoverage, PaintCommand};
use incular_scroll::ScrollController;
use incular_widgets::{
    BorderRadiusGeometry, ClipOverlapBehavior, ClipShape, CustomClipper, CustomScrollView,
    Directionality, GlobalKey, Sliver, SliverClipRRect, SliverClipRect, SliverFixedExtentList,
    SliverPersistentHeader, SliverToBoxAdapter, Widget, internal::WidgetTree,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const FILL: Color = Color::rgba(22, 55, 240, 255);

fn scene(
    scroll: &ScrollController,
    sliver: Box<dyn Sliver>,
    axis: Axis,
    reverse: bool,
    header: bool,
) -> Widget {
    let mut slivers = Vec::<Box<dyn Sliver>>::new();
    if header {
        let leaf = Widget::from(incular_widgets::IgnorePointer::new(Widget::box_(
            axis.size(100., 100.),
            Color::BLACK,
        )));
        slivers.push(Box::new(
            SliverPersistentHeader::new(100., leaf).pinned(true),
        ));
    }
    slivers.push(sliver);
    slivers.push(Box::new(SliverToBoxAdapter::new(Widget::box_(
        axis.size(1000., 100.),
        Color::WHITE,
    ))));
    Directionality::new(
        TextDirection::Ltr,
        CustomScrollView::new(slivers)
            .scroll_direction(axis)
            .reverse(reverse)
            .clip_behavior(Clip::None)
            .controller(scroll.clone()),
    )
    .into()
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(VIEW)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
}

fn hit(tree: &WidgetTree, key: GlobalKey, at: Offset) -> bool {
    let child = tree.element_with_key(&key.into()).unwrap();
    tree.hit_test(at).and_then(|id| tree.element_for_render(id)) == Some(child)
}

fn rect(tree: &mut WidgetTree) -> Vec<Rect> {
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushClip { rect } => Some(*rect),
            _ => None,
        })
        .collect()
}

#[derive(Clone)]
struct Inset(f32);
impl CustomClipper for Inset {
    fn get_clip(&self, size: Size) -> ClipShape {
        ClipShape::Rect(Rect::from_origin_size(
            Offset::new(0., self.0),
            Size::new(size.width, (size.height - self.0).max(0.)),
        ))
    }
}

struct HalfHeight;
impl CustomClipper for HalfHeight {
    fn get_clip(&self, size: Size) -> ClipShape {
        ClipShape::Rect(Rect::from_origin_size(
            Offset::ZERO,
            Size::new(size.width, size.height / 2.),
        ))
    }
}

#[test]
fn custom_clip_hits_inside_and_removal_keeps_the_mounted_child() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let widget = |behavior| {
        scene(
            &controller,
            Box::new(
                SliverClipRect::new(SliverToBoxAdapter::new(
                    Widget::box_(Size::new(800., 100.), FILL).with_key(key),
                ))
                .clipper(HalfHeight)
                .clip_behavior(behavior),
            ),
            Axis::Vertical,
            false,
            false,
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget(Clip::HardEdge)).unwrap();
    layout(&mut tree);
    let child = tree.element_with_key(&key.into());
    assert_eq!(
        rect(&mut tree),
        [Rect::from_origin_size(Offset::ZERO, Size::new(800., 50.))]
    );
    assert!(hit(&tree, key, Offset::new(400., 25.)));
    assert!(!hit(&tree, key, Offset::new(400., 75.)));
    tree.update(root, widget(Clip::None)).unwrap();
    layout(&mut tree);
    assert_eq!(tree.element_with_key(&key.into()), child);
    assert!(rect(&mut tree).is_empty());
    assert!(hit(&tree, key, Offset::new(400., 75.)));
}

#[test]
fn leading_edge_stays_pinned_until_the_inside_span_scrolls_away() {
    let controller = ScrollController::new();
    let sliver = SliverClipRect::new(SliverToBoxAdapter::new(Widget::box_(
        Size::new(800., 400.),
        FILL,
    )))
    .clipper(Inset(30.));
    let mut tree = WidgetTree::new();
    tree.mount(scene(
        &controller,
        Box::new(sliver),
        Axis::Vertical,
        false,
        false,
    ))
    .unwrap();
    for (scroll, top, height) in [(0., 30., 370.), (100., 0., 300.), (385., -15., 30.)] {
        controller.jump_to(scroll);
        layout(&mut tree);
        assert_eq!(
            rect(&mut tree),
            [Rect::from_origin_size(
                Offset::new(0., top),
                Size::new(800., height)
            )],
            "scroll {scroll}"
        );
    }
}

#[test]
fn rounded_clips_reject_corner_hits_and_share_one_isolation_across_list_rows() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let rows = SliverFixedExtentList::new(5, 40., move |i| {
        let row = Widget::box_(Size::new(800., 40.), FILL);
        if i == 0 { row.with_key(key) } else { row }
    });
    let sliver = SliverClipRRect::new(rows)
        .border_radius(BorderRadiusGeometry::circular(20.))
        .clip_behavior(Clip::AntiAliasWithSaveLayer);
    let mut tree = WidgetTree::new();
    tree.mount(scene(
        &controller,
        Box::new(sliver),
        Axis::Vertical,
        false,
        false,
    ))
    .unwrap();
    layout(&mut tree);
    let mut identities = Vec::new();
    for c in tree.paint().commands() {
        if let PaintCommand::PushClipRRect {
            rrect,
            coverage,
            isolation,
        } = c
        {
            assert_eq!(*coverage, ClipCoverage::AntiAliasWithSaveLayer);
            assert_eq!(
                rrect.rect,
                Rect::from_origin_size(Offset::ZERO, Size::new(800., 200.))
            );
            identities.push(isolation.expect("shared sliver isolation").layer);
        }
    }
    assert_eq!(identities.len(), 1, "one clip, not one per row");
    assert!(!hit(&tree, key, Offset::new(1., 1.)));
    assert!(hit(&tree, key, Offset::new(40., 10.)));
}

#[test]
fn rounded_preserve_shape_uses_the_straight_span_and_physical_axis() {
    for (axis, reverse, wanted) in [
        (
            Axis::Vertical,
            false,
            Rect::from_origin_size(Offset::new(0., 70.), Size::new(800., 80.)),
        ),
        (
            Axis::Horizontal,
            false,
            Rect::from_origin_size(Offset::new(70., 0.), Size::new(80., 600.)),
        ),
        (
            Axis::Vertical,
            true,
            Rect::from_origin_size(Offset::new(0., 450.), Size::new(800., 80.)),
        ),
    ] {
        let controller = ScrollController::new();
        let sliver = SliverClipRRect::new(SliverToBoxAdapter::new(Widget::box_(
            axis.size(100., 100.),
            FILL,
        )))
        .border_radius(BorderRadiusGeometry::circular(40.))
        .clip_overlap(ClipOverlapBehavior::PreserveShape);
        let mut tree = WidgetTree::new();
        tree.mount(scene(&controller, Box::new(sliver), axis, reverse, true))
            .unwrap();
        layout(&mut tree);
        assert!(controller.jump_to(50.));
        layout(&mut tree);
        let paint = tree.paint();
        let actual: Vec<_> = paint
            .commands()
            .iter()
            .filter_map(|cmd| match cmd {
                PaintCommand::PushClipRRect {
                    rrect,
                    coverage: ClipCoverage::AntiAlias,
                    ..
                } => Some(*rrect),
                _ => None,
            })
            .collect();
        assert_eq!(
            actual,
            [incular_rendering::RRect::uniform(wanted, 40.)],
            "{axis:?} reverse={reverse}"
        );
    }
}
