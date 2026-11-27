use std::hash::{DefaultHasher, Hash, Hasher};

use incular_config::TextDirection;
use incular_core::{Offset, Rect, Size};
use incular_rendering::{Canvas, FillRule};
use incular_widgets::{
    BorderRadius, BorderRadiusDirectional, BorderRadiusGeometry, BorderSide, CircleBorder, Radius,
    RoundedCircleOutline, RoundedRectangleBorder, ShapeBorder, StadiumBorder,
};

fn hash(outline: RoundedCircleOutline) -> u64 {
    let mut hasher = DefaultHasher::new();
    outline.hash(&mut hasher);
    hasher.finish()
}

fn match_timeline(
    target: RoundedCircleOutline,
    radius_label: Option<&str>,
    direction: Option<TextDirection>,
) {
    let stadium: RoundedCircleOutline = StadiumBorder::default().into();
    let early = stadium.interpolate(target, 0.1);
    let late = stadium.interpolate(target, 0.9);
    let circle = radius_label.is_none();
    let height = if circle { 20. } else { 50. };
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(100., height));
    for (outline, like_target) in [
        (stadium, false),
        (target, true),
        (early, false),
        (late, true),
        (late.interpolate(stadium, 0.1), true),
        (late.interpolate(stadium, 0.9), false),
        (early.interpolate(target, 0.1), false),
        (early.interpolate(target, 0.9), circle),
        (early.interpolate(late, 0.1), false),
        (early.interpolate(late, 0.9), true),
        (stadium.interpolate(late, 0.1), false),
        (stadium.interpolate(late, 0.9), true),
        (target.interpolate(early, 0.1), circle),
        (target.interpolate(early, 0.9), false),
    ] {
        let path = outline.outer_path_with_direction(rect, direction);
        let hit = |x, y| path.contains(Offset::new(x, y), FillRule::NonZero);
        if circle {
            assert!(hit(50., 10.));
            assert_eq!(hit(30., 10.), !like_target, "outline {outline}");
            assert!(!hit(1., 1.) && !hit(99., 19.));
        } else {
            for (x, y) in [(25., 25.), (50., 25.), (7.33, 7.33)] {
                assert!(hit(x, y), "outline {outline} should include ({x}, {y})");
            }
            if like_target {
                for (x, y) in [(4., 4.), (96., 4.), (96., 46.), (4., 46.)] {
                    assert!(hit(x, y), "outline {outline} should include ({x}, {y})");
                }
            } else {
                for (x, y) in [
                    (0.001, 0.001),
                    (99.999, 0.001),
                    (99.999, 49.999),
                    (0.001, 49.999),
                ] {
                    assert!(!hit(x, y), "outline {outline} should exclude ({x}, {y})");
                }
            }
        }
        let mut canvas = Canvas::default();
        outline.paint_with_direction(&mut canvas, rect, direction);
        assert!(
            canvas.finish().commands().is_empty(),
            "none style never paints a stroke"
        );
    }

    for (outline, percent) in [
        (early, "10.0"),
        (stadium.interpolate(target, 0.2), "20.0"),
        (early.interpolate(late, 0.9), "82.0"),
        (target.interpolate(stadium, 0.9), "10.0"),
        (target.interpolate(stadium, 0.8), "20.0"),
        (late.interpolate(early, 0.1), "82.0"),
    ] {
        let suffix = radius_label.map_or_else(
            || format!("{percent}% of the way to being a CircleBorder"),
            |radius| format!("{radius}, {percent}% of the way to being a RoundedRectangleBorder"),
        );
        assert_eq!(
            outline.to_string(),
            format!("StadiumBorder(BorderSide(width: 0.0, style: none), {suffix})")
        );
    }

    let repeat = stadium.interpolate(target, 0.1);
    assert_eq!(repeat, early);
    assert_eq!(hash(repeat), hash(early));
    let direct = stadium.interpolate(target, 0.5);
    let indirect = target
        .interpolate(stadium, 0.1)
        .interpolate(target.interpolate(stadium, 0.9), 0.5);
    assert_eq!(direct, indirect);
    assert_eq!(hash(direct), hash(indirect));
    assert_eq!(direct.to_string(), indirect.to_string());
}

#[test]
fn stadium_and_circle_morph() {
    match_timeline(CircleBorder::default().into(), None, None);
}

#[test]
fn stadium_and_square_corner_rectangle_morph() {
    match_timeline(
        RoundedRectangleBorder::default().into(),
        Some("BorderRadius.zero"),
        None,
    );
}

#[test]
fn stadium_and_circular_corner_rectangle_morph() {
    let target = RoundedRectangleBorder::new(BorderSide::NONE, BorderRadius::circular(10.));
    match_timeline(target.into(), Some("BorderRadius.circular(10.0)"), None);
}

#[test]
fn stadium_and_directional_corner_rectangle_morph() {
    let radius = BorderRadiusDirectional::only(
        Radius::circular(10.),
        Radius::ZERO,
        Radius::circular(10.),
        Radius::ZERO,
    );
    let target =
        RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::Directional(radius));
    match_timeline(
        target.into(),
        Some(
            "BorderRadiusDirectional.only(topStart: Radius.circular(10.0), bottomEnd: Radius.circular(10.0))",
        ),
        Some(TextDirection::Rtl),
    );
}
