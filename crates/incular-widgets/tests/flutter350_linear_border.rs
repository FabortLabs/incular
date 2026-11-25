//! Flutter 3.50 painting/linear_border_test.dart defaults and actual painted edges.

use incular_config::{EdgeInsets, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, FillRule, PaintCommand};
use incular_widgets::{BorderSide, LinearBorder, LinearBorderEdge, ShapeBorder};

const SIDE_COLOR: Color = Color::rgba(0, 255, 0, 15);
const SIDE: BorderSide = BorderSide::solid(SIDE_COLOR, 4.);

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(w, h))
}

#[test]
fn default_border_edge_has_full_size_and_center_alignment() {
    let edge = LinearBorderEdge::default();
    assert_eq!((edge.size, edge.alignment), (1., 0.));
}

#[test]
fn omitted_or_default_ink_never_paints_a_border() {
    let border = LinearBorder::default();
    assert_eq!(border, LinearBorder::NONE);
    assert_eq!(border.side, BorderSide::NONE);
    assert_eq!(border.dimensions(None), EdgeInsets::ZERO);
    assert!(!border.prefer_paint_interior());
    assert_eq!(
        [border.start, border.end, border.top, border.bottom],
        [None; 4]
    );

    let edge = LinearBorderEdge::default();
    for (i, border) in [
        LinearBorder::start(BorderSide::NONE, edge),
        LinearBorder::end(BorderSide::NONE, edge),
        LinearBorder::top(BorderSide::NONE, edge),
        LinearBorder::bottom(BorderSide::NONE, edge),
    ]
    .into_iter()
    .enumerate()
    {
        let mut expected = [None; 4];
        expected[i] = Some(edge);
        assert_eq!(
            [border.start, border.end, border.top, border.bottom],
            expected
        );
        assert_eq!(border.side, BorderSide::NONE);
        let mut canvas = Canvas::default();
        border.paint(&mut canvas, rect(0., 0., 100., 100.));
        assert!(canvas.finish().commands().is_empty());
    }
}

fn sample_rect(rect: Rect) -> [Offset; 5] {
    let (x, y, w, h) = (
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    );
    [
        Offset::new(x, y),
        Offset::new(x + w, y),
        Offset::new(x, y + h),
        Offset::new(x + w, y + h),
        Offset::new(x + w * 0.5, y + h * 0.5),
    ]
}

fn assert_painted_edge(border: LinearBorder, dir: TextDirection, include: Rect, exclude: Rect) {
    let mut canvas = Canvas::default();
    border.paint_with_direction(&mut canvas, rect(0., 0., 100., 100.), Some(dir));
    let list = canvas.finish();
    let [
        PaintCommand::FillPath {
            path,
            brush: Brush::Solid(color),
            fill_rule,
            ..
        },
    ] = list.commands()
    else {
        panic!(
            "expected exactly one solid filled path: {:?}",
            list.commands()
        );
    };
    assert_eq!(*color, SIDE_COLOR);
    assert_eq!(*fill_rule, FillRule::NonZero);
    assert_eq!(path.tight_bounds(), Some(include));
    for point in sample_rect(include) {
        assert!(
            path.contains(point, *fill_rule),
            "{dir:?}: did not include {point:?}"
        );
    }
    for point in sample_rect(exclude) {
        assert!(
            !path.contains(point, *fill_rule),
            "{dir:?}: unexpectedly includes {point:?}"
        );
    }
}

#[test]
fn logical_start_and_end_axes_resolve_in_both_directions() {
    let left = rect(0., 0., 4., 100.);
    let right = rect(96., 0., 4., 100.);
    let start = LinearBorder::start(SIDE, LinearBorderEdge::default());
    let end = LinearBorder::end(SIDE, LinearBorderEdge::default());
    assert_painted_edge(start, TextDirection::Ltr, left, right);
    assert_painted_edge(start, TextDirection::Rtl, right, left);
    assert_painted_edge(end, TextDirection::Ltr, right, left);
    assert_painted_edge(end, TextDirection::Rtl, left, right);
}

#[test]
fn top_and_bottom_stay_physical_in_both_directions() {
    let top = rect(0., 0., 100., 4.);
    let bottom = rect(0., 96., 100., 4.);
    let a = LinearBorder::top(SIDE, LinearBorderEdge::default());
    let b = LinearBorder::bottom(SIDE, LinearBorderEdge::default());
    for dir in [TextDirection::Ltr, TextDirection::Rtl] {
        assert_painted_edge(a, dir, top, bottom);
        assert_painted_edge(b, dir, bottom, top);
    }
}

#[test]
fn copied_border_has_the_same_value_and_hash_unless_a_field_changes() {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let hash = |border: LinearBorder| {
        let mut hasher = DefaultHasher::new();
        border.hash(&mut hasher);
        hasher.finish()
    };
    let original = LinearBorder::NONE;
    let copy = original.copy_with(None, None, None, None, None);
    assert_eq!(original, copy);
    assert_eq!(hash(original), hash(copy));
    let side = BorderSide::solid(Color::rgba(0x12, 0x34, 0x56, 0xff), 10.);
    assert_eq!(
        original.copy_with(Some(side), None, None, None, None),
        LinearBorder {
            side,
            ..LinearBorder::NONE
        },
    );
    let a = LinearBorder::start(SIDE, LinearBorderEdge::new(0.5, 0.0));
    let b = LinearBorder::start(SIDE, LinearBorderEdge::new(0.5, -0.0));
    assert_eq!(a, b);
    assert_eq!(hash(a), hash(b));
}

#[test]
fn nullable_interpolation_distinguishes_missing_shapes_and_missing_edges() {
    let edge = LinearBorderEdge::new(0.5, -0.25);
    let border = LinearBorder::start(SIDE, edge);
    assert_eq!(LinearBorder::lerp_optional(None, None, 0.), None);
    assert_eq!(
        LinearBorder::lerp_optional(Some(border), Some(border), 0.5),
        Some(border)
    );
    assert_eq!(LinearBorderEdge::lerp(None, None, 0.), None);
    assert_eq!(
        LinearBorderEdge::lerp(Some(edge), Some(edge), 0.5),
        Some(edge)
    );
    assert_eq!(
        LinearBorderEdge::lerp(None, Some(edge), 0.5),
        Some(LinearBorderEdge::new(0.25, -0.25))
    );
    assert_eq!(
        border.scale(0.5),
        LinearBorder {
            side: BorderSide::solid(SIDE_COLOR, 2.),
            ..LinearBorder::NONE
        }
    );
    assert_eq!(
        LinearBorder::lerp_optional(None, Some(border), 0.5),
        Some(border.scale(0.5))
    );
    let mut canvas = Canvas::default();
    border
        .scale(0.5)
        .paint(&mut canvas, rect(0., 0., 100., 100.));
    assert!(canvas.finish().commands().is_empty());
}

#[test]
fn selected_solid_zero_width_edge_stays_a_hairline() {
    let mut canvas = Canvas::default();
    let side = BorderSide::solid(SIDE_COLOR, 0.);
    LinearBorder::top(side, LinearBorderEdge::default())
        .paint(&mut canvas, rect(0., 0., 100., 100.));
    assert!(
        matches!(canvas.finish().commands(), [PaintCommand::StrokePath { stroke, .. }] if stroke.width == 0.)
    );
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "linear border size must be in 0..=1")]
fn an_invalid_edge_size_is_not_silently_painted() {
    let _ = LinearBorderEdge::new(-0.1, 0.);
}
