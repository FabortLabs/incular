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

/// Flutter's pinned matrix is checked against explicit reference outcomes as
/// well as the corresponding Incular path, so a missing morph cannot silently
/// pass by doing the same incorrect containment twice.
#[test]
fn all_fourteen_reference_primitive_shapes_match_the_eight_hit_points() {
    use incular_core::Lerp;
    use incular_rendering::FillRule;
    use incular_widgets::{BorderRadius, Decoration, RoundedSuperellipseBorder, ShapeDecoration};

    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(120., 80.));
    let samples = [
        (-10., 40.),
        (1., 1.),
        (0.5, 14.5),
        (1., 35.),
        (1., 68.),
        (11., 35.),
        (14.5, 13.5),
        (21., 35.),
    ];
    let circle = CircleBorder::default();
    let stadium = StadiumBorder::default();
    let radii = BorderRadius::circular(18.);
    let rounded = RoundedRectangleBorder::new(BorderSide::NONE, radii);
    let superellipse = RoundedSuperellipseBorder::new(BorderSide::NONE, radii);
    let morph = |a: ShapeDecoration, b: ShapeDecoration| {
        let Decoration::Shape(result) = Decoration::from(a).lerp(&Decoration::from(b), 0.5) else {
            unreachable!()
        };
        result
    };
    let cases = [
        (
            ShapeDecoration::new(Border::new(1., Color::BLACK)),
            "01111111",
        ),
        (ShapeDecoration::new(circle), "00000001"),
        (
            ShapeDecoration::new(circle.with_eccentricity(0.5)),
            "00000101",
        ),
        (ShapeDecoration::new(OvalBorder::default()), "00010101"),
        (
            ShapeDecoration::new(RoundedRectangleBorder::default()),
            "01111111",
        ),
        (ShapeDecoration::new(rounded), "00110111"),
        (
            ShapeDecoration::new(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusDirectional::only(
                    Radius::circular(18.),
                    Radius::ZERO,
                    Radius::ZERO,
                    Radius::ZERO,
                ),
            )),
            "00111111",
        ),
        (
            ShapeDecoration::new(RoundedSuperellipseBorder::default()),
            "01111111",
        ),
        (ShapeDecoration::new(superellipse), "00010111"),
        (ShapeDecoration::new(stadium), "00010111"),
        (
            morph(ShapeDecoration::new(circle), ShapeDecoration::new(rounded)),
            "00000111",
        ),
        (
            morph(
                ShapeDecoration::new(circle),
                ShapeDecoration::new(superellipse),
            ),
            "00000101",
        ),
        (
            morph(ShapeDecoration::new(stadium), ShapeDecoration::new(circle)),
            "00000101",
        ),
        (
            morph(ShapeDecoration::new(stadium), ShapeDecoration::new(rounded)),
            "00010111",
        ),
    ];
    let dir = Some(TextDirection::Ltr);
    for (index, (decoration, expected)) in cases.into_iter().enumerate() {
        let path = decoration.get_clip_path(rect, dir);
        for ((x, y), expected) in samples.into_iter().zip(expected.bytes()) {
            let point = Offset::new(x, y);
            let inside = expected == b'1';
            assert_eq!(
                decoration.shape.hit_test(rect, point, dir),
                inside,
                "fast case {index} at {point:?}"
            );
            assert_eq!(
                path.contains(point, FillRule::NonZero),
                inside,
                "path case {index} at {point:?}"
            );
        }
    }
}

#[test]
fn stadium_to_rounded_rectangle_hits_a_distinct_midpoint() {
    use incular_core::Lerp;
    use incular_rendering::FillRule;
    use incular_widgets::{BorderRadius, Decoration, ShapeDecoration};

    let stadium = ShapeDecoration::new(StadiumBorder::default());
    let rounded = ShapeDecoration::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::circular(18.),
    ));
    let from = Decoration::from(stadium.clone());
    let to = Decoration::from(rounded.clone());
    let size = Size::new(120., 80.);
    let inside = Offset::new(12., 8.);
    let outside = Offset::new(10., 5.);
    assert!(!stadium.hit_test(size, inside, None));
    assert!(rounded.hit_test(size, outside, None));
    for direction in [from.lerp(&to, 0.5), to.lerp(&from, 0.5)] {
        let Decoration::Shape(midpoint) = direction else {
            unreachable!()
        };
        assert!(midpoint.hit_test(size, inside, None));
        assert!(!midpoint.hit_test(size, outside, None));
        let clip = midpoint.get_clip_path(Rect::from_origin_size(Offset::ZERO, size), None);
        assert!(clip.contains(inside, FillRule::NonZero));
        assert!(!clip.contains(outside, FillRule::NonZero));
    }
}
