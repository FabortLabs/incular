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

#[test]
fn overlap_hit_testing_respects_policy_in_both_paint_directions() {
    use ClipOverlapBehavior::{FollowEdge, None, PreserveShape};
    let cases = [
        (FollowEdge, Axis::Vertical, false, false),
        (None, Axis::Vertical, false, true),
        (PreserveShape, Axis::Vertical, false, false),
        (FollowEdge, Axis::Horizontal, false, false),
        (None, Axis::Horizontal, false, true),
        (None, Axis::Vertical, true, true),
        (FollowEdge, Axis::Horizontal, true, false),
    ];
    for rounded in [false, true] {
        for (policy, axis, reverse, overlaps) in cases {
            let controller = ScrollController::new();
            let key = GlobalKey::new();
            let child =
                SliverToBoxAdapter::new(Widget::box_(axis.size(100., 100.), FILL).with_key(key));
            let sliver: Box<dyn Sliver> = if rounded {
                Box::new(
                    SliverClipRRect::new(child)
                        .border_radius(BorderRadiusGeometry::circular(8.))
                        .clip_overlap(policy),
                )
            } else {
                Box::new(SliverClipRect::new(child).clip_overlap(policy))
            };
            let mut tree = WidgetTree::new();
            tree.mount(scene(&controller, sliver, axis, reverse, true))
                .unwrap();
            layout(&mut tree);
            controller.jump_to(50.);
            layout(&mut tree);
            let logical = |main| {
                let physical = if reverse {
                    axis.main_extent(VIEW) - 50. - main
                } else {
                    50. + main
                };
                axis.offset(physical, 100.)
            };
            assert_eq!(
                hit(&tree, key, logical(25.)),
                overlaps,
                "{axis:?}, reverse={reverse}, {policy:?}, rounded={rounded}"
            );
            assert!(
                hit(&tree, key, logical(75.)),
                "{axis:?}, reverse={reverse}, {policy:?}, rounded={rounded}"
            );
        }
    }
}

#[derive(Clone)]
struct WatchedClip {
    top: std::rc::Rc<std::cell::Cell<f32>>,
    calls: std::rc::Rc<std::cell::Cell<usize>>,
    reclip: incular_widgets::ClipReclip,
}

impl CustomClipper for WatchedClip {
    fn get_clip(&self, size: Size) -> ClipShape {
        self.calls.set(self.calls.get() + 1);
        Inset(self.top.get()).get_clip(size)
    }

    fn reclip(&self) -> Option<incular_widgets::ClipReclip> {
        Some(self.reclip.clone())
    }
}

#[test]
fn live_reclip_updates_hit_and_paint_without_rebuilding_or_laying_out() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let watched = WatchedClip {
        top: std::rc::Rc::new(std::cell::Cell::new(50.)),
        calls: std::rc::Rc::new(std::cell::Cell::new(0)),
        reclip: incular_widgets::ClipReclip::default(),
    };
    let sliver = SliverClipRect::new(SliverToBoxAdapter::new(
        Widget::box_(Size::new(800., 100.), FILL).with_key(key),
    ))
    .clipper(watched.clone());
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
    let child = tree.element_with_key(&key.into());
    assert_eq!(
        rect(&mut tree),
        [Rect::from_origin_size(
            Offset::new(0., 50.),
            Size::new(800., 50.)
        )]
    );
    assert!(!hit(&tree, key, Offset::new(400., 40.)));

    let before = tree.diagnostics();
    let calls = watched.calls.get();
    watched.top.set(30.);
    watched.reclip.notify();
    assert!(hit(&tree, key, Offset::new(400., 40.)));
    assert_eq!(
        rect(&mut tree),
        [Rect::from_origin_size(
            Offset::new(0., 30.),
            Size::new(800., 70.)
        )]
    );
    assert_eq!(watched.calls.get(), calls + 1, "one new shared clip shape");
    assert_eq!(tree.element_with_key(&key.into()), child);
    assert_eq!(tree.diagnostics().layouts, before.layouts);
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
}

