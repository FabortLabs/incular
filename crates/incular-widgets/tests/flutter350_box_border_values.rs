use incular_config::{EdgeInsets, EdgeInsetsDirectional, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::PaintCommand;
use incular_widgets::{Border, BorderDirectional, BorderSide, BorderStyle, BoxDecoration};

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
