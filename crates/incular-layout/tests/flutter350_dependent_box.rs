use incular_layout::{Offset, Size, position_dependent_box};

#[test]
fn original_dependent_box_prefers_centering_and_only_clamps_when_margins_fit() {
    for (width, x, margin, expected_x) in [
        (20., 50., 0., 40.),
        (200., 50., 0., -50.),
        (200., 0., 0., -50.),
        (200., 100., 0., -50.),
        (50., 50., 20., 25.),
        (50., 50., 30., 25.),
        (50., 0., 20., 20.),
        (50., 0., 30., 25.),
        (50., 100., 20., 30.),
        (50., 100., 30., 25.),
    ] {
        assert_eq!(
            position_dependent_box(
                Size::new(100., 100.),
                Size::new(width, 10.),
                Offset::new(x, 50.),
                false,
                0.,
                margin,
            ),
            Offset::new(expected_x, 40.),
            "width={width}, x={x}, margin={margin}"
        );
    }
}

#[test]
fn vertical_fit_ties_clamping_and_horizontal_fallback_match_flutter() {
    for (target, height, prefer_below, offset, expected) in [
        ((0., 85.), 20., true, 5., (10., 60.)),
        ((100., 15.), 20., false, 5., (40., 20.)),
        ((50., 50.), 90., true, 7., (25., 57.)),
        ((50., 50.), 90., false, 7., (25., 10.)),
        ((50., 120.), 120., true, 7., (25., 90.)),
        ((50., -10.), 100., false, 7., (25., 10.)),
        ((f32::NAN, 50.), 20., true, 0., (40., 50.)),
    ] {
        assert_eq!(
            position_dependent_box(
                Size::new(100., 100.),
                Size::new(50., height),
                Offset::new(target.0, target.1),
                prefer_below,
                offset,
                10.,
            ),
            Offset::new(expected.0, expected.1),
        );
    }
}
