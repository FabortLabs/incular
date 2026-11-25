use incular_core::Color;
use incular_rendering::{BlendMode, Brush, PaintStyle};
use incular_widgets::{BorderSide, BorderStyle};

#[test]
fn copy_with_retains_every_unset_property() {
    let green = Color::rgba(0, 255, 0, 255);
    let blue = Color::rgba(0, 0, 255, 255);
    let side = BorderSide::solid(green, 2.0);
    assert_eq!(
        side.copy_with(Some(blue), Some(3.0), None, None),
        BorderSide::solid(blue, 3.0),
    );
    assert_eq!(
        side.copy_with(None, Some(3.0), None, None),
        BorderSide::solid(green, 3.0)
    );
    assert_eq!(
        side.copy_with(Some(blue), None, None, None),
        BorderSide::solid(blue, 2.0)
    );
    assert_eq!(
        side.copy_with(None, None, Some(BorderStyle::None), None),
        BorderSide::new(green, 2.0, BorderStyle::None),
    );

    let aligned = side.copy_with(None, None, None, Some(0.5));
    assert_eq!(
        aligned,
        BorderSide {
            stroke_align: 0.5,
            ..side
        }
    );
    assert_eq!(
        aligned.copy_with(Some(blue), None, None, None),
        BorderSide {
            color: blue,
            ..aligned
        }
    );
    assert_eq!(side.copy_with(None, None, None, None), side);
}

#[test]
fn solid_sides_paint_a_stroke_and_none_produces_a_transparent_paint() {
    let yellow = Color::rgba(255, 255, 0, 255);
    let solid = BorderSide::solid(yellow, 2.5);
    let none = BorderSide::new(yellow, 2.5, BorderStyle::None);
    for (side, color, width) in [(solid, yellow, 2.5), (none, Color::TRANSPARENT, 0.0)] {
        let paint = side.to_paint();
        assert_eq!(paint.stroke_value().width, width);
        assert_eq!(paint.style_value(), PaintStyle::Stroke);
        assert_eq!(paint.brush_value(), &Brush::Solid(color));
        assert_eq!(paint.blend_mode_value(), BlendMode::SrcOver);
        assert_eq!(
            side.copy_with(None, None, None, Some(1.0)).to_paint(),
            paint
        );
    }
}
