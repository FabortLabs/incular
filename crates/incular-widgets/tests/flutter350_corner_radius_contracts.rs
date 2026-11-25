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
fn physical_border_radius_lerp_invariants() {
    let a = BorderRadius::circular(10.);
    let b = BorderRadius::circular(20.);
    let lerp = BorderRadius::lerp_optional;
    let value = lerp(Some(a), Some(b), 0.25).unwrap();
    assert_eq!(value, a * 1.25);
    assert_eq!(value, b * 0.625);
    assert_eq!(value, a + BorderRadius::circular(2.5));
    assert_eq!(value, b - BorderRadius::circular(7.5));
    assert_eq!(lerp(None, None, 0.25), None);
    assert_eq!(lerp(None, Some(b), 0.25), Some(b * 0.25));
    assert_eq!(lerp(Some(a), None, 0.25), Some(a * 0.75));
}

#[test]
fn directional_border_radius_lerp_invariants() {
    let a = Directional::circular(10.);
    let b = Directional::circular(20.);
    let lerp = Directional::lerp_optional;
    let value = lerp(Some(a), Some(b), 0.25).unwrap();
    assert_eq!(value, a * 1.25);
    assert_eq!(value, b * 0.625);
    assert_eq!(value, a + Directional::circular(2.5));
    assert_eq!(value, b - Directional::circular(7.5));
    assert_eq!(lerp(None, None, 0.25), None);
    assert_eq!(lerp(None, Some(b), 0.25), Some(b * 0.25));
    assert_eq!(lerp(Some(a), None, 0.25), Some(a * 0.75));
}

#[test]
fn radius_and_border_radius_keep_overshooting_negative_components() {
    let a = r(10., 2.);
    let b = r(2., 10.);
    assert_eq!(a.lerp(&b, 0.5), r(6., 6.));
    assert_eq!(
        (
            a.lerp(&b, 1.5),
            BorderRadius::all(a).lerp(&BorderRadius::all(b), 1.5),
            Directional::all(a).lerp(&Directional::all(b), 1.5)
        ),
        (
            r(-2., 14.),
            BorderRadius::all(r(-2., 14.)),
            Directional::all(r(-2., 14.))
        )
    );
    assert_eq!(
        Radius::lerp_optional(None, Some(a), -0.5),
        Some(r(-5., -1.))
    );
    assert_eq!(Radius::lerp_optional(Some(a), None, 1.5), Some(r(-5., -1.)));
    assert_eq!(Radius::lerp_optional(None, None, 0.5), None);
}

#[test]
fn copy_with_replaces_and_restores_one_corner_at_a_time() {
    let radius = Radius::circular(10.);
    let border = BorderRadius::all(radius);
    let z = Some(Radius::ZERO);
    let old = Some(radius);
    let tl = border.copy_with(z, None, None, None);
    let tr = border.copy_with(None, z, None, None);
    let bl = border.copy_with(None, None, None, z);
    let br = border.copy_with(None, None, z, None);
    assert_eq!(tl.top_left, Radius::ZERO);
    assert_eq!(tr.top_right, Radius::ZERO);
    assert_eq!(bl.bottom_left, Radius::ZERO);
    assert_eq!(br.bottom_right, Radius::ZERO);
    assert_eq!(tl.copy_with(old, None, None, None), border);
    assert_eq!(tr.copy_with(None, old, None, None), border);
    assert_eq!(bl.copy_with(None, None, None, old), border);
    assert_eq!(br.copy_with(None, None, old, None), border);
}
