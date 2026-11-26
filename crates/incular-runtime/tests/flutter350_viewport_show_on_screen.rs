//! Mounted semantic ShowOnScreen forwards the target's real rectangle.

use incular_config::{Axis, Constraints, EdgeInsets};
use incular_core::{Offset, Size};
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticRole};
use incular_widgets::{
    CustomScrollView, Directionality, ListView, ScrollController, ScrollPhysics, SizedBox, Sliver,
    SliverPadding, SliverToBoxAdapter, TextDirection, Widget, internal::ExplicitSemantics,
};

fn labeled_box(width: f32, height: f32, label: impl Into<String>) -> Widget {
    Widget::from(SizedBox::new().width(width).height(height))
        .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label(label))
}

#[test]
fn automatic_reveal_keeps_the_visible_part_of_a_large_tile() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            check_oversized_reveal(axis, reverse);
        }
    }
}

fn check_oversized_reveal(axis: Axis, reverse: bool) {
    let controller = ScrollController::new().with_initial_scroll_offset(300.);
    let list = ListView::new((0..20).map(|index| labeled_box(300., 300., format!("tile {index}"))))
        .controller(controller.clone())
        .scroll_direction(axis)
        .reverse(reverse);
    let scene = Directionality::new(TextDirection::Ltr, list);
    let bounds = Constraints::tight(match axis {
        Axis::Vertical => Size::new(300., 200.),
        Axis::Horizontal => Size::new(200., 300.),
    });
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert_eq!(controller.offset(), 300.);
    let show = |runtime: &mut Runtime, label: &str, expected| {
        let (node, semantic) = runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some(label))
            .unwrap_or_else(|| panic!("missing semantic target {label}"));
        let geometry = semantic.bounds;
        assert_eq!(
            axis.main_extent(geometry.size),
            300.,
            "bounds for {label}: {geometry:?}"
        );
        assert!(runtime.dispatch_semantic_action(node, SemanticAction::ShowOnScreen));
        runtime.run_frame(bounds).unwrap();
        assert_eq!(
            controller.offset(),
            expected,
            "{axis:?} reverse={reverse} {label}: {geometry:?}"
        );
    };
    show(&mut runtime, "tile 1", 300.);
    show(&mut runtime, "tile 0", 100.);
    show(&mut runtime, "tile 1", 300.);
    assert!(controller.jump_to(250.));
    runtime.run_frame(bounds).unwrap();
    show(&mut runtime, "tile 0", 100.);
    assert!(controller.jump_to(150.));
    runtime.run_frame(bounds).unwrap();
    show(&mut runtime, "tile 1", 300.);
}

#[test]
fn an_implicit_scroll_veto_forwards_box_and_padded_sliver_bounds() {
    for padded in [false, true] {
        let horizontal = ScrollController::new();
        let vertical = ScrollController::new();
        let page_physics = ScrollPhysics::default().page();
        let inner: Widget = if padded {
            let slivers: [Box<dyn Sliver>; 2] = ["leading", "target"].map(|name| {
                Box::new(SliverPadding::new(
                    EdgeInsets::all(25.),
                    SliverToBoxAdapter::new(labeled_box(100., 50., name)),
                )) as Box<dyn Sliver>
            });
            CustomScrollView::new(slivers)
                .controller(horizontal.clone())
                .scroll_direction(Axis::Horizontal)
                .physics(page_physics)
                .into()
        } else {
            ListView::new([
                labeled_box(150., 100., "leading"),
                labeled_box(150., 100., "target"),
            ])
            .controller(horizontal.clone())
            .scroll_direction(Axis::Horizontal)
            .physics(page_physics)
            .into()
        };
        let spacer = || Widget::from(SizedBox::new().width(200.).height(150.));
        let outer = ListView::new([
            spacer(),
            SizedBox::new().width(200.).height(100.).child(inner).into(),
            spacer(),
        ])
        .controller(vertical.clone());
        let bounds = Constraints::tight(Size::new(200., 200.));
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, outer).into()).unwrap();
        runtime.run_frame(bounds).unwrap();
        let (node, semantic) = runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some("target"))
            .unwrap();
        assert_eq!(semantic.bounds.origin.y, if padded { 175. } else { 150. });
        assert!(semantic.bounds.origin.x >= Offset::ZERO.x);
        assert!(runtime.dispatch_semantic_action(node, SemanticAction::ShowOnScreen));
        runtime.run_frame(bounds).unwrap();
        assert_eq!(horizontal.offset(), 0.);
        assert_eq!(vertical.offset(), if padded { 25. } else { 50. });
    }
}

