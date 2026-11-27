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

#[derive(Clone)]
struct HalfWidth;
impl CustomClipper for HalfWidth {
    fn get_clip(&self, size: Size) -> ClipShape {
        ClipShape::Rect(Rect::from_origin_size(
            Offset::ZERO,
            Size::new(size.width / 2., size.height),
        ))
    }
}

#[test]
fn custom_clippers_keep_physical_half_width_and_half_height_under_axis_reversal() {
    for (axis, reverse) in [(Axis::Horizontal, false), (Axis::Vertical, true)] {
        let controller = ScrollController::new();
        let key = GlobalKey::new();
        let leaf = SliverToBoxAdapter::new(Widget::box_(axis.size(100., 100.), FILL).with_key(key));
        let clip = SliverClipRect::new(leaf);
        let clip = if axis.is_vertical() {
            clip.clipper(HalfHeight)
        } else {
            clip.clipper(HalfWidth)
        };
        let mut tree = WidgetTree::new();
        tree.mount(scene(&controller, Box::new(clip), axis, reverse, false))
            .unwrap();
        layout(&mut tree);
        let (inside, outside) = if reverse {
            (Offset::new(400., 525.), Offset::new(400., 575.))
        } else {
            (Offset::new(25., 50.), Offset::new(75., 50.))
        };
        assert!(hit(&tree, key, inside));
        assert!(!hit(&tree, key, outside));
    }
}

#[derive(Clone)]
struct RoundedInset(f32);
impl CustomClipper for RoundedInset {
    fn get_clip(&self, size: Size) -> ClipShape {
        let ClipShape::Rect(rect) = Inset(self.0).get_clip(size) else {
            unreachable!()
        };
        ClipShape::RRect(incular_rendering::RRect::uniform(rect, 0.))
    }
}

#[test]
fn overlap_never_expands_the_clippers_custom_inset() {
    for rounded in [false, true] {
        let controller = ScrollController::new();
        let leaf = SliverToBoxAdapter::new(Widget::box_(Size::new(800., 200.), FILL));
        let clip: Box<dyn Sliver> = if rounded {
            Box::new(SliverClipRRect::new(leaf).clipper(RoundedInset(30.)))
        } else {
            Box::new(SliverClipRect::new(leaf).clipper(Inset(30.)))
        };
        let header =
            SliverPersistentHeader::new(100., Widget::box_(Size::new(800., 100.), Color::BLACK))
                .pinned(true);
        let viewport = Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(vec![
                Box::new(header) as Box<dyn Sliver>,
                Box::new(SliverToBoxAdapter::new(
                    incular_widgets::SizedBox::new().height(50.),
                )),
                clip,
                Box::new(SliverToBoxAdapter::new(
                    incular_widgets::SizedBox::new().height(1000.),
                )),
            ])
            .clip_behavior(Clip::None)
            .controller(controller.clone()),
        );
        let mut tree = WidgetTree::new();
        tree.mount(viewport.into()).unwrap();
        layout(&mut tree);
        for (scroll, y, height) in [(60., 120., 170.), (100., 100., 150.)] {
            assert!(controller.jump_to(scroll));
            layout(&mut tree);
            assert_eq!(
                clipped_paint(&mut tree, rounded),
                [Rect::from_origin_size(
                    Offset::new(0., y),
                    Size::new(800., height)
                )],
            );
        }
    }
}

#[test]
fn default_rounded_clip_hits_after_the_corner_arc() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let clip = SliverClipRRect::new(SliverToBoxAdapter::new(
        Widget::box_(Size::new(800., 100.), FILL).with_key(key),
    ))
    .border_radius(BorderRadiusGeometry::circular(20.));
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
    assert!(!hit(&tree, key, Offset::ZERO));
    assert!(hit(&tree, key, Offset::new(400., 50.)));
    assert!(hit(&tree, key, Offset::new(25., 25.)));
}

#[derive(Clone)]
struct HalfWidthWithInset;

