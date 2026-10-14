use incular_core::{Color, Offset};
use incular_rendering::{BlendMode, Brush, LineCap, LineJoin, Paint, PaintStyle, Shadow, Stroke};

#[test]
fn stroke_default_preserves_public_values() {
    let stroke = Stroke::default();

    assert_eq!(stroke.width, 1.0);
    assert_eq!(stroke.cap, LineCap::Butt);
    assert_eq!(stroke.join, LineJoin::Miter);
    assert_eq!(stroke.miter_limit, 4.0);
}

#[test]
fn paint_default_preserves_public_values() {
    let paint = Paint::default();

    assert_eq!(paint.brush_value(), &Brush::Solid(Color::BLACK));
    assert_eq!(paint.style_value(), PaintStyle::Fill);
    assert_eq!(paint.stroke_value(), Stroke::default());
    assert_eq!(paint.blend_mode_value(), BlendMode::SrcOver);
    assert_eq!(paint.color_filter_value(), None);
}

#[test]
fn shadow_default_preserves_public_values() {
    let shadow = Shadow::default();

    assert_eq!(shadow.color, Color::BLACK);
    assert_eq!(shadow.offset, Offset::ZERO);
    assert_eq!(shadow.blur_radius, 0.0);
}
