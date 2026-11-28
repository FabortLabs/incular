use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, GridView, SingleChildScrollView, SliverGridDelegate, Text, Widget,
    internal::Key,
};

const BOUNDS: Size = Size {
    width: 800.,
    height: 600.,
};

fn tiles(count: u64) -> impl Iterator<Item = Widget> {
    (0..count).map(|index| Widget::from(Text::new(index.to_string())).with_key(index))
}

fn render(view: impl Into<Widget>, size: Size) -> Runtime {
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, view).into()).unwrap();
    runtime.run_frame(Constraints::tight(size)).unwrap();
    runtime
}

fn find(runtime: &Runtime, key: Key) -> Rect {
    let tree = runtime.tree();
    let node = tree
        .element_with_key(&key)
        .expect("tile or viewport is mounted");
    tree.element_bounds(node).unwrap()
}

#[test]
fn both_grid_convenience_constructors_honor_explicit_main_axis_extent() {
    for grid in [
        GridView::count(4, tiles(20)).main_axis_extent(100.),
        GridView::extent(200., tiles(20)).main_axis_extent(100.),
    ] {
        let runtime = render(grid, BOUNDS);
        assert_eq!(
            find(&runtime, Key::Value(4)),
            Rect::from_origin_size(Offset::new(0., 100.), Size::new(200., 100.)),
        );
    }
}

#[test]
fn both_grid_delegates_honor_explicit_main_axis_extent() {
    for delegate in [
        SliverGridDelegate::fixed_cross_axis_count(4).main_axis_extent(100.),
        SliverGridDelegate::max_cross_axis_extent(200.).main_axis_extent(100.),
    ] {
        let runtime = render(GridView::new(delegate, tiles(20)), BOUNDS);
        assert_eq!(find(&runtime, Key::Value(4)).size, Size::new(200., 100.));
    }
}

#[test]
fn shrink_wrapped_grid_in_an_unbounded_scroll_axis_lays_out_first_and_last_tile() {
    const GRID: Key = Key::Value(900);
    let grid = Widget::from(GridView::count(4, tiles(20)).shrink_wrap(true)).with_key(GRID);
    let runtime = render(SingleChildScrollView::new(grid), BOUNDS);
    assert_eq!(find(&runtime, GRID).size, Size::new(800., 1000.));
    for (tile, origin) in [(0, Offset::ZERO), (19, Offset::new(600., 800.))] {
        assert_eq!(
            find(&runtime, Key::Value(tile)),
            Rect::from_origin_size(origin, Size::new(200., 200.)),
        );
    }
}

#[test]
fn a_positive_fractional_maximum_grid_extent_is_not_rounded_up() {
    let runtime = render(GridView::extent(0.5, tiles(4)), Size::new(2., 1.));
    assert_eq!(
        find(&runtime, Key::Value(3)),
        Rect::from_origin_size(Offset::new(1.5, 0.), Size::new(0.5, 0.5)),
    );
}

#[cfg(debug_assertions)]
#[test]
fn grid_extent_rejects_a_zero_maximum_cross_axis_extent() {
    assert!(std::panic::catch_unwind(|| GridView::extent(0., tiles(0))).is_err());
}

#[cfg(debug_assertions)]
#[test]
fn grid_delegates_reject_negative_main_axis_extent_but_allow_zero() {
    for delegate in [
        SliverGridDelegate::fixed_cross_axis_count(3),
        SliverGridDelegate::max_cross_axis_extent(100.),
    ] {
        assert!(std::panic::catch_unwind(|| delegate.clone().main_axis_extent(-100.)).is_err());
        let runtime = render(
            GridView::new(delegate.main_axis_extent(0.), tiles(1)),
            BOUNDS,
        );
        assert_eq!(find(&runtime, Key::Value(0)).size.height, 0.);
    }
}
