use std::time::{Duration, Instant};

use incular_config::{
    Axis, Constraints, EdgeInsets, EdgeInsetsDirectional, RuntimeEnvironment, TextDirection,
};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::PaintCommand;
use incular_scroll::{MetricOwner, ScrollController, ScrollPhysics, ViewportMetricsUpdate};
use incular_widgets::{
    Directionality, MediaQuery, RawScrollbar, RawScrollbarStyle, ScrollBehavior,
    ScrollConfiguration, SingleChildScrollView, Widget, internal::WidgetTree,
};

const TRACK: Color = Color::rgba(1, 2, 3, 255);
const THUMB: Color = Color::rgba(101, 102, 103, 255);

fn resolve(style: RawScrollbarStyle, controller: &ScrollController) -> RawScrollbar {
    let mut bar = RawScrollbar::new(controller.clone());
    bar.set_style(RawScrollbarStyle {
        thumb_visibility: Some(true),
        track_color: TRACK,
        thumb_color: THUMB,
        ..style
    });
    bar
}

fn read(tree: &mut WidgetTree) -> (Rect, Rect) {
    let at = Instant::now();
    tree.pump_scrollbar_fades(at);
    tree.pump_scrollbar_fades(at + Duration::from_millis(300));
    let list = tree.paint();
    let by_color = |target| {
        list.commands()
            .iter()
            .rev()
            .find_map(|command| match command {
                PaintCommand::Rect { rect, color } if *color == target => Some(*rect),
                _ => None,
            })
            .unwrap()
    };
    (by_color(TRACK), by_color(THUMB))
}

fn paint(
    style: RawScrollbarStyle,
    controller: &ScrollController,
    size: Size,
    direction: TextDirection,
) -> (Rect, Rect) {
    let mut tree = WidgetTree::new();
    let bar = resolve(style, controller);
    let widget = Directionality::new(direction, bar.with_child(Widget::box_(size, Color::WHITE)));
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(size)).unwrap();
    read(&mut tree)
}

fn exact(value: f32, expected: f32) {
    assert!((value - expected).abs() < 0.0001, "{value} != {expected}");
}

fn settings(
    axis: Axis,
    reverse: bool,
    viewport: f32,
    before: f32,
    after: f32,
) -> (ScrollController, incular_scroll::MetricAttachment) {
    let controller = ScrollController::new();
    let lease = controller.try_attach(MetricOwner::of_tree(1)).unwrap();
    let mut metrics = ViewportMetricsUpdate::with_axis(
        viewport + before + after,
        viewport,
        axis,
        reverse,
        ScrollPhysics::default(),
    );
    metrics.leading = before;
    lease.update_extents(metrics).unwrap();
    (controller, lease)
}

#[test]
fn asymmetric_padding_sets_track_and_thumb_edges_in_all_four_directions() {
    let size = Size::new(60., 80.);
    let style = RawScrollbarStyle {
        padding: Some(EdgeInsets::only(1., 2., 3., 4.).into()),
        ..RawScrollbarStyle::default()
    };
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for reverse in [false, true] {
            let (controller, _lease) = settings(axis, reverse, axis.main_extent(size), 100., 240.);
            for first in [true, false] {
                let logical_positive = first == reverse;
                controller.jump_to(if logical_positive { 50_000. } else { -50_000. });
                let (track, thumb) = paint(style, &controller, size, TextDirection::Ltr);
                if axis.is_vertical() {
                    exact(track.origin.y, 2.);
                    exact(track.size.height, 74.);
                    exact(size.width - thumb.origin.x - thumb.size.width, 3.);
                    if first {
                        exact(thumb.origin.y, 2.);
                    } else {
                        exact(size.height - thumb.origin.y - thumb.size.height, 4.);
                    }
                } else {
                    exact(track.origin.x, 1.);
                    exact(track.size.width, 56.);
                    exact(size.height - thumb.origin.y - thumb.size.height, 4.);
                    if first {
                        exact(thumb.origin.x, 1.);
                    } else {
                        exact(size.width - thumb.origin.x - thumb.size.width, 3.);
                    }
                }
            }
        }
    }
}

#[test]
fn directional_padding_resolves_the_same_trailing_edge_in_ltr_and_rtl() {
    let size = Size::new(60., 80.);
    let style = RawScrollbarStyle {
        padding: Some(EdgeInsetsDirectional::only(1., 2., 3., 4.).into()),
        ..RawScrollbarStyle::default()
    };
    let (controller, _lease) = settings(Axis::Vertical, false, 80., 100., 240.);
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        for top in [true, false] {
            controller.jump_to(if top { -50_000. } else { 50_000. });
            let (_, thumb) = paint(style, &controller, size, direction);
            if direction == TextDirection::Ltr {
                exact(size.width - thumb.origin.x - thumb.size.width, 3.);
            } else {
                exact(thumb.origin.x, 3.);
            }
            if top {
                exact(thumb.origin.y, 2.);
            } else {
                exact(size.height - thumb.origin.y - thumb.size.height, 4.);
            }
        }
    }
}

