use incular_config::{Alignment, AlignmentDirectional, Constraints, TextDirection};
use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{
    Brush, GradientStop, GradientStops, GradientStopsError, PaintCommand, TileMode,
};
use incular_widgets::{
    BoxDecoration, BoxLinearGradient, BoxRadialGradient, DecoratedBox, SizedBox,
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

fn validate(grad: &BoxRadialGradient, offsets: &[f32], channels: &[u8]) {
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
fn radial_gradients_interpolate_original_stop_matrices() {
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
        let mut a = BoxRadialGradient::new(a);
        let mut b = BoxRadialGradient::new(b);
        let (expected_center, expected_radius) = if index < 2 {
            a = a.center(Alignment::TOP_LEFT).radius(20.);
            b = b.center(Alignment::TOP_RIGHT).radius(10.);
            (Alignment::TOP_CENTER, 15.)
        } else {
            (Alignment::CENTER, 0.5)
        };
        let current = a.lerp(&b, 0.5);
        assert_eq!(current.center, expected_center.into());
        assert_eq!(current.radius, expected_radius);
        assert_eq!(current.focal, None);
        assert_eq!(current.focal_radius, 0.);
        validate(&current, offsets, colors);
    }
}

#[test]
fn focal_alignment_and_radii_interpolate_then_lower_to_shortest_side() {
    let a = BoxRadialGradient::new(even(&[0x33, 0x66]))
        .center(Alignment::TOP_LEFT)
        .radius(20.)
        .focal(Alignment::CENTER_LEFT, 10.);
    let b = BoxRadialGradient::new(even(&[0x44, 0x88]))
        .center(Alignment::TOP_RIGHT)
        .radius(10.)
        .focal(Alignment::CENTER_RIGHT, 5.);
    let no_focal = BoxRadialGradient {
        focal: None,
        focal_radius: 0.,
        ..b.clone()
    };
    let m = a.lerp(&b, 0.5);
    assert_eq!(m.center, Alignment::TOP_CENTER.into());
    assert_eq!(m.focal, Some(Alignment::CENTER.into()));
    assert_eq!((m.radius, m.focal_radius), (15., 7.5));
    validate(&m, &[0., 1.], &[0x3b, 0x77]);
    let m2 = a.lerp(&no_focal, 0.5);
    assert_eq!(m2.focal, Some(Alignment::new(-0.5, 0.).into()));
    assert_eq!((m2.radius, m2.focal_radius), (15., 5.));
    validate(&m2, &[0., 1.], &[0x3b, 0x77]);
    let resolved = m2.resolve(
        Rect::from_origin_size(Offset::new(10., 20.), Size::new(100., 60.)),
        TextDirection::Ltr,
    );
    assert_eq!(resolved.center, Offset::new(60., 20.));
    assert_eq!(resolved.focal, Some(Offset::new(35., 50.)));
    assert_eq!((resolved.radius, resolved.focal_radius), (900., 300.));
    let beyond = a.lerp(&b, 3.);
    assert_eq!((beyond.radius, beyond.focal_radius), (0., 0.));
}

#[test]
fn radial_rotation_identity_and_color_operations_preserve_geometry() {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
    let a = BoxRadialGradient::new(even(&[0x33, 0x66])).rotated(FRAC_PI_4);
    let b = BoxRadialGradient::new(even(&[0x33, 0x66])).rotated(FRAC_PI_2);
    assert_eq!(a.lerp(&b, 0.), a);
    assert_eq!(a.lerp(&b, 0.5), b);
    assert_eq!(a.lerp(&b, 1.), b);
    assert_eq!(a.lerp(&a, 0.5).stops.id(), a.stops.id());
    assert!(BoxRadialGradient::lerp_optional(None, None, 0.).is_none());

    let channels = [Color::WHITE, Color::rgba(119, 119, 119, 175), gray(0x44)];
    for transform in [None, Some(1.)] {
        let a = BoxRadialGradient {
            rotation: transform,
            ..BoxRadialGradient::new(GradientStops::even(&channels))
                .center(Alignment::TOP_LEFT)
                .radius(20.)
                .focal(Alignment::CENTER_LEFT, 10.)
        };
        let changed = a.with_opacity(0.5);
        assert_eq!(
            changed,
            BoxRadialGradient {
                stops: GradientStops::even(&channels.map(|color| Color {
                    alpha: 128,
                    ..color
                })),
                ..a
            }
        );
    }
    let green = Color::rgba(0, 255, 0, 255);
    let original = BoxRadialGradient {
        tile_mode: TileMode::Mirror,
        ..BoxRadialGradient::new(even(&[0xff, 0x44]))
            .center(Alignment::TOP_LEFT)
            .radius(0.25)
            .focal(Alignment::CENTER_RIGHT, 0.1)
            .rotated(1.)
    };
    assert_eq!(
        original.from_color(green),
        BoxRadialGradient {
            stops: GradientStops::even(&[green; 2]),
            ..original
        }
    );
    let rgb = [
        Color::rgba(255, 0, 0, 255),
        Color::rgba(0, 255, 0, 255),
        Color::rgba(0, 0, 255, 255),
    ];
    let original = BoxRadialGradient {
        tile_mode: TileMode::Decal,
        ..BoxRadialGradient::new(GradientStops::even(&rgb))
            .center(Alignment::TOP_LEFT)
            .radius(20.)
            .focal(Alignment::CENTER_LEFT, 10.)
            .rotated(FRAC_PI_4)
    };
    assert_eq!(
        original.scale(0.5),
        BoxRadialGradient {
            stops: GradientStops::even(&rgb.map(|color| Color {
                alpha: 128,
                ..color
            })),
            ..original
        }
    );
}

#[test]
fn directional_radial_shader_requires_direction_but_physical_does_not() {
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(100., 100.));
    let logical = BoxRadialGradient::new(GradientStops::even(&[Color::WHITE; 2]))
        .center(AlignmentDirectional::TOP_START);
    assert!(matches!(
        logical.create_shader(rect, None),
        Err(TreeError::MissingDirectionality { .. })
    ));
    assert_eq!(
        logical
            .create_shader(rect, Some(TextDirection::Ltr))
            .unwrap()
            .center,
        Offset::ZERO
    );
    assert_eq!(
        logical
            .create_shader(rect, Some(TextDirection::Rtl))
            .unwrap()
            .center,
        Offset::new(100., 0.)
    );
    let physical = BoxRadialGradient {
        center: Alignment::TOP_LEFT.into(),
        ..logical
    };
    assert_eq!(
        physical.create_shader(rect, None).unwrap().center,
        Offset::ZERO
    );
}

