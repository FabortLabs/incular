//! Viewport `center` and `anchor` (slivers-010): Flutter
//! `RenderViewport._attemptLayout` (`rendering/viewport.dart:1618`). The
//! center sliver starts at scroll offset zero, `anchor` of the way down the
//! viewport; slivers before it grow in reverse, at negative offsets, and the
//! position's range is `min(0, minExtent + extent * anchor)` to
//! `max(0, maxExtent - extent * (1 - anchor))`.

use std::time::Instant;

use incular_config::{Axis, Constraints};
use incular_core::{Color, Size};
use incular_scroll::ScrollController;
use incular_widgets::{
    CustomScrollView, ShrinkWrappingViewport, Viewport, Widget,
    internal::{ElementId, Sliver, SliverList, SliverToBoxAdapter, WidgetTree},
};

fn viewport() -> Size {
    Size::new(100., 100.)
}

fn boxed(height: f32) -> Box<dyn Sliver> {
    Box::new(SliverToBoxAdapter::new(Widget::box_(
        Size::new(100., height),
        Color::WHITE,
    )))
}

struct Harness {
    tree: WidgetTree,
    root: ElementId,
    controller: ScrollController,
}

impl Harness {
    fn new(view: impl Into<Widget>, controller: ScrollController) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(view.into()).expect("mount");
        let mut harness = Self {
            tree,
            root,
            controller,
        };
        harness.layout();
        harness
    }

    fn layout(&mut self) {
        self.tree
            .layout(Constraints::tight(viewport()))
            .expect("layout");
        self.tree
            .update_compositor(Instant::now())
            .expect("compositor");
    }

    fn jump(&mut self, offset: f32) {
        self.controller.jump_to(offset);
        self.layout();
    }

    /// The main-axis `(top, bottom)` of each retained child, in order.
    fn spans(&self) -> Vec<(f32, f32)> {
        self.tree
            .children(self.root)
            .expect("viewport children")
            .iter()
            .map(|&child| {
                let rect = self.tree.element_bounds(child).expect("bounds");
                (rect.origin.y, rect.origin.y + rect.size.height)
            })
            .collect()
    }
}

/// Two slivers (30 and 40) before a center sliver (50) and one after (60).
fn four_slivers() -> Vec<Box<dyn Sliver>> {
    vec![boxed(30.), boxed(40.), boxed(50.), boxed(60.)]
}

#[test]
fn slivers_before_the_center_grow_upward_from_offset_zero() {
    let controller = ScrollController::new();
    let harness = Harness::new(
        CustomScrollView::new(four_slivers())
            .center(2)
            .controller(controller.clone()),
        controller.clone(),
    );
    // Sliver order is kept: 0 farthest above the center, then 1, the
    // center at the leading edge and 3 after it.
    assert_eq!(
        harness.spans(),
        [(-70., -40.), (-40., 0.), (0., 50.), (50., 110.)]
    );
    assert_eq!(controller.offset(), 0.);
    assert_eq!(controller.min_offset(), -70.);
    assert_eq!(controller.max_offset(), 10.);
}

#[test]
fn the_anchor_places_offset_zero_inside_the_viewport() {
    let controller = ScrollController::new();
    let mut harness = Harness::new(
        Viewport::new(four_slivers())
            .center(2)
            .anchor(0.5)
            .controller(controller.clone()),
        controller.clone(),
    );
    assert_eq!(
        harness.spans(),
        [(-20., 10.), (10., 50.), (50., 100.), (100., 160.)]
    );
    // min(0, -70 + 50) and max(0, 110 - 50).
    assert_eq!(controller.min_offset(), -20.);
    assert_eq!(controller.max_offset(), 60.);

    // Scrolling back to the minimum shows the first sliver at the top.
    harness.jump(-20.);
    assert_eq!(harness.spans()[0], (0., 30.));
    // The minimum is in range: nothing springs back from it.
    assert_eq!(controller.offset(), -20.);
    assert!(!controller.has_pending_settle());
    // One pixel past it is overscroll, sprung back on the next frame.
    controller.jump_to(-21.);
    assert!(controller.has_pending_settle());
}

#[test]
fn an_anchor_alone_lets_the_first_sliver_start_mid_viewport() {
    let controller = ScrollController::new();
    let harness = Harness::new(
        CustomScrollView::new(vec![boxed(80.), boxed(80.)])
            .anchor(0.25)
            .controller(controller.clone()),
        controller.clone(),
    );
    assert_eq!(harness.spans(), [(25., 105.), (105., 185.)]);
    assert_eq!(controller.min_offset(), 0.);
    // max(0, 160 - 100 * 0.75).
    assert_eq!(controller.max_offset(), 85.);
}

