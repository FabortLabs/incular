//! Flutter's Material `TextMagnifier` placement
//! (`_determineMagnifierPositionAndFocalPoint`) and `Magnifier` defaults.

use incular_config::TargetPlatform;
use incular_core::{Offset, Rect, Size};
use incular_material::{Magnifier, TextMagnifier};
use incular_widgets::MagnifierInfo;

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn close(actual: Offset, expected: Offset) {
    assert!(
        (actual.x - expected.x).abs() < 1e-3 && (actual.y - expected.y).abs() < 1e-3,
        "{actual:?} != {expected:?}"
    );
}

fn info(gesture: Offset, field: Rect) -> MagnifierInfo {
    MagnifierInfo {
        global_gesture_position: gesture,
        current_line_boundaries: rect(40., gesture.y - 10., 300., 20.),
        caret_rect: rect(gesture.x, gesture.y - 10., 2., 20.),
        field_bounds: field,
    }
}

const SCREEN: Size = Size {
    width: 400.,
    height: 800.,
};

#[test]
fn centered_above_the_gesture() {
    // The magnifier (77.37 x 37.9) is centered on the gesture x and its
    // bottom sits kStandardVerticalFocalPointShift (22) above the caret
    // center: (100 - 38.685, 300 - 59.9).
    let placed = TextMagnifier::position(
        &info(Offset::new(100., 300.), rect(40., 280., 300., 40.)),
        SCREEN,
    );
    close(placed.position, Offset::new(61.315, 240.1));
    close(placed.focal_point_offset, Offset::ZERO);
}

#[test]
fn kept_on_screen_and_in_the_field() {
    // Near the top the magnifier is pushed down to y = 0 and the focal point
    // follows by the 29.9 it moved.
    let placed = TextMagnifier::position(
        &info(Offset::new(100., 30.), rect(40., 10., 300., 40.)),
        SCREEN,
    );
    close(placed.position, Offset::new(61.315, 0.));
    close(placed.focal_point_offset, Offset::new(0., -29.9));
    // Near the field's left edge the focal point stays half the magnified
    // width (38.685 / 1.25 = 30.948) inside it: 70.948 - 45.
    let placed = TextMagnifier::position(
        &info(Offset::new(45., 300.), rect(40., 280., 300., 40.)),
        SCREEN,
    );
    close(placed.focal_point_offset, Offset::new(25.948, 0.));
    // A field narrower than 61.896 points at its center: 65 - 100.
    let placed = TextMagnifier::position(
        &info(Offset::new(100., 300.), rect(40., 280., 50., 40.)),
        SCREEN,
    );
    close(placed.focal_point_offset, Offset::new(-35., 0.));
}

#[test]
fn magnifier_defaults() {
    let magnifier = Magnifier::default();
    assert_eq!(Magnifier::default_magnifier_size(), Size::new(77.37, 37.9));
    let raw = magnifier.raw_magnifier();
    assert_eq!(raw.magnification_scale, 1.25);
    // kStandardVerticalFocalPointShift + height / 2 = 22 + 18.95.
    close(raw.focal_point_offset, Offset::new(0., 40.95));
    assert_eq!(raw.decoration.shadows.len(), 1);
    assert_eq!(
        TextMagnifier::JUMP_BETWEEN_LINES_ANIMATION_DURATION.as_millis(),
        70
    );
    assert!(TextMagnifier::displays_handles_in_magnifier(
        TargetPlatform::Ios
    ));
    assert!(!TextMagnifier::displays_handles_in_magnifier(
        TargetPlatform::Android
    ));
}
