use incular_config::{EdgeInsets, EdgeInsetsDirectional, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::PaintCommand;
use incular_widgets::{
    Border, BorderDirectional, BorderSide, BorderStyle, BoxBorder, BoxDecoration,
};

fn side(width: f32) -> BorderSide {
    BorderSide::solid(Color::BLACK, width)
}

fn aligned(width: f32, stroke_align: f32) -> BorderSide {
    BorderSide {
        stroke_align,
        ..side(width)
    }
}

#[test]
fn scaling_each_side_preserves_color_and_can_remove_the_stroke() {
    let magenta = Color::rgba(255, 0, 255, 255);
    let yellow = Color::rgba(255, 255, 0, 255);
    let b3 = Border::builder()
        .left(BorderSide::solid(magenta, 3.0))
        .build();
    let b6 = Border::builder()
        .left(BorderSide::solid(magenta, 6.0))
        .build();
    let no_yellow = Border::builder()
        .top(BorderSide::new(yellow, 0.0, BorderStyle::None))
        .build();
    let yellow_2 = Border::builder()
        .top(BorderSide::solid(yellow, 2.0))
        .build();

    assert_eq!(b3.scale(2.0), b6);
    assert_eq!(no_yellow.scale(3.0), no_yellow);
    assert_eq!(yellow_2.scale(0.0), no_yellow);
}

#[test]
fn dimensions_keep_the_physical_edge_order() {
    let border = Border::only(side(3.0), side(7.0), side(5.0), side(2.0));
    assert_eq!(border.dimensions(), EdgeInsets::only(2.0, 3.0, 7.0, 5.0));
}

#[test]
fn an_explicit_hairline_is_not_an_omitted_border_side() {
    let default = BorderDirectional::default();
    let built = BorderDirectional::builder().build();
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        for empty in [default, built] {
            let border = empty.resolve(direction);
            assert_eq!(border, Border::all(BorderSide::NONE));
            let size = Size::new(20.0, 10.0);
            assert!(
                BoxDecoration::new()
                    .border(border)
                    .paint_border(size)
                    .is_none()
            );
            assert_eq!(border.dimensions(), EdgeInsets::ZERO);
        }

        let hairline = BorderDirectional::builder()
            .start(side(0.0))
            .build()
            .resolve(direction);
        assert_ne!(hairline, Border::all(BorderSide::NONE));
        let strokes = BoxDecoration::new()
            .border(hairline)
            .paint_border(Size::new(20.0, 10.0))
            .expect("explicit hairline must be painted");
        let [PaintCommand::StrokePath { path, stroke, .. }] = strokes.commands() else {
            panic!(
                "expected exactly the leading stroke, got {:?}",
                strokes.commands()
            );
        };
        assert_eq!(stroke.width, 0.0);
        let leading_x = if direction == TextDirection::Ltr {
            0.0
        } else {
            20.0
        };
        assert_eq!(
            path.bounds(),
            Some(Rect::from_origin_size(
                Offset::new(leading_x, 0.0),
                Size::new(0.0, 10.0)
            )),
        );
    }
}

#[test]
fn dimensions_count_only_the_inset_of_each_physical_or_directional_stroke() {
    for (alignment, inset) in [(-1.0, 10.0), (0.0, 5.0), (1.0, 0.0)] {
        let side = aligned(10.0, alignment);
        assert_eq!(Border::all(side).dimensions(), EdgeInsets::all(inset));
        assert_eq!(
            BorderDirectional::only(side, side, side, side).dimensions(),
            EdgeInsetsDirectional::all(inset),
        );
    }
    for (leading, trailing, bottom, expected_leading, expected_bottom) in
        [(5.0, 15.0, 20.0, 5.0, 20.0), (10.0, 10.0, 10.0, 10.0, 10.0)]
    {
        let top = aligned(10.0, 0.0);
        let start = side(leading);
        let end = aligned(trailing, 1.0);
        let bottom = side(bottom);
        assert_eq!(
            Border::only(top, end, bottom, start).dimensions(),
            EdgeInsets::only(expected_leading, 5.0, 0.0, expected_bottom),
        );
        assert_eq!(
            BorderDirectional::only(top, start, end, bottom).dimensions(),
            EdgeInsetsDirectional::only(expected_leading, 5.0, 0.0, expected_bottom),
        );
    }
}

#[test]
fn uniformity_requires_equal_width_color_style_and_alignment() {
    let white = Color::WHITE;
    let almost = Color::rgba(255, 255, 254, 255);
    let none = BorderSide::new(Color::BLACK, 1.0, BorderStyle::None);
    let omitted = BorderSide::NONE;
    let baseline = side(1.0);
    let all_equal = Border::all(side(3.0));
    for (border, expected) in [
        (
            Border {
                bottom: side(3.1),
                ..all_equal
            },
            false,
        ),
        (all_equal, true),
        (
            Border {
                left: BorderSide::solid(almost, 1.0),
                ..Border::new(1.0, white)
            },
            false,
        ),
        (Border::new(1.0, white), true),
        (Border::only(none, none, side(0.0), none), false),
        (Border::only(none, none, omitted, none), false),
        (
            Border::only(aligned(1.0, 0.0), aligned(1.0, 1.0), omitted, baseline),
            false,
        ),
        (Border::all(omitted), true),
    ] {
        assert_eq!(border.is_uniform(), expected, "{border:?}");
    }
}

