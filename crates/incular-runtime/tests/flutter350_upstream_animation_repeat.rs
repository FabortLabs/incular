//! Repeat traces from Flutter 3.50.0 animation/animation_controller_test.dart.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_animation::{AnimationController, AnimationStatus, RepeatOptions};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::Widget;

fn frame_driver() -> impl FnMut(u64) {
    let size = Size::new(100.0, 100.0);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let base = Instant::now();
    runtime.run_frame_at(bounds, base).unwrap();
    // Some original cases feed the reference binding older timestamps
    // deliberately. Preserve the exact samples instead of sorting them.
    move |millis| {
        runtime
            .run_frame_at(bounds, base + Duration::from_millis(millis))
            .unwrap();
    }
}

fn controller(period_ms: u64) -> AnimationController {
    AnimationController::new(Duration::from_millis(period_ms)).with_vsync()
}

fn expect_value(
    frame: &mut impl FnMut(u64),
    controller: &AnimationController,
    millis: u64,
    expected: f32,
) {
    frame(millis);
    assert_eq!(controller.value(), expected, "at {millis} ms");
}

#[test]
fn reverse_repeat_alternates_from_each_initial_value() {
    // Upstream line 873.
    let mut frame = frame_driver();
    let controller = controller(100);
    let options = RepeatOptions::default().reverse(true);
    assert_eq!(controller.value(), 0.0);
    let _ = controller.repeat_on_next_tick(options);
    frame(0);
    expect_value(&mut frame, &controller, 25, 0.25);
    frame(0);
    expect_value(&mut frame, &controller, 125, 0.75);

    controller.reset();
    controller.set_value(1.0);
    assert_eq!(controller.value(), 1.0);
    let _ = controller.repeat_on_next_tick(options);
    frame(0);
    expect_value(&mut frame, &controller, 25, 0.75);
    frame(0);
    expect_value(&mut frame, &controller, 125, 0.25);

    controller.reset();
    controller.set_value(0.5);
    assert_eq!(controller.value(), 0.5);
    let _ = controller.repeat_on_next_tick(options);
    frame(0);
    expect_value(&mut frame, &controller, 50, 1.0);
    frame(0);
    expect_value(&mut frame, &controller, 150, 0.0);
}

#[test]
fn reverse_repeat_respects_subranges_and_equal_min_max() {
    // Upstream line 920.
    let mut frame = frame_driver();
    let controller = controller(100);
    let options = RepeatOptions::default().reverse(true);
    assert_eq!(controller.value(), 0.0);
    let _ = controller.repeat_on_next_tick(options.min(0.5).max(1.0));
    frame(0);
    expect_value(&mut frame, &controller, 50, 0.75);
    frame(0);
    expect_value(&mut frame, &controller, 100, 1.0);
    frame(0);
    expect_value(&mut frame, &controller, 200, 0.5);

    controller.reset();
    controller.set_value(0.0);
    assert_eq!(controller.value(), 0.0);
    let _ = controller.repeat_on_next_tick(options.min(1.0).max(1.0));
    frame(0);
    expect_value(&mut frame, &controller, 25, 1.0);
    frame(0);
    expect_value(&mut frame, &controller, 125, 1.0);

    controller.reset();
    controller.set_value(0.2);
    assert_eq!(controller.value(), 0.2);
    let _ = controller.repeat_on_next_tick(options.min(0.2).max(0.6));
    frame(0);
    expect_value(&mut frame, &controller, 50, 0.4);
}

#[test]
fn reverse_repeat_reports_direction_at_the_exact_turnaround() {
    // Upstream line 1126.
    let mut frame = frame_driver();
    let controller = controller(1000);
    let statuses = Rc::new(RefCell::new(Vec::new()));
    controller.add_status_listener({
        let statuses = Rc::clone(&statuses);
        move |status| statuses.borrow_mut().push(status)
    });
    let _ = controller.repeat_on_next_tick(RepeatOptions::default().reverse(true));
    frame(0);
    frame(999);
    assert_eq!(statuses.borrow().as_slice(), [AnimationStatus::Forward]);
    statuses.borrow_mut().clear();
    frame(1000);
    assert_eq!(statuses.borrow().as_slice(), [AnimationStatus::Reverse]);
}

#[test]
fn finite_forward_repeat_returns_to_min_at_the_count_boundary() {
    // Upstream line 1289.
    let mut frame = frame_driver();
    let controller = controller(100);
    assert_eq!(controller.value(), 0.0);
    let _ = controller.repeat_on_next_tick(RepeatOptions::default().count(1));
    frame(0);
    for (t, value) in [(25, 0.25), (50, 0.5), (99, 0.99), (100, 0.0)] {
        expect_value(&mut frame, &controller, t, value);
    }
    controller.reset();
    assert_eq!(controller.value(), 0.0);
    let _ = controller.repeat_on_next_tick(RepeatOptions::default().count(2));
    frame(0);
    for (t, value) in [(25, 0.25), (50, 0.5), (200, 0.0)] {
        expect_value(&mut frame, &controller, t, value);
    }
    controller.reset();
}

#[test]
fn finite_reverse_repeat_starts_back_from_max() {
    // Upstream line 1324; its last sample is at 60 ms after a 100 ms sample.
    let mut frame = frame_driver();
    let controller = controller(100);
    assert_eq!(controller.value(), 0.0);
    let _ = controller.repeat_on_next_tick(RepeatOptions::default().reverse(true).count(4));
    frame(0);
    for (t, value) in [(25, 0.25), (50, 0.5), (99, 0.99), (100, 1.0)] {
        expect_value(&mut frame, &controller, t, value);
    }
    frame(60);
    assert_eq!(
        format!("{:.1}", controller.value()).parse::<f64>().unwrap(),
        0.6
    );
    controller.reset();
}
