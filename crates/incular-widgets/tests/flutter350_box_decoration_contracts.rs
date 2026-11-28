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

#[test]
fn flutter_350_decorations_with_different_blend_modes_are_not_equal() {
    let one = BoxDecoration::new()
        .color(Color::TRANSPARENT)
        .background_blend_mode(BlendMode::Color);
    let two = BoxDecoration::new()
        .color(Color::TRANSPARENT)
        .background_blend_mode(BlendMode::Difference);
    assert_ne!(one, two);
}

#[test]
fn flutter_350_distinct_but_equal_decorations_still_interpolate() {
    let from = BoxDecoration::new()
        .color(Color::BLACK)
        .background_blend_mode(BlendMode::Multiply);
    let to = from.clone();
    assert_eq!(from, to);
    assert_eq!(from.lerp(&to, 0.5).background_blend_mode, None);
}

#[test]
fn identical_box_decoration_values_have_matching_hashes() {
    use incular_core::Color;
    use incular_widgets::BoxShadow;
    use std::hash::{DefaultHasher, Hash, Hasher};

    let make = || {
        BoxDecoration::new()
            .color(Color::WHITE)
            .box_shadow([BoxShadow::default()])
    };
    let hash = |value: &BoxDecoration| {
        let mut state = DefaultHasher::new();
        value.hash(&mut state);
        state.finish()
    };
    let (first, second) = (make(), make());
    assert_eq!(first, second);
    assert_eq!(hash(&first), hash(&second));
    assert_ne!(first, BoxDecoration::new().color(Color::WHITE));
}

#[test]
fn hashes_follow_normalized_paint_values_including_erased_image_sources() {
    use incular_config::Alignment;
    use incular_core::Color;
    use incular_rendering::GradientStops;
    use incular_widgets::{BoxLinearGradient, DecorationImage, MemoryImage};
    use std::hash::{DefaultHasher, Hash, Hasher};

    let gradient = |stops, rotation| {
        BoxDecoration::new().aligned_gradient(
            BoxLinearGradient::new(Alignment::TOP_LEFT, Alignment::BOTTOM_RIGHT, stops)
                .rotated(rotation),
        )
    };
    let colors = [Color::BLACK, Color::WHITE];
    let first = gradient(GradientStops::even(&colors), 0.).image(
        DecorationImage::from_provider(MemoryImage::new(&b"encoded image"[..])).on_error(|_| {}),
    );
    let second = gradient(
        GradientStops::from_colors(&colors, Some(&[0., 1.])).unwrap(),
        -0.,
    )
    .image(DecorationImage::from_provider(MemoryImage::new(
        &b"encoded image"[..],
    )));
    assert_eq!(first, second);
    let hash = |value: &BoxDecoration| {
        let mut state = DefaultHasher::new();
        value.hash(&mut state);
        state.finish()
    };
    assert_eq!(hash(&first), hash(&second));
}