#[test]
fn inherited_media_padding_positions_the_track_and_explicit_padding_replaces_it() {
    for explicit in [None, Some(100.)] {
        let size = Size::new(800., 600.);
        let controller = ScrollController::new();
        let bar = resolve(
            RawScrollbarStyle {
                min_thumb_extent: 21.,
                min_overscroll_thumb_extent: 8.,
                padding: explicit.map(|value| EdgeInsets::all(value).into()),
                ..RawScrollbarStyle::default()
            },
            &controller,
        );
        let scrollable =
            SingleChildScrollView::new(Widget::box_(Size::new(1000., 50_000.), Color::WHITE))
                .controller(controller);
        let viewport = ScrollConfiguration::new(bar.with_child(scrollable))
            .behavior(ScrollBehavior::new().scrollbars(false));
        let media = MediaQuery::new(
            RuntimeEnvironment {
                safe_insets: EdgeInsets::all(50.),
                ..RuntimeEnvironment::default()
            },
            viewport,
        );
        let scene = Directionality::new(TextDirection::Ltr, media);
        let mut tree = WidgetTree::new();
        tree.mount(scene.into()).unwrap();
        tree.layout(Constraints::tight(size)).unwrap();
        let (track, thumb) = read(&mut tree);
        let inset = explicit.unwrap_or(50.);
        let origin = Offset::new(800. - inset - 6., inset);
        assert_eq!(
            track,
            Rect::from_origin_size(origin, Size::new(6., 600. - inset * 2.))
        );
        assert_eq!(thumb, Rect::from_origin_size(origin, Size::new(6., 21.)));
    }
}

#[test]
fn signed_main_margin_and_ambient_cross_margin_both_affect_painted_geometry() {
    let size = Size::new(600., 23.);
    let (controller, _lease) = settings(Axis::Vertical, false, 23., 0., 100.);
    for margin in [-10., 1., 23. / 2. - 0.01] {
        let style = RawScrollbarStyle {
            main_axis_margin: margin,
            min_thumb_extent: 0.,
            ..RawScrollbarStyle::default()
        };
        controller.jump_to(-50_000.);
        let (_, top) = paint(style, &controller, size, TextDirection::Ltr);
        exact(top.origin.y, margin);
        controller.jump_to(50_000.);
        let (_, bottom) = paint(style, &controller, size, TextDirection::Ltr);
        exact(size.height - bottom.origin.y - bottom.size.height, margin);
    }

    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            let (controller, _lease) = settings(axis, reverse, 23., 0., 100.);
            let style = RawScrollbarStyle {
                cross_axis_margin: 4.,
                ..RawScrollbarStyle::default()
            };
            for direction in [TextDirection::Ltr, TextDirection::Rtl] {
                let (_, thumb) = paint(style, &controller, size, direction);
                if axis.is_horizontal() {
                    exact(size.height - thumb.origin.y - thumb.size.height, 4.);
                } else if direction == TextDirection::Rtl {
                    exact(thumb.origin.x, 4.);
                } else {
                    exact(size.width - thumb.origin.x - thumb.size.width, 4.);
                }
            }
        }
    }
}

#[test]
fn padded_thumb_keeps_length_until_overscroll_then_shrinks_to_its_floor() {
    let size = Size::new(60., 300.);
    let (controller, _lease) = settings(Axis::Vertical, false, 300., 0., 3000.);
    let style = RawScrollbarStyle {
        padding: Some(EdgeInsets::only(1., 2., 3., 4.).into()),
        min_thumb_extent: 36.,
        min_overscroll_thumb_extent: 8.,
        ..RawScrollbarStyle::default()
    };
    let extent = |offset| {
        controller.jump_to(offset);
        paint(style, &controller, size, TextDirection::Ltr)
            .1
            .size
            .height
    };
    let full = extent(0.);
    assert!(full > 18.);
    exact(extent(1500.), full);
    exact(extent(3000.), full);
    assert!((extent(3003.) - full).abs() <= 2.);
    exact(extent(50_000.), 8.);
}

#[test]
fn thumb_stays_monotonic_when_the_minimum_fills_the_viewport() {
    let viewport = Size::new(60., 80.);
    let (controller, _lease) = settings(Axis::Vertical, false, 80., 100., 240.);
    for minimum in [18., f32::INFINITY] {
        let mut scrollbar = RawScrollbar::new(controller.clone())
            .thumb_visibility(true)
            .min_lengths(minimum, Some(minimum));
        let start = Instant::now();
        scrollbar.tick(start);
        scrollbar.tick(start + Duration::from_millis(300));
        let mut previous: Option<Rect> = None;
        let mut pixels = -100.;
        while pixels <= 240. {
            controller.jump_to(pixels);
            let thumb = scrollbar.geometry(viewport).thumb;
            if let Some(before) = previous {
                let (top, bottom) = (thumb.origin.y, thumb.origin.y + thumb.size.height);
                let bottom_before = before.origin.y + before.size.height;
                if thumb.size.height == viewport.height {
                    assert!(before.origin.y <= top && bottom_before <= bottom);
                } else {
                    assert!(before.origin.y < top && bottom_before < bottom);
                }
            }
            previous = Some(thumb);
            pixels += 80. / 3.;
        }
        if minimum.is_infinite() {
            assert_eq!(previous.unwrap().size.height, viewport.height);
        }
    }
}
