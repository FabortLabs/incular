//! Public controller observations from Flutter 3.50.0 animation/animation_controller_test.dart.

use incular_animation::{AnimationController, AnimationStatus, Simulation};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::Widget;
use std::{cell::RefCell, rc::Rc, time::Duration};

fn frame_driver() -> impl FnMut(u64) {
    let size = Size::new(100.0, 100.0);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let base = Instant::now();
    runtime.run_frame_at(bounds, base).unwrap();
    // Preserve deliberately rewound frame times from the original tests.
    move |millis| {
        runtime
            .run_frame_at(bounds, base + Duration::from_millis(millis))
            .map(|_| ())
            .unwrap()
    }
}

fn controller(duration_ms: u64) -> AnimationController {
    AnimationController::new(Duration::from_millis(duration_ms)).with_vsync()
}

#[test]
fn forward_and_reverse_each_use_their_own_duration() {
    // Upstream :184. moreOrLessEquals uses absolute 1e-10. Widen the
    // controller observation rather than rounding Flutter's decimal to f32.
    let mut frame = frame_driver();
    let mut samples = Vec::new();
    for (forward_ms, reverse_ms, forward, reverse) in [
        (
            100,
            50,
            &[(30, 0.2), (60, 0.5), (90, 0.8), (120, 1.0)][..],
            &[(220, 0.8), (230, 0.6), (240, 0.4), (260, 0.0)][..],
        ),
        (
            50,
            100,
            &[(30, 0.4), (60, 1.0), (90, 1.0)][..],
            &[(220, 0.9), (230, 0.8), (240, 0.7), (260, 0.5), (310, 0.0)][..],
        ),
    ] {
        let animation = controller(forward_ms);
        animation.set_reverse_duration(Some(Duration::from_millis(reverse_ms)));
        for (direction, period, first_tick, sequence) in [
            ("forward", forward_ms, 10, forward),
            ("reverse", reverse_ms, 210, reverse),
        ] {
            let _ = if direction == "forward" {
                animation.forward_on_next_tick()
            } else {
                animation.reverse_on_next_tick()
            };
            frame(first_tick);
            for &(millis, expected) in sequence {
                frame(millis);
                let actual = f64::from(animation.value());
                samples.push((direction, period, millis, actual, expected));
            }
            animation.stop();
        }
    }

    assert_eq!(samples.len(), 16);
    let mismatches: Vec<_> = samples
        .into_iter()
        .filter(|&(_, _, _, actual, expected)| (actual - expected).abs() > 1e-10)
        .collect();
    assert!(
        mismatches.is_empty(),
        "(direction, duration ms, frame ms, actual, expected): {mismatches:?}"
    );
}
#[test]
fn linear_velocity_is_halved_at_start_interior_and_zero_when_stopped() {
    // Upstream :449; retain each original inclusive range and exact zero.
    let mut frame = frame_driver();
    let animation = controller(1000);
    let mut results = Vec::new();
    let mut check = |label, lower, upper| results.push((label, animation.velocity(), lower, upper));

    let _ = animation.forward_on_next_tick();
    frame(0);
    frame(500);
    check("mid-flight", 0.9, 1.1);

    let _ = animation.forward_on_next_tick();
    check("restart before first frame", 0.4, 0.6);
    frame(0);
    check("restart at first frame", 0.4, 0.6);
    frame(5);
    check("restart after 5 ms", 0.9, 1.1);

    animation.set_value(0.5);
    let _ = animation.forward_on_next_tick();
    check("from 0.5 before first frame", 0.4, 0.6);
    frame(0);
    check("from 0.5 at first frame", 0.4, 0.6);
    frame(5);
    check("from 0.5 after 5 ms", 0.9, 1.1);

    animation.set_value(1.0);
    let _ = animation.forward_on_next_tick();
    check("from 1.0 before first frame", 0.0, 0.0);
    frame(0);
    check("from 1.0 at first frame", 0.0, 0.0);
    frame(500);
    check("from 1.0 after 500 ms", 0.0, 0.0);

    let _ = animation.forward_on_next_tick();
    frame(0);
    frame(1000);
    check("already at end", 0.0, 0.0);
    animation.stop();

    for (label, value, lower, upper) in results {
        assert!(
            (lower..=upper).contains(&value),
            "{label}: {value} not in [{lower}, {upper}]"
        );
    }
}

// Upstream's test simulation returns seconds for both position and velocity,
// and continues indefinitely even after the bounded controller reaches 1.0.
struct SecondsSimulation;

impl Simulation for SecondsSimulation {
    fn position(&self, time: Duration) -> f32 {
        time.as_secs_f32()
    }
    fn velocity(&self, time: Duration) -> f32 {
        time.as_secs_f32()
    }
    fn is_done(&self, _: Duration) -> bool {
        false
    }
}

fn check_animate_with_statuses(after_reverse: bool) {
    use AnimationStatus::{Completed, Dismissed, Forward, Reverse};
    let mut frame = frame_driver();
    let animation = controller(1000);
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&events);
    let subscription = animation.add_status_listener(move |status| log.borrow_mut().push(status));

    if after_reverse {
        animation.set_value(1.0);
        let _ = animation.reverse_on_next_tick();
        frame(0);
        frame(2000);
        assert_eq!(events.take(), [Completed, Reverse, Dismissed]);
    }

    let _ = animation.animate_with(SecondsSimulation);
    frame(0);
    frame(2000);
    assert_eq!(events.take(), [Forward]);
    let _ = animation.remove_status_listener(subscription);
    animation.stop();
}

#[test]
fn animate_with_reports_forward_even_when_bounded_position_saturates() {
    // Upstream :1087.
    check_animate_with_statuses(false);
}

#[test]
fn animate_with_reports_forward_after_a_completed_reverse_run() {
    // Upstream :1102.
    check_animate_with_statuses(true);
}