#[test]
fn nested_standard_viewports_reveal_both_axes_with_one_request() {
    use std::time::Duration;

    use incular_core::time::Instant;
    use incular_widgets::{Scrollable, ShowOnScreen};

    let columns = (0..10)
        .map(|_| ScrollController::new().with_initial_scroll_offset(400.))
        .collect::<Vec<_>>();
    let rows = (0..10).map(|y| {
        let cells = (0..10).map(|x| {
            Widget::from(SizedBox::square(100.))
                .with_key(incular_widgets::internal::Key::String(format!("{x},{y}")))
        });
        SizedBox::new().height(100.).child(
            ListView::new(cells)
                .scroll_direction(Axis::Horizontal)
                .controller(columns[y].clone()),
        )
    });
    let vertical = ScrollController::new().with_initial_scroll_offset(400.);
    let scene = Directionality::new(
        TextDirection::Ltr,
        ListView::new(rows).controller(vertical.clone()),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let bounds = Constraints::tight(Size::new(200., 200.));
    let mut now = Instant::now();
    let mut frame = |runtime: &mut Runtime, elapsed| {
        now += Duration::from_millis(elapsed);
        runtime.run_frame_at(bounds, now).unwrap()
    };
    frame(&mut runtime, 0);
    let find = |runtime: &Runtime, x, y| {
        runtime
            .tree()
            .element_with_key(&incular_widgets::internal::Key::String(format!("{x},{y}")))
            .unwrap_or_else(|| panic!("missing cached tile {x},{y}"))
    };
    for (x, y, wanted_x, wanted_y) in [
        (4, 4, 400., 400.),
        (4, 3, 400., 300.),
        (4, 6, 400., 500.),
        (3, 4, 300., 400.),
        (6, 4, 500., 400.),
        (3, 3, 300., 300.),
        (3, 6, 300., 500.),
        (6, 3, 500., 300.),
        (6, 6, 500., 500.),
    ] {
        let _ = columns[y].jump_to(400.);
        let _ = vertical.jump_to(400.);
        frame(&mut runtime, 16);
        let target = find(&runtime, x, y);
        assert_eq!(
            Scrollable::show_on_screen(runtime.tree(), target, ShowOnScreen::new()),
            usize::from(wanted_x != 400.) + usize::from(wanted_y != 400.)
        );
        frame(&mut runtime, 16);
        assert_eq!(
            (columns[y].offset(), vertical.offset()),
            (wanted_x, wanted_y)
        );
    }

    assert!(columns[6].jump_to(400.));
    assert!(vertical.jump_to(400.));
    frame(&mut runtime, 16);
    let target = find(&runtime, 6, 6);
    assert_eq!(
        Scrollable::show_on_screen(
            runtime.tree(),
            target,
            ShowOnScreen::new().duration(Duration::from_secs(2)),
        ),
        2
    );
    frame(&mut runtime, 16);
    let (_, activity) = frame(&mut runtime, 1000);
    assert!(activity.requested_another_frame);
    for current in [columns[6].offset(), vertical.offset()] {
        assert!(current > 400. && current < 500., "midpoint: {current}");
    }
    frame(&mut runtime, 1040);
    assert_eq!((columns[6].offset(), vertical.offset()), (500., 500.));
}

#[test]
fn same_axis_reveal_forwards_the_post_move_inner_rectangle() {
    use incular_widgets::{Scrollable, ShowOnScreen};

    for (inner_start, outer_start, item, inner_end, outer_end) in [
        (0., 0., 0u64, 0., 100.),
        (0., 0., 4, 300., 200.),
        (200., 200., 2, 200., 200.),
        (200., 200., 5, 400., 200.),
        (0., 100., 1, 0., 200.),
    ] {
        let inner = ScrollController::new().with_initial_scroll_offset(inner_start);
        let outer = ScrollController::new().with_initial_scroll_offset(outer_start);
        let inner_list = ListView::new(
            (0u64..10).map(|i| Widget::from(SizedBox::new().height(100.)).with_key(i)),
        )
        .controller(inner.clone());
        let spacer = || Widget::from(SizedBox::new().height(200.));
        let outer_list = ListView::new([
            spacer(),
            SizedBox::new().height(200.).child(inner_list).into(),
            spacer(),
        ])
        .controller(outer.clone());
        let bounds = Constraints::tight(Size::new(300., 200.));
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, outer_list).into()).unwrap();
        runtime.run_frame(bounds).unwrap();
        assert_eq!((inner.offset(), outer.offset()), (inner_start, outer_start));
        let target = runtime.tree().element_with_key(&item.into()).unwrap();
        assert_eq!(
            Scrollable::show_on_screen(runtime.tree(), target, ShowOnScreen::new()),
            usize::from(inner_start != inner_end) + usize::from(outer_start != outer_end)
        );
        runtime.run_frame(bounds).unwrap();
        assert_eq!(
            (inner.offset(), outer.offset()),
            (inner_end, outer_end),
            "inner start {inner_start}, outer start {outer_start}, item {item}"
        );
    }
}

