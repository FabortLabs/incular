//! The Flutter 3.50.0 viewport query returns offset and projected subrect.

use incular_config::{Axis, Constraints};
use incular_core::{Offset, Rect, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, ListView, RevealedOffset, ScrollController, Scrollable, SizedBox,
    TextDirection, Widget,
};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(w, h))
}

fn check_reveal(axis: Axis, reverse: bool) {
    let ctl = ScrollController::new().with_initial_scroll_offset(300.);
    let (w, h, tile_w, tile_h) = match axis {
        Axis::Vertical => (300., 200., 300., 100.),
        Axis::Horizontal => (200., 300., 100., 300.),
    };
    let children = (0u64..20).map(|index| {
        let row: Widget = SizedBox::new().width(tile_w).height(tile_h).into();
        row.with_key(index)
    });
    let list = ListView::new(children)
        .controller(ctl.clone())
        .scroll_direction(axis)
        .reverse(reverse);
    let scene = Directionality::new(TextDirection::Ltr, list);
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(w, h)))
        .unwrap();
    assert_eq!(ctl.offset(), 300.);
    let tree = runtime.tree();
    let target = tree
        .element_with_key(&5u64.into())
        .expect("tile five remains materialized in the viewport cache");
    let query = |alignment, subrect| {
        Scrollable::get_offset_to_reveal(tree, target, alignment, subrect).unwrap()
    };
    let sub = Some(rect(40., 40., 10., 10.));
    let expected = match (axis, reverse) {
        (Axis::Vertical, false) => [
            (500., rect(0., 0., 300., 100.)),
            (400., rect(0., 100., 300., 100.)),
            (540., rect(40., 0., 10., 10.)),
            (350., rect(40., 190., 10., 10.)),
        ],
        (Axis::Vertical, true) => [
            (500., rect(0., 100., 300., 100.)),
            (400., rect(0., 0., 300., 100.)),
            (550., rect(40., 190., 10., 10.)),
            (360., rect(40., 0., 10., 10.)),
        ],
        (Axis::Horizontal, false) => [
            (500., rect(0., 0., 100., 300.)),
            (400., rect(100., 0., 100., 300.)),
            (540., rect(0., 40., 10., 10.)),
            (350., rect(190., 40., 10., 10.)),
        ],
        (Axis::Horizontal, true) => [
            (500., rect(100., 0., 100., 300.)),
            (400., rect(0., 0., 100., 300.)),
            (550., rect(190., 40., 10., 10.)),
            (360., rect(0., 40., 10., 10.)),
        ],
    };
    for ((alignment, subrect), (offset, rect)) in [(0., None), (1., None), (0., sub), (1., sub)]
        .into_iter()
        .zip(expected)
    {
        assert_eq!(
            query(alignment, subrect),
            RevealedOffset { offset, rect },
            "{axis:?} reverse={reverse}, alignment={alignment}, subrect={subrect:?}"
        );
    }
    assert!(
        Scrollable::get_offset_to_reveal(tree, target, f32::NAN, None).is_none(),
        "invalid alignment cannot produce non-finite geometry"
    );
    assert_eq!(ctl.offset(), 300., "a reveal query does not scroll");
}

#[test]
fn viewport_reveal_down_and_up() {
    check_reveal(Axis::Vertical, false);
    check_reveal(Axis::Vertical, true);
}

#[test]
fn viewport_reveal_right_and_left() {
    check_reveal(Axis::Horizontal, false);
    check_reveal(Axis::Horizontal, true);
}

