//! Flutter gradient `tileMode` and the focal (two-point conical) radial
//! gradient. Expected parameters are derived by hand from Skia's definitions.

use incular_core::{Color, Offset};
use incular_rendering::{
    GradientStop, GradientStops, LinearGradient, RadialGradient, SweepGradient, TileMode,
    two_point_conical_gradient_t,
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

#[test]
fn gradients_clamp_by_default() {
    assert_eq!(TileMode::default(), TileMode::Clamp);
    assert_eq!(
        LinearGradient::new(Offset::ZERO, Offset::new(1., 0.), stops()).tile_mode,
        TileMode::Clamp
    );
    let radial = RadialGradient::new(Offset::ZERO, 10., stops());
    assert_eq!(radial.tile_mode, TileMode::Clamp);
    assert_eq!(radial.focal, None);
    assert_eq!(radial.focal_radius, 0.);
    assert!(!radial.is_focal());
    assert_eq!(
        SweepGradient::new(Offset::ZERO, 0., stops()).tile_mode,
        TileMode::Clamp
    );
}

#[test]
fn tile_modes_map_parameters_outside_the_unit_range() {
    assert_eq!(TileMode::Clamp.apply(1.25), Some(1.));
    assert_eq!(TileMode::Clamp.apply(-0.5), Some(0.));
    assert_eq!(TileMode::Repeated.apply(1.25), Some(0.25));
    assert_eq!(TileMode::Repeated.apply(-0.25), Some(0.75));
    // Mirror reflects every other period: 1.25 -> 2 - 1.25, -0.25 -> 0.25.
    assert_eq!(TileMode::Mirror.apply(1.25), Some(0.75));
    assert_eq!(TileMode::Mirror.apply(-0.25), Some(0.25));
    assert_eq!(TileMode::Mirror.apply(0.5), Some(0.5));
    assert_eq!(TileMode::Decal.apply(1.25), None);
    assert_eq!(TileMode::Decal.apply(-0.01), None);
    assert_eq!(TileMode::Decal.apply(0.5), Some(0.5));
}

#[test]
fn a_focal_radius_or_off_center_focal_makes_a_two_point_gradient() {
    let mut radial = RadialGradient::new(Offset::ZERO, 10., stops());
    radial.focal = Some(Offset::ZERO);
    assert!(
        !radial.is_focal(),
        "a focal at the center is a plain radial"
    );
    radial.focal = Some(Offset::new(2., 0.));
    assert!(radial.is_focal());
    radial.focal = None;
    radial.focal_radius = 1.;
    assert!(radial.is_focal());
}

#[test]
fn concentric_focal_gradient_is_a_plain_radial() {
    // Circle (0, 0) radius 0 to circle (0, 0) radius 10: a point 5 away is
    // halfway.
    assert_eq!(
        two_point_conical_gradient_t(Offset::ZERO, 0., Offset::ZERO, 10., Offset::new(5., 0.)),
        Some(0.5)
    );
}

#[test]
fn off_center_focal_gradient_interpolates_circles() {
    // From (0, 0) radius 0 to (10, 0) radius 20: circle t is centered at
    // (10t, 0) with radius 20t.
    let t = |x: f32, y: f32| {
        two_point_conical_gradient_t(
            Offset::ZERO,
            0.,
            Offset::new(10., 0.),
            20.,
            Offset::new(x, y),
        )
    };
    assert_eq!(t(0., 0.), Some(0.));
    // (30, 0): |30 - 10t| = 20t gives t = 1.
    assert_eq!(t(30., 0.), Some(1.));
    // (-10, 0): 10 + 10t = 20t gives t = 1.
    assert_eq!(t(-10., 0.), Some(1.));
}

#[test]
fn points_outside_every_circle_paint_nothing() {
    // From (20, 0) radius 0 to (0, 0) radius 10: at (20, 15) the circles
    // need 300 t^2 = -225, which has no solution.
    assert_eq!(
        two_point_conical_gradient_t(
            Offset::new(20., 0.),
            0.,
            Offset::ZERO,
            10.,
            Offset::new(20., 15.)
        ),
        None
    );
}
