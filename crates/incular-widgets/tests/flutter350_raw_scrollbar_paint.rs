//! Flutter 3.50.0 RawScrollbar and ScrollbarPainter geometry.
use std::time::{Duration, Instant};

use incular_config::{Axis, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::PaintCommand;
use incular_scroll::ScrollController;
use incular_widgets::{
    Directionality, RawScrollbar, RawScrollbarOrientation, RawScrollbarStyle, ScrollBehavior,
    ScrollConfiguration, SingleChildScrollView, Widget, internal::WidgetTree,
};

fn bar(
    orientation: RawScrollbarOrientation,
    axis: Axis,
    viewport: f32,
    range: f32,
    minimum: f32,
) -> (RawScrollbar, ScrollController) {
    let controller = ScrollController::new();
    controller.try_set_metrics_context(axis, false).unwrap();
    controller
        .update_extents(viewport + range, viewport)
        .unwrap();
    let mut bar = RawScrollbar::new(controller.clone());
    bar.set_style(RawScrollbarStyle {
        orientation,
        thickness: 2.5,
        min_thumb_extent: minimum,
        min_overscroll_thumb_extent: minimum,
        thumb_visibility: Some(true),
        ..RawScrollbarStyle::default()
    });
    let at = Instant::now();
    bar.tick(at);
    bar.tick(at + Duration::from_millis(300));
    (bar, controller)
}

#[test]
fn a_small_explicit_minimum_is_respected_on_a_very_long_scroll_view() {
    let (bar, controller) = bar(
        RawScrollbarOrientation::Right,
        Axis::Vertical,
        10.,
        100_000.,
        3.5,
    );
    let size = Size::new(600., 10.);
    for pixels in [0., 1.] {
        controller.jump_to(pixels);
        let geometry = bar.geometry(size);
        assert!(geometry.visible);
        assert_eq!(geometry.thumb.origin.x, 597.5);
        assert_eq!(geometry.thumb.size.width, 2.5);
        assert!(geometry.thumb.size.height >= 3.5);
        if pixels == 0. {
            assert_eq!(geometry.thumb.origin.y, 0.);
        }
    }
}

#[test]
fn a_normally_scrolling_thumb_preserves_length_and_moves_proportionally() {
    let (bar, controller) = bar(
        RawScrollbarOrientation::Right,
        Axis::Vertical,
        23.,
        100.,
        0.,
    );
    let length = 23. * 23. / 123.;
    for pixels in [0.01, 23., 46., 69., 92., 99.99] {
        controller.jump_to(pixels);
        let thumb = bar.geometry(Size::new(600., 23.)).thumb;
        assert_eq!((thumb.origin.x, thumb.size.width), (597.5, 2.5));
        assert!((thumb.size.height - length).abs() < 0.001);
        assert!((thumb.origin.y - (23. - length) * pixels / 100.).abs() < 0.001);
    }
}

#[test]
fn explicit_edges_use_metrics_not_the_other_canvas_dimension_for_thumb_length() {
    use RawScrollbarOrientation::{Bottom, Left, Right, Top};
    for (orientation, axis, origin, size) in [
        (Left, Axis::Vertical, Offset::ZERO, Size::new(2.5, 18.)),
        (
            Right,
            Axis::Vertical,
            Offset::new(597.5, 0.),
            Size::new(2.5, 18.),
        ),
        (Top, Axis::Horizontal, Offset::ZERO, Size::new(18., 2.5)),
        (
            Bottom,
            Axis::Horizontal,
            Offset::new(0., 20.5),
            Size::new(18., 2.5),
        ),
    ] {
        let (bar, _) = bar(orientation, axis, 23., 100., 18.);
        assert_eq!(
            bar.geometry(Size::new(600., 23.)).thumb,
            Rect::from_origin_size(origin, size)
        );
    }
}

#[test]
fn a_mounted_explicit_scrollbar_paints_above_its_child_and_insets_only_its_thumb() {
    let viewport = Size::new(800., 600.);
    let controller = ScrollController::new();
    controller.update_extents(1000., 600.).unwrap();
    let track = Color::rgba(25, 50, 125, 255);
    let thumb = Color::rgba(120, 50, 25, 255);
    let mut scroll = RawScrollbar::new(controller.clone());
    scroll.set_style(RawScrollbarStyle {
        main_axis_margin: 10.,
        thumb_visibility: Some(true),
        track_color: track,
        thumb_color: thumb,
        ..RawScrollbarStyle::default()
    });
    let child = Widget::box_(viewport, Color::WHITE);
    let mut tree = WidgetTree::new();
    tree.mount(scroll.with_child(child)).unwrap();
    tree.layout(Constraints::tight(viewport)).unwrap();
    let extract = |tree: &mut WidgetTree| {
        tree.paint()
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Rect { rect, color } => Some((*rect, *color)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let at = Instant::now();
    tree.pump_scrollbar_fades(at);
    tree.pump_scrollbar_fades(at + Duration::from_millis(300));
    let visible = extract(&mut tree);
    let rect = |y, height| Rect::from_origin_size(Offset::new(794., y), Size::new(6., height));
    let start = [(rect(0., 600.), track), (rect(10., 348.), thumb)];
    assert!(visible.ends_with(&start), "{visible:?}");
    assert!(visible.iter().any(|(_, color)| *color == Color::WHITE));
    controller.jump_to(400.);
    tree.pump_scrollbar_fades(at + Duration::from_millis(320));
    assert!(extract(&mut tree).ends_with(&[(rect(0., 600.), track), (rect(242., 348.), thumb)]));
}

const RAW_INK: Color = Color::rgba(188, 188, 188, 102);

fn ink(tree: &mut WidgetTree) -> Option<Rect> {
    tree.paint()
        .commands()
        .iter()
        .rev()
        .find_map(|command| match command {
            PaintCommand::Rect { rect, color } if *color == RAW_INK => Some(*rect),
            _ => None,
        })
}

fn settle(tree: &mut WidgetTree, viewport: Size, now: Instant) -> Option<Rect> {
    tree.layout(Constraints::tight(viewport)).unwrap();
    tree.pump_scrollbar_fades(now);
    tree.pump_scrollbar_fades(now + Duration::from_millis(320));
    ink(tree)
}

fn visible_raw(controller: &ScrollController) -> RawScrollbar {
    let mut bar = RawScrollbar::new(controller.clone());
    bar.set_style(RawScrollbarStyle {
        thumb_visibility: Some(true),
        ..RawScrollbarStyle::default()
    });
    bar
}

#[test]
fn omitted_edge_follows_directionality_and_the_measured_scroll_axis() {
    let canvas = Size::new(600., 23.);
    let at = Instant::now();
    let controller = ScrollController::new();
    controller.update_extents(123., 23.).unwrap();
    let painter_like = |controller: &ScrollController| {
        let mut raw = visible_raw(controller);
        raw.set_style(RawScrollbarStyle {
            thickness: 2.5,
            ..raw.style()
        });
        raw
    };
    let overlay = |direction| {
        Directionality::new(
            direction,
            painter_like(&controller).with_child(Widget::box_(canvas, Color::WHITE)),
        )
        .into()
    };

    let mut tree = WidgetTree::new();
    let root = tree.mount(overlay(TextDirection::Ltr)).unwrap();
    assert_eq!(
        settle(&mut tree, canvas, at),
        Some(Rect::from_origin_size(
            Offset::new(597.5, 0.),
            Size::new(2.5, 18.)
        ))
    );
    tree.update(root, overlay(TextDirection::Rtl)).unwrap();
    assert_eq!(
        settle(&mut tree, canvas, at + Duration::from_secs(1)),
        Some(Rect::from_origin_size(Offset::ZERO, Size::new(2.5, 18.)))
    );

    let mut horizontal = WidgetTree::new();
    let controller = ScrollController::new();
    controller
        .try_set_metrics_context(Axis::Horizontal, false)
        .unwrap();
    controller.update_extents(123., 23.).unwrap();
    horizontal
        .mount(painter_like(&controller).with_child(Widget::box_(canvas, Color::WHITE)))
        .unwrap();
    assert_eq!(
        settle(&mut horizontal, canvas, at),
        Some(Rect::from_origin_size(
            Offset::new(0., 20.5),
            Size::new(18., 2.5)
        ))
    );
}

fn scroll_overlay(controller: &ScrollController, height: f32) -> Widget {
    let child = Widget::box_(Size::new(800., height), Color::WHITE);
    let view = SingleChildScrollView::new(child).controller(controller.clone());
    Directionality::new(
        TextDirection::Ltr,
        ScrollConfiguration::new(visible_raw(controller).with_child(view))
            .behavior(ScrollBehavior::new().scrollbars(false)),
    )
    .into()
}

#[test]
fn a_raw_scrollbar_has_a_rectangular_thumb_by_default() {
    let size = Size::new(800., 600.);
    let controller = ScrollController::new();
    let mut tree = WidgetTree::new();
    tree.mount(scroll_overlay(&controller, 1000.)).unwrap();
    assert_eq!(
        settle(&mut tree, size, Instant::now()),
        Some(Rect::from_origin_size(
            Offset::new(794., 0.),
            Size::new(6., 360.)
        ))
    );
}

#[test]
fn the_same_overlay_repaints_when_content_crosses_the_viewport_extent() {
    let size = Size::new(800., 600.);
    let controller = ScrollController::new();
    let at = Instant::now();
    let mut tree = WidgetTree::new();
    let root = tree.mount(scroll_overlay(&controller, 600.)).unwrap();
    assert_eq!(settle(&mut tree, size, at), None);
    tree.update(root, scroll_overlay(&controller, 600.1))
        .unwrap();
    assert!(settle(&mut tree, size, at + Duration::from_secs(1)).is_some());
    tree.update(root, scroll_overlay(&controller, 600.))
        .unwrap();
    assert_eq!(settle(&mut tree, size, at + Duration::from_secs(2)), None);
}

#[test]
fn the_same_overlay_repaints_when_the_window_height_changes_one_pixel() {
    let size = Size::new(800., 600.);
    let controller = ScrollController::new();
    let at = Instant::now();
    let mut tree = WidgetTree::new();
    tree.mount(scroll_overlay(&controller, 600.)).unwrap();
    assert_eq!(settle(&mut tree, size, at), None);
    assert_eq!(controller.offset(), 0.);
    assert!(
        settle(
            &mut tree,
            Size::new(800., 599.),
            at + Duration::from_secs(1)
        )
        .is_some()
    );
    assert_eq!(settle(&mut tree, size, at + Duration::from_secs(2)), None);
}
