use incular_config::{Alignment, AlignmentDirectional, Constraints, TextDirection};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{Brush, GradientStop, GradientStops, PaintCommand, TileMode};
use incular_widgets::{
    BoxDecoration, BoxRadialGradient, BoxSweepGradient, DecoratedBox, SizedBox,
    internal::{TreeError, WidgetTree},
};

fn gray(c: u8) -> Color {
    Color::rgba(c, c, c, c)
}

fn even(values: &[u8]) -> GradientStops {
    GradientStops::even(&values.iter().copied().map(gray).collect::<Vec<_>>())
}

fn stops(values: &[(f32, u8)]) -> GradientStops {
    GradientStops::new(
        values
            .iter()
            .map(|&(offset, c)| GradientStop {
                offset,
                color: gray(c),
            })
            .collect(),
    )
}

fn validate(grad: &BoxSweepGradient, offsets: &[f32], channels: &[u8]) {
    let stops = grad.stops.as_slice();
    assert_eq!(stops.len(), offsets.len());
    for ((stop, &offset), &value) in stops.iter().zip(offsets).zip(channels) {
        assert_eq!(stop.offset, offset);
        let c = stop.color;
        assert!(
            [c.red, c.green, c.blue, c.alpha]
                .iter()
                .all(|v| v.abs_diff(value) <= 1)
        );
    }
}

