//! Exact selected Flutter `painting/border_radius_test.dart` behaviors.

use incular_core::Lerp;
use incular_widgets::{
    BorderRadius, BorderRadiusDirectional, BorderRadiusGeometry as Geometry, BoxDecoration, Radius,
    TextDirection,
};

const LTR: TextDirection = TextDirection::Ltr;
const RTL: TextDirection = TextDirection::Rtl;
const Z: Radius = Radius::ZERO;

fn r(x: f32, y: f32) -> Radius {
    Radius::elliptical(x, y)
}

fn border(tl: Radius, tr: Radius, bl: Radius, br: Radius) -> BorderRadius {
    BorderRadius::only(tl, tr, br, bl)
}

fn directional(ts: Radius, te: Radius, bs: Radius, be: Radius) -> BorderRadiusDirectional {
    BorderRadiusDirectional::only(ts, te, be, bs)
}

fn reference_components() -> (Geometry, Geometry) {
    (
        border(r(10., 20.), r(30., 40.), r(50., 60.), Z).into(),
        directional(Z, r(100., 110.), r(120., 130.), r(140., 150.)).into(),
    )
}

fn assert_resolves(actual: Geometry, left_to_right: BorderRadius, right_to_left: BorderRadius) {
    assert_eq!(actual.resolve(LTR), left_to_right);
    assert_eq!(actual.resolve(RTL), right_to_left);
}

fn assert_interpolation(interpolate: impl Fn(Geometry, Geometry, f32) -> Geometry) {
    let (a, b) = reference_components();
    assert_resolves(
        interpolate(a, b, 0.5),
        border(r(5., 10.), r(65., 75.), r(85., 95.), r(70., 75.)),
        border(r(55., 65.), r(15., 20.), r(95., 105.), r(60., 65.)),
    );
    assert_eq!(interpolate(a, b, 0.).resolve(LTR), a.resolve(LTR));
    assert_eq!(interpolate(a, b, 1.).resolve(RTL), b.resolve(RTL));
}

#[test]
fn mixed_elliptical_interpolation_resolves_in_both_directions() {
    assert_interpolation(|a, b, t| {
        Geometry::lerp_optional(Some(a), Some(b), t).expect("intermediate geometry")
    });
}

#[test]
fn operators_can_build_the_same_mixed_interpolation() {
    assert_interpolation(|a, b, t| a + (b - a) * t);
}

#[test]
fn subtracting_logical_corners_keeps_their_negative_components() {
    let (a, b) = reference_components();
    assert_resolves(
        a - b,
        border(r(10., 20.), r(-70., -70.), r(-70., -70.), r(-140., -150.)),
        border(r(-90., -90.), r(30., 40.), r(-90., -90.), r(-120., -130.)),
    );
}

#[test]
fn adding_logical_corners_changes_the_correct_physical_sides() {
    let (a, b) = reference_components();
    assert_resolves(
        a + b,
        border(r(10., 20.), r(130., 150.), r(170., 190.), r(140., 150.)),
        border(r(110., 130.), r(30., 40.), r(190., 210.), r(120., 130.)),
    );
}

#[test]
fn scalar_multiply_scales_both_parts_of_the_mixed_sum() {
    let (a, b) = reference_components();
    assert_resolves(
        (a + b) * 0.5,
        border(r(5., 10.), r(65., 75.), r(85., 95.), r(70., 75.)),
        border(r(55., 65.), r(15., 20.), r(95., 105.), r(60., 65.)),
    );
}

#[test]
fn addition_with_nested_subtraction_preserves_independent_sides() {
    let a = Geometry::from(border(r(300., 500.), Z, Z, Z));
    let b = Geometry::from(directional(Z, r(30., 50.), Z, Z));
    let c = Geometry::from(border(Z, Z, r(3., 5.), Z));
    assert_resolves(
        a + (b - c),
        border(r(300., 500.), r(30., 50.), r(-3., -5.), Z),
        border(r(330., 550.), Z, r(-3., -5.), Z),
    );
}

#[test]
fn addition_with_nested_subtraction_handles_all_eight_corners() {
    let circle = Radius::circular;
    let a = Geometry::from(border(
        circle(300.),
        circle(500.),
        circle(700.),
        circle(900.),
    ));
    let b = Geometry::from(directional(
        circle(30.),
        circle(50.),
        circle(70.),
        circle(90.),
    ));
    let c = Geometry::from(border(circle(3.), circle(5.), circle(7.), circle(9.)));
    assert_resolves(
        a + (b - c),
        border(circle(327.), circle(545.), circle(763.), circle(981.)),
        border(circle(347.), circle(525.), circle(783.), circle(961.)),
    );
}

#[test]
fn a_decoration_resolves_the_same_value_for_each_direction_and_can_replace_it() {
    let (a, b) = reference_components();
    let geometry = a.lerp(&b, 0.5);
    let decoration = BoxDecoration::new().border_radius_geometry(geometry);
    for direction in [LTR, RTL] {
        let resolved = decoration.clone().resolved_for_direction(direction);
        assert_eq!(resolved.border_radius, Some(geometry.resolve(direction)));
        assert!(resolved.directional_border_radius.is_none());
    }

    let physical = BorderRadius::circular(8.);
    let replaced = decoration.border_radius_geometry(physical);
    assert_eq!(replaced.border_radius, Some(physical));
    assert!(replaced.directional_border_radius.is_none());
}
