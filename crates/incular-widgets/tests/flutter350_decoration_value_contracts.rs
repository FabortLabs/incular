use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    ptr,
    sync::Arc,
};

use incular_config::Alignment;
use incular_core::{Color, Lerp};
use incular_rendering::GradientStops;
use incular_widgets::{
    Border, BorderSide, BorderStyle, BoxDecoration, BoxLinearGradient, BoxShape, Decoration,
};

fn hash(value: impl Hash) -> u64 {
    let mut digest = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut digest);
    digest.finish()
}

fn changed_side() -> BorderSide {
    BorderSide::default().copy_with(
        Some(Color::rgba(0, 255, 255, 255)),
        Some(2.),
        Some(BorderStyle::Solid),
        None,
    )
}

#[test]
fn border_side_interpolates_width_color_and_style_without_creating_an_opaque_stroke() {
    let default = BorderSide::default();
    let changed = changed_side();
    assert_eq!(
        (
            default.color,
            default.width,
            default.style,
            default.stroke_align
        ),
        (Color::BLACK, 1., BorderStyle::Solid, -1.)
    );
    assert_eq!(default.to_string().lines().count(), 1);
    assert_ne!(hash(default), hash(changed));
    assert_eq!(
        changed,
        BorderSide::solid(Color::rgba(0, 255, 255, 255), 2.)
    );
    assert_eq!(default.lerp(&changed, 0.), default);
    assert_eq!(default.lerp(&changed, 1.), changed);
    assert_eq!(
        default.lerp(&changed, 0.5),
        BorderSide::solid(Color::BLACK.lerp(&changed.color, 0.5), 1.5)
    );
    let no_stroke = changed.copy_with(None, None, Some(BorderStyle::None), None);
    let fade_out = changed.lerp(&no_stroke, 0.2);
    let fade_in = no_stroke.lerp(&changed, 0.2);
    assert_eq!(
        fade_out,
        BorderSide::solid(Color::rgba(0, 255, 255, 204), 2.)
    );
    assert_eq!(fade_in, BorderSide::solid(Color::rgba(0, 255, 255, 51), 2.));
}

#[test]
fn border_side_display_omits_defaults_and_reports_changed_fields_in_order() {
    assert_eq!(BorderSide::default().to_string(), "BorderSide");
    assert_eq!(
        changed_side().to_string(),
        "BorderSide(color: Color(alpha: 1.0000, red: 0.0000, green: 1.0000, blue: 1.0000, colorSpace: ColorSpace.sRGB), width: 2.0)"
    );
    assert_eq!(
        BorderSide::NONE
            .copy_with(None, Some(3.), None, Some(1.))
            .to_string(),
        "BorderSide(width: 3.0, strokeAlign: 1.0, style: none)"
    );
}

#[test]
fn optional_uniform_borders_scale_in_or_out_from_their_present_endpoint() {
    let border = Border::all(BorderSide::solid(Color::BLACK, 4.));
    let growing = Border::lerp_optional(None, Some(border), 0.25).unwrap();
    let shrinking = Border::lerp_optional(Some(border), None, 0.25).unwrap();
    assert_eq!(border.to_string().lines().count(), 1);
    assert_ne!(hash(border), hash(growing));
    assert_eq!(growing.top.width, 1.);
    assert_eq!(shrinking.bottom.width, 3.);
    assert_eq!(growing.lerp(&shrinking, 0.5).left.width, 2.);
}

#[test]
fn border_display_collapses_uniform_sides_and_omits_absent_nonuniform_sides() {
    assert_eq!(
        Border::all(BorderSide::solid(Color::BLACK, 4.)).to_string(),
        "Border.all(BorderSide(width: 4.0))"
    );
    let side = BorderSide::solid(Color::BLACK, 3.);
    let border = Border {
        top: side,
        right: side,
        bottom: side,
        left: side,
    };
    assert_eq!(border.to_string(), "Border.all(BorderSide(width: 3.0))");
    assert_eq!(
        Border {
            top: side,
            ..Default::default()
        }
        .to_string(),
        "Border(top: BorderSide(width: 3.0))"
    );
}

#[test]
fn generic_decoration_interpolation_preserves_expected_box_colors() {
    let a = Decoration::from(BoxDecoration::new().color(Color::WHITE));
    let b = Decoration::from(BoxDecoration::new().color(Color::TRANSPARENT));
    for (t, expected) in [
        (0., Color::WHITE),
        (0.25, Color::WHITE.lerp(&Color::TRANSPARENT, 0.25)),
        (1., Color::TRANSPARENT),
    ] {
        let result = Decoration::lerp_optional(Some(&a), Some(&b), t).unwrap();
        let Decoration::Box(box_value) = result.as_ref() else {
            panic!("box endpoints produce a box")
        };
        assert_eq!(box_value.color, Some(expected));
    }
}

#[test]
fn identical_generic_decoration_is_borrowed_without_rebuilding() {
    assert!(Decoration::lerp_optional(None, None, 0.).is_none());
    let decoration = Decoration::from(BoxDecoration::new());
    let result = Decoration::lerp_optional(Some(&decoration), Some(&decoration), 0.5);
    let Some(Cow::Borrowed(result)) = result else {
        panic!("an identical decoration must be borrowed")
    };
    assert!(ptr::eq(result, &decoration));
}

#[test]
fn box_shape_switches_at_the_midpoint_not_via_a_morph() {
    let a = BoxDecoration::new();
    let b = BoxDecoration::new().shape(BoxShape::Circle);
    for t in [-1., 0., 0.25, 0.499] {
        assert_eq!(a.lerp(&b, t), a);
    }
    for t in [0.5, 0.75, 1., 2.] {
        assert_eq!(a.lerp(&b, t), b);
    }
}

fn gradient_with_alpha(alpha: u8) -> BoxLinearGradient {
    BoxLinearGradient::new(
        Alignment::CENTER_LEFT,
        Alignment::CENTER_RIGHT,
        GradientStops::even(&[Color::TRANSPARENT, Color::rgba(255, 255, 255, alpha)]),
    )
}

#[test]
fn a_missing_gradient_fades_opacity_without_destroying_the_transparent_rgb() {
    let plain = BoxDecoration::new();
    let shaded = BoxDecoration::new().aligned_gradient(gradient_with_alpha(255));
    for (t, alpha) in [(-1., 0), (0.2, 51), (1. / 3., 85), (1., 255), (2., 255)] {
        let actual = plain.lerp(&shaded, t);
        assert_eq!(
            actual.aligned_gradient,
            Some(Arc::new(gradient_with_alpha(alpha))),
            "gradient at {t}"
        );
    }
    assert!(plain.lerp(&shaded, 0.).aligned_gradient.is_none());
}
