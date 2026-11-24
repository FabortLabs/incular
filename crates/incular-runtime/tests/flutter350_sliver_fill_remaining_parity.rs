//! SliverFillRemaining sizing and semantics from pinned Flutter 3.50.0.
use incular_config::{Alignment, Axis, Constraints};
use incular_core::{Color, Rect, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    Align, Container, CustomScrollView, Directionality, Padding, ScrollController, SizedBox,
    Sliver, SliverFillRemaining, SliverFixedExtentList, SliverToBoxAdapter, Text, TextDirection,
    Widget, internal::Key,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const FILL: u64 = 10;
const PRECEDING: u64 = 11;
const BUTTON: u64 = 12;
const LABEL: &str = "Text in SliverFillRemaining";
fn frame(r: &mut Runtime) {
    r.run_frame(Constraints::tight(VIEW)).unwrap();
}
fn viewport(slivers: Vec<Box<dyn Sliver>>, axis: Axis, controller: &ScrollController) -> Runtime {
    let view = CustomScrollView::new(slivers)
        .cache_extent(250.)
        .controller(controller.clone())
        .scroll_direction(axis);
    let mut r = Runtime::new(Directionality::new(TextDirection::Ltr, view).into()).unwrap();
    frame(&mut r);
    r
}
fn keyed(w: impl Into<Widget>, n: u64) -> Widget {
    w.into().with_key(Key::Value(n))
}
fn bounds(r: &Runtime, n: u64) -> Option<Rect> {
    let t = r.tree();
    t.element_with_key(&Key::Value(n))
        .and_then(|id| t.element_bounds(id))
}
fn preceding(extent: f32, axis: Axis) -> Box<dyn Sliver> {
    let size = match axis {
        Axis::Horizontal => Size::new(extent, 150.),
        Axis::Vertical => Size::new(150., extent),
    };
    Box::new(SliverToBoxAdapter::new(keyed(
        Widget::box_(size, Color::WHITE),
        PRECEDING,
    )))
}
fn empty_fill() -> Widget {
    keyed(Widget::box_(Size::ZERO, Color::WHITE), FILL)
}
fn fill(child: Widget, scroll_body: bool) -> Box<dyn Sliver> {
    let d = SliverFillRemaining::new(child);
    if scroll_body {
        Box::new(d)
    } else {
        Box::new(d.has_scroll_body(false))
    }
}

#[test]
fn default_fill_with_no_sibling_lays_out_full_viewport_across_jumps() {
    let ctrl = ScrollController::new();
    let mut r = viewport(
        vec![fill(keyed(Container::new(), FILL), true)],
        Axis::Vertical,
        &ctrl,
    );
    let actual = [0., 50., -100., 0.].map(|offset| {
        let _ = ctrl.jump_to(offset);
        frame(&mut r);
        bounds(&r, FILL).expect("fill mounted").size.height
    });
    assert_eq!(actual, [600.; 4]);
}
#[test]
fn default_fill_with_one_sibling_tracks_remaining_paint_extent_for_all_jumps() {
    let ctrl = ScrollController::new();
    let mut r = viewport(
        vec![
            preceding(100., Axis::Vertical),
            fill(keyed(Container::new(), FILL), true),
        ],
        Axis::Vertical,
        &ctrl,
    );
    let actual = [0., 50., -100., 0.].map(|offset| {
        let _ = ctrl.jump_to(offset);
        frame(&mut r);
        bounds(&r, FILL).expect("fill mounted").size.height
    });
    assert_eq!(actual, [500., 550., 400., 500.]);
}

fn jump_past_sibling(scroll_body: bool) -> (f32, bool, bool) {
    let ctrl = ScrollController::new();
    let mut r = viewport(
        vec![
            preceding(150., Axis::Vertical),
            fill(empty_fill(), scroll_body),
        ],
        Axis::Vertical,
        &ctrl,
    );
    let onscreen = |r: &Runtime, n| {
        bounds(r, n).is_some_and(|b| {
            b.size.height > 0. && b.origin.y + b.size.height > 0. && b.origin.y < 600.
        })
    };
    assert!(
        onscreen(&r, PRECEDING) && onscreen(&r, FILL),
        "both mount inside the first viewport"
    );
    let _ = ctrl.jump_to(150.);
    // The Flutter originals assert after pumpAndSettle, not immediately
    // after the out-of-range programmatic jump.
    let now = Instant::now();
    for i in 0..120 {
        r.run_frame_at(
            Constraints::tight(VIEW),
            now + std::time::Duration::from_millis(i * 16),
        )
        .unwrap();
    }
    (ctrl.offset(), onscreen(&r, PRECEDING), onscreen(&r, FILL))
}
#[test]
fn default_scroll_body_can_move_preceding_sibling_fully_offscreen() {
    assert_eq!(jump_past_sibling(true), (150., false, true));
}
#[test]
fn not_a_scroll_body_does_not_extend_scroll_range_beyond_viewport() {
    assert_eq!(jump_past_sibling(false), (0., true, true));
}
#[test]
fn non_scrolling_fill_sizes_natural_zero_extent_across_both_axes() {
    let actual = [Axis::Vertical, Axis::Horizontal].map(|axis| {
        let ctrl = ScrollController::new();
        let r = viewport(
            vec![preceding(150., axis), fill(empty_fill(), false)],
            axis,
            &ctrl,
        );
        bounds(&r, FILL).unwrap().size
    });
    assert_eq!(actual, [Size::new(800., 450.), Size::new(650., 600.)]);
}
#[test]
fn non_scrolling_fill_can_grow_past_viewport_for_a_large_child() {
    let actual = [Axis::Vertical, Axis::Horizontal].map(|axis| {
        let ctrl = ScrollController::new();
        let child = keyed(Widget::box_(Size::new(1000., 600.), Color::WHITE), FILL);
        let r = viewport(vec![preceding(150., axis), fill(child, false)], axis, &ctrl);
        bounds(&r, FILL).unwrap().size
    });
    assert_eq!(actual, [Size::new(800., 600.), Size::new(1000., 600.)]);
}
#[test]
fn non_scrolling_fill_centers_smaller_bottom_aligned_child_in_both_axes() {
    let actual = [Axis::Vertical, Axis::Horizontal].map(|axis| {
        let ctrl = ScrollController::new();
        let button = keyed(Widget::box_(Size::new(116., 48.), Color::BLACK), BUTTON);
        let child = keyed(Align::new(Alignment::BOTTOM_CENTER, button), FILL);
        let r = viewport(vec![preceding(150., axis), fill(child, false)], axis, &ctrl);
        let b = bounds(&r, BUTTON).unwrap();
        (
            bounds(&r, FILL).unwrap().size,
            b.origin.x + b.size.width / 2.,
            b.origin.y + b.size.height,
        )
    });
    assert_eq!(
        actual,
        [
            (Size::new(800., 450.), 400., 600.),
            (Size::new(650., 600.), 475., 600.)
        ]
    );
}

fn semantic_visibility(scroll_body: bool, lead: f32) -> Option<bool> {
    let key = keyed(SizedBox::new().height(100.).child(Text::new(LABEL)), FILL);
    let ctrl = ScrollController::new();
    let r = viewport(
        vec![preceding(lead, Axis::Vertical), fill(key, scroll_body)],
        Axis::Vertical,
        &ctrl,
    );
    let mut matches = r
        .tree()
        .semantics()
        .iter()
        .filter(|(_, node)| node.label.as_deref() == Some(LABEL));
    let found = matches.next().map(|(_, node)| node.state.hidden);
    if let Some(b) = bounds(&r, FILL) {
        assert_eq!(
            b.origin.y, lead,
            "fill must be physically positioned after its sibling"
        );
    }
    assert!(
        matches.next().is_none(),
        "the fill label must not be duplicated"
    );
    found
}
#[test]
fn scroll_body_semantics_respects_viewport_and_cache_boundaries() {
    let actual = [500., 600., 850.].map(|lead| semantic_visibility(true, lead));
    assert_eq!(actual, [Some(false), Some(true), None]);
}
#[test]
fn non_scroll_body_semantics_respects_viewport_and_cache_boundaries() {
    let actual = [500., 600., 850.].map(|lead| semantic_visibility(false, lead));
    assert_eq!(actual, [Some(false), Some(true), None]);
}

#[test]
fn non_scrolling_fill_measures_nested_child_after_long_preceding_list() {
    let list = SliverFixedExtentList::new(5, 150., |_| Widget::box_(Size::ZERO, Color::WHITE));
    let button = keyed(Widget::box_(Size::new(116., 48.), Color::BLACK), BUTTON);
    let target = keyed(
        Align::new(Alignment::CENTER, Padding::all(50., button)),
        FILL,
    );
    let ctrl = ScrollController::new();
    let mut r = viewport(
        vec![Box::new(list), fill(target, false)],
        Axis::Vertical,
        &ctrl,
    );
    let _ = ctrl.jump_to(ctrl.max_offset());
    frame(&mut r);
    let actual = (
        bounds(&r, FILL).map(|r| r.size.height),
        bounds(&r, BUTTON).map(|b| (b.origin.x + b.size.width / 2., b.origin.y + b.size.height)),
    );
    assert_eq!(actual, (Some(148.), Some((400., 550.))));
}