impl CustomClipper for HalfWidthWithInset {
    fn get_clip(&self, size: Size) -> ClipShape {
        ClipShape::Rect(Rect::from_origin_size(
            Offset::new(0., 10.),
            Size::new(size.width / 2., (size.height - 10.).max(0.)),
        ))
    }
}

#[test]
fn nested_sliver_clips_share_layers_and_both_gate_pointer_hits() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let build = |outer_behavior| {
        let rows = SliverFixedExtentList::new(5, 40., move |i| {
            let row = Widget::box_(Size::new(800., 40.), FILL);
            if i == 0 { row.with_key(key) } else { row }
        });
        let inner = SliverClipRRect::new(rows)
            .border_radius(BorderRadiusGeometry::circular(20.))
            .clip_behavior(Clip::AntiAliasWithSaveLayer);
        let outer = SliverClipRect::new(inner)
            .clipper(HalfWidthWithInset)
            .clip_behavior(outer_behavior);
        scene(&controller, Box::new(outer), Axis::Vertical, false, false)
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(Clip::HardEdge)).unwrap();
    layout(&mut tree);
    let leaf = tree.element_with_key(&key.into()).unwrap();
    let active_clips = |tree: &mut WidgetTree| {
        let paint = tree.paint();
        paint
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::PushClip { .. } => Some("outer"),
                PaintCommand::PushClipRRect {
                    isolation: Some(_), ..
                } => Some("inner"),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(active_clips(&mut tree), ["outer", "inner"]);
    assert!(hit(&tree, key, Offset::new(50., 20.)));
    assert!(!hit(&tree, key, Offset::new(450., 20.)));
    assert!(!hit(&tree, key, Offset::new(0., 10.)));

    tree.update(root, build(Clip::None)).unwrap();
    layout(&mut tree);
    assert_eq!(tree.element_with_key(&key.into()), Some(leaf));
    assert_eq!(active_clips(&mut tree), ["inner"]);
    assert!(hit(&tree, key, Offset::new(450., 20.)));
    assert!(!hit(&tree, key, Offset::new(0., 10.)));
}

#[test]
fn both_default_sliver_clip_shapes_paint_their_text_child() {
    for rounded in [false, true] {
        let controller = ScrollController::new();
        let key = GlobalKey::new();
        let text = Widget::from(incular_widgets::Text::new("Hello World")).with_key(key);
        let leaf = SliverToBoxAdapter::new(text);
        let clip: Box<dyn Sliver> = if rounded {
            Box::new(SliverClipRRect::new(leaf).border_radius(BorderRadiusGeometry::circular(10.)))
        } else {
            Box::new(SliverClipRect::new(leaf))
        };
        let mut tree = WidgetTree::new();
        tree.mount(scene(&controller, clip, Axis::Vertical, false, false))
            .unwrap();
        layout(&mut tree);
        let element = tree.element_with_key(&key.into()).unwrap();
        assert!(tree.element_bounds(element).unwrap().size.height > 0.);
        assert!(
            tree.paint()
                .commands()
                .iter()
                .any(|op| matches!(op, PaintCommand::GlyphRun { .. }))
        );
    }
}

fn rounded_overlap_probe(
    controller: &ScrollController,
    key: GlobalKey,
    reverse: bool,
    policy: ClipOverlapBehavior,
    radius: f32,
    extent: f32,
    behavior: Clip,
) -> Widget {
    let child = SliverToBoxAdapter::new(Widget::box_(Size::new(800., extent), FILL).with_key(key));
    let clip = SliverClipRRect::new(child)
        .border_radius(BorderRadiusGeometry::circular(radius))
        .clip_overlap(policy)
        .clip_behavior(behavior);
    scene(controller, Box::new(clip), Axis::Vertical, reverse, true)
}

