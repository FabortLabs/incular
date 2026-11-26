//! Value and signed-edge geometry from Flutter 3.50.0 painting/edge_insets_test.dart.

use incular_config::{Axis, EdgeInsets, EdgeInsetsDirectional, TextDirection};
use incular_core::{Offset, Rect, Size};

#[test]
fn physical_constructors_resolve_identically_in_both_directions() {
    let ltrb = EdgeInsets::from_ltrb;
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        for (value, expected) in [
            (ltrb(10., 20., 30., 40.), ltrb(10., 20., 30., 40.)),
            (EdgeInsets::all(10.), ltrb(10., 10., 10., 10.)),
            (
                EdgeInsets::only(10., 20., 30., 40.),
                ltrb(10., 20., 30., 40.),
            ),
            (EdgeInsets::symmetric(10., 20.), ltrb(10., 20., 10., 20.)),
        ] {
            assert_eq!(value.resolve(direction), expected);
        }
    }
}

#[test]
fn physical_lerp_treats_a_single_absent_endpoint_as_zero() {
    let a = EdgeInsets::all(10.);
    let b = EdgeInsets::all(20.);
    let lerp = EdgeInsets::lerp_optional;
    assert_eq!(lerp(Some(a), Some(b), 0.25), Some(a * 1.25));
    assert_eq!(lerp(Some(a), Some(b), 0.25), Some(b * 0.625));
    assert_eq!(lerp(Some(a), Some(b), 0.25), Some(a + EdgeInsets::all(2.5)));
    assert_eq!(lerp(Some(a), Some(b), 0.25), Some(b - EdgeInsets::all(7.5)));
    assert_eq!(lerp(None, None, 0.25), None);
    assert_eq!(lerp(None, Some(b), 0.25), Some(b * 0.25));
    assert_eq!(lerp(Some(a), None, 0.25), Some(a * 0.75));
    assert_eq!(lerp(Some(a), Some(b), 1.5), Some(EdgeInsets::all(25.)));
}

#[test]
fn copies_change_only_the_named_physical_or_directional_edges() {
    let physical = EdgeInsets::from_ltrb(1., 2., 4., 3.);
    let directional = EdgeInsetsDirectional::from_steb(1., 2., 4., 3.);
    let copied = physical.copy_with(Some(5.), Some(6.), None, None);
    assert_eq!(copied, EdgeInsets::from_ltrb(5., 6., 4., 3.));
    assert_eq!(physical, EdgeInsets::from_ltrb(1., 2., 4., 3.));
    let copied = directional.copy_with(Some(5.), Some(6.), None, None);
    assert_eq!(copied, EdgeInsetsDirectional::from_steb(5., 6., 4., 3.));
    assert_eq!(
        copied.resolve(TextDirection::Rtl),
        EdgeInsets::from_ltrb(4., 6., 5., 3.)
    );
    assert_eq!(
        directional,
        EdgeInsetsDirectional::from_steb(1., 2., 4., 3.)
    );
}

#[test]
fn physical_operators_apply_independently_to_every_edge() {
    let new = EdgeInsets::from_ltrb;
    let a = new(1., 2., 3., 5.);
    assert_eq!(a * 2., new(2., 4., 6., 10.));
    assert_eq!(a / 2., new(0.5, 1., 1.5, 2.5));
    assert_eq!(a % 2., new(1., 0., 1., 1.));
    assert_eq!(a.truncating_div(2.), new(0., 1., 1., 2.));
    assert_eq!(a + a, a * 2.);
    assert_eq!(a - a, EdgeInsets::ZERO);
    assert_eq!((-a) % 2., new(1., 0., 1., 1.));
    assert_eq!((-a).truncating_div(2.), new(0., -1., -1., -2.));
}

#[test]
fn directional_operators_preserve_start_and_end_until_resolution() {
    let new = EdgeInsetsDirectional::from_steb;
    let a = new(1., 2., 3., 5.);
    assert_eq!(a * 2., new(2., 4., 6., 10.));
    assert_eq!(a / 2., new(0.5, 1., 1.5, 2.5));
    assert_eq!(a % 2., new(1., 0., 1., 1.));
    assert_eq!(a.truncating_div(2.), new(0., 1., 1., 2.));
    assert_eq!(a + a, a * 2.);
    assert_eq!(a - a, EdgeInsetsDirectional::ZERO);
    assert_eq!((-a) % 2., new(1., 0., 1., 1.));
    assert_eq!((-a).truncating_div(2.), new(0., -1., -1., -2.));
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
    Rect::from_origin_size(
        Offset::new(left, top),
        Size::new(right - left, bottom - top),
    )
}

#[test]
fn physical_control_corners_dimensions_and_rectangle_edges() {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
    let hash = |value| BuildHasherDefault::<DefaultHasher>::default().hash_one(value);
    let insets = EdgeInsets::from_ltrb(5., 7., 11., 13.);
    assert_eq!(insets.to_string(), "EdgeInsets(5.0, 7.0, 11.0, 13.0)");
    assert_eq!(hash(insets), hash(EdgeInsets::only(5., 7., 11., 13.)));
    assert_eq!(hash(EdgeInsets::all(-0.)), hash(EdgeInsets::ZERO));
    assert_eq!(insets.top_left(), Offset::new(5., 7.));
    assert_eq!(insets.top_right(), Offset::new(-11., 7.));
    assert_eq!(insets.bottom_left(), Offset::new(5., -13.));
    assert_eq!(insets.bottom_right(), Offset::new(-11., -13.));
    assert_eq!(insets.collapsed_size(), Some(Size::new(16., 20.)));
    assert_eq!(insets.flipped(), EdgeInsets::from_ltrb(11., 13., 5., 7.));
    assert_eq!(insets.along(Axis::Horizontal), 16.);
    assert_eq!(insets.along(Axis::Vertical), 20.);
    let outer = rect(23., 32., 124., 143.);
    assert_eq!(insets.inflate_rect(outer), Some(rect(18., 25., 135., 156.)));
    assert_eq!(insets.deflate_rect(outer), Some(rect(28., 39., 113., 130.)));
    assert_eq!(
        insets.inflate_size(Size::new(100., 125.)),
        Some(Size::new(116., 145.))
    );
    assert_eq!(
        insets.deflate_size(Size::new(100., 125.)),
        Some(Size::new(84., 105.))
    );
    assert_eq!(insets / 2., EdgeInsets::from_ltrb(2.5, 3.5, 5.5, 6.5));
    assert_eq!(
        insets.truncating_div(2.),
        EdgeInsets::from_ltrb(2., 3., 5., 6.)
    );
    assert_eq!(insets % 5., EdgeInsets::from_ltrb(0., 2., 1., 3.));
    assert_eq!(insets.deflate_rect(rect(0., 0., 5., 5.)), None);
    assert_eq!(insets.deflate_size(Size::new(10., 10.)), None);
}