#[test]
fn a_variable_box_list_reports_unclamped_geometry_and_scrolls_within_its_range() {
    let ctl = ScrollController::new();
    let first = SizedBox::new().width(120.).height(60.);
    let second = Widget::from(SizedBox::new().width(120.).height(80.)).with_key("second row");
    let list = ListView::new([Widget::from(first), second]).controller(ctl.clone());
    let bounds = Constraints::tight(Size::new(120., 100.));
    let mut runtime = Runtime::new(list.into()).unwrap();
    runtime.run_frame(bounds).unwrap();
    let target = runtime
        .tree()
        .element_with_key(&"second row".into())
        .unwrap();
    assert_eq!(ctl.max_offset(), 40.);
    assert_eq!(
        Scrollable::get_offset_to_reveal(runtime.tree(), target, 0., None),
        Some(RevealedOffset {
            offset: 60.,
            rect: rect(0., 0., 120., 80.),
        })
    );
    assert_eq!(ctl.offset(), 0.);
    assert_eq!(
        Scrollable::ensure_visible(
            runtime.tree(),
            target,
            incular_widgets::EnsureVisible::new()
        ),
        1
    );
    runtime.run_frame(bounds).unwrap();
    assert_eq!(ctl.offset(), 40.);
    assert_eq!(
        runtime.tree().element_bounds(target),
        Some(rect(0., 20., 120., 80.))
    );
}

#[test]
fn padded_box_slivers_reveal_at_the_correct_logical_edge() {
    use incular_config::EdgeInsets;
    use incular_widgets::{CustomScrollView, Sliver, SliverPadding, SliverToBoxAdapter};

    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            let ctl = ScrollController::new().with_initial_scroll_offset(300.);
            let (w, h, tile_w, tile_h, padding) = match axis {
                Axis::Vertical => (300., 200., 300., 100., EdgeInsets::only(0., 22., 0., 23.)),
                Axis::Horizontal => (200., 300., 100., 300., EdgeInsets::only(22., 0., 23., 0.)),
            };
            let slivers = (0u64..20).map(|index| -> Box<dyn Sliver> {
                let row: Widget = SizedBox::new().width(tile_w).height(tile_h).into();
                Box::new(SliverPadding::new(
                    padding,
                    SliverToBoxAdapter::new(row.with_key(index)),
                ))
            });
            let viewport = CustomScrollView::new(slivers)
                .controller(ctl.clone())
                .scroll_direction(axis)
                .reverse(reverse);
            let mut runtime =
                Runtime::new(Directionality::new(TextDirection::Ltr, viewport).into()).unwrap();
            runtime
                .run_frame(Constraints::tight(Size::new(w, h)))
                .unwrap();
            assert_eq!(ctl.offset(), 300.);
            let tree = runtime.tree();
            let target = tree
                .element_with_key(&5u64.into())
                .expect("first three pixels of the target are inside the cache");
            let subrect = Some(rect(1., 2., 2., 2.));
            let expected = match (axis, reverse) {
                (Axis::Vertical, false) => [747., 647., 749., 551.],
                (Axis::Horizontal, false) => [747., 647., 748., 550.],
                (Axis::Vertical, true) => [748., 648., 844., 646.],
                (Axis::Horizontal, true) => [748., 648., 845., 647.],
            };
            for ((alignment, subrect), expected) in
                [(0., None), (1., None), (0., subrect), (1., subrect)]
                    .into_iter()
                    .zip(expected)
            {
                assert_eq!(
                    Scrollable::get_offset_to_reveal(tree, target, alignment, subrect)
                        .unwrap()
                        .offset,
                    expected,
                    "{axis:?} reverse={reverse}, alignment={alignment}, subrect={subrect:?}"
                );
            }
            assert_eq!(ctl.offset(), 300.);
        }
    }
}