#[test]
fn centered_reversed_list_can_animate_an_offstage_negative_item_into_view() {
    use std::time::Duration;

    use incular_core::{Rect, time::Instant};
    use incular_widgets::{Scrollable, ShowOnScreen, SliverList};

    let controller = ScrollController::new();
    let negative = SliverList::builder(10, |index| {
        Widget::from(SizedBox::new().height(40.)).with_key(index as u64)
    });
    let center = SliverList::builder(1, |_| SizedBox::new().height(40.));
    let positive = SliverList::builder(10, |_| SizedBox::new().height(40.));
    let slivers: [Box<dyn Sliver>; 3] = [Box::new(negative), Box::new(center), Box::new(positive)];
    let root = Directionality::new(
        TextDirection::Ltr,
        CustomScrollView::new(slivers)
            .controller(controller.clone())
            .center(1)
            .reverse(true),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    let bounds = Constraints::tight(Size::new(400., 400.));
    let viewport = Rect::from_origin_size(Offset::ZERO, Size::new(400., 400.));
    let now = Instant::now();
    runtime.run_frame_at(bounds, now).unwrap();
    let target = runtime.tree().element_with_key(&0u64.into()).unwrap();
    let before = runtime.tree().element_bounds(target).unwrap();
    assert!(!viewport.intersects(before), "{before:?}");
    assert_eq!(controller.offset(), 0.);
    assert_eq!(
        Scrollable::show_on_screen(
            runtime.tree(),
            target,
            ShowOnScreen::new().duration(Duration::from_secs(1)),
        ),
        1
    );
    runtime
        .run_frame_at(bounds, now + Duration::from_millis(16))
        .unwrap();
    runtime
        .run_frame_at(bounds, now + Duration::from_millis(1040))
        .unwrap();
    let after = runtime.tree().element_bounds(target).unwrap();
    assert!(viewport.intersects(after), "{after:?}");
    assert!(controller.offset() < 0.);
}

#[test]
fn reveal_does_not_move_a_visible_pinned_header_but_exposes_obstructed_content() {
    use incular_widgets::{
        Scrollable, ShowOnScreen, SliverHeaderScrollBehavior, SliverPersistentHeader,
    };

    let controller = ScrollController::new().with_initial_scroll_offset(300.);
    let slivers = (0u64..20).map(|index| -> Box<dyn Sliver> {
        if index == 10 {
            Box::new(
                SliverPersistentHeader::with_delegate(100., 300., |_, _| {
                    Widget::from(SizedBox::expand()).with_key("pinned")
                })
                .scroll_behavior(SliverHeaderScrollBehavior::Pinned),
            )
        } else {
            let tile = Widget::from(SizedBox::new().height(300.)).with_key(index);
            Box::new(SliverToBoxAdapter::new(tile))
        }
    });
    let list = CustomScrollView::new(slivers).controller(controller.clone());
    let root = Directionality::new(TextDirection::Ltr, list);
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime.run_frame(bounds).unwrap();

    controller.jump_to(4500.);
    runtime.run_frame(bounds).unwrap();
    let header = runtime.tree().element_with_key(&"pinned".into()).unwrap();
    let header_bounds = runtime.tree().element_bounds(header).unwrap();
    assert_eq!(
        (header_bounds.origin.y, header_bounds.size.height),
        (0., 100.)
    );
    assert_eq!(
        Scrollable::show_on_screen(runtime.tree(), header, ShowOnScreen::new()),
        0
    );
    runtime.run_frame(bounds).unwrap();
    assert_eq!(controller.offset(), 4500.);

    // The upstream jump starts with the tile above the viewport. Also
    // check a closer offset where the header covers part of a visible tile.
    for (pixels, before) in [(3600., -300.), (3400., -100.)] {
        controller.jump_to(pixels);
        runtime.run_frame(bounds).unwrap();
        let content = runtime.tree().element_with_key(&11u64.into()).unwrap();
        assert_eq!(
            runtime.tree().element_bounds(content).unwrap().origin.y,
            before
        );
        assert_eq!(
            Scrollable::show_on_screen(runtime.tree(), content, ShowOnScreen::new()),
            1
        );
        runtime.run_frame(bounds).unwrap();
        assert_eq!(controller.offset(), 3200.);
        assert_eq!(
            runtime.tree().element_bounds(content).unwrap().origin.y,
            100.
        );
    }
}

#[test]
fn reveal_forwards_the_inner_tile_projected_into_the_outer_viewport_with_no_cache() {
    use incular_widgets::{Scrollable, ShowOnScreen};

    let inner = ScrollController::new();
    let outer = ScrollController::new();
    let inner_slivers = (0u64..5).map(|index| -> Box<dyn Sliver> {
        let tile = Widget::from(SizedBox::new().height(300.)).with_key(index);
        Box::new(SliverToBoxAdapter::new(tile))
    });
    let inner_list = CustomScrollView::new(inner_slivers).controller(inner.clone());
    let outer_slivers: [Box<dyn Sliver>; 2] = [
        Box::new(SliverToBoxAdapter::new(
            SizedBox::new().height(300.).child(inner_list),
        )),
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(300.))),
    ];
    let outer_list = CustomScrollView::new(outer_slivers)
        .controller(outer.clone())
        .cache_extent(0.);
    let bounds = Constraints::tight(Size::new(800., 300.));
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, outer_list).into()).unwrap();
    runtime.run_frame(bounds).unwrap();

    let target = runtime.tree().element_with_key(&1u64.into()).unwrap();
    assert_eq!(
        runtime.tree().element_bounds(target).unwrap().origin.y,
        300.
    );
    assert_eq!(
        Scrollable::show_on_screen(runtime.tree(), target, ShowOnScreen::new()),
        1
    );
    runtime.run_frame(bounds).unwrap();
    assert_eq!((inner.offset(), outer.offset()), (300., 0.));
    let target_bounds = runtime.tree().element_bounds(target).unwrap();
    assert_eq!(
        (target_bounds.origin.y, target_bounds.size.height),
        (0., 300.)
    );
}

