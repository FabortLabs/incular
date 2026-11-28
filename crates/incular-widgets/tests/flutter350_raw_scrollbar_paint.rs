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

fn explicit_track_commands(
    orientation: RawScrollbarOrientation,
    configure: impl FnOnce(RawScrollbar) -> RawScrollbar,
) -> Vec<PaintCommand> {
    let viewport = Size::new(800., 600.);
    let controller = ScrollController::new();
    let (axis, main) = match orientation {
        RawScrollbarOrientation::Top | RawScrollbarOrientation::Bottom => (Axis::Horizontal, 800.),
        _ => (Axis::Vertical, 600.),
    };
    controller.try_set_metrics_context(axis, false).unwrap();
    controller.update_extents(4000., main).unwrap();
    let mut bar = RawScrollbar::new(controller);
    bar.set_style(RawScrollbarStyle {
        thumb_visibility: Some(true),
        orientation,
        ..RawScrollbarStyle::default()
    });
    let mut tree = WidgetTree::new();
    tree.mount(configure(bar).with_child(Widget::box_(viewport, Color::WHITE)))
        .unwrap();
    tree.layout(Constraints::tight(viewport)).unwrap();
    let now = Instant::now();
    tree.pump_scrollbar_fades(now);
    tree.pump_scrollbar_fades(now + Duration::from_millis(300));
    let list = tree.paint();
    list.commands()
        .iter()
        .filter(|command| match command {
            PaintCommand::PushTransform { transform } => {
                assert_eq!(*transform, incular_core::Transform::IDENTITY);
                false
            }
            PaintCommand::PopTransform => false,
            _ => true,
        })
        .cloned()
        .collect()
}

fn expected_track_border(start: Offset, end: Offset, color: Color) -> PaintCommand {
    use incular_rendering::{Path, Stroke};
    let mut path = Path::builder();
    path.move_to(start);
    path.line_to(end);
    PaintCommand::StrokePath {
        path: std::sync::Arc::new(path.build()),
        brush: color.into(),
        stroke: Stroke::default(),
    }
}

#[test]
fn a_default_explicit_thumb_does_not_draw_an_unrequested_track_or_border() {
    let commands = explicit_track_commands(RawScrollbarOrientation::Right, |bar| {
        bar.track_color(Color::BLACK)
            .track_border_color(Color::BLACK)
    });
    assert_eq!(
        commands.as_slice(),
        &[
            PaintCommand::Rect {
                rect: Rect::from_origin_size(Offset::ZERO, Size::new(800., 600.)),
                color: Color::WHITE,
            },
            PaintCommand::Rect {
                rect: Rect::from_origin_size(Offset::new(794., 0.), Size::new(6., 90.)),
                color: RAW_INK,
            },
        ],
    );
}

#[test]
fn enabled_track_paints_the_one_pixel_inner_edge_between_track_and_thumb() {
    let rect = |height| Rect::from_origin_size(Offset::new(794., 0.), Size::new(6., height));
    let start = Offset::new(794., 0.);
    let end = Offset::new(794., 600.);
    let commands = explicit_track_commands(RawScrollbarOrientation::Right, |bar| {
        bar.track_visibility(true)
    });
    assert!(commands.ends_with(&[
        PaintCommand::Rect {
            rect: rect(600.),
            color: Color::rgba(0, 0, 0, 8),
        },
        expected_track_border(start, end, Color::rgba(0, 0, 0, 26)),
        PaintCommand::Rect {
            rect: rect(90.),
            color: RAW_INK,
        },
    ]));
    let track = Color::rgba(33, 150, 243, 255);
    let border = Color::rgba(255, 235, 59, 255);
    let thumb = Color::rgba(244, 67, 54, 255);
    let commands = explicit_track_commands(RawScrollbarOrientation::Right, |mut bar| {
        let mut style = bar.style();
        style.thumb_color = thumb;
        bar.set_style(style);
        bar.track_visibility(true)
            .track_color(track)
            .track_border_color(border)
    });
    assert!(commands.ends_with(&[
        PaintCommand::Rect {
            rect: rect(600.),
            color: track
        },
        expected_track_border(start, end, border),
        PaintCommand::Rect {
            rect: rect(90.),
            color: thumb
        },
    ]));
}