#[test]
fn centered_reverse_growth_keeps_negative_reveal_offsets_unclamped() {
    use incular_config::EdgeInsets;
    use incular_widgets::{CustomScrollView, Sliver, SliverPadding, SliverToBoxAdapter};

    for (axis, reverse) in [
        (Axis::Vertical, false),
        (Axis::Vertical, true),
        (Axis::Horizontal, false),
        (Axis::Horizontal, true),
    ] {
        let ctl = ScrollController::new();
        let (tile_w, tile_h, padding) = match axis {
            Axis::Vertical => (300., 100., EdgeInsets::only(0., 22., 0., 23.)),
            Axis::Horizontal => (100., 200., EdgeInsets::only(22., 0., 23., 0.)),
        };
        let slivers = ["lower", "center"]
            .into_iter()
            .map(|key| -> Box<dyn Sliver> {
                let leaf = Widget::from(SizedBox::new().width(tile_w).height(tile_h)).with_key(key);
                Box::new(SliverPadding::new(padding, SliverToBoxAdapter::new(leaf)))
            });
        let viewport = CustomScrollView::new(slivers)
            .controller(ctl.clone())
            .center(1)
            .scroll_direction(axis)
            .reverse(reverse);
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, viewport).into()).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(300., 200.)))
            .unwrap();
        let tree = runtime.tree();
        let target = tree
            .element_with_key(&"lower".into())
            .expect("the reverse-growth box is cached before the center");
        let subrect = Some(rect(1., 2., 2., 2.));
        let expected = match (axis, reverse) {
            (Axis::Vertical, false) => [-123., -223., -121., -319.],
            (Axis::Vertical, true) => [-122., -222., -26., -224.],
            (Axis::Horizontal, false) => [-123., -323., -122., -420.],
            (Axis::Horizontal, true) => [-122., -322., -25., -323.],
        };
        for ((alignment, subrect), expected) in
            [(0., None), (1., None), (0., subrect), (1., subrect)]
                .into_iter()
                .zip(expected)
        {
            assert_eq!(
                Scrollable::get_offset_to_reveal(tree, target, alignment, subrect)
                    .unwrap()
                    .offset,
                expected,
                "{axis:?}, reverse={reverse}, alignment={alignment}, subrect={subrect:?}"
            );
        }
        assert_eq!(ctl.offset(), 0.);
    }
}

#[test]
fn a_pinned_header_reveals_at_its_current_scroll_offset_once_pinned() {
    use incular_widgets::{
        CustomScrollView, Sliver, SliverList, SliverPersistentHeader, SliverToBoxAdapter,
    };

    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            let ctl = ScrollController::new();
            let (width, height, header_w, header_h, before_w, before_h) = match axis {
                Axis::Vertical => (300., 400., 300., 50., 300., 300.),
                Axis::Horizontal => (400., 300., 50., 300., 300., 300.),
            };
            let header = Widget::from(SizedBox::new().width(header_w).height(header_h))
                .with_key("pinned header");
            let preceding = SizedBox::new().width(before_w).height(before_h);
            let slivers: [Box<dyn Sliver>; 3] = [
                Box::new(SliverToBoxAdapter::new(preceding)),
                Box::new(SliverPersistentHeader::new(50., header).pinned(true)),
                Box::new(SliverList::builder(20, move |_| {
                    SizedBox::new().width(header_w).height(header_h)
                })),
            ];
            let viewport = CustomScrollView::new(slivers)
                .controller(ctl.clone())
                .scroll_direction(axis)
                .reverse(reverse);
            let scene = Directionality::new(TextDirection::Ltr, viewport);
            let bounds = Constraints::tight(Size::new(width, height));
            let mut runtime = Runtime::new(scene.into()).unwrap();
            runtime.run_frame(bounds).unwrap();
            let rect = match (axis, reverse) {
                (Axis::Vertical, false) => rect(0., 0., 300., 50.),
                (Axis::Vertical, true) => rect(0., 350., 300., 50.),
                (Axis::Horizontal, false) => rect(0., 0., 50., 300.),
                (Axis::Horizontal, true) => rect(350., 0., 50., 300.),
            };
            let query = |runtime: &Runtime| {
                let tree = runtime.tree();
                let header = tree.element_with_key(&"pinned header".into()).unwrap();
                Scrollable::get_offset_to_reveal(tree, header, 0., None)
            };
            assert_eq!(
                query(&runtime),
                Some(RevealedOffset { offset: 300., rect }),
                "{axis:?} reverse={reverse} before pinning"
            );
            assert!(ctl.jump_to(500.));
            runtime.run_frame(bounds).unwrap();
            assert_eq!(
                query(&runtime),
                Some(RevealedOffset { offset: 500., rect }),
                "{axis:?} reverse={reverse} after pinning"
            );
            assert_eq!(ctl.offset(), 500.);
        }
    }
}

