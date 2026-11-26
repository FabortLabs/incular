use std::borrow::Cow;

use incular_core::{Color, Offset, Rect, Size};
use incular_image::ImageHandle;
use incular_rendering::{Brush, GradientStops, LinearGradient};
use incular_widgets::{
    BorderRadius, BorderSide, BoxDecoration, BoxShadow, BoxShape, CircleBorder, Decoration,
    DecorationImage, RoundedRectangleBorder, ShapeDecoration,
};

#[test]
fn a_shape_lerp_preserves_identity_and_both_missing_returns_none() {
    let reference = ShapeDecoration::new(CircleBorder::default());
    let result = ShapeDecoration::lerp_optional(Some(&reference), Some(&reference), 0.5);
    assert!(matches!(result, Some(Cow::Borrowed(value)) if std::ptr::eq(value, &reference)));
    assert!(ShapeDecoration::lerp_optional(None, None, 0.).is_none());
}

#[test]
fn public_decoration_lerp_preserves_nullable_default_geometry() {
    let size = Size::new(200., 100.);
    let rect = Rect::from_origin_size(Offset::ZERO, size);
    let circle = ShapeDecoration::new(CircleBorder::default());
    let rounded = ShapeDecoration::new(RoundedRectangleBorder::default());
    let a = Decoration::from(circle.clone());
    let b = Decoration::from(rounded.clone());
    for (value, expected) in [
        (Decoration::lerp_optional(Some(&a), None, 0.), circle),
        (Decoration::lerp_optional(None, Some(&b), 0.), rounded),
    ] {
        let value = value.expect("a missing decoration isn't an empty result");
        let Decoration::Shape(actual) = value.as_ref() else {
            panic!("expected shape")
        };
        assert_eq!(
            actual.get_clip_path(rect, None),
            expected.get_clip_path(rect, None)
        );
        assert_eq!(
            actual.shape.dimensions(None),
            expected.shape.dimensions(None)
        );
        assert_eq!(actual.color, expected.color);
        assert_eq!(actual.gradient, expected.gradient);
        assert!(actual.image.is_none() && actual.shadows.is_empty());
    }
    assert!(Decoration::lerp_optional(None, None, 0.).is_none());
}

#[test]
fn both_null_directions_scale_fill_corners_border_and_shadow() {
    let color = Color::rgba(50, 80, 120, 200);
    let shadow = BoxShadow::new(Color::rgba(48, 64, 80, 128), Offset::new(16., 8.), 8., 4.);
    let value = ShapeDecoration::new(RoundedRectangleBorder::new(
        BorderSide::solid(Color::BLACK, 8.),
        BorderRadius::circular(24.),
    ))
    .color(color)
    .shadows([shadow]);
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(100., 100.));
    let from_null = ShapeDecoration::lerp_optional(None, Some(&value), 0.25).unwrap();
    let to_null = ShapeDecoration::lerp_optional(Some(&value), None, 0.75).unwrap();
    let expected = ShapeDecoration::new(RoundedRectangleBorder::new(
        BorderSide::solid(Color::BLACK, 2.),
        BorderRadius::circular(6.),
    ));
    for actual in [from_null, to_null] {
        assert_eq!(actual.color, Some(Color::rgba(50, 80, 120, 50)));
        assert_eq!(actual.shape.dimensions(None).left, 2.);
        assert_eq!(
            actual.get_clip_path(rect, None),
            expected.get_clip_path(rect, None)
        );
        assert_eq!(
            actual.shadows,
            [BoxShadow::new(shadow.color, Offset::new(4., 2.), 2., 1.)]
        );
    }
    let zero = ShapeDecoration::lerp_optional(Some(&value), None, 1.).unwrap();
    assert_eq!(zero.color.unwrap().alpha, 0);
    assert_eq!(zero.shape.dimensions(None).left, 0.);
}

#[test]
fn nullable_box_and_shape_keep_image_geometry_and_scale_opacity() {
    let mut image = DecorationImage::new(ImageHandle::from_rgba8(1, 1, vec![255; 4]).unwrap());
    image.opacity = 0.8;
    image.scale = 2.;
    let stops = GradientStops::even(&[Color::rgba(100, 20, 30, 200), Color::BLACK]);
    let gradient = LinearGradient::new(Offset::ZERO, Offset::new(50., 0.), stops);
    let box_value = Decoration::from(
        BoxDecoration::new()
            .shape(BoxShape::Circle)
            .image(image.clone())
            .gradient(gradient.clone()),
    );
    let shape_value = Decoration::from(
        ShapeDecoration::new(CircleBorder::default())
            .image(image)
            .gradient(gradient),
    );
    for value in [&box_value, &shape_value] {
        let faded = Decoration::lerp_optional(None, Some(value), 0.25).unwrap();
        let (brush, image) = match faded.as_ref() {
            Decoration::Box(value) => {
                assert_eq!(value.shape, BoxShape::Circle);
                (&value.gradient, &value.image)
            }
            Decoration::Shape(value) => (&value.gradient, &value.image),
        };
        let Some(Brush::LinearGradient(brush)) = brush else {
            panic!("expected gradient");
        };
        assert_eq!(
            brush.stops.as_slice()[0].color,
            Color::rgba(100, 20, 30, 50)
        );
        assert_eq!(brush.end, Offset::new(50., 0.));
        assert_eq!(image.as_ref().unwrap().scale, 2.);
        assert_eq!(image.as_ref().unwrap().opacity, 0.2);
    }
}
