use incular_config::TextDirection;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{FillRule, Path};
use incular_widgets::{Border, BorderDirectional, BorderSide};

fn pair(x: f32, y: f32) -> Offset {
    Offset::new(x, y)
}

fn check_points(path: &Path, included: &[(f32, f32)], excluded: &[(f32, f32)]) {
    for (expected, points) in [(true, included), (false, excluded)] {
        for &(x, y) in points {
            assert_eq!(
                path.contains(pair(x, y), FillRule::NonZero),
                expected,
                "({x}, {y})"
            );
        }
    }
}

#[test]
fn outer_bounds_do_not_shrink_but_inner_bounds_resolve_the_stroke_side() {
    let rect = Rect::from_origin_size(pair(50.0, 60.0), Size::new(60.0, 130.0));
    let none = BorderSide::NONE;
    let top = BorderSide::solid(Color::BLACK, 10.0);
    let end = BorderSide::solid(Color::BLACK, 20.0);
    let border = Border::only(top, end, none, none);
    let directional = BorderDirectional::only(top, none, end, none);
    let outer_in = [
        (50.0, 60.0),
        (60.0, 60.0),
        (60.0, 70.0),
        (80.0, 190.0),
        (109.0, 189.0),
        (110.0, 80.0),
        (110.0, 190.0),
    ];
    let outer_out = [
        (40.0, 60.0),
        (50.0, 50.0),
        (111.0, 190.0),
        (110.0, 191.0),
        (111.0, 191.0),
        (0.0, 0.0),
        (-10.0, -10.0),
        (0.0, -10.0),
        (-10.0, 0.0),
        (1000.0, 1000.0),
    ];
    for outer in [border.outer_path(rect), directional.outer_path(rect)] {
        assert_eq!(outer.bounds(), Some(rect));
        check_points(&outer, &outer_in, &outer_out);
    }
    let common_in = [
        (70.0, 70.0),
        (70.0, 71.0),
        (71.0, 70.0),
        (71.0, 71.0),
        (80.0, 180.0),
        (80.0, 190.0),
        (89.0, 189.0),
        (90.0, 190.0),
    ];
    let common_out = [
        (40.0, 60.0),
        (50.0, 50.0),
        (50.0, 60.0),
        (60.0, 60.0),
        (0.0, 0.0),
        (-10.0, -10.0),
        (0.0, -10.0),
        (-10.0, 0.0),
        (89.0, 191.0),
        (90.0, 191.0),
        (91.0, 191.0),
        (1000.0, 1000.0),
    ];
    for path in [
        border.inner_path(rect),
        directional.inner_path(rect, TextDirection::Ltr),
    ] {
        assert_eq!(
            path.bounds(),
            Some(Rect::from_origin_size(
                pair(50.0, 70.0),
                Size::new(40.0, 120.0)
            ))
        );
        check_points(&path, &common_in, &common_out);
        check_points(
            &path,
            &[(50.0, 70.0), (55.0, 70.0), (50.0, 75.0)],
            &[
                (110.0, 80.0),
                (91.0, 189.0),
                (91.0, 190.0),
                (109.0, 189.0),
                (110.0, 190.0),
            ],
        );
    }
    let rtl = directional.inner_path(rect, TextDirection::Rtl);
    assert_eq!(
        rtl.bounds(),
        Some(Rect::from_origin_size(
            pair(70.0, 70.0),
            Size::new(40.0, 120.0)
        ))
    );
    check_points(&rtl, &common_in, &common_out);
    check_points(
        &rtl,
        &[
            (91.0, 189.0),
            (91.0, 190.0),
            (109.0, 189.0),
            (110.0, 80.0),
            (110.0, 190.0),
        ],
        &[
            (50.0, 70.0),
            (50.0, 75.0),
            (55.0, 70.0),
            (110.0, 191.0),
            (111.0, 190.0),
            (111.0, 191.0),
        ],
    );
}
