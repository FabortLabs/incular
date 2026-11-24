//! Custom counterparts to the pinned Flutter 3.50.0 border_side_edges_test.

use incular_core::{Color, Lerp};
use incular_widgets::{BorderSide, BorderStyle};

const OPAQUE: Color = Color::rgba(0x12, 0x58, 0x9a, 0xff);
const TRANSPARENT: Color = Color::rgba(0x12, 0x58, 0x9a, 0);

#[test]
fn flutter_350_border_side_scale_resets_nondefault_stroke_alignment() {
    let cases = [
        (0.5, 2., BorderStyle::Solid),
        (2., 8., BorderStyle::Solid),
        (0., 0., BorderStyle::None),
        (-1., 0., BorderStyle::None),
    ];
    let scaled_geometry = [0., 1.].map(|stroke_align| {
        let side = BorderSide {
            stroke_align,
            ..BorderSide::solid(OPAQUE, 4.)
        };
        cases.map(|(factor, width, style)| {
            let scaled = side.scale(factor);
            assert_eq!(
                (scaled.width, scaled.style, scaled.color),
                (width, style, OPAQUE),
                "stroke_align={stroke_align}, factor={factor}"
            );
            (
                scaled.stroke_align,
                scaled.stroke_inset(),
                scaled.stroke_outset(),
            )
        })
    });
    let inside_geometry = cases.map(|(_, width, _)| (-1., width, 0.));
    assert_eq!(
        scaled_geometry, [inside_geometry; 2],
        "(alignment, inset, outset) for centered and outside inputs"
    );
}

#[test]
fn flutter_350_border_side_lerp_fades_none_endpoint_color_both_ways() {
    let none = BorderSide::new(OPAQUE, 2., BorderStyle::None);
    let solid = BorderSide::solid(OPAQUE, 2.);
    let transparent_solid = BorderSide::solid(TRANSPARENT, 2.);
    assert_eq!(none.lerp(&solid, 0.), none);
    assert_eq!(none.lerp(&solid, 1.), solid);
    assert_eq!(solid.lerp(&none, 0.), solid);
    assert_eq!(solid.lerp(&none, 1.), none);

    let colors = |start: BorderSide, end| [0.25, 0.5, 0.75].map(|t| start.lerp(&end, t).color);
    let increasing = [64, 128, 191].map(|alpha| Color::rgba(0x12, 0x58, 0x9a, alpha));
    let decreasing = [191, 128, 64].map(|alpha| Color::rgba(0x12, 0x58, 0x9a, alpha));
    // Ordinary color lerp with a transparent Solid endpoint is the control.
    assert_eq!(colors(transparent_solid, solid), increasing);
    assert_eq!(colors(solid, transparent_solid), decreasing);
    assert_eq!(
        [colors(none, solid), colors(solid, none)],
        [increasing, decreasing],
        "None to Solid and Solid to None at t=0.25, 0.50, 0.75"
    );
}
