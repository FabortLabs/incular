//! Shader descriptors observed at retained BoxDecoration and ShapeDecoration paint boundaries.
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use incular_core::{Color, Lerp, Offset, Size};
use incular_rendering::{
    Brush, DisplayList, GradientStop, GradientStops, LinearGradient, PaintCommand, RadialGradient,
    SweepGradient, TileMode,
};
use incular_widgets::{Border, BoxDecoration, ShapeDecoration};

const RECT: Size = Size {
    width: 100.,
    height: 40.,
};

fn stops(colors: &[Color]) -> GradientStops {
    GradientStops::from_colors(colors, None).unwrap()
}

fn box_paint(decoration: BoxDecoration) -> Brush {
    brush(&decoration.paint_background(RECT).unwrap())
}

fn brush(display: &DisplayList) -> Brush {
    display
        .commands()
        .iter()
        .find_map(|cmd| match cmd {
            PaintCommand::RRect { brush, .. } | PaintCommand::FillPath { brush, .. } => {
                Some(brush.clone())
            }
            _ => None,
        })
        .expect("an actual brush paint command")
}

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.0001, "{a} != {b}");
}

#[test]
fn raw_linear_geometry_and_stop_union_reach_box_and_shape_paint() {
    let a = Brush::LinearGradient(LinearGradient::new(
        Offset::new(0., 20.),
        Offset::new(100., 20.),
        stops(&[Color::BLACK, Color::WHITE]),
    ));
    let b = Brush::LinearGradient(LinearGradient {
        start: Offset::new(50., 0.),
        end: Offset::new(50., 40.),
        stops: stops(&[Color::BLACK, Color::BLACK, Color::WHITE]),
        tile_mode: TileMode::Mirror,
    });
    let a_box = BoxDecoration::new().gradient(a.clone());
    let b_box = BoxDecoration::new().gradient(b.clone());
    let result = box_paint(a_box.lerp(&b_box, 0.25));
    let Brush::LinearGradient(g) = &result else {
        panic!("linear")
    };
    assert_eq!(g.start, Offset::new(12.5, 15.));
    assert_eq!(g.end, Offset::new(87.5, 25.));
    assert_eq!(g.tile_mode, TileMode::Clamp);
    assert_eq!(
        &g.stops.as_slice()[1],
        &GradientStop {
            offset: 0.5,
            color: Color::rgba(96, 96, 96, 255),
        }
    );
    assert_eq!(g.stops.as_slice().len(), 3);
    let Brush::LinearGradient(midpoint) = box_paint(a_box.lerp(&b_box, 0.5)) else {
        panic!("linear")
    };
    assert_eq!(midpoint.tile_mode, TileMode::Mirror);
    let a_shape = ShapeDecoration::new(Border::default()).gradient(a);
    let b_shape = ShapeDecoration::new(Border::default()).gradient(b);
    assert_eq!(brush(&a_shape.lerp(&b_shape, 0.25).paint(RECT)), result);
}

#[test]
fn absolute_radial_and_sweep_geometry_interpolate_without_changing_class() {
    let radial_a = Brush::RadialGradient(RadialGradient {
        center: Offset::ZERO,
        radius: 10.,
        focal: Some(Offset::new(0., 40.)),
        focal_radius: 2.5,
        stops: stops(&[Color::BLACK, Color::WHITE]),
        tile_mode: TileMode::Clamp,
    });
    let radial_b = Brush::RadialGradient(RadialGradient {
        center: Offset::new(100., 40.),
        radius: 30.,
        focal: Some(Offset::new(100., 0.)),
        focal_radius: 7.5,
        stops: stops(&[Color::WHITE, Color::BLACK]),
        tile_mode: TileMode::Clamp,
    });
    let Brush::RadialGradient(radial) = box_paint(
        BoxDecoration::new()
            .gradient(radial_a)
            .lerp(&BoxDecoration::new().gradient(radial_b), 0.25),
    ) else {
        panic!("radial")
    };
    assert_eq!(radial.center, Offset::new(25., 10.));
    assert_eq!(radial.focal, Some(Offset::new(25., 30.)));
    assert_eq!((radial.radius, radial.focal_radius), (15., 3.75));

    let sweep_a = Brush::SweepGradient(
        SweepGradient::new(
            Offset::new(50., 20.),
            0.,
            stops(&[Color::BLACK, Color::WHITE]),
        )
        .with_end_angle(PI),
    );
    let sweep_b = Brush::SweepGradient(
        SweepGradient::new(
            Offset::new(100., 40.),
            FRAC_PI_2,
            stops(&[Color::WHITE, Color::BLACK]),
        )
        .with_end_angle(TAU),
    );
    let Brush::SweepGradient(sweep) = box_paint(
        BoxDecoration::new()
            .gradient(sweep_a)
            .lerp(&BoxDecoration::new().gradient(sweep_b), 0.25),
    ) else {
        panic!("sweep")
    };
    assert_eq!(sweep.center, Offset::new(62.5, 25.));
    close(sweep.start_angle, PI / 8.);
    close(sweep.end_angle.unwrap(), PI * 1.25);
}

#[test]
fn different_raw_classes_fade_to_transparency_and_null_scales_the_existing_family() {
    let linear = BoxDecoration::new().gradient(LinearGradient::new(
        Offset::ZERO,
        Offset::new(80., 0.),
        stops(&[Color::BLACK, Color::WHITE]),
    ));
    let radial = BoxDecoration::new().gradient(RadialGradient::new(
        Offset::new(50., 20.),
        20.,
        stops(&[Color::WHITE, Color::BLACK]),
    ));
    for (fraction, alpha, uses_radial) in [(0.25, 128, false), (0.5, 0, true), (0.75, 128, true)] {
        let g = box_paint(linear.lerp(&radial, fraction));
        assert_eq!(matches!(g, Brush::RadialGradient(_)), uses_radial);
        let stops = match g {
            Brush::LinearGradient(g) => g.stops,
            Brush::RadialGradient(g) => g.stops,
            _ => panic!("crossfade must keep a gradient"),
        };
        assert!(
            stops
                .as_slice()
                .iter()
                .all(|stop| stop.color.alpha == alpha)
        );
    }
    let Brush::LinearGradient(g) = box_paint(BoxDecoration::new().lerp(&linear, 0.25)) else {
        panic!("a missing shader scales opacity rather than replacing its class");
    };
    assert_eq!(g.stops.as_slice()[0].color.alpha, 64);
}

#[test]
fn mixing_an_implicit_full_turn_and_an_explicit_sweep_preserves_the_branch_at_each_end() {
    let a = SweepGradient::new(
        Offset::ZERO,
        FRAC_PI_2,
        stops(&[Color::BLACK, Color::WHITE]),
    );
    let b = SweepGradient::new(
        Offset::ZERO,
        FRAC_PI_4,
        stops(&[Color::WHITE, Color::BLACK]),
    )
    .with_end_angle(PI);
    let before = Brush::SweepGradient(a.clone());
    let after = Brush::SweepGradient(b.clone());
    let sample = Offset::new(-1., 0.5);
    for (fraction, endpoint) in [(0.00001, a), (0.99999, b)] {
        let Brush::SweepGradient(gradient) =
            before.interpolate_same_kind(&after, fraction).unwrap()
        else {
            panic!("sweep")
        };
        close(gradient.parameter(sample), endpoint.parameter(sample));
    }
}
