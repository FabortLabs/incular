//! Regression tests for Flutter parity findings in the sliver family:
//! floating-header default snap and snap mode, varied-extent row sizing,
//! fill-remaining shrink after growth, keep-alive retention, and
//! `SliverFillRemaining` overscroll filling.

use std::time::Instant;

use incular_config::{Axis, Constraints};
use incular_core::{Color, Size};
use incular_scroll::{ScrollController, ScrollPhysics};
use incular_widgets::{
    CustomScrollView, KeepAlive, Sliver, SliverFillRemaining, SliverFloatingHeader, SliverList,
    SliverToBoxAdapter, SliverVariedExtentList, Widget,
    internal::{ElementId, WidgetTree},
};

fn end_scroll_by(controller: &ScrollController, delta: f32) {
    controller.notify_user_scroll(delta);
    let _ = controller.apply_physics(ScrollPhysics::default(), delta);
    controller.end_activity();
}

fn pump_frame(tree: &mut WidgetTree, constraints: Constraints, now: Instant) -> (bool, bool) {
    tree.layout(constraints).expect("layout");
    tree.update_compositor(now).expect("compositor")
}

fn header_viewport(
    header: Box<dyn Sliver>,
    controller: ScrollController,
) -> (WidgetTree, ElementId) {
    let slivers: Vec<Box<dyn Sliver>> = vec![
        header,
        Box::new(SliverToBoxAdapter::new(Widget::box_(
            Size::new(800., 200.),
            Color::BLACK,
        ))),
    ];
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            CustomScrollView::new(slivers)
                .controller(controller)
                .scroll_direction(Axis::Vertical)
                .physics(ScrollPhysics::default())
                .into(),
        )
        .expect("mount");
    (tree, root)
}

/// Flutter's `SliverFloatingHeader` snaps at scroll end by default
/// (widgets/sliver_floating_header.dart:137-142). A header that is partially
/// revealed (30 of 70 px) starts a snap run on the scroll-end compositor pass
/// without any `snap(true)` call.
#[test]
fn floating_header_default_snap_starts_a_run_after_scroll_end() {
    let controller = ScrollController::new();
    let header = SliverFloatingHeader::new(Widget::box_(Size::new(200., 70.), Color::WHITE));
    let (mut tree, _root) = header_viewport(Box::new(header), controller.clone());
    let constraints = Constraints::tight(Size::new(200., 200.));
    tree.layout(constraints).expect("layout");

    end_scroll_by(&controller, 400.);
    tree.layout(constraints).expect("hidden layout");
    end_scroll_by(&controller, -40.);
    tree.layout(constraints).expect("partial layout");

    let start = Instant::now();
    let (_, active) = pump_frame(&mut tree, constraints, start);
    assert!(active, "the default-enabled snap must schedule its run");
}

/// `snap(false)` remains an explicit opt-out: the same partial reveal starts
/// no run and schedules no frames.
#[test]
fn floating_header_snap_can_be_disabled_explicitly() {
    let controller = ScrollController::new();
    let header =
        SliverFloatingHeader::new(Widget::box_(Size::new(200., 70.), Color::WHITE)).snap(false);
    let (mut tree, _root) = header_viewport(Box::new(header), controller.clone());
    let constraints = Constraints::tight(Size::new(200., 200.));
    tree.layout(constraints).expect("layout");

    end_scroll_by(&controller, 400.);
    tree.layout(constraints).expect("hidden layout");
    end_scroll_by(&controller, -40.);
    tree.layout(constraints).expect("partial layout");

    let start = Instant::now();
    let (_, active) = pump_frame(&mut tree, constraints, start);
    assert!(!active, "opt-out must not schedule snap frames");
}

/// Flutter's `SliverVariedExtentList` lays each row out at its indexed extent
/// on the main axis (rendering/sliver_fixed_extent_list.dart:268-270). A row
/// with no intrinsic height must still take 30 + 7 * index.
#[test]
fn varied_extent_rows_take_their_indexed_main_extent() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            CustomScrollView::new(vec![Box::new(SliverVariedExtentList::new(
                4,
                |index| 30. + index as f32 * 7.,
                |_| Widget::box_(Size::new(0., 0.), Color::BLACK),
            )) as Box<dyn Sliver>])
            .into(),
        )
        .expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 600.)))
        .expect("layout");
    let rows = tree.children(root).expect("rows").to_vec();
    assert_eq!(rows.len(), 4);
    for (index, row) in rows.iter().enumerate() {
        let render = tree.render_id(*row).expect("row render");
        assert_eq!(
            tree.render_size(render).map(|size| size.height),
            Some(30. + index as f32 * 7.),
            "row {index} height"
        );
    }
}