#[test]
fn a_reverse_growth_header_reveals_at_its_trailing_pinned_edge() {
    use incular_widgets::{CustomScrollView, Sliver, SliverPersistentHeader, SliverToBoxAdapter};

    for axis in [Axis::Vertical, Axis::Horizontal] {
        let ctl = ScrollController::new();
        let box_of = |extent| {
            let (w, h) = match axis {
                Axis::Vertical => (300., extent),
                Axis::Horizontal => (extent, 300.),
            };
            SizedBox::new().width(w).height(h)
        };
        let header = Widget::from(box_of(50.)).with_key("reverse header");
        let slivers: [Box<dyn Sliver>; 4] = [
            Box::new(SliverToBoxAdapter::new(box_of(500.))),
            Box::new(SliverPersistentHeader::new(50., header).pinned(true)),
            Box::new(SliverToBoxAdapter::new(box_of(300.))),
            Box::new(SliverToBoxAdapter::new(box_of(100.))),
        ];
        let viewport = CustomScrollView::new(slivers)
            .controller(ctl.clone())
            .scroll_direction(axis)
            .center(3);
        let bounds = Constraints::tight(match axis {
            Axis::Vertical => Size::new(300., 400.),
            Axis::Horizontal => Size::new(400., 300.),
        });
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, viewport).into()).unwrap();
        runtime.run_frame(bounds).unwrap();
        let current_reveal = |runtime: &Runtime| {
            let tree = runtime.tree();
            let header = tree.element_with_key(&"reverse header".into()).unwrap();
            Scrollable::get_offset_to_reveal(tree, header, 1., None)
                .unwrap()
                .offset
        };
        assert_eq!(current_reveal(&runtime), -300., "{axis:?} before pinning");
        assert!(ctl.jump_to(-500.));
        runtime.run_frame(bounds).unwrap();
        assert_eq!(current_reveal(&runtime), -500., "{axis:?} after pinning");
    }
}

#[test]
fn changing_scroll_direction_reroutes_subsequent_drag_motion() {
    use std::time::Duration;

    use incular_core::{PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, time::Instant};
    use incular_widgets::{Center, RawPointerEvent, SingleChildScrollView};

    let controller = ScrollController::new();
    let build = |axis| {
        Directionality::new(
            TextDirection::Ltr,
            Center::new(
                SizedBox::square(100.).child(
                    SingleChildScrollView::new(SizedBox::square(200.))
                        .controller(controller.clone())
                        .scroll_direction(axis),
                ),
            ),
        )
        .into()
    };
    let mut runtime = Runtime::new(build(Axis::Vertical)).unwrap();
    let frame = Constraints::tight(Size::new(800., 600.));
    let now = Instant::now();
    runtime.run_frame_at(frame, now).unwrap();
    assert_eq!(controller.offset(), 0.);
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, build(Axis::Horizontal))
        .unwrap();
    runtime
        .run_frame_at(frame, now + Duration::from_millis(16))
        .unwrap();
    assert_eq!(controller.offset(), 0.);

    for (ms, phase, x, y, expected) in [
        (18, PointerPhase::Down, 400., 300., 0.),
        (20, PointerPhase::Move, 400., 310., 0.),
        (22, PointerPhase::Move, 400., 300., 0.),
        (24, PointerPhase::Move, 390., 300., 10.),
        (26, PointerPhase::Move, 400., 300., 0.),
        (28, PointerPhase::Up, 400., 300., 0.),
    ] {
        let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
            1,
            1,
            PointerDeviceKind::Touch,
            if phase == PointerPhase::Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            Offset::new(x, y),
            phase,
            now + Duration::from_millis(ms),
        ));
        assert_eq!(controller.offset(), expected, "{phase:?} at ({x},{y})");
    }
}
