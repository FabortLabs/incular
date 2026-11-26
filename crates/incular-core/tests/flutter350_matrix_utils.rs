use std::f64::consts::PI;

use incular_core::{CylindricalOrientation::Vertical, Matrix4, Point64, Rect64};

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-5, "{actual} != {expected}");
}

fn rect_close(actual: Rect64, expected: Rect64) {
    for (actual, expected) in [actual.x0, actual.y0, actual.x1, actual.y1]
        .into_iter()
        .zip([expected.x0, expected.y0, expected.x1, expected.y1])
    {
        close(actual, expected);
    }
}

#[test]
fn huge_finite_edges_do_not_overflow_when_translated() {
    let input = Rect64::new(0., -f64::MAX, 800., f64::MAX);
    let output = Matrix4::translation(10., 0., 0.).transform_rect(input);
    assert!(output.is_finite());
    assert_eq!(output, Rect64::new(10., -f64::MAX, 810., f64::MAX));
}

#[test]
fn multiplication_writes_only_to_the_right_operand() {
    let a = Matrix4::translation(10., 0., 0.);
    let mut b = Matrix4::translation(20., 0., 0.);
    let expected = a.multiply(b);
    a.multiply_in_place(&mut b);
    assert_eq!(a.as_translation(), Some(Point64::new(10., 0.)));
    assert_eq!(b, expected);
    assert_eq!(b.as_translation(), Some(Point64::new(30., 0.)));
    let mut b = Matrix4::translation(3., 4., 0.);
    Matrix4::scale(2., 5., 1.).multiply_in_place(&mut b);
    assert_eq!(b.transform_point(Point64::ORIGIN), Point64::new(6., 20.));
}

#[test]
fn translation_extraction_requires_the_rest_of_the_matrix_to_be_identity() {
    assert_eq!(Matrix4::IDENTITY.as_translation(), Some(Point64::ORIGIN));
    assert_eq!(Matrix4::ZERO.as_translation(), None);
    assert_eq!(Matrix4::rotation_x(1.).as_translation(), None);
    assert_eq!(Matrix4::rotation_z(1.).as_translation(), None);
    assert_eq!(Matrix4::translation(1., 2., 3.).as_translation(), None);
    assert_eq!(
        Matrix4::translation(1., 2., 0.).as_translation(),
        Some(Point64::new(1., 2.))
    );
    assert_eq!(Matrix4::rotation_x(2.).as_translation(), None);
    assert_eq!(Matrix4::scale(2., 2., 2.).as_translation(), None);
    let t = Matrix4::translation(2., -2., 0.);
    assert_eq!(t.as_translation(), Some(Point64::new(2., -2.)));
    assert_eq!(
        t.multiply(Matrix4::translation(4., 8., 0.))
            .as_translation(),
        Some(Point64::new(6., 6.))
    );
}

#[test]
fn zero_radius_angle_and_perspective_make_the_identity() {
    assert_eq!(
        Matrix4::cylindrical_projection_with(0., 0., 0., Vertical),
        Matrix4::IDENTITY
    );
}

#[test]
fn zero_radius_and_perspective_leave_only_the_rotation() {
    assert_eq!(
        Matrix4::cylindrical_projection_with(0., PI / 2., 0., Vertical),
        Matrix4::rotation_x(PI / 2.)
    );
}

#[test]
fn a_large_cylinder_is_still_identity_without_rotation_or_perspective() {
    assert_eq!(
        Matrix4::cylindrical_projection_with(1_000_000., 0., 0., Vertical),
        Matrix4::IDENTITY
    );
}

#[test]
fn default_perspective_and_column_order_match_the_numeric_spot_check() {
    let actual = Matrix4::cylindrical_projection(100., PI / 3.);
    let expected = [
        1.,
        0.,
        0.,
        0.,
        0.,
        0.5,
        0.866_025_403_784_438_6,
        -0.000_866_025_403_784_438_6,
        0.,
        -0.866_025_403_784_438_6,
        0.5,
        -0.0005,
        0.,
        -86.602_540_378_443_86,
        -50.,
        1.05,
    ];
    for (&actual, expected) in actual.columns().iter().zip(expected) {
        close(actual, expected);
    }
}

#[test]
fn forced_points_ignores_the_original_xy_position() {
    let point = Point64::new(20., -30.);
    let forced = Matrix4::force_to_point(point);
    for sample in [
        point,
        Point64::ORIGIN,
        Point64::new(1., 1.),
        Point64::new(-1., -1.),
        Point64::new(-20., 30.),
        Point64::new(-1.2344, 1_422_434.23),
    ] {
        assert_eq!(forced.transform_point(sample), point);
    }
}

#[test]
fn affine_rect_bounds_match_identity_scale_and_rotation() {
    let rect = Rect64::new(10., 20., 30., 40.);
    assert_eq!(Matrix4::IDENTITY.transform_rect(rect), rect);
    assert_eq!(
        Matrix4::scale(2., 2., 2.).transform_rect(rect),
        Rect64::new(20., 40., 60., 80.)
    );
    rect_close(
        Matrix4::rotation_z(PI / 2.).transform_rect(rect),
        Rect64::new(-40., 10., -20., 30.),
    );
}

#[test]
fn perspective_rect_bounds_enclose_the_independently_projected_corners() {
    let (radius, angle, perspective) = (10., PI / 8., 0.3);
    let matrix = Matrix4::cylindrical_projection_with(radius, angle, perspective, Vertical);
    let (sin, cos) = angle.sin_cos();
    let reference = |x: f64, y: f64| {
        let depth = sin * y + radius * cos - radius;
        let w = 1. - perspective * depth;
        Point64::new(x / w, (cos * y - radius * sin) / w)
    };
    for i in 1..10_000 {
        let k = f64::from(i);
        let r = Rect64::new(11. * k, 12. * k, 15. * k, 18. * k);
        let corners = [
            reference(r.x0, r.y0),
            reference(r.x1, r.y0),
            reference(r.x0, r.y1),
            reference(r.x1, r.y1),
        ];
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for p in corners {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
        rect_close(
            matrix.transform_rect(r),
            Rect64::new(min_x, min_y, max_x, max_y),
        );
    }
}

#[test]
fn pure_scale_extraction_rejects_translation_and_depth_scale() {
    assert_eq!(Matrix4::scale(2., 2., 1.).as_scale(), Some(2.));
    assert_eq!(Matrix4::scale(-2., -2., 1.).as_scale(), Some(-2.));
    assert_eq!(Matrix4::scale(2., 2., 2.).as_scale(), None);
    assert_eq!(Matrix4::scale(2., 3., 1.).as_scale(), None);
    assert_eq!(Matrix4::translation(1., 0., 0.).as_scale(), None);
}

#[test]
fn horizontal_cylinder_projects_with_the_vertical_axis_of_rotation() {
    let (radius, angle, perspective) = (70., PI / 6., 0.005);
    let projected = Matrix4::cylindrical_projection_with(
        radius,
        angle,
        perspective,
        incular_core::CylindricalOrientation::Horizontal,
    )
    .transform_point(Point64::new(20., 9.));
    let (sin, cos) = angle.sin_cos();
    let depth = -20. * sin + radius * cos - radius;
    let w = 1. - perspective * depth;
    close(projected.x, (20. * cos + radius * sin) / w);
    close(projected.y, 9. / w);
}
