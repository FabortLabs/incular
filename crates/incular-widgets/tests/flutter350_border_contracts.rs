//! Public border value contracts from Flutter 3.50.0 painting tests.

use incular_core::{Color, Lerp};
use incular_widgets::{Border, BorderSide, BorderStyle};

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
