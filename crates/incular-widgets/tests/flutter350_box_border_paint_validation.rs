#![cfg(debug_assertions)]

use incular_config::TextDirection;
use incular_core::{Color, Size};
use incular_rendering::DisplayList;
use incular_widgets::{
    Border, BorderDirectional, BorderRadius, BorderSide, BorderStyle, BoxDecoration, BoxShape,
    Radius,
};
use std::panic::catch_unwind;

fn side(width: f32, stroke_align: f32) -> BorderSide {
    BorderSide {
        stroke_align,
        ..BorderSide::solid(Color::BLACK, width)
    }
}

fn paint(border: Border, shape: BoxShape, radius: Option<f32>) -> Option<DisplayList> {
    let mut decoration = BoxDecoration::new().shape(shape).border(border);
    if let Some(radius) = radius {
        decoration = decoration.border_radius(BorderRadius::all(Radius::circular(radius)));
    }
    decoration.paint_border(Size::new(40.0, 50.0))
}

fn rejects(border: Border, shape: BoxShape, radius: Option<f32>, diagnostic: &str) {
    let error = catch_unwind(|| paint(border, shape, radius))
        .expect_err("unsupported border configuration silently painted");
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .expect("border error should carry a diagnostic");
    assert!(message.contains(diagnostic), "{message:?}");
}

fn accepts(border: Border, shape: BoxShape, radius: Option<f32>) {
    let list = paint(border, shape, radius).expect("valid border did not paint");
    assert!(!list.commands().is_empty());
}

#[test]
fn rectangular_multicolor_border_requires_inside_alignment() {
    let border = Border::builder()
        .left(BorderSide {
            color: Color::rgba(0, 0, 1, 255),
            ..side(1.0, 0.0)
        })
        .right(BorderSide {
            color: Color::rgba(0, 0, 2, 255),
            ..side(1.0, 1.0)
        })
        .build();
    rejects(border, BoxShape::Rectangle, None, "stroke alignment");
    let inside = Border {
        left: BorderSide {
            stroke_align: -1.0,
            ..border.left
        },
        right: BorderSide {
            stroke_align: -1.0,
            ..border.right
        },
        ..border
    };
    accepts(inside, BoxShape::Rectangle, None);
}

#[test]
fn nonuniform_border_variations_validate_at_the_paint_boundary() {
    let rect = BoxShape::Rectangle;
    let circle = BoxShape::Circle;
    let mixed = Border::only(
        side(10.0, 0.0),
        side(15.0, 1.0),
        side(20.0, -1.0),
        side(5.0, -1.0),
    );
    rejects(mixed, rect, None, "stroke alignment");
    accepts(mixed, rect, Some(25.0));
    accepts(mixed, circle, None);

    let invisible_left = Border::only(
        side(10.0, -1.0),
        side(15.0, -1.0),
        side(20.0, -1.0),
        BorderSide::new(Color::BLACK, 5.0, BorderStyle::None),
    );
    accepts(invisible_left, rect, Some(25.0));

    let off_color = Border {
        left: BorderSide {
            color: Color::rgba(0x12, 0x34, 0x56, 255),
            style: BorderStyle::Solid,
            ..invisible_left.left
        },
        ..invisible_left
    };
    rejects(off_color, rect, Some(20.0), "uniform visible colors");

    let hairline = Border::builder().bottom(side(0.0, -1.0)).build();
    accepts(hairline, rect, Some(0.0));
    rejects(hairline, rect, Some(40.0), "hairline");

    let directional = BorderDirectional::only(mixed.top, mixed.left, mixed.right, mixed.bottom);
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        let resolved = directional.resolve(direction);
        rejects(resolved, rect, None, "stroke alignment");
        accepts(resolved, rect, Some(25.0));
        accepts(resolved, circle, None);
    }
}
