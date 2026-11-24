//! Flutter 3.50.0 elliptical corner radius and overshoot checks.

use incular_core::Lerp;
use incular_widgets::{BorderRadius, BorderRadiusDirectional as Directional, Radius};

fn r(x: f32, y: f32) -> Radius {
    Radius::elliptical(x, y)
}

#[test]
fn physical_border_radius_lerps_elliptical_corners_independently() {
    let a = BorderRadius {
        top_left: r(10., 20.),
        top_right: r(30., 40.),
        bottom_left: r(50., 60.),
        bottom_right: Radius::ZERO,
    };
    let b = BorderRadius {
        top_left: Radius::ZERO,
        top_right: r(100., 110.),
        bottom_left: r(120., 130.),
        bottom_right: r(140., 150.),
    };
    assert_eq!(
        a.lerp(&b, 0.5),
        BorderRadius {
            top_left: r(5., 10.),
            top_right: r(65., 75.),
            bottom_left: r(85., 95.),
            bottom_right: r(70., 75.)
        }
    );
}

#[test]
fn directional_border_radius_lerps_elliptical_corners_independently() {
    let a = Directional {
        top_start: r(10., 20.),
        top_end: r(30., 40.),
        bottom_start: r(50., 60.),
        bottom_end: Radius::ZERO,
    };
    let b = Directional {
        top_start: Radius::ZERO,
        top_end: r(100., 110.),
        bottom_start: r(120., 130.),
        bottom_end: r(140., 150.),
    };
    assert_eq!(
        a.lerp(&b, 0.5),
        Directional {
            top_start: r(5., 10.),
            top_end: r(65., 75.),
            bottom_start: r(85., 95.),
            bottom_end: r(70., 75.)
        }
    );
}

#[test]
fn non_null_border_radius_quarter_lerp_preserves_expected_values() {
    assert_eq!(
        BorderRadius::circular(10.).lerp(&BorderRadius::circular(20.), 0.25),
        BorderRadius::circular(12.5)
    );
}

#[test]
fn non_null_directional_radius_quarter_lerp_preserves_expected_values() {
    assert_eq!(
        Directional::circular(10.).lerp(&Directional::circular(20.), 0.25),
        Directional::circular(12.5)
    );
}

#[test]
fn radius_and_border_radius_keep_overshooting_negative_components() {
    let a = r(10., 2.);
    let b = r(2., 10.);
    assert_eq!(a.lerp(&b, 0.5), r(6., 6.));
    assert_eq!(
        (
            a.lerp(&b, 1.5),
            BorderRadius::all(a).lerp(&BorderRadius::all(b), 1.5)
        ),
        (r(-2., 14.), BorderRadius::all(r(-2., 14.)))
    );
}