#[test]
fn merging_physical_and_directional_borders_combines_only_compatible_sides() {
    let magenta_3 = BorderSide::solid(Color::rgba(255, 0, 255, 255), 3.0);
    let magenta_6 = BorderSide {
        width: 6.0,
        ..magenta_3
    };
    let yellow_2 = BorderSide::solid(Color::rgba(255, 255, 0, 255), 2.0);
    let yellow_absent = BorderSide {
        width: 0.0,
        style: BorderStyle::None,
        ..yellow_2
    };
    let none = BorderSide::NONE;
    let physical =
        |[top, end, bottom, start]: [BorderSide; 4]| Border::only(top, end, bottom, start);
    let directional = |[top, end, bottom, start]: [BorderSide; 4]| {
        BorderDirectional::only(top, start, end, bottom)
    };
    for (a, b, expected) in [
        (
            [yellow_2, none, none, none],
            [none, magenta_3, none, none],
            Some([yellow_2, magenta_3, none, none]),
        ),
        (
            [none, none, magenta_3, none],
            [none, none, magenta_3, none],
            Some([none, none, magenta_6, none]),
        ),
        (
            [none, yellow_absent, none, magenta_3],
            [none, yellow_2, none, none],
            Some([none, yellow_2, none, magenta_3]),
        ),
        ([none; 4], [none; 4], Some([none; 4])),
        (
            [none, none, none, magenta_3],
            [none, none, none, yellow_2],
            None,
        ),
    ] {
        let merged = std::panic::catch_unwind(|| physical(a).merge(physical(b)));
        assert_eq!(merged.ok(), expected.map(physical));
        let merged = std::panic::catch_unwind(|| directional(a).merge(directional(b)));
        assert_eq!(merged.ok(), expected.map(directional));
    }
}

#[test]
fn directional_dimensions_and_scaling_preserve_logical_edge_order() {
    let test_directional = BorderDirectional::only(side(3.0), side(2.0), side(7.0), side(5.0));
    assert_eq!(
        test_directional.dimensions(),
        EdgeInsetsDirectional::only(2.0, 3.0, 7.0, 5.0)
    );
    let magenta = Color::rgba(255, 0, 255, 255);
    let yellow = Color::rgba(255, 255, 0, 255);
    let b3 = BorderDirectional::builder()
        .start(BorderSide::solid(magenta, 3.0))
        .build();
    let b6 = BorderDirectional::builder()
        .start(BorderSide::solid(magenta, 6.0))
        .build();
    let no_yellow = BorderDirectional::builder()
        .top(BorderSide::new(yellow, 0.0, BorderStyle::None))
        .build();
    let yellow_2 = BorderDirectional::builder()
        .top(BorderSide::solid(yellow, 2.0))
        .build();
    assert_eq!(b3.scale(2.0), b6);
    assert_eq!(no_yellow.scale(3.0), no_yellow);
    assert_eq!(yellow_2.scale(0.0), no_yellow);
}

#[test]
fn directional_uniformity_requires_equal_values_on_every_side() {
    let w = BorderSide::solid(Color::WHITE, 1.0);
    let almost_w = BorderSide::solid(Color::rgba(255, 255, 254, 255), 1.0);
    let none = BorderSide::new(Color::BLACK, 1.0, BorderStyle::None);
    let absent = BorderSide::NONE;
    for ([top, start, end, bottom], expected) in [
        ([side(3.0), side(3.0), side(3.0), side(3.1)], false),
        ([side(3.0); 4], true),
        ([w, almost_w, w, w], false),
        ([w; 4], true),
        ([none, none, none, side(0.0)], false),
        ([none, none, none, absent], false),
        ([absent; 4], true),
        ([side(1.0), side(1.0), aligned(1.0, 0.0), side(1.0)], false),
    ] {
        assert_eq!(
            BorderDirectional::only(top, start, end, bottom).is_uniform(),
            expected
        );
    }
}

#[test]
fn a_uniform_border_copies_the_entire_side_to_each_edge() {
    let default = BorderSide::default();
    assert_eq!(default, BorderSide::solid(Color::BLACK, 1.0));
    for side in [
        default,
        BorderSide {
            stroke_align: 1.0,
            ..BorderSide::solid(Color::WHITE, 3.0)
        },
    ] {
        let border = Border::from_border_side(side);
        assert_eq!(
            [border.left, border.top, border.right, border.bottom],
            [side; 4]
        );
    }
}