/// Flutter's `SliverFillRemaining(hasScrollBody: false)` recomputes its extent
/// from the current remaining space on every layout
/// (rendering/sliver_fill.dart:154-166). Growth above then shrinkage must
/// settle at 600 - 200 = 400, not keep the earlier 555.
#[test]
fn fill_remaining_without_scroll_body_follows_preceding_growth_back() {
    fn slivers(adapter_height: f32) -> Widget {
        CustomScrollView::new(vec![
            Box::new(SliverToBoxAdapter::new(Widget::box_(
                Size::new(800., adapter_height),
                Color::rgba(0x21, 0x96, 0xF3, 0xFF),
            ))) as Box<dyn Sliver>,
            Box::new(
                SliverFillRemaining::new(Widget::box_(Size::new(0., 0.), Color::WHITE))
                    .has_scroll_body(false),
            ),
        ])
        .into()
    }
    let mut tree = WidgetTree::new();
    let root = tree.mount(slivers(45.)).expect("mount");
    let constraints = Constraints::tight(Size::new(800., 600.));
    tree.layout(constraints).expect("layout");
    tree.update(root, slivers(200.)).expect("grow adapter");
    tree.layout(constraints).expect("grown layout");
    let fill = tree.children(root).expect("slivers")[1];
    let render = tree.render_id(fill).expect("fill render");
    assert_eq!(
        tree.render_size(render).map(|size| size.height),
        Some(400.),
        "fill must match the current remaining extent"
    );
}

/// Flutter's `KeepAlive(keepAlive: true)` retains a child that scrolled away
/// from the cache window (widgets/sliver.dart:1577-1600). Without the request
/// the same child is released.
#[test]
fn keep_alive_child_stays_mounted_after_scrolling_out_of_the_cache_window() {
    for keep in [true, false] {
        let controller = ScrollController::new();
        let mut tree = WidgetTree::new();
        let root = tree
            .mount(
                CustomScrollView::new(vec![Box::new(SliverList::builder(40, move |index| {
                    KeepAlive::new(
                        keep && index == 2,
                        Widget::box_(Size::new(80., 50.), Color::BLACK),
                    )
                })) as Box<dyn Sliver>])
                .controller(controller.clone())
                .into(),
            )
            .expect("mount");
        let constraints = Constraints::tight(Size::new(100., 100.));
        tree.layout(constraints).expect("layout");
        let row2 = tree.children(root).expect("rows")[2];
        assert!(controller.jump_to(1000.));
        tree.layout(constraints).expect("layout after jump");
        assert_eq!(tree.element_exists(row2), keep, "keep-alive request {keep}");
        assert_eq!(tree.children(root).unwrap().contains(&row2), keep);
    }
}

/// Flutter 3.41.7 widgets/sliver_floating_header.dart with a wheel scroll
/// (scenario slivers__floating_header__default_snap_after_wheel_scroll):
/// `pointerScroll` ends with the position idle, so the layout clamps the
/// effective offset to the scroll offset (only 10 of 70 show at offset 60)
/// instead of revealing by the wheel delta. The scroll end saw a forward
/// direction, so the snap animates from the previous effective offset (100)
/// to fully revealed over 300 ms, and the pre-layout tick lays it out in the
/// same frame.
#[test]
fn wheel_back_reveals_the_floating_header_through_the_snap_only() {
    use incular_core::Offset;
    use std::time::Duration;
    let controller = ScrollController::new();
    let header = SliverFloatingHeader::new(Widget::box_(Size::new(200., 70.), Color::WHITE));
    let (mut tree, root) = header_viewport(Box::new(header), controller.clone());
    let constraints = Constraints::tight(Size::new(200., 200.));
    tree.layout(constraints).expect("layout");
    let render = tree
        .render_id(tree.children(root).expect("header")[0])
        .expect("render");
    assert!(controller.jump_to(100.));
    tree.layout(constraints).expect("hidden layout");
    assert_eq!(tree.render_origin(render), Offset::new(0., -70.));

    assert!(tree.scroll_at(Offset::new(100., 100.), Offset::new(0., -40.)));
    assert_eq!(controller.offset(), 60.);
    let start = Instant::now();
    tree.tick_animations(start);
    tree.layout(constraints).expect("wheel layout");
    assert_eq!(tree.render_origin(render), Offset::new(0., -60.));
    let _ = tree.update_compositor(start).expect("compositor");

    let done = start + Duration::from_millis(300);
    tree.tick_animations(done);
    tree.layout(constraints).expect("revealed layout");
    assert_eq!(tree.render_origin(render), Offset::new(0., 0.));
}

