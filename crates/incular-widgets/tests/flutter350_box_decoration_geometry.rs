use incular_core::{Offset, Rect, Size};
use incular_rendering::FillRule;
use incular_widgets::{BorderRadius, BoxDecoration, BoxShape, Radius};

const SIZE: Size = Size {
    width: 100.0,
    height: 20.0,
};

fn assert_sample_points(
    predicate: impl Fn(Offset) -> bool,
    includes: [(f32, f32); 2],
    excludes: [(f32, f32); 2],
) {
    for (x, y) in includes {
        assert!(predicate(Offset::new(x, y)), "expected ({x}, {y}) inside");
    }
    for (x, y) in excludes {
        assert!(!predicate(Offset::new(x, y)), "expected ({x}, {y}) outside");
    }
}

#[test]
fn clip_path_with_border_radius_excludes_the_corners() {
    let decorated = BoxDecoration::new().border_radius(BorderRadius::all(Radius::circular(10.0)));
    let path = decorated.clip_path(Rect::from_origin_size(Offset::ZERO, SIZE));
    assert_sample_points(
        |point| path.contains(point, FillRule::NonZero),
        [(30.0, 10.0), (50.0, 10.0)],
        [(1.0, 1.0), (99.0, 19.0)],
    );
}

#[test]
fn circle_clip_path_uses_the_shortest_side_and_the_box_center() {
    let path = BoxDecoration::new()
        .shape(BoxShape::Circle)
        .clip_path(Rect::from_origin_size(Offset::ZERO, SIZE));
    assert_sample_points(
        |point| path.contains(point, FillRule::NonZero),
        [(50.0, 0.0), (40.0, 10.0)],
        [(40.0, 0.0), (10.0, 10.0)],
    );
}

#[test]
fn circle_hit_test_includes_the_left_and_top_tangencies() {
    let decorated = BoxDecoration::new().shape(BoxShape::Circle);
    assert_sample_points(
        |point| decorated.hit_test(SIZE, point),
        [(50.0, 0.0), (40.0, 10.0)],
        [(40.0, 0.0), (10.0, 10.0)],
    );
    assert!(!decorated.hit_test(SIZE, Offset::new(50.0, 20.0)));
}

#[test]
fn elliptical_corners_hit_the_same_interior_as_a_translated_clip() {
    let decorated =
        BoxDecoration::new().border_radius(BorderRadius::all(Radius::elliptical(45.0, 30.0)));
    let origin = Offset::new(12.0, -23.0);
    let path = decorated.clip_path(Rect::from_origin_size(origin, SIZE));
    for (x, y, expected) in [
        (2.0, 2.0, false),
        (8.0, 8.0, true),
        (15.0, 1.0, true),
        (50.0, 10.0, true),
        (99.0, 19.0, false),
    ] {
        let point = Offset::new(x, y);
        assert_eq!(decorated.hit_test(SIZE, point), expected);
        assert_eq!(path.contains(point + origin, FillRule::NonZero), expected);
    }
}
