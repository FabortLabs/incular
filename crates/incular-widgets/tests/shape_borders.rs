//! Flutter's `StadiumBorder`, `StarBorder` and `RoundedSuperellipseBorder`
//! paths, with bounds derived by hand from the Dart definitions.

use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, FillRule, PaintCommand, Path};
use incular_widgets::{
    BorderRadius, BorderSide, RoundedSuperellipseBorder, ShapeBorder, StadiumBorder, StarBorder,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn assert_bounds(path: &Path, expected: Rect) {
    assert_bounds_within(path, expected, 1e-3);
}

fn assert_bounds_within(path: &Path, expected: Rect, tolerance: f32) {
    let bounds = path.bounds().expect("non-empty path");
    for (actual, wanted) in [
        (bounds.origin.x, expected.origin.x),
        (bounds.origin.y, expected.origin.y),
        (bounds.size.width, expected.size.width),
        (bounds.size.height, expected.size.height),
    ] {
        assert!(
            (actual - wanted).abs() < tolerance,
            "{bounds:?} != {expected:?}"
        );
    }
}

#[test]
fn stadium_paths_and_stroke() {
    // Radius = shortestSide / 2 = 50. An inside stroke (strokeAlign -1) of
    // width 4 insets the inner path by strokeInset 4 and is drawn on the
    // outline inflated by strokeOffset / 2 = -2.
    let border = StadiumBorder::new(BorderSide::solid(Color::BLACK, 4.));
    let area = rect(0., 0., 200., 100.);
    let outer = border.outer_path(area);
    assert_bounds(&outer, area);
    assert!(outer.contains(Offset::new(100., 50.), FillRule::NonZero));
    assert!(!outer.contains(Offset::new(2., 2.), FillRule::NonZero));
    assert_bounds(&border.inner_path(area), rect(4., 4., 192., 92.));
    let mut canvas = Canvas::default();
    border.paint(&mut canvas, area);
    let list = canvas.finish();
    let [PaintCommand::StrokePath { path, stroke, .. }] = list.commands() else {
        panic!("one stroke: {:?}", list.commands());
    };
    assert_eq!(stroke.width, 4.);
    assert_bounds(path, rect(2., 2., 196., 96.));
    let mut canvas = Canvas::default();
    StadiumBorder::default().scale(0.).paint(&mut canvas, area);
    assert!(
        canvas.finish().is_empty(),
        "BorderSide.scale(0) removes the side"
    );
}

#[test]
fn star_bounds() {
    // Five points of radius 50 around (50, 50), the first at the top: the
    // side points at -18° reach 50 ± 50·cos 18° = 2.4472 / 97.5528 and the
    // lower ones at 54° reach 50 + 50·sin 54° = 90.4508.
    let star = StarBorder::new().outer_path(rect(0., 0., 100., 100.));
    assert_bounds(&star, rect(2.4472, 0., 95.1057, 90.4508));
    assert!(star.contains(Offset::new(50., 50.), FillRule::NonZero));
    assert!(!star.contains(Offset::new(50., 95.), FillRule::NonZero));
}

#[test]
fn polygon_squash_and_rotation() {
    // A square polygon is a diamond with its points on the circle of radius
    // 50. In a 200x100 rectangle squash 0 keeps it round (x 50..150) and
    // squash 1 stretches it to the longer side.
    let diamond = StarBorder::polygon().points(4.);
    assert_bounds(
        &diamond.outer_path(rect(0., 0., 100., 100.)),
        rect(0., 0., 100., 100.),
    );
    assert_bounds(
        &diamond.outer_path(rect(0., 0., 200., 100.)),
        rect(50., 0., 100., 100.),
    );
    assert_bounds(
        &diamond.squash(1.).outer_path(rect(0., 0., 200., 100.)),
        rect(0., 0., 200., 100.),
    );
    // Rotated 45° the valleys lead: inner radius 50·(cos 45°·0.998 + 0.002)
    // = 35.3846 from the center.
    assert_bounds(
        &diamond.rotation(45.).outer_path(rect(0., 0., 100., 100.)),
        rect(14.6154, 14.6154, 70.7693, 70.7693),
    );
    assert!((diamond.inner_ratio() - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
}

#[test]
fn rounded_superellipse_border() {
    let area = rect(0., 0., 100., 100.);
    let square = RoundedSuperellipseBorder::default();
    assert_bounds(&square.outer_path(area), area);
    let rounded = RoundedSuperellipseBorder::new(
        BorderSide::solid(Color::BLACK, 2.),
        BorderRadius::circular(20.),
    );
    let outer = rounded.outer_path(area);
    assert!(outer.contains(Offset::new(5.9, 5.9), FillRule::NonZero));
    assert!(!outer.contains(Offset::new(5.8, 5.8), FillRule::NonZero));
    // Flutter's fitted superellipse cubics bulge past the edge by ~0.0024 px.
    assert_bounds_within(&rounded.inner_path(area), rect(2., 2., 96., 96.), 0.01);
    let mut canvas = Canvas::default();
    rounded.paint(&mut canvas, area);
    let list = canvas.finish();
    let [PaintCommand::StrokePath { path, .. }] = list.commands() else {
        panic!("one stroke");
    };
    // (strokeOutset - strokeInset) / 2 = (0 - 2) / 2 = -1.
    assert_bounds_within(path, rect(1., 1., 98., 98.), 0.01);
}