#[derive(Clone)]
struct ReplaceableInset {
    top: f32,
    invalidate: bool,
}

impl CustomClipper for ReplaceableInset {
    fn get_clip(&self, size: Size) -> ClipShape {
        Inset(self.top).get_clip(size)
    }

    fn should_reclip(&self, _: &Self) -> bool {
        self.invalidate
    }
}

#[test]
fn replacement_respects_should_reclip_until_paint_geometry_changes() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let widget = |top, invalidate| {
        scene(
            &controller,
            Box::new(
                SliverClipRect::new(SliverToBoxAdapter::new(
                    Widget::box_(Size::new(800., 100.), FILL).with_key(key),
                ))
                .clipper(ReplaceableInset { top, invalidate }),
            ),
            Axis::Vertical,
            false,
            false,
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget(50., false)).unwrap();
    layout(&mut tree);
    let child = tree.element_with_key(&key.into());
    assert!(!hit(&tree, key, Offset::new(400., 40.)));

    tree.update(root, widget(30., false)).unwrap();
    layout(&mut tree);
    assert_eq!(rect(&mut tree)[0].origin.y, 50.);
    assert!(!hit(&tree, key, Offset::new(400., 40.)));

    assert!(controller.jump_to(10.));
    layout(&mut tree);
    assert_eq!(rect(&mut tree)[0].origin.y, 20.);
    assert!(hit(&tree, key, Offset::new(400., 40.)));

    tree.update(root, widget(55., true)).unwrap();
    layout(&mut tree);
    assert_eq!(rect(&mut tree)[0].origin.y, 45.);
    assert!(!hit(&tree, key, Offset::new(400., 40.)));
    assert_eq!(tree.element_with_key(&key.into()), child);
}

#[derive(Clone)]
struct ApproximateInset {
    semantic_top: std::rc::Rc<std::cell::Cell<f32>>,
    reclip: incular_widgets::ClipReclip,
}

impl CustomClipper for ApproximateInset {
    fn get_clip(&self, size: Size) -> ClipShape {
        Inset(20.).get_clip(size)
    }

    fn get_approximate_clip_rect(&self, size: Size) -> Rect {
        let top = self.semantic_top.get();
        Rect::from_origin_size(
            Offset::new(0., top),
            Size::new(size.width, (size.height - top).max(0.)),
        )
    }

    fn reclip(&self) -> Option<incular_widgets::ClipReclip> {
        Some(self.reclip.clone())
    }
}

#[test]
fn approximate_clip_controls_accessibility_bounds_and_membership_independently() {
    use incular_widgets::{Column, Semantics, SizedBox};
    let controller = ScrollController::new();
    let approximate = ApproximateInset {
        semantic_top: std::rc::Rc::new(std::cell::Cell::new(50.)),
        reclip: incular_widgets::ClipReclip::default(),
    };
    let label = |name, height| {
        Widget::from(
            Semantics::new(SizedBox::new().width(800.).height(height))
                .container(true)
                .label(name),
        )
    };
    let child = Column::new([
        label("before", 40.),
        label("crossing", 30.),
        label("after", 30.),
    ]);
    let clip = SliverClipRect::new(SliverToBoxAdapter::new(child)).clipper(approximate.clone());
    let mut tree = WidgetTree::new();
    let root = Directionality::new(
        TextDirection::Ltr,
        CustomScrollView::new(vec![Box::new(clip) as Box<dyn Sliver>]).controller(controller),
    );
    tree.mount(root.into()).unwrap();
    layout(&mut tree);
    tree.update_semantics();
    let find = |tree: &WidgetTree, label| {
        tree.semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some(label))
            .map(|(id, node)| (id, node.bounds, node.state.hidden))
    };
    assert_eq!(find(&tree, "before"), None);
    let (crossing, bounds, hidden) = find(&tree, "crossing").unwrap();
    assert_eq!(
        bounds,
        Rect::from_origin_size(Offset::new(0., 50.), Size::new(800., 20.))
    );
    assert!(!hidden);
    assert_eq!(
        find(&tree, "after").unwrap().1,
        Rect::from_origin_size(Offset::new(0., 70.), Size::new(800., 30.))
    );

    let before = tree.diagnostics();
    approximate.semantic_top.set(0.);
    approximate.reclip.notify();
    tree.update_semantics();
    assert!(find(&tree, "before").is_some());
    assert_eq!(
        find(&tree, "crossing"),
        Some((
            crossing,
            Rect::from_origin_size(Offset::new(0., 40.), Size::new(800., 30.)),
            false
        ))
    );
    assert!(rect(&mut tree).contains(&Rect::from_origin_size(
        Offset::new(0., 20.),
        Size::new(800., 80.)
    )));
    assert_eq!(tree.diagnostics().layouts, before.layouts);
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
}