#[test]
fn a_reverse_list_lays_out_its_items_away_from_the_center() {
    let controller = ScrollController::new();
    let list: Box<dyn Sliver> = Box::new(SliverList::builder(50, |index| {
        // Uneven heights, so the estimates and the measurements differ.
        Widget::box_(
            Size::new(100., if index % 2 == 0 { 20. } else { 30. }),
            Color::WHITE,
        )
    }));
    let mut harness = Harness::new(
        CustomScrollView::new(vec![list, boxed(100.)])
            .center(1)
            .controller(controller.clone()),
        controller.clone(),
    );
    // Only the center is visible; scroll up into the list.
    harness.jump(-60.);
    let spans = harness.spans();
    // Item 0 sits right above the center, item 1 above it, and so on.
    assert!(spans.contains(&(40., 60.)), "{spans:?}");
    assert!(spans.contains(&(10., 40.)), "{spans:?}");
    assert!(spans.contains(&(-10., 10.)), "{spans:?}");
    // The center follows below.
    assert!(spans.contains(&(60., 160.)), "{spans:?}");
    assert!(controller.min_offset() < -60.);
}

#[test]
fn the_center_keeps_its_place_when_reverse_content_grows() {
    let controller = ScrollController::new();
    let mut harness = Harness::new(
        CustomScrollView::new(vec![boxed(30.), boxed(100.)])
            .center(1)
            .controller(controller.clone()),
        controller.clone(),
    );
    assert_eq!(harness.spans(), [(-30., 0.), (0., 100.)]);
    // Content added before the center extends the minimum and moves nothing
    // on screen, the point of a centered viewport.
    harness
        .tree
        .update(
            harness.root,
            CustomScrollView::new(vec![boxed(40.), boxed(30.), boxed(100.)])
                .center(2)
                .controller(controller.clone())
                .into(),
        )
        .expect("update");
    harness.layout();
    assert_eq!(controller.offset(), 0.);
    assert_eq!(controller.min_offset(), -70.);
    assert_eq!(harness.spans(), [(-70., -30.), (-30., 0.), (0., 100.)]);
}

#[test]
fn a_reversed_axis_mirrors_the_centered_layout() {
    let controller = ScrollController::new();
    let harness = Harness::new(
        CustomScrollView::new(four_slivers())
            .center(2)
            .reverse(true)
            .controller(controller.clone()),
        controller.clone(),
    );
    // Offset zero is at the bottom edge: the center grows up from it and
    // the reverse slivers continue below the viewport.
    assert_eq!(
        harness.spans(),
        [
            (170., 140.).min_max(),
            (140., 100.).min_max(),
            (50., 100.),
            (-10., 50.)
        ]
    );
}

#[test]
fn a_shrink_wrapping_viewport_takes_its_axis_and_controller() {
    let controller = ScrollController::new();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            ShrinkWrappingViewport::new(vec![boxed(30.), boxed(40.)])
                .axis_direction(Axis::Horizontal)
                .controller(controller.clone())
                .into(),
        )
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 50.)))
        .expect("layout");
    assert_eq!(controller.viewport_extent(), 200.);
    assert_eq!(controller.content_extent(), 200.);
    assert_eq!(tree.element_bounds(root).expect("bounds").size.width, 200.);
}

trait MinMax {
    fn min_max(self) -> (f32, f32);
}

impl MinMax for (f32, f32) {
    fn min_max(self) -> (f32, f32) {
        (self.0.min(self.1), self.0.max(self.1))
    }
}

#[test]
fn a_reverse_sliver_scrolled_out_parks_at_the_leading_edge() {
    let controller = ScrollController::new();
    let mut harness = Harness::new(
        CustomScrollView::new(vec![boxed(30.), boxed(80.), boxed(200.)])
            .center(1)
            .anchor(0.25)
            .controller(controller.clone()),
        controller.clone(),
    );
    assert_eq!(harness.spans(), [(-5., 25.), (25., 105.), (105., 305.)]);
    harness.jump(-60.);
    assert_eq!(harness.spans(), [(55., 85.), (85., 165.), (165., 365.)]);
    // With the center line above the viewport, Flutter lays the reverse
    // slivers out from the leading edge (`initialLayoutOffset` is clamped),
    // so the hidden first sliver sits just above it, not above the center.
    harness.jump(100.);
    assert_eq!(harness.spans(), [(-30., 0.), (-75., 5.), (5., 205.)]);
}