#[test]
fn reverse_growing_floating_pinned_header_reveals_without_unpinning() {
    use incular_widgets::{
        Scrollable, ShowOnScreen, SliverHeaderScrollBehavior, SliverPersistentHeader,
    };
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let controller = ScrollController::new().with_initial_scroll_offset(300.);
        let slivers = (0u64..20).map(|index| -> Box<dyn Sliver> {
            if index == 10 {
                Box::new(
                    SliverPersistentHeader::with_delegate(100., 300., |_, _| {
                        Widget::from(SizedBox::expand()).with_key("pinned floating header")
                    })
                    .scroll_behavior(SliverHeaderScrollBehavior::FloatingPinned),
                )
            } else {
                let tile = Widget::from(SizedBox::square(300.)).with_key(index);
                Box::new(SliverToBoxAdapter::new(tile))
            }
        });
        let viewport = CustomScrollView::new(slivers)
            .scroll_direction(axis)
            .center(19)
            .controller(controller.clone());
        let root = Directionality::new(TextDirection::Ltr, viewport);
        let bounds = Constraints::tight(Size::new(400., 400.));
        let mut runtime = Runtime::new(root.into()).unwrap();
        runtime.run_frame(bounds).unwrap();

        controller.jump_to(-4500.);
        runtime.run_frame(bounds).unwrap();
        let tree = runtime.tree();
        let header = tree
            .element_with_key(&"pinned floating header".into())
            .unwrap();
        let header_bounds = tree.element_bounds(header).unwrap();
        let start = |offset: Offset| match axis {
            Axis::Vertical => offset.y,
            Axis::Horizontal => offset.x,
        };
        assert_eq!(
            start(header_bounds.origin) + axis.main_extent(header_bounds.size),
            400.,
            "{axis:?}"
        );
        assert_eq!(
            Scrollable::show_on_screen(tree, header, ShowOnScreen::new()),
            0,
            "{axis:?}"
        );
        runtime.run_frame(bounds).unwrap();
        assert_eq!(controller.offset(), -4500.);

        controller.jump_to(-3100.);
        runtime.run_frame(bounds).unwrap();
        let tree = runtime.tree();
        let tile = tree.element_with_key(&9u64.into()).unwrap();
        assert_eq!(
            Scrollable::show_on_screen(tree, tile, ShowOnScreen::new()),
            1,
            "{axis:?}"
        );
        runtime.run_frame(bounds).unwrap();
        assert_eq!(controller.offset(), -3000., "{axis:?}");
    }
}
