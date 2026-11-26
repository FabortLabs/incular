use incular_core::{Color, Lerp, Offset, Size};
use incular_rendering::{
    Brush, GradientStops, LinearGradient, PaintCommand, RadialGradient, SweepGradient,
};
use incular_widgets::{CircleBorder, Decoration, RoundedRectangleBorder, ShapeDecoration};

const RED: Color = Color::rgba(255, 0, 0, 255);
const GREEN: Color = Color::rgba(0, 255, 0, 255);

fn shape_value(value: Decoration) -> ShapeDecoration {
    match value {
        Decoration::Shape(value) => value,
        _ => panic!("the decoration must remain a shape"),
    }
}

#[test]
fn solid_and_linear_fills_interpolate_without_holding_two_fills_or_jumping() {
    let circle = ShapeDecoration::new(CircleBorder::default()).color(RED);
    let gradient = LinearGradient::new(
        Offset::ZERO,
        Offset::new(100., 0.),
        GradientStops::even(&[RED, GREEN]),
    );
    let round = ShapeDecoration::new(RoundedRectangleBorder::default()).gradient(gradient.clone());
    let solid: Decoration = circle.clone().into();
    let shaded: Decoration = round.clone().into();

    let start = shape_value(solid.lerp(&shaded, 0.));
    let end = shape_value(solid.lerp(&shaded, 1.));
    assert_eq!(start.color, Some(RED));
    assert!(start.gradient.is_none());
    assert!(std::rc::Rc::ptr_eq(&start.shape, &circle.shape));
    assert_eq!(end.gradient, Some(Brush::LinearGradient(gradient.clone())));
    assert!(std::rc::Rc::ptr_eq(&end.shape, &round.shape));

    for t in [0.1_f32, 0.25, 0.49, 0.5, 0.51, 0.75, 0.9] {
        let forward = shape_value(solid.lerp(&shaded, t));
        let reverse = shape_value(shaded.lerp(&solid, t));
        // Flutter interpolates normalized channels in double precision.
        // Round the independently computed channel only on conversion to Incular's u8.
        let green = f64::from(t);
        let red = 1.0 - green;
        let encode = |channel: f64| (channel * 255.0).round() as u8;
        for (value, expected) in [
            (forward, Color::rgba(encode(red), encode(green), 0, 255)),
            (reverse, Color::rgba(encode(green), encode(red), 0, 255)),
        ] {
            assert!(value.color.is_none());
            let Some(Brush::LinearGradient(actual)) = &value.gradient else {
                panic!("the intermediate fill must stay a linear gradient at {t}");
            };
            assert_eq!(actual.start, gradient.start);
            assert_eq!(actual.end, gradient.end);
            assert_eq!(actual.stops.as_slice()[0].color, RED);
            assert_eq!(actual.stops.as_slice()[1].color, expected);
            assert!(
                value
                    .paint_fill(Size::new(100., 80.))
                    .commands()
                    .iter()
                    .any(|cmd| {
                        matches!(
                            cmd,
                            PaintCommand::FillPath {
                                brush: Brush::LinearGradient(_),
                                ..
                            } | PaintCommand::RRect {
                                brush: Brush::LinearGradient(_),
                                ..
                            }
                        )
                    })
            );
        }
    }

    let near = shape_value(solid.lerp(&shaded, 0.001));
    let Some(Brush::LinearGradient(g)) = near.gradient else {
        panic!("must stay gradient");
    };
    assert_eq!(g.stops.as_slice().len(), 2);
    for stop in g.stops.as_slice() {
        assert!((i16::from(stop.color.red) - 255).abs() <= 1);
        assert!(stop.color.green <= 1 && stop.color.blue == 0);
    }
}

#[test]
fn radial_and_sweep_templates_keep_their_geometry_during_the_color_transition() {
    let base: Decoration = ShapeDecoration::new(CircleBorder::default())
        .color(RED)
        .into();
    let stops = GradientStops::even(&[RED, GREEN]);
    let center = Offset::new(42., 35.);
    let radial = RadialGradient::new(center, 50., stops.clone());
    let sweep = SweepGradient::new(center, 0.2, stops).with_end_angle(4.);
    let from_brush =
        |brush| Decoration::from(ShapeDecoration::new(CircleBorder::default()).gradient(brush));
    let radial_blend =
        shape_value(base.lerp(&from_brush(Brush::RadialGradient(radial.clone())), 0.25));
    let Some(Brush::RadialGradient(result)) = radial_blend.gradient else {
        panic!()
    };
    assert_eq!(
        (result.center, result.radius),
        (radial.center, radial.radius)
    );
    assert_eq!(result.stops.as_slice()[1].color, RED.lerp(&GREEN, 0.25));

    let sweep_blend =
        shape_value(base.lerp(&from_brush(Brush::SweepGradient(sweep.clone())), 0.25));
    let Some(Brush::SweepGradient(result)) = sweep_blend.gradient else {
        panic!()
    };
    assert_eq!(
        (result.center, result.start_angle, result.end_angle),
        (sweep.center, sweep.start_angle, sweep.end_angle)
    );
    assert_eq!(result.stops.as_slice()[1].color, RED.lerp(&GREEN, 0.25));
}
