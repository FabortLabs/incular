use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::FillRule;
use incular_widgets::{
    BorderRadius, BorderSide, BorderStyle, Radius, RoundedSuperellipseBorder, ShapeBorder,
};

const DISTANCES: [f32; 11] = [
    0.35, 1.35, 3.35, 4.35, 7.35, 12.35, 20.35, 35.35, 50.35, 72.35, 100.35,
];

fn sample(rect: Rect, corner: usize, x: f32, y: f32) -> Offset {
    let right = corner == 1 || corner == 2;
    let bottom = corner >= 2;
    Offset::new(
        if right {
            rect.origin.x + rect.size.width - x
        } else {
            rect.origin.x + x
        },
        if bottom {
            rect.origin.y + rect.size.height - y
        } else {
            rect.origin.y + y
        },
    )
}

fn verify(
    contains: impl Fn(Offset) -> bool,
    rect: Rect,
    corner: usize,
    bits: &str,
    scenario: &str,
    layer: &str,
    diffs: &mut Vec<String>,
) {
    assert_eq!(bits.len(), DISTANCES.len().pow(2));
    for (&expected, (&y, &x)) in bits.as_bytes().iter().zip(
        DISTANCES
            .iter()
            .flat_map(|y| DISTANCES.iter().map(move |x| (y, x))),
    ) {
        let position = sample(rect, corner, x, y);
        let actual = contains(position);
        if actual != (expected == b'1') {
            diffs.push(format!("{scenario} {layer} corner {corner} at {position:?} (dx {x}, dy {y}): Flutter {}, native {actual}", expected == b'1'));
        }
    }
}

#[test]
fn compares_public_paths_and_hit_testing_at_all_four_corners() {
    let common = Rect::from_origin_size(Offset::new(7.25, 13.5), Size::new(120., 300.));
    let wide = Rect::from_origin_size(Offset::new(-99_200., 50.), Size::new(100_000., 500.));
    let zero = Radius::ZERO;
    let radii = [
        ("zero", common, BorderRadius::zero(), 4.),
        ("circular", common, BorderRadius::circular(36.), 4.),
        (
            "elliptical",
            common,
            BorderRadius::all(Radius::elliptical(20., 50.)),
            4.,
        ),
        ("clamp-uniform", common, BorderRadius::circular(600.), 4.),
        (
            "clamp-asymmetric",
            common,
            BorderRadius::only(
                Radius::elliptical(1000., 1000.),
                Radius::elliptical(0., 1000.),
                Radius::elliptical(800., 1000.),
                Radius::elliptical(100., 500.),
            ),
            4.,
        ),
        (
            "sharp-bottom",
            common,
            BorderRadius::only(Radius::circular(16.), Radius::circular(16.), zero, zero),
            4.,
        ),
        (
            "very-wide",
            wide,
            BorderRadius::only(zero, Radius::circular(100.), Radius::circular(30.), zero),
            1.,
        ),
    ];
    let mut reference = include_str!("fixtures/flutter350_superellipse_corner_masks.txt")
        .lines()
        .filter(|line| !line.starts_with('#'));
    let mut mismatches = Vec::new();
    for (name, rect, radius, stroke) in radii {
        let border = RoundedSuperellipseBorder::new(
            BorderSide::new(Color::BLACK, stroke, BorderStyle::Solid),
            radius,
        );
        let outer = border.outer_path(rect);
        let inner = border.inner_path(rect);
        let center = rect.origin + Offset::new(rect.size.width / 2., rect.size.height / 2.);
        assert!(outer.contains(center, FillRule::NonZero));
        assert!(inner.contains(center, FillRule::NonZero));
        for corner in 0..4 {
            for layer in ["outer", "inner", "hit"] {
                let row = reference.next().expect("missing fixture row");
                let (prefix, mask) = row.rsplit_once(' ').unwrap();
                assert_eq!(prefix, format!("{name} {layer} {corner}"));
                let contains = |point| match layer {
                    "outer" => outer.contains(point, FillRule::NonZero),
                    "inner" => inner.contains(point, FillRule::NonZero),
                    "hit" => border.hit_test(rect, point, None),
                    _ => unreachable!(),
                };
                verify(contains, rect, corner, mask, name, layer, &mut mismatches);
            }
        }
    }
    assert!(reference.next().is_none());
    assert!(
        mismatches.is_empty(),
        "{} mismatched cells:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(45)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn retained_decoration_can_paint_outside_its_analytical_hit_region() {
    use incular_config::Constraints;
    use incular_rendering::PaintCommand;
    use incular_widgets::{DecoratedShapeBox, ShapeDecoration, SizedBox, internal::WidgetTree};

    let extent = Size::new(120., 300.);
    let paint_only = Offset::new(50.35, 1.35);
    let fill = Color::rgba(120, 120, 120, 255);
    let mut tree = WidgetTree::new();
    tree.mount(
        DecoratedShapeBox::new(
            ShapeDecoration::new(RoundedSuperellipseBorder::new(
                BorderSide::NONE,
                BorderRadius::circular(600.),
            ))
            .color(fill),
            SizedBox::from_size(extent),
        )
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(extent)).unwrap();
    let scene = tree.paint();
    let path = scene
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::FillPath { path, .. } => Some(path),
            _ => None,
        })
        .expect("the real decoration painted a fill");
    assert!(path.contains(paint_only, FillRule::NonZero));
    assert!(tree.hit_test(paint_only).is_none());
    assert!(tree.hit_test(Offset::new(60., 150.)).is_some());
}
