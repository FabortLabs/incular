//! `PathBuilder::conic_to` and `PathBuilder::rounded_superellipse`, checked
//! against geometry derived by hand from Flutter's definitions.

use incular_core::{Offset, Rect, Size};
use incular_rendering::{CornerRadii, FillRule, Path};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn assert_bounds(path: &Path, expected: Rect) {
    assert_bounds_within(path, expected, 1e-3);
}

fn assert_bounds_within(path: &Path, expected: Rect, tolerance: f32) {
    let bounds = path.bounds().expect("non-empty path");
    for (actual, wanted) in [
        (bounds.origin.x, expected.origin.x),
        (bounds.origin.y, expected.origin.y),
        (bounds.size.width, expected.size.width),
        (bounds.size.height, expected.size.height),
    ] {
        assert!(
            (actual - wanted).abs() < tolerance,
            "{bounds:?} != {expected:?}"
        );
    }
}

#[test]
fn conic_quarter_circle() {
    // From (0, 0) through control (1, 0) to (1, 1) with weight cos(45°) is
    // the quarter circle centered at (0, 1); its midpoint is
    // (sin 45°, 1 - cos 45°) = (0.7071, 0.2929). Closed by the chord, the
    // region spans y in (0.2929, 0.7071) at x = 0.7071.
    let mut builder = Path::builder();
    builder
        .move_to(Offset::ZERO)
        .conic_to(
            Offset::new(1., 0.),
            Offset::new(1., 1.),
            std::f32::consts::FRAC_1_SQRT_2,
        )
        .close();
    let path = builder.build();
    assert!(path.contains(Offset::new(0.7071, 0.30), FillRule::NonZero));
    assert!(!path.contains(Offset::new(0.7071, 0.285), FillRule::NonZero));
    assert_bounds(&path, rect(0., 0., 1., 1.));
}

#[test]
fn rounded_superellipse_meets_the_diagonal_where_the_rounded_rect_does() {
    // The corner's circular arc ends at M = (a - g, a - g) from the quadrant
    // center, g = (1 - cos 45°)·r: for a 100 px square with radius 20 the
    // top-left crossing is at 50 - (50 - 5.8579) = 5.8579.
    let mut builder = Path::builder();
    builder.rounded_superellipse(rect(0., 0., 100., 100.), CornerRadii::uniform(20.));
    let path = builder.build();
    // The superellipse arcs are Flutter's fitted cubics, which bulge past
    // the edge by about 0.0024 px.
    assert_bounds_within(&path, rect(0., 0., 100., 100.), 0.01);
    assert!(path.contains(Offset::new(50., 50.), FillRule::NonZero));
    assert!(path.contains(Offset::new(5.9, 5.9), FillRule::NonZero));
    assert!(!path.contains(Offset::new(5.8, 5.8), FillRule::NonZero));
    assert!(path.contains(Offset::new(0.5, 50.), FillRule::NonZero));
}

#[test]
fn rounded_superellipse_with_square_corners_is_the_rectangle() {
    let mut builder = Path::builder();
    builder.rounded_superellipse(rect(10., 20., 30., 40.), CornerRadii::ZERO);
    let path = builder.build();
    assert_bounds(&path, rect(10., 20., 30., 40.));
    assert!(path.contains(Offset::new(10.5, 20.5), FillRule::NonZero));
}
