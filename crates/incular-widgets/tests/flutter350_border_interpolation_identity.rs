use std::{borrow::Cow, rc::Rc};

use incular_config::TextDirection;
use incular_core::Color;
use incular_widgets::{
    Border, BorderDirectional, BorderRadius, BorderRadiusDirectional, BorderRadiusGeometry,
    BoxBorder, Radius, RoundedRectangleBorder, ShapeBorder, lerp_shape_border,
};

fn borrows<T: Clone>(actual: Option<Cow<'_, T>>, original: &T) {
    assert!(matches!(actual, Some(Cow::Borrowed(value)) if std::ptr::eq(value, original)));
}

#[test]
fn physical_and_box_borders_borrow_the_identical_endpoint_and_keep_absence() {
    let border = Border::default();
    assert!(Border::lerp_ref(None, None, 0.).is_none());
    borrows(Border::lerp_ref(Some(&border), Some(&border), 0.5), &border);

    let box_border: BoxBorder = Border::default();
    assert!(BoxBorder::lerp_ref(None, None, 0.).is_none());
    borrows(
        BoxBorder::lerp_ref(Some(&box_border), Some(&box_border), 0.5),
        &box_border,
    );
    assert!(
        matches!(Border::lerp_ref(Some(&border), Some(&box_border), 0.5), Some(Cow::Owned(value)) if value == border)
    );

    let other = Border::new(4., Color::WHITE);
    let faded = Border::lerp_ref(None, Some(&other), 0.25).unwrap();
    assert!(matches!(faded, Cow::Owned(_)));
    assert_eq!(faded.top.width, 1.);
    assert_eq!(faded.top.color, Color::WHITE);
    let direction = BorderDirectional::default();
    borrows(
        BorderDirectional::lerp_ref(Some(&direction), Some(&direction), 0.5),
        &direction,
    );
}

#[test]
fn an_identical_outlined_border_keeps_its_shared_shape_handle() {
    assert!(lerp_shape_border(None, None, 0.).is_none());
    let outline: Rc<dyn ShapeBorder> = Rc::new(RoundedRectangleBorder::<BorderRadius>::default());
    let result = lerp_shape_border(Some(&outline), Some(&outline), 0.5).unwrap();
    assert!(Rc::ptr_eq(&result, &outline));
}

#[test]
fn physical_logical_and_mixed_radius_interpolation_preserves_identity() {
    let physical = BorderRadius::ZERO;
    assert!(BorderRadius::lerp_ref(None, None, 0.).is_none());
    borrows(
        BorderRadius::lerp_ref(Some(&physical), Some(&physical), 0.5),
        &physical,
    );

    let logical = BorderRadiusDirectional::ZERO;
    assert!(BorderRadiusDirectional::lerp_ref(None, None, 0.).is_none());
    borrows(
        BorderRadiusDirectional::lerp_ref(Some(&logical), Some(&logical), 0.5),
        &logical,
    );

    let geometry = BorderRadiusGeometry::from(physical);
    assert!(BorderRadiusGeometry::lerp_ref(None, None, 0.).is_none());
    borrows(
        BorderRadiusGeometry::lerp_ref(Some(&geometry), Some(&geometry), 0.5),
        &geometry,
    );
    assert!(matches!(
        BorderRadiusGeometry::lerp_ref(Some(&geometry), Some(&BorderRadiusGeometry::ZERO), 0.5),
        Some(Cow::Owned(value)) if value == geometry
    ));

    let logical = BorderRadiusDirectional::only(
        Radius::circular(8.),
        Radius::ZERO,
        Radius::ZERO,
        Radius::ZERO,
    );
    let direction = BorderRadiusGeometry::from(logical);
    let sample = BorderRadiusGeometry::lerp_ref(Some(&geometry), Some(&direction), 0.5).unwrap();
    assert!(matches!(sample, Cow::Owned(_)));
    assert_eq!(
        sample.resolve(TextDirection::Ltr).top_left,
        Radius::circular(4.)
    );
    assert_eq!(
        sample.resolve(TextDirection::Rtl).top_right,
        Radius::circular(4.)
    );
}
