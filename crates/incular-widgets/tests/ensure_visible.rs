//! `Scrollable::ensure_visible` (scroll-004): Flutter `Scrollable.ensureVisible`
//! (`widgets/scrollable.dart:491-531`) with `ScrollPosition.ensureVisible`
//! (`widgets/scroll_position.dart:812-878`) and `getOffsetToReveal`
//! (`rendering/viewport.dart`, `RenderSingleChildViewport` and
//! `RenderViewportBase`): target = leading - (viewport - target) * alignment,
//! clamped to the scroll range.

#![allow(clippy::float_cmp)]

use std::time::Duration;

use incular_config::{Axis, Constraints};
use incular_core::{Color, Size};
use incular_scroll::ScrollController;
use incular_widgets::{
    Column, EnsureVisible, ListView, Row, ScrollPositionAlignmentPolicy, Scrollable,
    SingleChildScrollView, SizedBox, Widget,
    internal::{ElementId, Key, WidgetTree},
};

const VIEWPORT: Size = Size {
    width: 200.,
    height: 200.,
};

/// Twenty 50px rows; row `index` is keyed `Value(index)`.
fn rows() -> Vec<Widget> {
    (0..20)
        .map(|index| Widget::box_(Size::new(200., 50.), Color::WHITE).with_key(Key::Value(index)))
        .collect()
}

fn mount(root: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(root).expect("mount");
    tree.layout(Constraints::tight(VIEWPORT)).expect("layout");
    tree
}

fn row(tree: &WidgetTree, index: u64) -> ElementId {
    tree.element_with_key(&Key::Value(index))
        .expect("keyed row")
}

fn column_tree(controller: &ScrollController, reverse: bool) -> WidgetTree {
    mount(
        SingleChildScrollView::new(Column::new(rows()))
            .controller(controller.clone())
            .reverse(reverse)
            .into(),
    )
}

#[test]
fn explicit_alignment_places_the_target_and_clamps_to_the_range() {
    let controller = ScrollController::new();
    let mut tree = column_tree(&controller, false);
    let target = row(&tree, 10);
    // Row 10 leads at 500: alignment 0 puts it at the top.
    assert_eq!(
        Scrollable::ensure_visible(&tree, target, EnsureVisible::new()),
        1
    );
    assert_eq!(controller.offset(), 500.);
    tree.layout(Constraints::tight(VIEWPORT)).expect("layout");
    // Already there: nothing moves (`if (target == pixels) return`).
    assert_eq!(
        Scrollable::ensure_visible(&tree, target, EnsureVisible::new()),
        0
    );
    // 0.5 centres it: 500 - (200 - 50) * 0.5.
    Scrollable::ensure_visible(&tree, target, EnsureVisible::new().alignment(0.5));
    assert_eq!(controller.offset(), 425.);
    tree.layout(Constraints::tight(VIEWPORT)).expect("layout");
    // 1.0 puts its trailing edge at the bottom: 500 - 150.
    Scrollable::ensure_visible(&tree, target, EnsureVisible::new().alignment(1.0));
    assert_eq!(controller.offset(), 350.);
    tree.layout(Constraints::tight(VIEWPORT)).expect("layout");
    // The last row's 950 is clamped to the 800 maximum.
    Scrollable::ensure_visible(&tree, row(&tree, 19), EnsureVisible::new());
    assert_eq!(controller.offset(), 800.);
}

#[test]
fn keep_visible_policies_only_move_toward_their_edge() {
    let controller = ScrollController::new();
    let mut tree = column_tree(&controller, false);
    let end =
        EnsureVisible::new().alignment_policy(ScrollPositionAlignmentPolicy::KeepVisibleAtEnd);
    let start =
        EnsureVisible::new().alignment_policy(ScrollPositionAlignmentPolicy::KeepVisibleAtStart);
    // keepVisibleAtEnd reveals row 10 at the bottom edge (alignment 1.0).
    Scrollable::ensure_visible(&tree, row(&tree, 10), end);
    assert_eq!(controller.offset(), 350.);
    tree.layout(Constraints::tight(VIEWPORT)).expect("layout");
    // A row above the viewport would need a smaller offset, which
    // keepVisibleAtEnd never takes.
    assert_eq!(Scrollable::ensure_visible(&tree, row(&tree, 2), end), 0);
    // keepVisibleAtStart reveals it at the top edge instead.
    Scrollable::ensure_visible(&tree, row(&tree, 2), start);
    assert_eq!(controller.offset(), 100.);
    tree.layout(Constraints::tight(VIEWPORT)).expect("layout");
    // And never moves forward.
    assert_eq!(Scrollable::ensure_visible(&tree, row(&tree, 10), start), 0);
    assert_eq!(controller.offset(), 100.);
}

