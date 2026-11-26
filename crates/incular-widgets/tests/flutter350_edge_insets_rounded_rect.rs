//! Flutter 3.50.0 EdgeInsets RRect geometry and retained paint.

use incular_config::EdgeInsets;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::PaintCommand;
use incular_widgets::{
    Border, BorderRadius, BorderSide, BoxDecoration, EllipticalRoundedRect, Radius,
};

const EDGES: EdgeInsets = EdgeInsets::from_ltrb(2., 3., 4., 5.);

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
    Rect::from_origin_size(
        Offset::new(left, top),
        Size::new(right - left, bottom - top),
    )
}

fn original(r: [f32; 4]) -> EllipticalRoundedRect {
    let [tl, tr, br, bl] = r.map(Radius::circular);
    BorderRadius::only(tl, tr, br, bl).to_rounded_rect(rect(10., 20., 30., 60.))
}

fn corners(value: EllipticalRoundedRect) -> [(f32, f32); 4] {
    let r = value.radii;
    [r.top_left, r.top_right, r.bottom_right, r.bottom_left].map(|p| (p.x, p.y))
}

#[test]
fn enlargement_updates_the_two_axes_of_each_adjacent_corner() {
    let outer = original([5., 6., 7., 8.]).inflate(EDGES).unwrap();
    assert_eq!(outer.rect, rect(8., 17., 34., 65.));
    assert_eq!(
        corners(outer),
        [(7., 8.), (10., 9.), (11., 12.), (10., 13.)]
    );
    assert_eq!(outer.to_path().tight_bounds(), Some(outer.rect));
}

#[test]
fn shrinking_updates_the_two_axes_of_each_adjacent_corner() {
    let inner = original([8., 9., 10., 11.]).deflate(EDGES).unwrap();
    assert_eq!(inner.rect, rect(12., 23., 26., 55.));
    assert_eq!(corners(inner), [(6., 5.), (5., 6.), (6., 5.), (9., 6.)]);
    assert_eq!(inner.to_path().tight_bounds(), Some(inner.rect));
}

#[test]
fn expanding_by_zero_preserves_every_corner() {
    let value = original([5., 0., 0., 0.]);
    assert_eq!(value.inflate(EdgeInsets::ZERO), Some(value));
}

#[test]
fn shrinking_by_zero_preserves_every_corner() {
    let value = original([5., 0., 0., 0.]);
    assert_eq!(value.deflate(EdgeInsets::ZERO), Some(value));
}

#[test]
fn exceeding_the_radius_clamps_each_axis_to_zero() {
    let inner = original([3.; 4]).deflate(EdgeInsets::all(5.)).unwrap();
    assert_eq!(corners(inner), [(0., 0.); 4]);
    let asymmetric = original([1., 8., 10., 9.])
        .deflate(EdgeInsets::from_ltrb(2., 0.5, 3., 4.))
        .unwrap();
    assert_eq!(corners(asymmetric)[0], (0., 0.5));
}

#[test]
fn growing_then_shrinking_or_the_reverse_restores_the_authored_geometry() {
    let value = original([5., 6., 7., 8.]);
    assert_eq!(value.inflate(EDGES).unwrap().deflate(EDGES), Some(value));
    assert_eq!(value.deflate(EDGES).unwrap().inflate(EDGES), Some(value));
}

#[test]
fn inside_border_orders_an_inverted_hole_but_omits_an_exactly_empty_one() {
    let decoration = BoxDecoration::new()
        .border_radius(BorderRadius::all(Radius::elliptical(2., 1.)))
        .border(Border::all(BorderSide::solid(Color::BLACK, 3.)));
    for (width, height, center_is_painted) in [
        (5., 4., false),
        (6., 6., true),
        (6., 7., true),
        (7., 7., false),
    ] {
        let list = decoration.paint_border(Size::new(width, height)).unwrap();
        let painted = |point| {
            list.commands().iter().any(|command| {
            matches!(command, PaintCommand::FillPath { path, fill_rule, .. } if path.contains(point, *fill_rule))
        })
        };
        assert!(painted(Offset::new(0.5, 2.5)), "outside the hole");
        assert_eq!(
            painted(Offset::new(width / 2., height / 2.)),
            center_is_painted
        );
    }
}