#[test]
fn enabled_track_and_thumb_round_independently_without_forcing_border_ink() {
    use incular_rendering::RRect;
    use incular_widgets::Radius;
    let round = |bar: RawScrollbar| {
        bar.track_visibility(true)
            .track_radius(Radius::circular(1.))
            .radius(Radius::circular(2.))
    };
    let with_border = explicit_track_commands(RawScrollbarOrientation::Right, round);
    let commands = explicit_track_commands(RawScrollbarOrientation::Right, |bar| {
        round(bar).track_border_color(Color::TRANSPARENT)
    });
    let rounded = |height, radius, color: Color| PaintCommand::RRect {
        rrect: RRect::uniform(
            Rect::from_origin_size(Offset::new(794., 0.), Size::new(6., height)),
            radius,
        ),
        brush: color.into(),
    };
    assert!(with_border.ends_with(&[
        rounded(600., 1., Color::rgba(0, 0, 0, 8)),
        expected_track_border(
            Offset::new(794., 0.),
            Offset::new(794., 600.),
            Color::rgba(0, 0, 0, 26),
        ),
        rounded(90., 2., RAW_INK),
    ]));
    assert_eq!(commands.len(), 3);
    assert!(commands.ends_with(&[
        rounded(600., 1., Color::rgba(0, 0, 0, 8)),
        rounded(90., 2., RAW_INK),
    ]));
}

#[test]
fn explicitly_transparent_track_fill_and_border_are_preserved_on_all_four_edges() {
    use RawScrollbarOrientation::{Bottom, Left, Right, Top};
    for (side, start, end) in [
        (Left, Offset::new(6., 0.), Offset::new(6., 600.)),
        (Right, Offset::new(794., 0.), Offset::new(794., 600.)),
        (Top, Offset::new(0., 6.), Offset::new(800., 6.)),
        (Bottom, Offset::new(0., 594.), Offset::new(800., 594.)),
    ] {
        let commands = explicit_track_commands(side, |bar| {
            bar.track_visibility(true).track_color(Color::TRANSPARENT)
        });
        assert_eq!(commands.len(), 3, "side={side:?}: {commands:?}");
        assert_eq!(
            commands[1],
            expected_track_border(start, end, Color::rgba(0, 0, 0, 26)),
        );
    }
}

#[cfg(debug_assertions)]
#[test]
fn a_visible_track_rejects_an_explicitly_hidden_thumb_in_either_builder_order() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let create = || RawScrollbar::new(ScrollController::new());
    assert!(
        catch_unwind(AssertUnwindSafe(|| create()
            .thumb_visibility(false)
            .track_visibility(true)))
        .is_err()
    );
    assert!(
        catch_unwind(AssertUnwindSafe(|| create()
            .track_visibility(true)
            .thumb_visibility(false)))
        .is_err()
    );
    let mut raw = create();
    let original = raw.style();
    let invalid = RawScrollbarStyle {
        thumb_visibility: Some(false),
        track_visibility: Some(true),
        ..original
    };
    assert!(catch_unwind(AssertUnwindSafe(|| raw.set_style(invalid))).is_err());
    assert_eq!(
        raw.style(),
        original,
        "reject the descriptor before changing mounted state"
    );

    let accepted = create().track_visibility(true);
    assert_eq!(accepted.style().thumb_visibility, None);
    let accepted = accepted.thumb_visibility(true);
    assert_eq!(accepted.style().thumb_visibility, Some(true));
}

fn shaped_overlay(controller: &ScrollController, thickness: f32) -> RawScrollbar {
    let mut bar = visible_raw(controller);
    bar.set_style(RawScrollbarStyle {
        thickness,
        ..bar.style()
    });
    bar
}

fn shape_child(controller: &ScrollController) -> Widget {
    ScrollConfiguration::new(
        SingleChildScrollView::new(Widget::box_(Size::new(800., 1000.), Color::WHITE))
            .controller(controller.clone()),
    )
    .behavior(ScrollBehavior::new().scrollbars(false))
    .into()
}

fn shape_frame() -> (WidgetTree, ScrollController, Size, Instant) {
    (
        WidgetTree::new(),
        ScrollController::new(),
        Size::new(800., 600.),
        Instant::now(),
    )
}

