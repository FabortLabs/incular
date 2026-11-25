use incular_config::EdgeInsets;
use incular_core::{Offset, Rect, Size};
use incular_rendering::Canvas;
use incular_widgets::{
    BeveledRectangleBorder, BorderRadius, BorderSide, CircleBorder, ContinuousRectangleBorder,
    LinearBorder, OvalBorder, RoundedRectangleBorder, RoundedSuperellipseBorder, ShapeBorder,
    StadiumBorder, StarBorder,
};

#[test]
fn an_unspecified_outline_has_no_side_and_emits_no_stroke() {
    let stadium = StadiumBorder::default();
    let rounded = RoundedRectangleBorder::default();
    let superellipse = RoundedSuperellipseBorder::default();
    let circle = CircleBorder::default();
    let oval = OvalBorder::default();
    let beveled = BeveledRectangleBorder::default();
    let continuous = ContinuousRectangleBorder::default();
    let linear = LinearBorder::default();
    let star = StarBorder::default();
    for radius in [
        rounded.border_radius,
        superellipse.border_radius,
        beveled.border_radius,
        continuous.border_radius,
    ] {
        assert_eq!(radius, BorderRadius::ZERO);
    }
    assert_eq!(linear, LinearBorder::NONE);
    assert!(!linear.prefer_paint_interior());
    let shapes: [(&str, BorderSide, &dyn ShapeBorder); 9] = [
        ("StadiumBorder", stadium.side, &stadium),
        ("RoundedRectangleBorder", rounded.side, &rounded),
        (
            "RoundedSuperellipseBorder",
            superellipse.side,
            &superellipse,
        ),
        ("CircleBorder", circle.side, &circle),
        ("OvalBorder", oval.side, &oval),
        ("BeveledRectangleBorder", beveled.side, &beveled),
        ("ContinuousRectangleBorder", continuous.side, &continuous),
        ("LinearBorder", linear.side, &linear),
        ("StarBorder", star.side, &star),
    ];
    let r = Rect::from_origin_size(Offset::ZERO, Size::new(40., 30.));
    let mismatches: Vec<_> = shapes
        .iter()
        .filter_map(|(name, side, shape)| {
            let mut canvas = Canvas::default();
            shape.paint(&mut canvas, r);
            let commands = canvas.finish().commands().len();
            assert_eq!(shape.dimensions(None), EdgeInsets::ZERO, "{name}");
            (*side != BorderSide::NONE || commands != 0).then_some((*name, *side, commands))
        })
        .collect();
    assert!(mismatches.is_empty(), "{mismatches:?}");
}
