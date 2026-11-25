use incular_core::{Lerp, Offset, Rect, Size};
use incular_widgets::RelativeRect;

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
    Rect::from_origin_size(
        Offset::new(left, top),
        Size {
            width: right - left,
            height: bottom - top,
        },
    )
}

const INSETS: RelativeRect = RelativeRect::from_ltrb(10., 20., 30., 40.);

#[test]
fn to_rect_uses_container_local_coordinates() {
    assert_eq!(
        INSETS.to_rect(rect(10., 20., 90., 180.)),
        rect(10., 20., 50., 120.)
    );
}

#[test]
fn explicit_and_size_relative_rects_compare_equal() {
    assert_eq!(
        INSETS,
        RelativeRect::from_size(rect(10., 20., 10., 20.), Size::new(40., 60.))
    );
}

#[test]
fn shifts_preserve_width_and_height_by_moving_opposite_edges() {
    assert_eq!(
        INSETS.shift(Offset::new(5., 50.)),
        RelativeRect::from_ltrb(15., 70., 25., -10.)
    );
}

#[test]
fn inflate_moves_every_edge_outward() {
    assert_eq!(
        INSETS.inflate(5.),
        RelativeRect::from_ltrb(5., 15., 25., 35.)
    );
}

#[test]
fn deflate_moves_every_edge_inward() {
    assert_eq!(
        INSETS.deflate(5.),
        RelativeRect::from_ltrb(15., 25., 35., 45.)
    );
}

#[test]
fn interpolates_each_inset_independently() {
    assert_eq!(
        RelativeRect::FILL.lerp(&INSETS, 0.5),
        RelativeRect::from_ltrb(5., 10., 15., 20.)
    );
}

#[test]
fn directional_construction_swaps_only_horizontal_insets() {
    use incular_widgets::TextDirection::{Ltr, Rtl};
    assert_eq!(
        RelativeRect::from_directional(10., 20., 30., 40., Ltr),
        INSETS
    );
    assert_eq!(
        RelativeRect::from_directional(10., 20., 30., 40., Rtl),
        RelativeRect::from_ltrb(30., 20., 10., 40.)
    );
}

#[test]
fn intersection_takes_the_innermost_distance_from_each_edge() {
    let other = RelativeRect::from_ltrb(0., 30., 60., 0.);
    assert_eq!(INSETS.intersect(other), other.intersect(INSETS));
    assert_eq!(
        INSETS.intersect(other),
        RelativeRect::from_ltrb(10., 30., 60., 40.)
    );
}

#[test]
fn size_is_container_extent_less_both_insets() {
    assert_eq!(INSETS.to_size(Size::new(80., 160.)), Size::new(40., 100.));
}

#[test]
fn nullable_lerp_preserves_absence_and_equal_values() {
    assert_eq!(RelativeRect::lerp_optional(None, None, 0.), None);
    let fill = RelativeRect::FILL;
    assert_eq!(
        RelativeRect::lerp_optional(Some(fill), Some(fill), 0.5),
        Some(fill)
    );
}

#[test]
fn local_and_global_coordinates_round_trip_distinct_origins() {
    let container = rect(7., 9., 107., 89.);
    let item = rect(1., 20., 57., 99.);
    let relative = RelativeRect::from_rect(item, container);
    assert_eq!(relative, RelativeRect::from_ltrb(-6., 11., 50., -10.));
    assert_eq!(relative.to_rect(container), rect(-6., 11., 50., 90.));
    assert_eq!(relative.to_global_rect(container), item);
    assert_eq!(relative.to_size(container.size), Size::new(56., 79.));
}

#[test]
fn disjoint_intersection_preserves_signed_extents_without_panicking() {
    let near = RelativeRect::from_ltrb(10., 10., 70., 70.);
    let far = RelativeRect::from_ltrb(70., 70., 10., 10.);
    let disjoint = near.intersect(far);
    let container = rect(17., 19., 117., 119.);
    assert_eq!(disjoint, RelativeRect::from_ltrb(70., 70., 70., 70.));
    assert_eq!(disjoint.to_rect(container), rect(70., 70., 30., 30.));
    assert_eq!(
        disjoint.to_size(container.size),
        Size {
            width: -40.,
            height: -40.
        }
    );
}

#[test]
fn optional_interpolation_treats_each_missing_endpoint_as_fill() {
    let rect = RelativeRect::from_ltrb(4., 8., 12., 16.);
    assert_eq!(
        RelativeRect::lerp_optional(None, Some(rect), 0.25),
        Some(RelativeRect::from_ltrb(1., 2., 3., 4.))
    );
    assert_eq!(
        RelativeRect::lerp_optional(Some(rect), None, 0.25),
        Some(RelativeRect::from_ltrb(3., 6., 9., 12.))
    );
}