#[test]
fn ignored_overlap_does_not_round_a_new_corner_at_the_pinned_header_edge() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let mut tree = WidgetTree::new();
    tree.mount(rounded_overlap_probe(
        &controller,
        key,
        false,
        ClipOverlapBehavior::None,
        50.,
        200.,
        Clip::AntiAlias,
    ))
    .unwrap();
    layout(&mut tree);
    assert!(controller.jump_to(50.));
    layout(&mut tree);
    let child = tree.element_with_key(&key.into()).unwrap();
    let origin = tree.element_bounds(child).unwrap().origin;
    assert_eq!(origin, Offset::new(0., 50.));
    assert!(hit(&tree, key, origin + Offset::new(1., 51.)));
}

#[test]
fn rounded_follow_edge_clips_covered_local_hit_points_after_scrolling() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let mut tree = WidgetTree::new();
    tree.mount(rounded_overlap_probe(
        &controller,
        key,
        false,
        ClipOverlapBehavior::FollowEdge,
        40.,
        100.,
        Clip::AntiAlias,
    ))
    .unwrap();
    layout(&mut tree);
    let child = tree.element_with_key(&key.into()).unwrap();
    let probe = |tree: &WidgetTree| {
        let origin = tree.element_bounds(child).unwrap().origin;
        origin + Offset::new(400., 15.)
    };
    assert!(hit(&tree, key, probe(&tree)));
    assert!(controller.jump_to(50.));
    layout(&mut tree);
    assert!(!hit(&tree, key, probe(&tree)));
    assert!(hit(&tree, key, probe(&tree) + Offset::new(0., 45.)));
}

#[test]
fn rounded_overlap_cut_keeps_the_configured_corner_shape() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let mut tree = WidgetTree::new();
    tree.mount(rounded_overlap_probe(
        &controller,
        key,
        false,
        ClipOverlapBehavior::FollowEdge,
        40.,
        100.,
        Clip::AntiAlias,
    ))
    .unwrap();
    layout(&mut tree);
    assert!(controller.jump_to(20.));
    layout(&mut tree);
    let child = tree.element_with_key(&key.into()).unwrap();
    let origin = tree.element_bounds(child).unwrap().origin;
    assert_eq!(origin, Offset::new(0., 80.));
    assert!(!hit(&tree, key, origin + Offset::new(1., 21.)));
    assert!(hit(&tree, key, origin + Offset::new(400., 25.)));
}

#[test]
fn rounded_follow_edge_subtracts_overlap_on_the_correct_side() {
    for (reverse, top, relative) in [(false, 100., 50.), (true, 450., 0.)] {
        let controller = ScrollController::new();
        let key = GlobalKey::new();
        let mut tree = WidgetTree::new();
        tree.mount(rounded_overlap_probe(
            &controller,
            key,
            reverse,
            ClipOverlapBehavior::FollowEdge,
            40.,
            100.,
            Clip::AntiAlias,
        ))
        .unwrap();
        layout(&mut tree);
        assert!(controller.jump_to(50.));
        layout(&mut tree);
        let child = tree.element_with_key(&key.into()).unwrap();
        let origin = tree.element_bounds(child).unwrap().origin;
        let commands = tree.paint();
        let rrect = commands
            .commands()
            .iter()
            .find_map(|cmd| match cmd {
                PaintCommand::PushClipRRect {
                    rrect,
                    coverage: ClipCoverage::AntiAlias,
                    ..
                } => Some(*rrect),
                _ => None,
            })
            .unwrap();
        let wanted = Rect::from_origin_size(Offset::new(0., top), Size::new(800., 50.));
        assert_eq!(rrect, incular_rendering::RRect::uniform(wanted, 40.));
        assert_eq!(rrect.rect.origin.y - origin.y, relative);
    }
}

#[test]
fn leading_rounded_clip_only_releases_after_its_inside_span_is_consumed() {
    let controller = ScrollController::new();
    let sliver = SliverClipRRect::new(SliverToBoxAdapter::new(Widget::box_(
        Size::new(800., 400.),
        FILL,
    )))
    .clipper(RoundedInset(30.));
    let mut tree = WidgetTree::new();
    tree.mount(scene(
        &controller,
        Box::new(sliver),
        Axis::Vertical,
        false,
        false,
    ))
    .unwrap();
    for (offset, top, height) in [(0., 30., 370.), (100., 0., 300.), (385., -15., 30.)] {
        controller.jump_to(offset);
        layout(&mut tree);
        assert_eq!(controller.offset(), offset);
        assert_eq!(
            clipped_paint(&mut tree, true),
            [Rect::from_origin_size(
                Offset::new(0., top),
                Size::new(800., height)
            ),]
        );
    }
}

