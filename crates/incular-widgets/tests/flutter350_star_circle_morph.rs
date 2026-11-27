use std::rc::Rc;

use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, FillRule, PaintCommand, Path};
use incular_widgets::{BorderSide, CircleBorder, ShapeBorder, StarBorder, lerp_shape_border};

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.00001,
        "{actual} != {expected}"
    );
}

fn circle() -> Rc<dyn ShapeBorder> {
    Rc::new(CircleBorder::new(BorderSide::solid(Color::BLACK, 2.)).with_eccentricity(0.6))
}

fn sample(path: &Path) -> String {
    let mut coverage = String::new();
    for y in [5., 20., 40., 60., 75.] {
        for x in [10., 25., 45., 60., 75., 95., 110.] {
            coverage.push(if path.contains(Offset::new(x, y), FillRule::NonZero) {
                '1'
            } else {
                '0'
            });
        }
    }
    coverage
}

#[test]
fn fractional_circle_morph_matches_parameters_inner_coverage_and_stroke() {
    let mut endpoint = StarBorder::new()
        .points(5.5)
        .rotation(15.)
        .squash(0.1)
        .point_rounding(0.2)
        .valley_rounding(0.1)
        .inner_radius_ratio(0.35);
    endpoint.side = BorderSide::solid(Color::BLACK, 6.);
    let star: Rc<dyn ShapeBorder> = Rc::new(endpoint);
    let circle = circle();
    let r = Rect::from_origin_size(Offset::ZERO, Size::new(120., 80.));

    for (a, b, t) in [(&circle, &star, 0.25), (&star, &circle, 0.75)] {
        let shape = lerp_shape_border(Some(a), Some(b), t).unwrap();
        let star = shape
            .star_geometry()
            .expect("not a discrete circle fallback");
        close(star.side.width, 3.);
        close(star.point_count(), 5.875);
        close(star.rotation_degrees(), 15.);
        close(star.squash_factor(), 0.475);
        close(star.point_rounding_factor(), 0.8);
        close(star.valley_rounding_factor(), 0.025);
        let seed = (std::f32::consts::PI / 5.875).cos();
        close(star.inner_ratio(), seed + (0.35 - seed) * 0.25);
        assert_eq!(
            sample(&shape.outer_path(r)),
            "00011000111110011111001111100011000"
        );
        assert_eq!(
            sample(&shape.inner_path(r)),
            "00010000111110011111000111100011000"
        );

        let mut canvas = Canvas::default();
        shape.paint(&mut canvas, r);
        let display = canvas.finish();
        let [PaintCommand::StrokePath { stroke, .. }] = display.commands() else {
            panic!("expected a single requested outline");
        };
        close(stroke.width, 3.);
    }
}

#[test]
fn two_point_circle_morph_has_distinct_directional_counts_and_equal_rounding() {
    let star: Rc<dyn ShapeBorder> = Rc::new(
        StarBorder::new()
            .points(2.2)
            .point_rounding(0.1)
            .valley_rounding(0.1)
            .inner_radius_ratio(0.4),
    );
    let circle = circle();
    for (a, b, t, count) in [(&circle, &star, 0.25, 2.15), (&star, &circle, 0.75, 2.05)] {
        let shape = lerp_shape_border(Some(a), Some(b), t).unwrap();
        let star = shape.star_geometry().unwrap();
        close(star.point_count(), count);
        close(star.point_rounding_factor(), 0.4);
        close(star.valley_rounding_factor(), 0.4);
        close(star.inner_ratio(), 0.85);
    }
}

#[test]
fn stadium_and_rounded_rectangle_morphs_follow_every_flutter_stage() {
    use incular_widgets::{BorderRadius, Radius, RoundedRectangleBorder, StadiumBorder};

    const STADIUM_OUTER: &str = "00111000111110111111101111100011100";
    const STADIUM_INNER: &str = "00111000111110011111001111100011100";
    const STAR_OUTER: &str = "00001000011100011111000111000010000";
    const STAR_INNER: &str = "00000000011100001110000111000000000";
    let side = BorderSide::solid(Color::BLACK, 2.);
    let stadium: Rc<dyn ShapeBorder> = Rc::new(StadiumBorder::new(side));
    let rectangle: Rc<dyn ShapeBorder> = Rc::new(RoundedRectangleBorder::new(
        side,
        BorderRadius::only(
            Radius::elliptical(18., 12.),
            Radius::elliptical(12., 24.),
            Radius::circular(4.),
            Radius::ZERO,
        ),
    ));
    let mut star = StarBorder::new()
        .points(5.5)
        .inner_radius_ratio(0.35)
        .rotation(15.);
    star.side = BorderSide::solid(Color::BLACK, 6.);
    let star: Rc<dyn ShapeBorder> = Rc::new(star);
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(120., 80.));

    for (anchor, progress, width, outer, inner) in [
        (&stadium, 0.25, 2.5, STADIUM_OUTER, STADIUM_INNER),
        (&stadium, 0.75, 5.5, STAR_OUTER, STAR_INNER),
        (
            &rectangle,
            1. / 6.,
            7. / 3.,
            "01111101111111111111111111111111111",
            "01111101111111111111111111110111110",
        ),
        (&rectangle, 0.5, 4., STADIUM_OUTER, STADIUM_INNER),
        (&rectangle, 5. / 6., 17. / 3., STAR_OUTER, STAR_INNER),
    ] {
        for (a, b, t) in [(anchor, &star, progress), (&star, anchor, 1. - progress)] {
            let result = lerp_shape_border(Some(a), Some(b), t).unwrap();
            close(result.dimensions(None).left, width);
            assert_eq!(sample(&result.outer_path(rect)), outer, "outer at {t}");
            assert_eq!(sample(&result.inner_path(rect)), inner, "inner at {t}");
        }
    }
}
