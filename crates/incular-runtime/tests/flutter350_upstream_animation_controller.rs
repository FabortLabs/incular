//! Public assertions from Flutter 3.50.0 test/animation/animation_controller_test.dart.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::{AnimationController, AnimationStatus};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::Widget;

fn frame_driver() -> impl FnMut(Duration) {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let base = Instant::now();
    runtime.run_frame_at(bounds, base).unwrap();
    move |elapsed| {
        runtime.run_frame_at(bounds, base + elapsed).unwrap();
    }
}

fn vsync_controller() -> AnimationController {
    AnimationController::new(Duration::from_millis(100)).with_vsync()
}

#[test]
fn can_set_value_during_status_callback() {
    // Upstream registration at line 59.
    let mut frame = frame_driver();
    let controller = vsync_controller();
    let completed = Rc::new(Cell::new(false));
    let dismissed = Rc::new(Cell::new(false));
    let listener = controller.add_status_listener({
        let controller = controller.clone();
        let completed = Rc::clone(&completed);
        let dismissed = Rc::clone(&dismissed);
        move |status| {
            match status {
                AnimationStatus::Completed => completed.set(true),
                AnimationStatus::Dismissed => dismissed.set(true),
                _ => return,
            }
            controller.set_value(0.0);
            let _ = controller.forward_on_next_tick();
        }
    });
    let seen = || (completed.get(), dismissed.get());

    let _ = controller.forward_on_next_tick();
    let before_ticks = seen();
    frame(Duration::from_secs(1));
    let after_first_tick = seen();
    frame(Duration::from_secs(2));
    let after_second_tick = seen();

    controller.stop();
    let _ = controller.remove_status_listener(listener);
    assert_eq!(
        [before_ticks, after_first_tick, after_second_tick],
        [(false, false), (false, false), (true, true)],
    );
}

#[test]
fn receives_status_callbacks_for_forward_and_reverse() {
    // Upstream registration at line 92.
    use AnimationStatus::{Dismissed, Forward};

    let mut frame = frame_driver();
    let controller = vsync_controller();
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let values = Rc::new(RefCell::new(Vec::new()));
    controller.add_status_listener({
        let statuses = Rc::clone(&statuses);
        move |status| statuses.borrow_mut().push(status)
    });
    controller.add_listener({
        let values = Rc::clone(&values);
        move |value| values.borrow_mut().push(value)
    });
    let snapshot = || (statuses.borrow().clone(), values.borrow().clone());
    let mut trace = vec![snapshot()];

    let _ = controller.forward_on_next_tick();
    trace.push(snapshot());
    let _ = controller.reverse_on_next_tick();
    trace.push(snapshot());
    let _ = controller.reverse_on_next_tick();
    trace.push(snapshot());

    statuses.borrow_mut().clear();
    let _ = controller.forward_on_next_tick();
    trace.push(snapshot());
    let _ = controller.forward_on_next_tick();
    trace.push(snapshot());
    let _ = controller.reverse_on_next_tick();
    statuses.borrow_mut().clear();

    for elapsed in [10, 20, 30, 40] {
        frame(Duration::from_secs(elapsed));
        trace.push(snapshot());
    }
    controller.stop();

    let expected_statuses = [
        &[][..],
        &[Forward],
        &[Forward, Dismissed],
        &[Forward, Dismissed],
        &[Forward],
        &[Forward],
        &[],
        &[],
        &[],
        &[],
    ];
    for (step, ((statuses, _), expected)) in trace.iter().zip(expected_statuses).enumerate() {
        assert_eq!(statuses, expected, "status log at step {step}");
    }
    let value_trace: Vec<_> = trace.iter().map(|(_, values)| values.as_slice()).collect();
    assert!(
        value_trace.iter().all(|values| values.is_empty()),
        "no value notifications expected at any step, got {value_trace:?}",
    );
}

#[test]
fn last_elapsed_duration_is_measured_from_first_tick() {
    // Upstream registration at line 415.
    let mut frame = frame_driver();
    let controller = vsync_controller();
    let _ = controller.forward_on_next_tick();
    for elapsed in [20, 30, 40] {
        frame(Duration::from_millis(elapsed));
    }
    assert_eq!(controller.elapsed(), Duration::from_millis(20));
    controller.stop();
}

#[test]
fn setting_value_directly_sets_correct_status() {
    // Upstream registration at line 726. Rust requires a duration even
    // though this test never starts a timed run.
    use AnimationStatus::{Completed, Dismissed, Forward};

    let controller = AnimationController::new(Duration::ZERO)
        .with_value(0.0)
        .with_vsync();
    assert_eq!((controller.value(), controller.status()), (0.0, Dismissed));
    for (value, expected) in [
        (0.5, Forward),
        (1.0, Completed),
        (0.5, Forward),
        (0.0, Dismissed),
    ] {
        controller.set_value(value);
        assert_eq!((controller.value(), controller.status()), (value, expected));
    }
}
