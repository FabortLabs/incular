//! Public BoxDecoration value and validity counterparts to the pinned
//! box_decoration_contracts_test.dart reference.

use incular_core::{Color, Lerp};
use incular_image::ImageHandle;
use incular_rendering::BlendMode;
use incular_widgets::{BoxDecoration, BoxShape, DecorationImage};

#[test]
fn flutter_350_box_decoration_self_lerp_preserves_its_blend_mode() {
    let from = BoxDecoration::new()
        .color(Color::rgba(127, 31, 255, 255))
        .background_blend_mode(BlendMode::Multiply);
    let to = from.clone().background_blend_mode(BlendMode::Difference);

    assert_ne!(from, to);
    assert_eq!(from.lerp(&to, 0.), from);
    assert_eq!(from.lerp(&to, 1.), to);
    assert_eq!(from.lerp(&to, 0.5).background_blend_mode, None);
    assert_eq!(from.lerp(&from, 0.5), from);
}

#[test]
fn flutter_350_box_decoration_switches_shape_at_midpoint() {
    let rectangle = BoxDecoration::new().color(Color::BLACK);
    let circle = rectangle.clone().shape(BoxShape::Circle);

    assert_eq!(rectangle.lerp(&circle, 0.25).shape, BoxShape::Rectangle);
    assert_eq!(rectangle.lerp(&circle, 0.5).shape, BoxShape::Circle);
    assert_eq!(rectangle.lerp(&circle, 0.75).shape, BoxShape::Circle);
}

#[test]
fn flutter_350_box_decoration_image_alone_cannot_enable_background_blend() {
    let image = ImageHandle::from_rgba8(1, 1, vec![0, 0, 0, 255]).unwrap();
    let image_only = BoxDecoration::new().image(DecorationImage::new(image));

    assert!(image_only.is_valid());
    assert!(
        image_only
            .clone()
            .color(Color::BLACK)
            .background_blend_mode(BlendMode::Multiply)
            .is_valid()
    );
    assert!(
        !BoxDecoration::new()
            .background_blend_mode(BlendMode::Multiply)
            .is_valid()
    );
    assert!(
        !image_only
            .background_blend_mode(BlendMode::Multiply)
            .is_valid(),
        "the blend applies to a background color or gradient, not the image"
    );
}
