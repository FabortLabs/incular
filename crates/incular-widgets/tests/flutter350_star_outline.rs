use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{FillRule, Path};
use incular_widgets::{BorderSide, ShapeBorder, StarBorder};

#[test]
fn star_outline_and_stroke_inset_match_flutter_point_samples() {
    const EXPECTED: &str = include_str!(
        "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/star-border-outlines.txt"
    );
    let rect = Rect::from_origin_size(Offset::new(7., 11.), Size::new(200., 100.));
    let sample = |path: Path| {
        let xs = [
            1.25, 19.75, 41.25, 63.75, 84.25, 101.75, 117.25, 137.75, 155.25, 177.75, 195.25,
        ];
        let ys = [2.25, 16.75, 30.25, 43.75, 58.25, 71.75, 86.25, 98.75];
        ys.into_iter()
            .map(|y| {
                xs.into_iter()
                    .map(|x| {
                        if path.contains(rect.origin + Offset::new(x, y), FillRule::NonZero) {
                            '#'
                        } else {
                            '.'
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("/")
    };
    for line in EXPECTED.lines() {
        let [name, outer, inner]: [&str; 3] = line
            .split_ascii_whitespace()
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let border = match name {
            "default" => StarBorder::new(),
            "triangle" => StarBorder::polygon(),
            "two-point-polygon" => StarBorder::polygon()
                .points(2.)
                .point_rounding(0.2)
                .squash(0.7),
            "zero-valley" => StarBorder::new()
                .inner_radius_ratio(0.)
                .points(6.)
                .rotation(27.)
                .squash(0.7),
            "inside" | "center" | "outside" => StarBorder::new()
                .points(5.2)
                .inner_radius_ratio(0.55)
                .point_rounding(0.35)
                .valley_rounding(0.4)
                .rotation(27.)
                .squash(0.6)
                .side(BorderSide {
                    stroke_align: match name {
                        "inside" => -1.,
                        "center" => 0.,
                        _ => 1.,
                    },
                    ..BorderSide::solid(Color::BLACK, 9.5)
                }),
            _ => panic!("unknown reference {name}"),
        };
        let path = border.outer_path(rect);
        assert!(
            path.contains(rect.origin + Offset::new(100., 50.), FillRule::NonZero),
            "center: {name}"
        );
        assert_eq!(sample(path), outer, "outer {name}");
        assert_eq!(sample(border.inner_path(rect)), inner, "inner {name}");
    }
}
