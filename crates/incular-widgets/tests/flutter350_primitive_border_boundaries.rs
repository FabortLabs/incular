use incular_config::TextDirection;
use incular_core::{Color, Offset, Rect, Size};
use incular_widgets::{
    Border, BorderRadiusDirectional, BorderSide, CircleBorder, OvalBorder, Radius,
    RoundedRectangleBorder, ShapeBorder, StadiumBorder,
};

fn frame() -> Rect {
    Rect::from_origin_size(Offset::new(10., 20.), Size::new(120., 80.))
}

#[test]
fn box_and_stadium_use_half_open_bounds() {
    let borders: [&dyn ShapeBorder; 2] =
        [&Border::new(1., Color::BLACK), &StadiumBorder::default()];
    for border in borders {
        for (x, y, expected) in [
            (10., 60., true),
            (70., 20., true),
            (130., 60., false),
            (70., 100., false),
            (70., 99.999, true),
        ] {
            assert_eq!(
                border.hit_test(frame(), Offset::new(x, y), None),
                expected,
                "({x}, {y})"
            );
        }
        assert!(!border.hit_test(
            Rect::from_origin_size(Offset::ZERO, Size::ZERO),
            Offset::ZERO,
            None
        ));
    }
    assert!(!StadiumBorder::default().hit_test(frame(), Offset::new(11., 21.), None));
}

#[test]
fn circle_hit_tests_use_the_eccentric_fitted_ellipse() {
    let circle = CircleBorder::default();
    let halfway = circle.with_eccentricity(0.5);
    let oval = OvalBorder::default();
    let shapes: [(&dyn ShapeBorder, f32, f32); 3] = [
        (&circle, 30., 110.),
        (&halfway, 20., 120.),
        (&oval, 10., 130.),
    ];
    for (border, left, right) in shapes {
        for (x, expected) in [
            (left, true),
            (left - 0.001, false),
            (right, false),
            (right - 0.001, true),
        ] {
            assert_eq!(
                border.hit_test(frame(), Offset::new(x, 60.), None),
                expected,
                "({x}, 60)"
            );
        }
    }
    assert!(oval.hit_test(frame(), Offset::new(106., 92.), None));
    assert!(!oval.hit_test(frame(), Offset::new(106.001, 92.), None));
}

#[test]
fn elliptical_corners_resolve_direction_and_scale_axes_together() {
    let rounded = RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadiusDirectional::only(
            Radius::elliptical(60., 40.),
            Radius::elliptical(90., 8.),
            Radius::ZERO,
            Radius::ZERO,
        ),
    );
    for (direction, start, inward) in [
        (TextDirection::Ltr, 10., 1.),
        (TextDirection::Rtl, 130., -1.),
    ] {
        for (x, y, expected) in [
            (start + 8. * inward, 38., true),
            (start + 8. * inward, 33., false),
            (70., 100., false),
        ] {
            assert_eq!(
                rounded.hit_test(frame(), Offset::new(x, y), Some(direction)),
                expected,
                "{direction:?} ({x}, {y})"
            );
        }
    }
}
