//! Geometry, diagnostics, and nested values from Flutter's rounded border morph.

use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, FillRule, PaintCommand};
use incular_widgets::{
    BorderRadius, BorderSide, CircleBorder, RoundedCircleOutline, RoundedRectangleBorder,
    RoundedSuperellipseBorder, ShapeBorder,
};
use std::hash::{DefaultHasher, Hash, Hasher};

fn hash(value: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn nested_rectangle_circle_morphs_preserve_coverage_value_and_diagnostics() {
    let rectangle = RoundedCircleOutline::from(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::circular(10.),
    ));
    assert_nested_morph(rectangle, "RoundedRectangleBorder");
}

#[test]
fn nested_superellipse_circle_morphs_preserve_coverage_value_and_diagnostics() {
    let superellipse = RoundedCircleOutline::from(RoundedSuperellipseBorder::new(
        BorderSide::NONE,
        BorderRadius::circular(10.),
    ));
    assert_nested_morph(superellipse, "RoundedSuperellipseBorder");
}

fn assert_nested_morph(rectangle: RoundedCircleOutline, name: &str) {
    let circle = RoundedCircleOutline::from(CircleBorder::default());
    let early = rectangle.interpolate(circle, 0.1);
    let late = rectangle.interpolate(circle, 0.9);
    let area = Rect::from_origin_size(Offset::ZERO, Size::new(100., 20.));
    for (outline, like_rectangle) in [
        (rectangle, true),
        (circle, false),
        (early, true),
        (late, false),
        (late.interpolate(rectangle, 0.1), false),
        (late.interpolate(rectangle, 0.9), true),
        (early.interpolate(circle, 0.1), true),
        (early.interpolate(circle, 0.9), false),
        (early.interpolate(late, 0.1), true),
        (early.interpolate(late, 0.9), false),
        (rectangle.interpolate(late, 0.1), true),
        (rectangle.interpolate(late, 0.9), false),
        (circle.interpolate(early, 0.1), false),
        (circle.interpolate(early, 0.9), true),
    ] {
        let path = outline.outer_path(area);
        assert!(path.contains(Offset::new(50., 10.), FillRule::NonZero));
        assert_eq!(
            path.contains(Offset::new(30., 10.), FillRule::NonZero),
            like_rectangle
        );
        assert!(!path.contains(Offset::new(1., 1.), FillRule::NonZero));
        assert!(!path.contains(Offset::new(99., 19.), FillRule::NonZero));
    }

    for (outline, percentage) in [
        (early, "10.0"),
        (rectangle.interpolate(circle, 0.2), "20.0"),
        (early.interpolate(late, 0.9), "82.0"),
        (circle.interpolate(rectangle, 0.9), "10.0"),
        (circle.interpolate(rectangle, 0.8), "20.0"),
        (late.interpolate(early, 0.1), "82.0"),
    ] {
        let expected = format!(
            "{name}(BorderSide(width: 0.0, style: none), BorderRadius.circular(10.0), {percentage}% of the way to being a CircleBorder)"
        );
        assert_eq!(outline.to_string(), expected);
    }
    assert_eq!(early, rectangle.interpolate(circle, 0.1));
    assert_eq!(hash(early), hash(rectangle.interpolate(circle, 0.1)));
    let direct = rectangle.interpolate(circle, 0.5);
    let indirect = circle
        .interpolate(rectangle, 0.1)
        .interpolate(circle.interpolate(rectangle, 0.9), 0.5);
    assert_eq!(direct, indirect);
    assert_eq!(hash(direct), hash(indirect));
    assert_eq!(direct.to_string(), indirect.to_string());
}

#[test]
fn morph_fits_eccentric_contours_and_paints_the_actual_side() {
    let area = Rect::from_origin_size(Offset::ZERO, Size::new(100., 20.));
    let rectangle = RoundedCircleOutline::from(RoundedRectangleBorder::new(
        BorderSide::solid(Color::BLACK, 2.),
        BorderRadius::circular(10.),
    ));
    let circle = RoundedCircleOutline::from(CircleBorder::new(BorderSide::solid(Color::BLACK, 4.)));
    let oval = RoundedCircleOutline::from(
        CircleBorder::new(BorderSide::solid(Color::BLACK, 4.)).with_eccentricity(1.),
    );
    let morph = rectangle.interpolate(circle, 0.5);
    let stretched = rectangle.interpolate(oval, 0.5);
    assert_eq!(morph.side().width, 3.);
    assert_eq!(morph.outer_path(area).bounds().unwrap().size.width, 60.);
    assert_eq!(
        stretched.outer_path(area).bounds().unwrap().size.width,
        100.
    );
    assert_ne!(morph, stretched);

    let mut canvas = Canvas::default();
    morph.paint(&mut canvas, area);
    assert!(matches!(
        canvas.finish().commands(),
        [PaintCommand::StrokePath { stroke, .. }] if stroke.width == 3.
    ));
}

#[test]
fn superellipse_inner_query_and_painted_centerline_use_different_insets() {
    use incular_config::EdgeInsets;
    let rect = |x, y, w, h| Rect::from_origin_size(Offset::new(x, y), Size::new(w, h));
    let side = BorderSide::solid(Color::BLACK, 4.);
    let source = RoundedCircleOutline::from(RoundedSuperellipseBorder::new(
        side,
        BorderRadius::circular(10.),
    ));
    let target = RoundedCircleOutline::from(CircleBorder::new(side));
    let mixed = source.interpolate(target, 0.5);
    let area = rect(0., 0., 100., 20.);
    assert_eq!(mixed.dimensions(None), EdgeInsets::all(4.));
    assert_eq!(
        mixed.outer_path(area).bounds(),
        Some(rect(20., 0., 60., 20.))
    );
    assert_eq!(
        mixed.inner_path(area).bounds(),
        Some(rect(28., 8., 44., 4.))
    );
    let mut canvas = Canvas::default();
    mixed.paint(&mut canvas, area);
    let picture = canvas.finish();
    let [PaintCommand::StrokePath { path, stroke, .. }] = picture.commands() else {
        panic!("one true superellipse stroke")
    };
    assert_eq!(path.bounds(), Some(rect(22., 2., 56., 16.)));
    assert_eq!(stroke.width, 4.);
}
