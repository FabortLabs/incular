use incular_config::{Alignment, AlignmentDirectional, Constraints, TextDirection};
use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{Brush, GradientStop, GradientStops, PaintCommand, TileMode};
use incular_widgets::{
    BoxDecoration, BoxLinearGradient, DecoratedBox, Directionality, SizedBox, internal::WidgetTree,
};

fn gray(value: u8) -> Color {
    Color::rgba(value, value, value, value)
}

fn even(values: &[u8]) -> GradientStops {
    GradientStops::even(&values.iter().copied().map(gray).collect::<Vec<_>>())
}

fn stops(values: &[(f32, u8)]) -> GradientStops {
    GradientStops::new(
        values
            .iter()
            .map(|&(offset, value)| GradientStop {
                offset,
                color: gray(value),
            })
            .collect(),
    )
}

fn base(stops: GradientStops) -> BoxLinearGradient {
    BoxLinearGradient::new(Alignment::CENTER_LEFT, Alignment::CENTER_RIGHT, stops)
}

fn assert_stops(gradient: &BoxLinearGradient, offsets: &[f32], colors: &[u8]) {
    let actual = gradient.stops.as_slice();
    assert_eq!(actual.len(), offsets.len());
    for ((stop, &offset), &channel) in actual.iter().zip(offsets).zip(colors) {
        assert_eq!(stop.offset, offset);
        let c = stop.color;
        // Flutter stores floating point color channels; the framework
        // reference matcher accepts a one-step color granularity.
        assert!(
            [c.red, c.green, c.blue, c.alpha]
                .iter()
                .all(|v| v.abs_diff(channel) <= 1),
            "{c:?} ~ {channel:#x}"
        );
    }
}

#[test]
fn missing_endpoint_fades_opacity_without_modifying_stop_rgb_or_geometry() {
    let gradient = BoxLinearGradient::new(
        Alignment::BOTTOM_RIGHT,
        Alignment::new(0.7, 1.),
        GradientStops::even(&[
            Color::rgba(255, 255, 255, 0),
            Color::rgba(119, 119, 119, 17),
            gray(68),
        ]),
    );
    let faded = BoxLinearGradient::lerp_optional(None, Some(&gradient), 0.25).unwrap();
    assert_eq!(
        (faded.begin, faded.end, faded.tile_mode),
        (gradient.begin, gradient.end, gradient.tile_mode)
    );
    assert_eq!(
        faded
            .stops
            .as_slice()
            .iter()
            .map(|s| s.color)
            .collect::<Vec<_>>(),
        [
            Color::rgba(255, 255, 255, 0),
            Color::rgba(119, 119, 119, 4),
            Color::rgba(68, 68, 68, 17)
        ]
    );
    let same = gradient.lerp(&gradient, 0.5);
    assert_eq!(same.stops.id(), gradient.stops.id());
    assert!(BoxLinearGradient::lerp_optional(None, None, 0.5).is_none());
}

#[test]
fn linear_interpolation_moves_both_endpoints_and_resamples_four_reference_cases() {
    let geometry = |a, b, expect_offsets, expect_colors| {
        let first = BoxLinearGradient::new(Alignment::TOP_LEFT, Alignment::BOTTOM_LEFT, a);
        let second = BoxLinearGradient::new(Alignment::TOP_RIGHT, Alignment::TOP_LEFT, b);
        let middle = first.lerp(&second, 0.5);
        assert_eq!(middle.begin, Alignment::TOP_CENTER.into());
        assert_eq!(middle.end, Alignment::CENTER_LEFT.into());
        assert_stops(&middle, expect_offsets, expect_colors);
    };
    geometry(
        even(&[0x33, 0x66]),
        even(&[0x44, 0x88]),
        &[0., 1.],
        &[0x3b, 0x77],
    );
    geometry(
        stops(&[(0., 0x33), (0.5, 0x66)]),
        stops(&[(0.5, 0x44), (1., 0x88)]),
        &[0., 0.5, 1.],
        &[0x3b, 0x55, 0x77],
    );
    assert_stops(
        &base(even(&[0x22, 0x66])).lerp(&base(even(&[0x44, 0x66, 0x88])), 0.5),
        &[0., 0.5, 1.],
        &[0x33, 0x55, 0x77],
    );
    assert_stops(
        &base(stops(&[(0., 0x33), (0.5, 0x66)]))
            .lerp(&base(stops(&[(0.5, 0x44), (0.7, 0x48), (1., 0x88)])), 0.5),
        &[0., 0.5, 0.7, 1.],
        &[0x3b, 0x55, 0x57, 0x77],
    );
}

#[test]
fn actual_box_decoration_paints_interpolated_shader_in_both_directions() {
    let mut a = BoxLinearGradient::new(
        Alignment::TOP_LEFT,
        Alignment::BOTTOM_RIGHT,
        even(&[0x22, 0x66]),
    );
    a.tile_mode = TileMode::Repeated;
    let mut b = BoxLinearGradient::new(
        AlignmentDirectional::TOP_START,
        AlignmentDirectional::BOTTOM_END,
        even(&[0x44, 0x66, 0x88]),
    );
    b.tile_mode = TileMode::Mirror;
    let decoration = BoxDecoration::new()
        .aligned_gradient(a)
        .lerp(&BoxDecoration::new().aligned_gradient(b), 0.5);
    let frame = Size::new(100., 60.);
    for (direction, offset) in [
        (TextDirection::Ltr, Offset::ZERO),
        (TextDirection::Rtl, Offset::new(50., 0.)),
    ] {
        let mut tree = WidgetTree::new();
        tree.mount(
            Directionality::new(
                direction,
                DecoratedBox::new(SizedBox::expand()).decoration(decoration.clone()),
            )
            .into(),
        )
        .unwrap();
        tree.layout(Constraints::tight(frame)).unwrap();
        let paint = tree.paint();
        let shader = paint
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::RRect {
                    brush: Brush::LinearGradient(shader),
                    ..
                }
                | PaintCommand::FillPath {
                    brush: Brush::LinearGradient(shader),
                    ..
                } => Some(shader),
                _ => None,
            })
            .unwrap();
        assert_eq!(shader.start, offset);
        assert_eq!(shader.tile_mode, TileMode::Mirror);
        assert_eq!(shader.stops.as_slice()[1].color, gray(0x55));
        assert_eq!(
            *shader,
            decoration
                .aligned_gradient
                .as_ref()
                .unwrap()
                .resolve(Rect::from_origin_size(Offset::ZERO, frame), direction)
        );
    }
}
