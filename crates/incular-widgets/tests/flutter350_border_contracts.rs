//! Public border value contracts from Flutter 3.50.0 painting tests.

use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{Canvas, PaintCommand};
use incular_widgets::{Border, BorderSide, BorderStyle, BoxShape};

#[test]
fn flutter_350_border_side_merge_only_combines_compatible_sides() {
    // packages/flutter/test/painting/border_side_test.dart:14.
    let blue = Color::rgba(0, 0, 255, 255);
    let green = Color::rgba(0, 255, 0, 255);
    let solid = BorderSide::default();
    let none = BorderSide::new(Color::BLACK, 1., BorderStyle::None);
    let empty = BorderSide::NONE;
    let yellow_empty = BorderSide::new(Color::rgba(255, 255, 0, 255), 0., BorderStyle::None);
    let blue2 = BorderSide::solid(blue, 2.);
    let green2 = BorderSide::solid(green, 2.);
    let green3 = BorderSide::solid(green, 3.);
    let none2 = BorderSide::new(blue, 2., BorderStyle::None);
    let none3 = BorderSide::new(Color::BLACK, 3., BorderStyle::None);
    let side2 = BorderSide::solid(Color::BLACK, 2.);
    let side3 = BorderSide::solid(Color::BLACK, 3.);
    let side5 = BorderSide::solid(Color::BLACK, 5.);
    for (a, b, expected) in [
        (empty, empty, Some(empty)),
        (empty, side2, Some(side2)),
        (empty, yellow_empty, Some(empty)),
        (
            BorderSide::solid(green, 1.),
            BorderSide::solid(blue, 1.),
            None,
        ),
        (green2, blue2, None),
        (green2, green3, Some(BorderSide::solid(green, 5.))),
        (green2, none2, None),
        (none3, empty, Some(none3)),
        (none3, side2, None),
        (none3, yellow_empty, Some(none3)),
        (side2, empty, Some(side2)),
        (side2, none3, None),
        (side2, side3, Some(side5)),
        (side2, yellow_empty, Some(side2)),
        (side3, side2, Some(side5)),
        (solid, none, None),
        (yellow_empty, side2, Some(side2)),
        (yellow_empty, yellow_empty, Some(empty)),
    ] {
        assert_eq!(a.can_merge(&b), expected.is_some(), "{a:?} and {b:?}");
        assert_eq!(std::panic::catch_unwind(|| a.merge(&b)).ok(), expected);
    }
    let outside = BorderSide {
        stroke_align: 1.,
        ..side3
    };
    assert_eq!(
        side2.merge(&outside),
        BorderSide {
            stroke_align: 1.,
            ..side5
        }
    );
}

#[test]
fn flutter_350_border_side_scale() {
    // packages/flutter/test/painting/border_side_test.dart:79.
    let blue = Color::rgba(0, 0, 255, 255);
    let side3 = BorderSide::solid(blue, 3.);
    let side6 = BorderSide::solid(blue, 6.);
    let none = BorderSide::new(blue, 0., BorderStyle::None);
    assert_eq!(side3.scale(2.), side6);
    assert_eq!(side6.scale(0.5), side3);
    assert_eq!(side6.scale(0.), none);
    assert_eq!(side6.scale(-1.), none);
    assert_eq!(none.scale(2.), none);
}

#[test]
fn flutter_350_border_side_lerp_never_returns_negative_width() {
    // packages/flutter/test/painting/border_side_test.dart:109.
    let side0 = BorderSide::solid(Color::BLACK, 0.);
    let side1 = BorderSide::default();
    let side2 = BorderSide::solid(Color::BLACK, 2.);
    let actual = [
        side2.lerp(&side1, 10.),
        side1.lerp(&side2, -10.),
        side0.lerp(&side1, 2.),
        side1.lerp(&side0, 2.),
        side2.lerp(&side1, 2.),
    ];
    assert_eq!(
        actual,
        [
            BorderSide::NONE,
            BorderSide::NONE,
            side2,
            BorderSide::NONE,
            side0
        ],
        "negative extrapolations must produce no border; exact zero is a solid hairline"
    );
}

#[test]
fn flutter_350_border_side_lerp_interpolates_stroke_alignment() {
    // packages/flutter/test/painting/border_side_test.dart:126.
    let aligned = |stroke_align| BorderSide {
        stroke_align,
        ..BorderSide::solid(Color::BLACK, 2.)
    };
    let inside = aligned(-1.);
    let center = aligned(0.);
    let outside = aligned(1.);
    assert_eq!(inside.lerp(&outside, 0.), inside);
    assert_eq!(inside.lerp(&outside, 0.5), center);
    assert_eq!(inside.lerp(&outside, 1.), outside);
    assert_eq!(inside.lerp(&center, 0.), inside);
    assert_eq!(inside.lerp(&center, 0.5), aligned(-0.5));
    assert_eq!(inside.lerp(&center, 1.), center);
}

#[test]
fn flutter_350_border_symmetric_assigns_vertical_to_left_and_right() {
    // packages/flutter/test/painting/border_test.dart:27.
    // BorderSide is Copy in Rust, so compare the value assigned to each edge.
    let vertical = BorderSide::solid(Color::WHITE, 1.);
    let horizontal = BorderSide::default();
    let border = Border::symmetric(vertical, horizontal);
    assert_eq!(
        [border.left, border.top, border.right, border.bottom],
        [vertical, horizontal, vertical, horizontal],
        "sides in left, top, right, bottom order"
    );
}

#[test]
fn flutter_350_a_single_border_side_populates_every_physical_edge() {
    let side = BorderSide::default();
    let border = Border::from_border_side(side);
    assert_eq!(
        [border.left, border.top, border.right, border.bottom],
        [side; 4]
    );
    assert_eq!(border, Border::all(side));
}

#[test]
fn omitted_horizontal_sides_emit_no_strokes_or_fills_at_the_top_and_bottom() {
    let size = Size::new(40., 30.);
    let bounds = Rect::from_origin_size(Offset::ZERO, size);
    let mut canvas = Canvas::default();
    assert!(!Border::default().paint(&mut canvas, bounds, BoxShape::Rectangle, None));
    assert!(canvas.finish().commands().is_empty());

    let vertical = BorderSide::solid(Color::rgba(255, 0, 0, 255), 3.);
    let mut canvas = Canvas::default();
    let border = Border::symmetric(vertical, BorderSide::NONE);
    assert!(border.paint(&mut canvas, bounds, BoxShape::Rectangle, None));
    let display = canvas.finish();
    let commands = display.commands();
    assert_eq!(commands.len(), 2);
    let covers = |point| {
        commands.iter().any(|paint| {
            let PaintCommand::FillPath {
                path, fill_rule, ..
            } = paint
            else {
                panic!("expected only the two vertical fill bands");
            };
            path.contains(point, *fill_rule)
        })
    };
    assert!(covers(Offset::new(0.5, 15.5)));
    assert!(covers(Offset::new(39.5, 15.5)));
    assert!(!covers(Offset::new(20.5, 0.5)));
    assert!(!covers(Offset::new(20.5, 29.5)));
    assert!(!covers(Offset::new(3.5, 15.5)));
    assert!(!covers(Offset::new(36.5, 15.5)));
}