#[test]
fn box_border_factories_keep_physical_and_directional_edge_order() {
    let [a, b, c, d] = [1., 2., 3., 4.].map(side);
    let physical = BoxBorder::from_ltrb(a, b, c, d);
    assert_eq!(physical, Border::only(b, c, d, a));
    assert_eq!(physical.dimensions(), EdgeInsets::only(1., 2., 3., 4.));
    assert_eq!(BoxBorder::all(d), Border::new(4., Color::BLACK));
    assert_eq!(BoxBorder::from_border_side(c), Border::all(c));
    assert_eq!(BoxBorder::symmetric(c, b), Border::only(b, c, b, c));

    let directional = BoxBorder::from_steb(a, b, c, d);
    assert_eq!(directional, BorderDirectional::only(b, a, c, d));
    assert_eq!(directional.resolve(TextDirection::Ltr), physical);
    assert_eq!(
        directional.resolve(TextDirection::Rtl),
        Border::from_ltrb(c, b, a, d)
    );
}

#[test]
fn constructing_a_border_side_accepts_signed_zero_and_rejects_negative_width_in_debug() {
    let black_solid = |width| BorderSide::new(Color::BLACK, width, BorderStyle::Solid);
    assert_eq!(BorderSide::default(), black_solid(1.0));
    assert_eq!(black_solid(-0.0), black_solid(0.0));
    assert_eq!(BorderSide::builder().width(-0.0).build(), black_solid(0.0));
    #[cfg(debug_assertions)]
    {
        assert!(std::panic::catch_unwind(|| black_solid(-1.0)).is_err());
        assert!(std::panic::catch_unwind(|| BorderSide::builder().width(-1.0).build()).is_err());
        assert!(std::panic::catch_unwind(|| black_solid(f32::NAN)).is_err());
    }
}

#[test]
fn adding_physical_borders_merges_compatible_sides_and_layers_conflicts() {
    use incular_rendering::{Brush, Canvas, FillRule};
    use incular_widgets::{BorderComposition, ShapeBorder};

    let magenta = Color::rgba(255, 0, 255, 255);
    let yellow = Color::rgba(255, 255, 0, 255);
    let m3 = BorderSide::solid(magenta, 3.);
    let m6 = BorderSide::solid(magenta, 6.);
    let y2 = BorderSide::solid(yellow, 2.);
    let y0 = BorderSide::new(yellow, 0., BorderStyle::None);
    let none = Border::default();

    let top_y2 = Border::builder().top(y2).build();
    let right_m3 = Border::builder().right(m3).build();
    assert_eq!(
        top_y2 + right_m3,
        BorderComposition::Merged(Border::builder().top(y2).right(m3).build())
    );
    let bottom_m3 = Border::builder().bottom(m3).build();
    assert_eq!(
        bottom_m3 + bottom_m3,
        BorderComposition::Merged(Border::builder().bottom(m6).build())
    );
    let left_m3 = Border::builder().left(m3).build();
    let right_y2 = Border::builder().right(y2).build();
    assert_eq!(
        Border::builder().left(m3).right(y0).build() + right_y2,
        BorderComposition::Merged(Border::builder().left(m3).right(y2).build())
    );
    let top_m3 = Border::builder().top(m3).build();
    assert_eq!(
        top_m3 + top_m3,
        BorderComposition::Merged(Border::builder().top(m6).build())
    );
    let top_y0 = Border::builder().top(y0).build();
    for (a, b) in [
        (none, none),
        (top_y0, top_y0),
        (top_y0, none),
        (none, top_y0),
    ] {
        assert_eq!(a + b, BorderComposition::Merged(none));
    }

    let left_y2 = Border::builder().left(y2).build();
    let compound = left_m3 + left_y2;
    assert_eq!(
        compound,
        BorderComposition::Layered {
            outside: left_y2,
            inside: left_m3
        }
    );
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(30., 20.));
    let inner = ShapeBorder::inner_path(&compound, rect);
    assert_eq!(
        inner.bounds(),
        Some(Rect::from_origin_size(
            Offset::new(5., 0.),
            Size::new(25., 20.)
        ))
    );
    let mut canvas = Canvas::default();
    ShapeBorder::paint(&compound, &mut canvas, rect);
    let display = canvas.finish();
    let [
        PaintCommand::FillPath {
            path: outside,
            brush: Brush::Solid(outer),
            ..
        },
        PaintCommand::FillPath {
            path: inside,
            brush: Brush::Solid(inner),
            ..
        },
    ] = display.commands()
    else {
        panic!(
            "expected two distinct painted sides: {:?}",
            display.commands()
        );
    };
    assert_eq!((*outer, *inner), (yellow, magenta));
    for (path, x, expected) in [
        (outside, 1., true),
        (outside, 3., false),
        (inside, 1., false),
        (inside, 3., true),
    ] {
        assert_eq!(
            path.contains(Offset::new(x, 10.), FillRule::NonZero),
            expected
        );
    }
}