#[test]
fn rounded_clip_none_removes_clip_commands_but_not_child_paint_or_identity() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let scene = |mode| {
        rounded_overlap_probe(
            &controller,
            key,
            false,
            ClipOverlapBehavior::FollowEdge,
            20.,
            100.,
            mode,
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(Clip::AntiAlias)).unwrap();
    layout(&mut tree);
    let child = tree.element_with_key(&key.into()).unwrap();
    assert_eq!(clipped_paint(&mut tree, true).len(), 1);
    tree.update(root, scene(Clip::None)).unwrap();
    layout(&mut tree);
    assert_eq!(tree.element_with_key(&key.into()), Some(child));
    let list = tree.paint();
    assert!(
        !list
            .commands()
            .iter()
            .any(|c| matches!(c, PaintCommand::PushClipRRect { .. }))
    );
    assert!(
        list.commands()
            .iter()
            .any(|c| matches!(c, PaintCommand::Rect { color, .. } if *color == FILL))
    );
}

#[test]
fn sliver_clip_rect_reconfigures_from_hard_edge_to_antialias_in_place() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let build = |changed| {
        let sliver = SliverClipRect::new(SliverToBoxAdapter::new(
            Widget::box_(Size::new(800., 100.), FILL).with_key(key),
        ));
        let sliver = if changed {
            sliver
                .clip_overlap(ClipOverlapBehavior::None)
                .clip_behavior(Clip::AntiAlias)
        } else {
            sliver
        };
        scene(&controller, Box::new(sliver), Axis::Vertical, false, true)
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(false)).unwrap();
    layout(&mut tree);
    let child = tree.element_with_key(&key.into()).unwrap();
    assert!(controller.jump_to(50.));
    layout(&mut tree);
    let wanted =
        |top, height| Rect::from_origin_size(Offset::new(0., top), Size::new(800., height));
    assert_eq!(rect(&mut tree), [wanted(100., 50.)]);
    tree.update(root, build(true)).unwrap();
    layout(&mut tree);
    assert_eq!(clipped_paint(&mut tree, true), [wanted(50., 100.)]);
    assert_eq!(tree.element_with_key(&key.into()), Some(child));
}

#[test]
fn sliver_clip_rrect_reconfigures_coverage_radius_and_overlap_in_place() {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let build = |changed| {
        let (overlap, radius, behavior) = if changed {
            (ClipOverlapBehavior::None, 20., Clip::HardEdge)
        } else {
            (ClipOverlapBehavior::FollowEdge, 10., Clip::AntiAlias)
        };
        rounded_overlap_probe(&controller, key, false, overlap, radius, 100., behavior)
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(false)).unwrap();
    layout(&mut tree);
    let child = tree.element_with_key(&key.into()).unwrap();
    assert!(controller.jump_to(50.));
    layout(&mut tree);
    let assert_clip = |tree: &mut WidgetTree, coverage: ClipCoverage, y, h, radius| {
        let list = tree.paint();
        let found = list
            .commands()
            .iter()
            .find_map(|c| match c {
                PaintCommand::PushClipRRect {
                    rrect, coverage, ..
                } => Some((*rrect, *coverage)),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            found,
            (
                incular_rendering::RRect::uniform(
                    Rect::from_origin_size(Offset::new(0., y), Size::new(800., h)),
                    radius
                ),
                coverage,
            )
        );
    };
    assert_clip(&mut tree, ClipCoverage::AntiAlias, 100., 50., 10.);
    tree.update(root, build(true)).unwrap();
    layout(&mut tree);
    assert_clip(&mut tree, ClipCoverage::HardEdge, 50., 100., 20.);
    assert_eq!(tree.element_with_key(&key.into()), Some(child));
}
