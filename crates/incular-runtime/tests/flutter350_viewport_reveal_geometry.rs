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
