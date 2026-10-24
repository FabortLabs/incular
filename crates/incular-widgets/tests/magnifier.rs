//! Flutter's `MagnifierController.shiftWithinBounds`, the
//! `_RenderMagnification` backdrop matrix and `TextMagnifierConfiguration`
//! defaults.

use incular_core::{Offset, Rect, Size};
use incular_widgets::{
    MagnifierController, MagnifierInfo, RawMagnifier, TextMagnifierConfiguration,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

#[test]
fn shift_within_bounds_moves_the_least_distance() {
    let bounds = rect(0., 0., 100., 100.);
    assert_eq!(
        MagnifierController::shift_within_bounds(rect(-10., 5., 20., 20.), bounds),
        rect(0., 5., 20., 20.)
    );
    assert_eq!(
        MagnifierController::shift_within_bounds(rect(90., 95., 20., 20.), bounds),
        rect(80., 80., 20., 20.)
    );
    assert_eq!(
        MagnifierController::shift_within_bounds(rect(10., 10., 20., 20.), bounds),
        rect(10., 10., 20., 20.)
    );
}

#[test]
fn backdrop_transform_centers_the_focal_point() {
    // A 100x50 magnifier at (10, 20) has its center at (60, 45); with focal
    // offset (0, 40) and scale 2 the matrix is translate(2·(0 - 60) + 60,
    // 2·(-40 - 45) + 45) = (-60, -125) after scale 2, mapping the focal
    // point (60, 85) onto the center.
    let magnifier = RawMagnifier::new(Size::new(100., 50.))
        .magnification_scale(2.)
        .focal_point_offset(Offset::new(0., 40.));
    let transform = magnifier.backdrop_transform(Offset::new(10., 20.));
    assert_eq!(
        transform.transform_point(Offset::new(60., 85.)),
        Offset::new(60., 45.)
    );
    assert_eq!(
        transform.transform_point(Offset::ZERO),
        Offset::new(-60., -125.)
    );
    assert_eq!(
        RawMagnifier::new(Size::new(10., 10.))
            .backdrop_transform(Offset::new(3., 4.))
            .transform_point(Offset::new(7., 8.)),
        Offset::new(7., 8.),
        "scale 1 with no focal offset is the identity"
    );
}

#[test]
fn text_magnifier_configuration_defaults() {
    let disabled = TextMagnifierConfiguration::disabled();
    assert!(disabled.displays_handles_in_magnifier());
    assert!(disabled.build(MagnifierInfo::EMPTY).is_none());
}
