//! Count validation from Flutter 3.50.0 animation/animation_controller_test.dart.

#![cfg(debug_assertions)]

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    time::Duration,
};

use incular_animation::{AnimationController, RepeatOptions};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::Widget;

#[test]
fn zero_repeat_count_is_rejected_instead_of_repeating_indefinitely() {
    // Upstream line 1267, with a valid-one control before the rejected call.
    let size = Size::new(100.0, 100.0);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let base = Instant::now();
    let mut frame = |millis| {
        runtime
            .run_frame_at(bounds, base + Duration::from_millis(millis))
            .unwrap();
    };
    frame(0);

    let period = Duration::from_millis(100);
    let options = RepeatOptions::default().reverse(true);
    let valid = AnimationController::new(period).with_vsync();
    let finite = valid.repeat_on_next_tick(options.count(1));
    frame(0);
    frame(25);
    assert_eq!(valid.value(), 0.25);
    assert!(finite.is_pending());
    frame(100);
    assert_eq!(valid.value(), 1.0);
    assert!(finite.is_complete());
    frame(125);
    assert_eq!(valid.value(), 1.0);

    let invalid = AnimationController::new(period)
        .with_value(0.0)
        .with_vsync();
    assert_eq!(invalid.value(), 0.0);
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            invalid.repeat_on_next_tick(options.count(0))
        }))
        .is_err()
    );
    assert_eq!(invalid.value(), 0.0);
    assert!(!invalid.is_animating());

    let active = invalid.forward_on_next_tick();
    frame(200);
    frame(225);
    assert_eq!(invalid.value(), 0.25);
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            invalid.repeat_with(base + Duration::from_millis(225), options.count(0))
        }))
        .is_err()
    );
    assert_eq!(invalid.value(), 0.25);
    assert!(active.is_pending());
    frame(250);
    assert_eq!(invalid.value(), 0.5);
}
