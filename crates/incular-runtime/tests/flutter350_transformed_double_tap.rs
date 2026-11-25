//! Pinned Flutter transformed_double_tap_test.dart:12,53.
use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Center, GestureDetector, Transform, Widget};
use std::{cell::Cell, rc::Rc, time::Duration};

fn check(scale: f32, child_size: Size) {
    let tally = Rc::new(Cell::new(0));
    let detector = GestureDetector::new(Widget::box_(child_size, Color::rgba(255, 0, 0, 255)))
        .on_double_tap({
            let tally = tally.clone();
            move || tally.set(tally.get() + 1)
        });
    let mut runtime = Runtime::new(Center::new(Transform::scale(scale, detector)).into()).unwrap();
    let now = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), now)
        .unwrap();
    let mut tap = |pointer, millis, x| {
        for (phase, buttons) in [
            (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
            (PointerPhase::Up, 0),
        ] {
            let _ = runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer,
                    device: 1,
                    kind: PointerDeviceKind::Touch,
                    button: Some(PRIMARY_POINTER_BUTTON),
                    phase,
                    buttons,
                    position: Offset::new(x, 300.),
                    sample: PointerSampleMetadata::EMPTY,
                },
                now + Duration::from_millis(millis),
            );
        }
    };
    tap(1, 0, 400.);
    tap(2, 40, 499.);
    assert_eq!(
        tally.replace(0),
        1,
        "99 screen pixels is inside double-tap slop"
    );
    tap(3, 50, 400.);
    tap(4, 90, 501.);
    assert_eq!(
        tally.get(),
        0,
        "101 screen pixels is outside double-tap slop"
    );
}

#[test]
fn slop_uses_global_positions_in_a_scaled_up_detector() {
    check(2., Size::new(100., 150.));
}

#[test]
fn slop_uses_global_positions_in_a_scaled_down_detector() {
    check(0.5, Size::new(500., 500.));
}