fn clipped_paint(tree: &mut WidgetTree, rounded: bool) -> Vec<Rect> {
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushClip { rect } if !rounded => Some(*rect),
            PaintCommand::PushClipRRect {
                rrect,
                coverage: ClipCoverage::AntiAlias,
                ..
            } if rounded => Some(rrect.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn live_clip_options_follow_overlap_even_when_child_geometry_is_unchanged() {
    for rounded in [false, true] {
        let controller = ScrollController::new();
        let key = GlobalKey::new();
        let build = |policy, behavior| {
            let header: Box<dyn Sliver> = Box::new(
                SliverPersistentHeader::new(
                    100.,
                    incular_widgets::IgnorePointer::new(Widget::box_(
                        Size::new(800., 100.),
                        Color::BLACK,
                    )),
                )
                .pinned(true),
            );
            let leaf =
                SliverToBoxAdapter::new(Widget::box_(Size::new(800., 100.), FILL).with_key(key));
            let clip: Box<dyn Sliver> = if rounded {
                Box::new(
                    SliverClipRRect::new(leaf)
                        .clip_behavior(behavior)
                        .clip_overlap(policy),
                )
            } else {
                Box::new(
                    SliverClipRect::new(leaf)
                        .clip_behavior(behavior)
                        .clip_overlap(policy),
                )
            };
            Directionality::new(
                TextDirection::Ltr,
                CustomScrollView::new(vec![
                    header,
                    Box::new(SliverToBoxAdapter::new(
                        incular_widgets::SizedBox::new().height(100.),
                    )),
                    clip,
                    Box::new(SliverToBoxAdapter::new(
                        incular_widgets::SizedBox::new().height(1000.),
                    )),
                ])
                .clip_behavior(Clip::None)
                .controller(controller.clone()),
            )
            .into()
        };
        let normal = if rounded {
            Clip::AntiAlias
        } else {
            Clip::HardEdge
        };
        let mut tree = WidgetTree::new();
        let root = tree
            .mount(build(ClipOverlapBehavior::FollowEdge, normal))
            .unwrap();
        layout(&mut tree);
        let leaf = tree.element_with_key(&key.into()).unwrap();
        let clip_at =
            |y, height| Rect::from_origin_size(Offset::new(0., y), Size::new(800., height));
        assert_eq!(clipped_paint(&mut tree, rounded), [clip_at(200., 100.)]);
        assert!(controller.jump_to(150.));
        layout(&mut tree);
        assert_eq!(
            tree.element_bounds(leaf).unwrap().size,
            Size::new(800., 100.)
        );
        assert_eq!(clipped_paint(&mut tree, rounded), [clip_at(100., 50.)]);
        assert!(!hit(&tree, key, Offset::new(100., 75.)));

        tree.update(root, build(ClipOverlapBehavior::None, normal))
            .unwrap();
        layout(&mut tree);
        assert_eq!(clipped_paint(&mut tree, rounded), [clip_at(50., 100.)]);
        assert!(hit(&tree, key, Offset::new(100., 75.)));

        tree.update(root, build(ClipOverlapBehavior::FollowEdge, Clip::None))
            .unwrap();
        layout(&mut tree);
        assert!(clipped_paint(&mut tree, rounded).is_empty());
        assert!(hit(&tree, key, Offset::new(100., 75.)));

        tree.update(root, build(ClipOverlapBehavior::None, normal))
            .unwrap();
        layout(&mut tree);
        assert_eq!(clipped_paint(&mut tree, rounded), [clip_at(50., 100.)]);
        assert_eq!(tree.element_with_key(&key.into()), Some(leaf));
        assert_eq!(controller.offset(), 150.);
    }
}

#[test]
fn directional_corners_recompute_when_the_radius_or_ambient_direction_changes() {
    use incular_widgets::{BorderRadius, BorderRadiusDirectional, Radius};
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let build = |direction, radius| {
        let sliver = SliverClipRRect::new(SliverToBoxAdapter::new(
            Widget::box_(Size::new(800., 100.), FILL).with_key(key),
        ))
        .border_radius(radius);
        Directionality::new(
            direction,
            CustomScrollView::new(vec![Box::new(sliver) as Box<dyn Sliver>])
                .clip_behavior(Clip::None)
                .controller(controller.clone()),
        )
        .into()
    };
    let logical = BorderRadiusGeometry::from(BorderRadiusDirectional::only(
        Radius::circular(10.),
        Radius::ZERO,
        Radius::ZERO,
        Radius::ZERO,
    ));
    let physical = BorderRadiusGeometry::from(BorderRadius::only(
        Radius::ZERO,
        Radius::circular(20.),
        Radius::ZERO,
        Radius::ZERO,
    ));
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(TextDirection::Ltr, logical)).unwrap();
    layout(&mut tree);
    let leaf = tree.element_with_key(&key.into()).unwrap();
    for (direction, radii, expected) in [
        (TextDirection::Ltr, logical, (10., 0.)),
        (TextDirection::Ltr, physical, (0., 20.)),
        (TextDirection::Ltr, logical, (10., 0.)),
        (TextDirection::Rtl, logical, (0., 10.)),
    ] {
        tree.update(root, build(direction, radii)).unwrap();
        layout(&mut tree);
        let paint = tree.paint();
        let command = paint
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::PushClipRRect { rrect, .. } => Some(*rrect),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            command.rect,
            Rect::from_origin_size(Offset::ZERO, Size::new(800., 100.))
        );
        assert_eq!((command.radii.top_left, command.radii.top_right), expected);
        assert_eq!(tree.element_with_key(&key.into()), Some(leaf));
    }
}

#[derive(Clone)]
struct ClipSizeLog(std::rc::Rc<std::cell::RefCell<Vec<Size>>>);

impl CustomClipper for ClipSizeLog {
    fn get_clip(&self, size: Size) -> ClipShape {
        self.0.borrow_mut().push(size);
        ClipShape::Rect(Rect::from_origin_size(Offset::ZERO, size))
    }
}

#[test]
fn custom_clipper_only_sees_the_committed_child_size() {
    let controller = ScrollController::new();
    let sizes = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let clip = SliverClipRect::new(SliverToBoxAdapter::new(Widget::box_(
        Size::new(800., 100.),
        FILL,
    )))
    .clipper(ClipSizeLog(sizes.clone()));
    let mut tree = WidgetTree::new();
    tree.mount(scene(
        &controller,
        Box::new(clip),
        Axis::Vertical,
        false,
        false,
    ))
    .unwrap();
    layout(&mut tree);
    assert_eq!(
        rect(&mut tree),
        [Rect::from_origin_size(Offset::ZERO, Size::new(800., 100.))]
    );
    assert_eq!(*sizes.borrow(), [Size::new(800., 100.)]);
}
