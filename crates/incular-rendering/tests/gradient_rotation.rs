//! `Brush::rotated` is Flutter's `GradientRotation` (`gradient.dart`):
//! the field rotates clockwise (y-down) about the centre of the shader
//! bounds, so the value at a rotated point equals the original value at
//! the point before rotation.
use std::f32::consts::FRAC_PI_2;

use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{
    Brush, GradientStop, GradientStops, LinearGradient, RadialGradient, SweepGradient,
    sweep_gradient_t, two_point_conical_gradient_t,
};

fn stops() -> GradientStops {
    GradientStops::new(vec![
        GradientStop {
            offset: 0.,
            color: Color::BLACK,
        },
        GradientStop {
            offset: 1.,
            color: Color::WHITE,
        },
    ])
}

fn bounds() -> Rect {
    Rect::from_origin_size(Offset::new(10., 20.), Size::new(100., 60.))
}

/// `point` rotated by a quarter turn clockwise about the bounds centre.
fn quarter_turn(point: Offset) -> Offset {
    let center = Offset::new(60., 50.);
    Offset::new(
        center.x - (point.y - center.y),
        center.y + (point.x - center.x),
    )
}

fn close(a: Offset, b: Offset) -> bool {
    (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3
}

#[test]
fn linear_rotation_turns_endpoints_about_the_bounds_centre() {
    let brush = Brush::LinearGradient(LinearGradient::new(
        Offset::new(10., 50.),
        Offset::new(110., 50.),
        stops(),
    ));
    let Brush::LinearGradient(rotated) = brush.rotated(FRAC_PI_2, bounds()) else {
        panic!("rotation keeps the brush kind");
    };
    // A left-to-right gradient turned a quarter clockwise runs top to bottom.
    assert!(close(rotated.start, Offset::new(60., 0.)));
    assert!(close(rotated.end, Offset::new(60., 100.)));
    assert!(close(rotated.start, quarter_turn(Offset::new(10., 50.))));
}

#[test]
fn radial_rotation_moves_centre_and_focal_and_keeps_values() {
    let mut gradient = RadialGradient::new(Offset::new(30., 40.), 25., stops());
    gradient.focal = Some(Offset::new(35., 45.));
    gradient.focal_radius = 2.;
    let Brush::RadialGradient(rotated) =
        Brush::RadialGradient(gradient.clone()).rotated(FRAC_PI_2, bounds())
    else {
        panic!("rotation keeps the brush kind");
    };
    assert!(close(rotated.center, quarter_turn(gradient.center)));
    assert!(close(
        rotated.focal.expect("focal kept"),
        quarter_turn(Offset::new(35., 45.))
    ));
    assert_eq!(rotated.radius, gradient.radius);
    assert_eq!(rotated.focal_radius, gradient.focal_radius);
    let point = Offset::new(42., 37.);
    let before = two_point_conical_gradient_t(
        gradient.focal.unwrap(),
        gradient.focal_radius,
        gradient.center,
        gradient.radius,
        point,
    )
    .unwrap();
    let after = two_point_conical_gradient_t(
        rotated.focal.unwrap(),
        rotated.focal_radius,
        rotated.center,
        rotated.radius,
        quarter_turn(point),
    )
    .unwrap();
    assert!((before - after).abs() < 1e-4);
}

#[test]
fn sweep_rotation_advances_the_start_angle() {
    let gradient = SweepGradient::new(Offset::new(40., 30.), 0.3, stops());
    let Brush::SweepGradient(rotated) =
        Brush::SweepGradient(gradient.clone()).rotated(FRAC_PI_2, bounds())
    else {
        panic!("rotation keeps the brush kind");
    };
    assert!((rotated.start_angle - (0.3 + FRAC_PI_2)).abs() < 1e-6);
    for point in [Offset::new(70., 10.), Offset::new(5., 90.)] {
        let before = sweep_gradient_t(gradient.center, gradient.start_angle, point);
        let after = sweep_gradient_t(rotated.center, rotated.start_angle, quarter_turn(point));
        let diff = (before - after).abs();
        assert!(diff < 1e-4 || (1. - diff) < 1e-4, "{before} vs {after}");
    }
}

#[test]
fn solid_and_zero_rotations_are_unchanged() {
    let solid = Brush::Solid(Color::WHITE);
    assert_eq!(solid.rotated(1., bounds()), solid);
    let linear = Brush::LinearGradient(LinearGradient::new(
        Offset::ZERO,
        Offset::new(1., 0.),
        stops(),
    ));
    assert_eq!(linear.rotated(0., bounds()), linear);
}