#[test]
fn crossing_gradient_kinds_fades_to_transparent_and_paints_radial_geometry() {
    let source = BoxLinearGradient::new(
        Alignment::TOP_LEFT,
        Alignment::BOTTOM_RIGHT,
        even(&[0x33, 0x66]),
    );
    let target = BoxRadialGradient::new(even(&[0x44, 0x88])).radius(20.);
    let a = BoxDecoration::new().aligned_gradient(source);
    let b = BoxDecoration::new().aligned_radial_gradient(target.clone());
    assert_eq!(a.lerp(&b, 0.), a);
    assert_eq!(a.lerp(&b, 1.), b);
    let midpoint = a.lerp(&b, 0.5);
    assert!(midpoint.aligned_gradient.is_none());
    assert_eq!(
        midpoint.aligned_radial_gradient.as_deref(),
        Some(&target.scale(0.))
    );
    let frame = Size::new(100., 60.);
    let mut tree = WidgetTree::new();
    tree.mount(
        DecoratedBox::new(SizedBox::expand())
            .decoration(midpoint)
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(frame)).unwrap();
    let paint = tree.paint();
    assert!(paint.commands().iter().any(|c| matches!(c, PaintCommand::RRect {brush:Brush::RadialGradient(shader),..} if shader.center==Offset::new(50.,30.) && shader.radius==1200. && shader.stops.as_slice()[0].color.alpha==0)));
}

#[test]
fn generic_radial_lerp_keeps_each_endpoint_and_the_same_midpoint_in_reverse() {
    let a = BoxDecoration::new().aligned_radial_gradient(
        BoxRadialGradient::new(stops(&[(0., 0x33), (1., 0x66)]))
            .center(Alignment::TOP_LEFT)
            .radius(20.),
    );
    let b = BoxDecoration::new().aligned_radial_gradient(
        BoxRadialGradient::new(stops(&[(0., 0x44), (1., 0x88)]))
            .center(Alignment::TOP_RIGHT)
            .radius(10.),
    );
    for (start, end) in [(&a, &b), (&b, &a)] {
        assert_eq!(&start.lerp(end, 0.), start);
        assert_eq!(&start.lerp(end, 1.), end);
        let midpoint = start.lerp(end, 0.5);
        let radial = midpoint.aligned_radial_gradient.as_ref().unwrap();
        assert_eq!(radial.center, Alignment::TOP_CENTER.into());
        assert_eq!(radial.radius, 15.);
        validate(radial, &[0., 1.], &[0x3b, 0x77]);
    }
}

#[test]
fn mismatched_positions_are_rejected_and_implied_positions_reach_each_shader() {
    let colors = [gray(0x11), gray(0x22), gray(0x33)];
    assert_eq!(
        GradientStops::from_colors(&colors, Some(&[0., 1.])),
        Err(GradientStopsError::LengthMismatch {
            colors: 3,
            stops: 2
        })
    );
    let implied = GradientStops::from_colors(&colors, None).unwrap();
    let rect = Rect::from_origin_size(Offset::new(1., 2.), Size::new(3., 4.));
    let linear = BoxLinearGradient::new(
        Alignment::CENTER_LEFT,
        Alignment::CENTER_RIGHT,
        implied.clone(),
    )
    .create_shader(rect, None)
    .unwrap();
    let radial = BoxRadialGradient::new(implied)
        .create_shader(rect, None)
        .unwrap();
    let expected = [
        GradientStop {
            offset: 0.,
            color: colors[0],
        },
        GradientStop {
            offset: 0.5,
            color: colors[1],
        },
        GradientStop {
            offset: 1.,
            color: colors[2],
        },
    ];
    assert_eq!(linear.stops.as_slice(), expected);
    assert_eq!(radial.stops.as_slice(), expected);
    assert_eq!(linear.start, Offset::new(1., 4.));
    assert_eq!(linear.end, Offset::new(4., 4.));
    assert_eq!(radial.center, Offset::new(2.5, 4.));
    assert_eq!(radial.radius, 1.5);
    assert_eq!(
        GradientStops::from_colors(&colors[..1], None),
        Err(GradientStopsError::TooFewColors { colors: 1 })
    );
    let custom = GradientStops::from_colors(&colors, Some(&[0.05, 0.7, 0.95])).unwrap();
    assert_eq!(
        custom
            .as_slice()
            .iter()
            .map(|stop| stop.offset)
            .collect::<Vec<_>>(),
        [0.05, 0.7, 0.95]
    );
}