#[test]
fn sweep_gradients_interpolate_original_stop_matrices() {
    for (index, (a, b, offsets, colors)) in [
        (
            even(&[0x33, 0x66]),
            even(&[0x44, 0x88]),
            &[0., 1.][..],
            &[0x3b, 0x77][..],
        ),
        (
            stops(&[(0., 0x33), (0.5, 0x66)]),
            stops(&[(0.5, 0x44), (1., 0x88)]),
            &[0., 0.5, 1.][..],
            &[0x3b, 0x55, 0x77][..],
        ),
        (
            even(&[0x22, 0x66]),
            even(&[0x44, 0x66, 0x88]),
            &[0., 0.5, 1.][..],
            &[0x33, 0x55, 0x77][..],
        ),
        (
            stops(&[(0., 0x33), (0.5, 0x66)]),
            stops(&[(0.5, 0x44), (0.7, 0x48), (1., 0x88)]),
            &[0., 0.5, 0.7, 1.][..],
            &[0x3b, 0x55, 0x57, 0x77][..],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut a = BoxSweepGradient::new(a);
        let mut b = BoxSweepGradient::new(b);
        let (expected_center, expected_start, expected_end) = if index < 2 {
            a = a.center(Alignment::TOP_LEFT).angles(0., FRAC_PI_2);
            b = b.center(Alignment::TOP_RIGHT).angles(FRAC_PI_2, PI);
            (Alignment::TOP_CENTER, FRAC_PI_4, 3. * FRAC_PI_4)
        } else {
            (Alignment::CENTER, 0., TAU)
        };
        let current = a.lerp(&b, 0.5);
        assert_eq!(current.center, expected_center.into());
        assert_eq!(
            (current.start_angle, current.end_angle),
            (expected_start, expected_end)
        );
        assert_eq!(current.tile_mode, TileMode::Clamp);
        assert_eq!(current.rotation, None);
        validate(&current, offsets, colors);
    }
}

#[test]
fn sweep_rotation_and_tiles_select_the_second_endpoint_at_the_midpoint() {
    let a = BoxSweepGradient::new(stops(&[(0., 0x33), (1., 0x66)])).rotated(FRAC_PI_4);
    let b = BoxSweepGradient::new(stops(&[(0., 0x33), (1., 0x66)])).rotated(FRAC_PI_2);
    assert_eq!(a.lerp(&b, 0.), a);
    assert_eq!(a.lerp(&b, 1.), b);
    assert_eq!(a.lerp(&b, 0.5), b);
    assert!(BoxSweepGradient::lerp_optional(None, None, 0.).is_none());
    assert_eq!(
        BoxSweepGradient::lerp_optional(Some(&a), Some(&a), 0.5).as_ref(),
        Some(&a)
    );

    let a = BoxSweepGradient {
        tile_mode: TileMode::Mirror,
        ..a.angles(0., FRAC_PI_4)
    };
    let b = BoxSweepGradient {
        tile_mode: TileMode::Decal,
        ..b.angles(FRAC_PI_2, PI)
    };
    assert_eq!(a.lerp(&b, 0.499).tile_mode, TileMode::Mirror);
    assert_eq!(a.lerp(&b, 0.5).tile_mode, TileMode::Decal);
    let extrapolated = a.lerp(&b, -2.);
    assert_eq!((extrapolated.start_angle, extrapolated.end_angle), (0., 0.));
}

#[test]
fn sweep_recolor_opacity_and_scale_preserve_each_original_descriptor() {
    let sweep = |colors: &[Color]| {
        BoxSweepGradient::new(GradientStops::even(colors))
            .center(Alignment::TOP_LEFT)
            .angles(0., FRAC_PI_2)
    };
    let green = Color::rgba(0, 255, 0, 255);
    let original = BoxSweepGradient {
        tile_mode: TileMode::Repeated,
        ..sweep(&[Color::WHITE, gray(0x44)])
            .angles(0.1, 2.)
            .rotated(1.)
    };
    assert_eq!(
        original.from_color(green),
        BoxSweepGradient {
            stops: GradientStops::even(&[green; 2]),
            ..original
        }
    );
    let colors = [
        Color::WHITE,
        Color::rgba(0x77, 0x77, 0x77, 0xaf),
        gray(0x44),
    ];
    for original in [sweep(&colors), sweep(&colors).rotated(1.)] {
        assert_eq!(
            original.with_opacity(0.5),
            BoxSweepGradient {
                stops: GradientStops::even(&colors.map(|c| Color { alpha: 128, ..c })),
                ..original
            }
        );
    }
    let first = [
        Color::rgba(0x33, 0x33, 0x33, 255),
        Color::rgba(0x66, 0x66, 0x66, 255),
    ];
    let second = [
        Color::rgba(255, 0, 0, 255),
        green,
        Color::rgba(0, 0, 255, 255),
    ];
    let rotated = BoxSweepGradient {
        tile_mode: TileMode::Decal,
        ..sweep(&second)
            .angles(FRAC_PI_4, 3. * FRAC_PI_4)
            .rotated(FRAC_PI_4)
    };
    for (original, expected) in [
        (
            sweep(&first),
            GradientStops::even(&first.map(|c| Color { alpha: 128, ..c })),
        ),
        (
            rotated,
            GradientStops::even(&second.map(|c| Color { alpha: 128, ..c })),
        ),
    ] {
        assert_eq!(
            original.scale(0.5),
            BoxSweepGradient {
                stops: expected,
                ..original
            }
        );
    }
}

#[test]
fn directional_sweep_resolves_bounds_and_decoration_paints_cross_kind_fade() {
    let rect = Rect::from_origin_size(Offset::new(10., 20.), Size::new(100., 60.));
    let sweep = BoxSweepGradient::new(even(&[0xff, 0xff]))
        .center(AlignmentDirectional::TOP_START)
        .angles(FRAC_PI_4, 3. * FRAC_PI_4);
    assert!(matches!(
        sweep.create_shader(rect, None),
        Err(TreeError::MissingDirectionality { .. })
    ));
    assert_eq!(
        sweep
            .create_shader(rect, Some(TextDirection::Ltr))
            .unwrap()
            .center,
        Offset::new(10., 20.)
    );
    assert_eq!(
        sweep
            .create_shader(rect, Some(TextDirection::Rtl))
            .unwrap()
            .center,
        Offset::new(110., 20.)
    );

    let target = sweep.center(Alignment::TOP_CENTER);
    let a =
        BoxDecoration::new().aligned_radial_gradient(BoxRadialGradient::new(even(&[0xff, 0xff])));
    let b = BoxDecoration::new().aligned_sweep_gradient(target.clone());
    assert_eq!(
        a.lerp(&b, 0.25)
            .aligned_radial_gradient
            .as_ref()
            .unwrap()
            .stops
            .as_slice()[0]
            .color
            .alpha,
        128
    );
    let middle = a.lerp(&b, 0.5);
    assert!(middle.aligned_radial_gradient.is_none());
    assert_eq!(
        middle.aligned_sweep_gradient.as_deref(),
        Some(&target.scale(0.))
    );
    let mut tree = WidgetTree::new();
    tree.mount(
        DecoratedBox::new(SizedBox::expand())
            .decoration(a.lerp(&b, 0.75))
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(100., 60.)))
        .unwrap();
    assert!(
        tree.paint()
            .commands()
            .iter()
            .any(|command| matches!(command,
                PaintCommand::RRect {brush: Brush::SweepGradient(shader), ..}
                if shader.center == Offset::new(50., 0.)
                    && shader.start_angle == FRAC_PI_4
                    && shader.end_angle == Some(3. * FRAC_PI_4)
                    && shader.stops.as_slice()[0].color.alpha == 128
            ))
    );
}
