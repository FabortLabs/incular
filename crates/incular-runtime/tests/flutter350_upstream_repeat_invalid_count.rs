//! Count validation from Flutter 3.50.0 animation/animation_controller_test.dart.

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
    let result = catch_unwind(AssertUnwindSafe(|| {
        invalid.repeat_on_next_tick(options.count(0))
    }));
    if let Ok(future) = result {
        frame(200);
        let samples = [25, 100, 125, 225, 325, 425].map(|elapsed| {
            frame(200 + elapsed);
            (elapsed, invalid.value(), future.is_pending())
        });
        invalid.stop();
        panic!("count=0 was accepted; samples (elapsed ms, value, future pending): {samples:?}");
    }
}