#[test]
fn reversed_axis_measures_the_leading_edge_from_the_far_side() {
    // AxisDirection.up: leadingScrollOffset = child height - bounds.bottom
    // (RenderSingleChildViewport.getOffsetToReveal) = 1000 - 550.
    let controller = ScrollController::new();
    let tree = column_tree(&controller, true);
    Scrollable::ensure_visible(&tree, row(&tree, 10), EnsureVisible::new());
    assert_eq!(controller.offset(), 450.);
}

#[test]
fn sliver_list_rows_are_revealed_like_box_rows() {
    let controller = ScrollController::new();
    let mut tree = mount(ListView::new(rows()).controller(controller.clone()).into());
    // Row 3 (150..200) already ends at the bottom edge: alignment 1.0 asks
    // for 150 - (200 - 50) = 0, the current offset, so nothing moves.
    assert_eq!(
        Scrollable::ensure_visible(&tree, row(&tree, 3), EnsureVisible::new().alignment(1.0)),
        0
    );
    Scrollable::ensure_visible(&tree, row(&tree, 3), EnsureVisible::new());
    assert_eq!(controller.offset(), 150.);
    tree.layout(Constraints::tight(VIEWPORT)).expect("layout");
    Scrollable::ensure_visible(&tree, row(&tree, 4), EnsureVisible::new().alignment(0.5));
    assert_eq!(controller.offset(), 125.);
}

#[test]
fn nested_viewports_each_move_from_the_pre_scroll_geometry() {
    // Outer vertical view: a 300px spacer, a 100px-high horizontal view of
    // ten 50px cells, then 500px of content (900 total, max 700).
    let outer = ScrollController::new();
    let inner = ScrollController::new();
    let cells = (0..10)
        .map(|index| {
            Widget::box_(Size::new(50., 100.), Color::BLACK).with_key(Key::Value(100 + index))
        })
        .collect::<Vec<_>>();
    let tree = mount(
        SingleChildScrollView::new(Column::new(vec![
            Widget::box_(Size::new(200., 300.), Color::WHITE),
            SizedBox::from_dimensions(
                Some(200.),
                Some(100.),
                Some(
                    SingleChildScrollView::new(
                        Row::new(cells).text_direction(incular_widgets::TextDirection::Ltr),
                    )
                    .scroll_direction(Axis::Horizontal)
                    .controller(inner.clone()),
                ),
            )
            .into(),
            Widget::box_(Size::new(200., 500.), Color::WHITE),
        ]))
        .controller(outer.clone())
        .into(),
    );
    // Innermost first: cell 6 leads at 300 in the inner view (max 300). The
    // outer view then reveals the inner view where the cell sits, at 300.
    assert_eq!(
        Scrollable::ensure_visible(&tree, row(&tree, 106), EnsureVisible::new()),
        2
    );
    assert_eq!(inner.offset(), 300.);
    assert_eq!(outer.offset(), 300.);
}

#[test]
fn a_duration_animates_instead_of_jumping() {
    let controller = ScrollController::new();
    let tree = column_tree(&controller, false);
    Scrollable::ensure_visible(
        &tree,
        row(&tree, 10),
        EnsureVisible::new().duration(Duration::from_millis(200)),
    );
    assert_eq!(controller.offset(), 0.);
    assert!(controller.has_pending_animation());
}

#[test]
fn an_element_outside_any_viewport_moves_nothing() {
    let tree = mount(Widget::box_(Size::new(50., 50.), Color::WHITE).with_key(Key::Value(7)));
    assert_eq!(
        Scrollable::ensure_visible(&tree, row(&tree, 7), EnsureVisible::new()),
        0
    );
}
