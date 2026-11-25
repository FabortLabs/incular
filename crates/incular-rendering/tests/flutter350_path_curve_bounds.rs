//! Paired with the pinned Flutter 3.50 Path.getBounds probe; no GPU required.

use incular_core::{Offset, Rect, Size, Transform};
use incular_rendering::{FillRule, Path};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

#[test]
fn flutter_350_path_bounds_include_bezier_control_points() {
    let mut rectangle = Path::builder();
    rectangle
        .move_to(Offset::new(10., 10.))
        .line_to(Offset::new(50., 10.))
        .line_to(Offset::new(50., 40.))
        .line_to(Offset::new(10., 40.))
        .close();
    let rectangle = rectangle.build();
    assert_eq!(rectangle.bounds(), Some(rect(10., 10., 40., 30.)));
    assert_eq!(
        rectangle
            .transformed(Transform::translation(Offset::new(2., 5.)))
            .bounds(),
        Some(rect(12., 15., 40., 30.)),
    );

    let mut quadratic = Path::builder();
    quadratic
        .move_to(Offset::new(10., 10.))
        .quadratic_to(Offset::new(30., 50.), Offset::new(50., 10.))
        .close();
    let mut cubic = Path::builder();
    cubic
        .move_to(Offset::new(10., 10.))
        .cubic_to(
            Offset::new(20., 70.),
            Offset::new(40., 70.),
            Offset::new(50., 10.),
        )
        .close();
    let quadratic = quadratic.build();
    let cubic = cubic.build();
    for path in [&quadratic, &cubic] {
        assert!(path.contains(Offset::new(30., 20.), FillRule::NonZero));
        assert!(!path.contains(Offset::new(30., 60.), FillRule::NonZero));
    }
    assert_eq!(
        [quadratic.bounds(), cubic.bounds()],
        [
            Some(rect(10., 10., 40., 40.)),
            Some(rect(10., 10., 40., 60.))
        ],
        "Flutter reports control-point-enclosing bounds, not tight curve bounds"
    );
    assert_eq!(
        [quadratic.tight_bounds(), cubic.tight_bounds()],
        [
            Some(rect(10., 10., 40., 20.)),
            Some(rect(10., 10., 40., 45.))
        ],
        "tight culling bounds retain the actual curve extrema"
    );
    let same_curve = quadratic.transformed(Transform::IDENTITY);
    assert_eq!(same_curve.id(), quadratic.id());
    assert_eq!(same_curve.bounds(), quadratic.bounds());
}