/// Flutter `SliverPersistentHeaderDelegate.build(context, shrinkOffset,
/// overlapsContent)` (widgets/sliver_persistent_header.dart:20-125;
/// rendering/sliver_persistent_header.dart `layoutChild`): the delegate builds
/// with `min(scrollOffset, maxExtent)` and the child takes
/// `max(minExtent, maxExtent - shrinkOffset)`. A first pinned header sees no
/// incoming overlap, so `overlapsContent` stays false.
#[test]
fn persistent_header_delegate_builds_with_the_shrink_offset() {
    use incular_widgets::{SliverHeaderScrollBehavior, SliverPersistentHeader};
    let controller = ScrollController::new();
    let shrinks = std::rc::Rc::new(std::cell::RefCell::new(Vec::<(f32, bool)>::new()));
    let record = shrinks.clone();
    let header = SliverPersistentHeader::with_delegate(40., 120., move |shrink, overlaps| {
        record.borrow_mut().push((shrink, overlaps));
        Widget::box_(Size::new(200., 120.), Color::WHITE)
    })
    .scroll_behavior(SliverHeaderScrollBehavior::Pinned);
    let (mut tree, _root) = header_viewport(Box::new(header), controller.clone());
    let constraints = Constraints::tight(Size::new(200., 100.));
    tree.layout(constraints).expect("layout");
    assert_eq!(shrinks.borrow().last().copied(), Some((0., false)));
    assert!(controller.jump_to(60.));
    tree.layout(constraints).expect("layout");
    assert_eq!(shrinks.borrow().last().copied(), Some((60., false)));
    assert!(controller.jump_to(200.));
    tree.layout(constraints).expect("layout");
    assert_eq!(shrinks.borrow().last().copied(), Some((120., false)));
}

/// Flutter `SliverVisibility` (widgets/visibility.dart:356-380): a hidden
/// sliver that maintains its size keeps its scroll extent; one that does not
/// takes none, and a replacement sliver stands in when state is not kept.
#[test]
fn sliver_visibility_maintain_size_keeps_the_scroll_extent() {
    use incular_widgets::SliverVisibility;
    let extent_with = |visibility: SliverVisibility| {
        let controller = ScrollController::new();
        let slivers: Vec<Box<dyn Sliver>> = vec![
            Box::new(visibility),
            Box::new(SliverToBoxAdapter::new(Widget::box_(
                Size::new(200., 300.),
                Color::BLACK,
            ))),
        ];
        let mut tree = WidgetTree::new();
        tree.mount(
            CustomScrollView::new(slivers)
                .controller(controller.clone())
                .into(),
        )
        .expect("mount");
        tree.layout(Constraints::tight(Size::new(200., 100.)))
            .expect("layout");
        controller.content_extent()
    };
    let sliver = || SliverToBoxAdapter::new(Widget::box_(Size::new(200., 50.), Color::WHITE));
    assert_eq!(extent_with(SliverVisibility::new(true, sliver())), 350.);
    assert_eq!(extent_with(SliverVisibility::new(false, sliver())), 300.);
    assert_eq!(
        extent_with(SliverVisibility::maintain(false, sliver())),
        350.
    );
    assert_eq!(
        extent_with(SliverVisibility::new(false, sliver()).replacement_sliver(
            SliverToBoxAdapter::new(Widget::box_(Size::new(200., 20.), Color::WHITE))
        )),
        320.
    );
}