#[test]
fn shaped_scrollbar_fills_beveled_thumb_without_an_unsolicited_border() {
    use incular_widgets::{BeveledRectangleBorder, BorderRadius, BorderSide, Radius};

    let (mut tree, controller, viewport, now) = shape_frame();
    tree.mount(
        shaped_overlay(&controller, 6.)
            .shape(BeveledRectangleBorder::new(
                BorderSide::NONE,
                BorderRadius::all(Radius::circular(8.)),
            ))
            .with_child(shape_child(&controller)),
    )
    .unwrap();
    settle(&mut tree, viewport, now);
    let list = tree.paint();
    let (path, rule) = list
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::FillPath {
                path,
                brush,
                fill_rule,
            } if *brush == RAW_INK.into() => Some((path, *fill_rule)),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        path.tight_bounds(),
        Some(Rect::from_origin_size(
            Offset::new(794., 0.),
            Size::new(6., 360.),
        ))
    );
    for (x, y, inside) in [
        (797., 0., true),
        (797., 18., true),
        (796., 0., false),
        (798., 0., false),
    ] {
        assert_eq!(
            path.contains(Offset::new(x, y), rule),
            inside,
            "point ({x}, {y})"
        );
    }
    assert!(!list.commands().iter().any(|command| matches!(
        command,
        PaintCommand::StrokePath { .. } | PaintCommand::Border { .. }
    )));
}

#[test]
fn shaped_scrollbar_circle_fills_then_strokes_the_exact_inset_circle() {
    use incular_widgets::{BorderSide, CircleBorder};

    let (mut tree, controller, viewport, now) = shape_frame();
    let child = shape_child(&controller);
    let bar = shaped_overlay(&controller, 36.);
    let root = tree
        .mount(
            bar.clone()
                .shape(CircleBorder::new(BorderSide::solid(Color::BLACK, 2.)))
                .with_child(child.clone()),
        )
        .unwrap();
    settle(&mut tree, viewport, now);
    let list = tree.paint();
    let paths = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::FillPath { path, brush, .. } if *brush == RAW_INK.into() => {
                Some((path.tight_bounds().unwrap(), None))
            }
            PaintCommand::StrokePath {
                path,
                brush,
                stroke,
            } if *brush == Color::BLACK.into() => {
                Some((path.tight_bounds().unwrap(), Some(stroke.width)))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let rect =
        |x, y, diameter| Rect::from_origin_size(Offset::new(x, y), Size::new(diameter, diameter));
    assert_eq!(
        paths,
        vec![
            (rect(764., 162., 36.), None),
            (rect(765., 163., 34.), Some(2.))
        ]
    );

    // Removing only the shape invalidates the retained picture and leaves a
    // regular square thumb, without an old circle or outline behind it.
    tree.update(root, bar.with_child(child)).unwrap();
    assert_eq!(
        settle(&mut tree, viewport, now + Duration::from_secs(1)),
        Some(Rect::from_origin_size(
            Offset::new(764., 0.),
            Size::new(36., 360.)
        ),)
    );
    assert!(!tree.paint().commands().iter().any(|command| matches!(
        command,
        PaintCommand::FillPath { .. } | PaintCommand::StrokePath { .. }
    )));
}

#[test]
fn shaped_scrollbar_independently_rounds_only_the_requested_corner() {
    use incular_widgets::{BorderRadius, BorderSide, Radius, RoundedRectangleBorder};

    let (mut tree, controller, viewport, now) = shape_frame();
    let border = RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius {
            top_left: Radius::circular(8.),
            ..BorderRadius::ZERO
        },
    );
    tree.mount(
        shaped_overlay(&controller, 20.)
            .shape(border)
            .with_child(shape_child(&controller)),
    )
    .unwrap();
    settle(&mut tree, viewport, now);
    let list = tree.paint();
    let (path, rule) = list
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::FillPath {
                path,
                brush,
                fill_rule,
            } if *brush == RAW_INK.into() => Some((path, *fill_rule)),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        path.tight_bounds(),
        Some(Rect::from_origin_size(
            Offset::new(780., 0.),
            Size::new(20., 360.),
        ))
    );
    assert!(!path.contains(Offset::new(781., 1.), rule));
    assert!(path.contains(Offset::new(788., 1.), rule));
    assert!(path.contains(Offset::new(799., 1.), rule));
    assert!(path.contains(Offset::new(781., 359.), rule));
    assert!(!list.commands().iter().any(|command| matches!(
        command,
        PaintCommand::StrokePath { .. } | PaintCommand::Border { .. }
    )));
}

#[cfg(debug_assertions)]
#[test]
fn either_builder_order_guards_a_thumb_shape_and_radius_combination() {
    use incular_widgets::{CircleBorder, Radius};

    for shape_first in [false, true] {
        assert!(
            std::panic::catch_unwind(|| {
                let bar = RawScrollbar::new(ScrollController::new());
                if shape_first {
                    bar.shape(CircleBorder::default())
                        .radius(Radius::circular(4.))
                } else {
                    bar.radius(Radius::circular(4.))
                        .shape(CircleBorder::default())
                }
            })
            .is_err()
        );
    }
}
