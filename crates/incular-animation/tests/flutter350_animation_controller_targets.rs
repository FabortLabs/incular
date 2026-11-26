//! The Flutter 3.50.0 controller target, status and reset contracts.
use incular_animation::{AnimationController, AnimationStatus, Curve, scheduler};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

fn ms(count: u64) -> Duration {
    Duration::from_millis(count)
}

fn observe_status(controller: &AnimationController) -> Rc<RefCell<Vec<AnimationStatus>>> {
    let log = Rc::new(RefCell::new(Vec::new()));
    controller.add_status_listener({
        let log = log.clone();
        move |status| log.borrow_mut().push(status)
    });
    log
}

#[test]
fn the_current_target_finishes_synchronously_at_interior_and_both_bounds() {
    for initial in [0.5, 1., 0.] {
        let controller = AnimationController::new(Duration::ZERO).with_value(initial);
        let log = observe_status(&controller);
        let now = Instant::now();
        let current = controller.animate_to_curved(now, initial, Some(ms(100)), Curve::Linear);
        assert!(current.is_complete());
        assert!(!controller.is_animating());
        assert_eq!(controller.value(), initial);
        assert_eq!(&*log.borrow(), &[AnimationStatus::Completed]);
    }
}

#[test]
fn retargeting_the_current_value_cancels_a_forward_or_reverse_run() {
    for (initial, running) in [
        (0., AnimationStatus::Forward),
        (1., AnimationStatus::Reverse),
    ] {
        for halfway in [500, 333] {
            let controller = AnimationController::new(ms(1000)).with_value(initial);
            let log = observe_status(&controller);
            let now = Instant::now();
            let old = if initial == 0. {
                controller.forward(now)
            } else {
                controller.reverse(now)
            };
            let _ = controller.tick(now);
            let _ = controller.tick(now + ms(halfway));
            assert_eq!(&*log.borrow(), &[running]);
            let (public, wide) = (controller.value(), controller.value_f64());
            if halfway == 333 {
                assert_ne!(f64::from(public), wide);
            }
            let target = controller.animate_to_curved(
                now + ms(halfway),
                public,
                Some(ms(100)),
                Curve::Linear,
            );
            assert!(old.is_canceled());
            assert!(
                target.is_complete(),
                "an already-current target must not animate"
            );
            assert!(!controller.is_animating());
            assert_eq!(&*log.borrow(), &[running, AnimationStatus::Completed]);
            assert_eq!(controller.value(), public);
            assert_eq!(
                controller.value_f64(),
                wide,
                "a no-op target preserves internal precision"
            );
        }
    }
}

#[test]
fn explicit_zero_duration_cancels_the_ticker_without_scheduling_a_replacement() {
    let controller = AnimationController::new(ms(100))
        .with_value(0.2)
        .with_vsync();
    let now = Instant::now();
    let active = controller.forward_on_next_tick();
    assert_eq!(controller.value(), 0.2);
    assert_eq!(scheduler::transient_callback_count(), 1);
    let finished = controller.animate_to_curved(now, 1., Some(Duration::ZERO), Curve::Linear);
    assert!(active.is_canceled());
    assert!(finished.is_complete());
    assert_eq!(controller.value(), 1.);
    assert_eq!(scheduler::transient_callback_count(), 0);
}

#[test]
fn reset_returns_to_the_lower_bound_from_every_controller_phase() {
    use AnimationStatus::*;
    let controller = AnimationController::new(ms(100));
    let log = observe_status(&controller);
    let now = Instant::now();
    controller.reset();
    assert_eq!((controller.value(), controller.status()), (0., Dismissed));
    assert!(log.borrow().is_empty());
    for (initial, direction, elapsed, expected) in [
        (0., Forward, 50, vec![Forward, Dismissed]),
        (1., Reverse, 50, vec![Reverse, Dismissed]),
        (0., Forward, 150, vec![Forward, Completed, Dismissed]),
    ] {
        controller.set_value(initial);
        log.borrow_mut().clear();
        let active = if direction == Forward {
            controller.forward(now)
        } else {
            controller.reverse(now)
        };
        let _ = controller.tick(now);
        let _ = controller.tick(now + ms(elapsed));
        assert_eq!(
            controller.status(),
            if elapsed == 150 { Completed } else { direction }
        );
        controller.reset();
        assert_eq!((controller.value(), controller.status()), (0., Dismissed));
        assert_eq!(*log.borrow(), expected);
        assert_eq!(active.is_canceled(), elapsed == 50);
    }
}

#[test]
fn animate_to_always_reports_forward_then_completed_in_either_numeric_direction() {
    use AnimationStatus::*;
    let controller = AnimationController::new(ms(100));
    let log = observe_status(&controller);
    let now = Instant::now();
    let animate = |target| {
        let f = controller.animate_to(now, target);
        let _ = controller.tick(now);
        let _ = controller.tick(now + ms(150));
        assert!(f.is_complete());
        assert_eq!(controller.value(), target);
        assert_eq!(&*log.borrow(), &[Forward, Completed]);
        log.borrow_mut().clear();
    };
    for target in [0.5, 1., 0.5, 0.] {
        animate(target);
    }
    for (initial, direction, status) in [(1., Reverse, Dismissed), (0., Forward, Completed)] {
        controller.set_value(initial);
        log.borrow_mut().clear();
        if direction == Reverse {
            controller.reverse(now);
        } else {
            controller.forward(now);
        }
        let _ = controller.tick(now);
        let _ = controller.tick(now + ms(150));
        assert_eq!(&*log.borrow(), &[direction, status]);
        log.borrow_mut().clear();
        animate(0.5);
    }
}

#[test]
fn a_projected_endpoint_does_not_replace_the_real_completion_bound() {
    let controller = AnimationController::new(Duration::from_secs(1));
    let start = Instant::now();
    let near_end = start + Duration::from_nanos(999_999_990);
    let _ = controller.forward(start);
    let _ = controller.tick(near_end);
    assert_eq!(controller.value(), 1.);
    assert!(controller.value_f64() < 1.);
    let finish = controller.forward(near_end);
    let _ = controller.tick(near_end + ms(1));
    assert_eq!(controller.value_f64(), 1.);
    assert!(finish.is_complete());
}

#[test]
fn directional_from_setters_publish_the_assigned_value_and_direction_before_the_run() {
    use AnimationStatus::*;
    for reverse_first in [false, true] {
        let controller = AnimationController::new(ms(100));
        let statuses = observe_status(&controller);
        let values = Rc::new(RefCell::new(Vec::new()));
        controller.add_listener({
            let values = values.clone();
            move |value| values.borrow_mut().push(value)
        });
        let now = Instant::now();
        let active = if reverse_first {
            controller.reverse_from(now, 0.2)
        } else {
            controller.forward_from(now, 0.2)
        };
        assert_eq!(
            &*statuses.borrow(),
            &[if reverse_first { Reverse } else { Forward }]
        );
        assert_eq!(&*values.borrow(), &[0.2]);
        assert_eq!(controller.value(), 0.2);
        if reverse_first {
            statuses.borrow_mut().clear();
            values.borrow_mut().clear();
            assert!(!controller.forward_from(now, 0.).is_complete());
            assert!(active.is_canceled());
            assert_eq!(&*statuses.borrow(), &[Dismissed, Forward]);
            assert_eq!(&*values.borrow(), &[0.]);
            assert_eq!(controller.value(), 0.);
        }
    }
}

#[test]
fn toggle_honors_unequal_durations_and_switches_an_active_forward_run() {
    use AnimationStatus::*;
    let start = Instant::now();
    for (forward, reverse, samples, reverse_samples) in [
        (
            100,
            50,
            vec![(20, 0.2), (50, 0.5), (80, 0.8), (110, 1.)],
            vec![(10, 0.8), (20, 0.6), (30, 0.4), (50, 0.)],
        ),
        (
            50,
            100,
            vec![(20, 0.4), (50, 1.), (80, 1.)],
            vec![(10, 0.9), (20, 0.8), (30, 0.7), (50, 0.5), (100, 0.)],
        ),
    ] {
        let controller = AnimationController::new(ms(forward)).with_reverse_duration(ms(reverse));
        controller.toggle(start);
        assert!(controller.is_forward_or_completed());
        for (at, value) in samples {
            let _ = controller.tick(start + ms(at));
            assert!((controller.value() - value).abs() < 1e-6);
        }
        controller.stop();
        let start_reverse = start + ms(210);
        controller.toggle(start_reverse);
        assert!(!controller.is_forward_or_completed());
        for (at, value) in reverse_samples {
            let _ = controller.tick(start_reverse + ms(at));
            assert!((controller.value() - value).abs() < 1e-6);
        }
    }
    let controller = AnimationController::new(ms(100)).with_reverse_duration(ms(100));
    controller.forward(start);
    assert_eq!(controller.status(), Forward);
    let _ = controller.tick(start + ms(50));
    assert_eq!(controller.value(), 0.5);
    assert!(controller.is_forward_or_completed());
    controller.toggle(start + ms(50));
    assert_eq!(controller.status(), Reverse);
    assert!(!controller.is_forward_or_completed());
    let _ = controller.tick(start + ms(150));
    assert_eq!((controller.value(), controller.status()), (0., Dismissed));
}

fn panic_text(action: impl FnOnce()) -> String {
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action))
        .expect_err("the missing timing must be rejected");
    error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            error
                .downcast_ref::<&str>()
                .map(|value| (*value).to_owned())
        })
        .expect("a diagnostic panic")
}

#[test]
fn absent_duration_reports_the_operation_that_cannot_start() {
    let controller = AnimationController::without_duration();
    let now = Instant::now();
    assert_eq!(controller.optional_duration(), None);
    for (operation, error) in [
        (
            "forward",
            panic_text(|| {
                let _ = controller.forward(now);
            }),
        ),
        (
            "reverse",
            panic_text(|| {
                let _ = controller.reverse(now);
            }),
        ),
        (
            "animateTo",
            panic_text(|| {
                let _ = controller.animate_to(now, 0.8);
            }),
        ),
        (
            "animateBack",
            panic_text(|| {
                let _ = controller.animate_back(now, 0.8);
            }),
        ),
        (
            "repeat",
            panic_text(|| {
                let _ = controller.repeat(now, false);
            }),
        ),
    ] {
        assert!(
            error.contains(&format!("AnimationController.{operation}()")),
            "{error}"
        );
        assert!(error.contains("no default duration"), "{error}");
        if operation == "reverse" || operation == "animateBack" {
            assert!(error.contains("reverseDuration"), "{error}");
        }
    }
    assert!(!controller.is_animating());
    assert_eq!(controller.value(), 0.);
}

#[test]
fn reverse_duration_alone_is_enough_for_animate_back_but_not_forward() {
    use AnimationStatus::*;
    let controller = AnimationController::without_duration().with_reverse_duration(ms(2000));
    let log = observe_status(&controller);
    let now = Instant::now();
    let result = controller.animate_back(now, 0.8);
    assert_eq!(&*log.borrow(), &[Reverse]);
    let _ = controller.tick(now);
    let _ = controller.tick(now + ms(2000));
    assert!(result.is_complete());
    assert_eq!(controller.value(), 0.8);
    assert_eq!(&*log.borrow(), &[Reverse, Dismissed]);
    let _ = panic_text(|| {
        let _ = controller.forward(now);
    });
}

#[test]
fn explicit_timing_and_explicit_zero_differ_from_an_absent_default() {
    use incular_animation::RepeatOptions;
    let controller = AnimationController::without_duration();
    let now = Instant::now();
    let active = controller.animate_to_curved(now, 0.8, Some(ms(200)), Curve::Linear);
    let _ = controller.tick(now + ms(100));
    let value = controller.value();
    let _ = panic_text(|| {
        let _ = controller.forward_from(now + ms(100), 1.);
    });
    assert!(!active.is_canceled());
    assert!(controller.is_animating());
    assert_eq!(controller.value(), value);
    let explicit = controller.repeat_with(now, RepeatOptions::default().period(ms(30)));
    assert!(active.is_canceled());
    assert!(!explicit.is_complete());
    assert!(controller.is_animating());

    controller.set_duration(Duration::ZERO);
    assert_eq!(controller.optional_duration(), Some(Duration::ZERO));
    let immediate = controller.forward(now);
    assert!(explicit.is_canceled());
    assert!(immediate.is_complete());
    assert_eq!(controller.value(), 1.);
    assert!(!controller.is_animating());
}

#[test]
fn resync_before_or_during_an_active_tick_keeps_its_original_clock_and_future() {
    for resync_midflight in [false, true] {
        let controller = AnimationController::new(ms(100)).with_vsync();
        let log = observe_status(&controller);
        let start = Instant::now();
        let original = controller.forward_on_next_tick();
        if !resync_midflight {
            controller.resync();
        }
        assert!(!original.is_canceled());
        assert_eq!(scheduler::transient_callback_count(), 1);
        scheduler::begin_frame(start);
        scheduler::end_frame(start);
        assert_eq!(controller.value(), 0.);
        scheduler::begin_frame(start + ms(40));
        scheduler::end_frame(start + ms(40));
        assert_eq!(controller.value(), 0.4);
        if resync_midflight {
            controller.resync();
        }
        assert_eq!(scheduler::transient_callback_count(), 1);
        scheduler::begin_frame(start + ms(70));
        scheduler::end_frame(start + ms(70));
        assert_eq!(controller.value(), 0.7);
        scheduler::begin_frame(start + ms(101));
        scheduler::end_frame(start + ms(101));
        assert_eq!(controller.value(), 1.);
        assert!(original.is_complete());
        assert_eq!(
            &*log.borrow(),
            &[AnimationStatus::Forward, AnimationStatus::Completed]
        );
        assert_eq!(scheduler::transient_callback_count(), 0);
    }
}

#[test]
fn repeating_ranges_keep_the_initial_phase_on_either_side_of_zero() {
    use incular_animation::RepeatOptions;
    let controller = AnimationController::with_bounds(ms(100), -1., 3.).with_value(1.);
    let now = Instant::now();
    assert_eq!(controller.value(), 1.);
    let first = controller.repeat_with(now, RepeatOptions::default().min(1.).max(3.));
    let _ = controller.tick(now);
    assert_eq!(controller.value(), 1.);
    let _ = controller.tick(now + ms(50));
    assert_eq!(controller.value(), 2.);
    controller.reset();
    assert!(first.is_canceled());
    controller.set_value(0.);
    controller.repeat_with(now, RepeatOptions::default().min(-1.).max(3.));
    let _ = controller.tick(now);
    assert_eq!(controller.value(), 0.);
    let _ = controller.tick(now + ms(25));
    assert_eq!(controller.value(), 1.);
}

#[test]
fn rust_default_still_selects_an_explicit_immediate_duration() {
    let controller = AnimationController::default();
    assert_eq!(controller.optional_duration(), Some(Duration::ZERO));
    let result = controller.forward(Instant::now());
    assert!(result.is_complete());
    assert_eq!(controller.value(), 1.);
}

#[cfg(debug_assertions)]
#[test]
fn a_controller_reports_one_shared_create_and_dispose_event() {
    use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};
    let seen = Rc::new(RefCell::new(Vec::new()));
    let _subscription = MemoryAllocations::current().add_listener({
        let seen = seen.clone();
        move |event| {
            if event.class_name == "AnimationController" {
                seen.borrow_mut().push(event);
            }
        }
    });
    let controller = AnimationController::new(ms(100)).with_vsync();
    let holder = controller.clone();
    drop(controller);
    assert_eq!(seen.borrow().len(), 1);
    assert_eq!(seen.borrow()[0].kind, ObjectEventKind::Created);
    drop(holder);
    let events = seen.borrow();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, events[1].id);
    assert_eq!(events[1].kind, ObjectEventKind::Disposed);
    assert_eq!(events[0].library, "animation");
}

#[test]
fn controller_description_is_one_line_at_idle_and_during_both_directions() {
    let controller = AnimationController::new(ms(100));
    let now = Instant::now();
    assert_eq!(controller.to_string(), "AnimationController(⏮ 0.000; paused)");
    let _forward = controller.forward_on_next_tick();
    let _ = controller.tick(now + ms(10));
    let _ = controller.tick(now + ms(20));
    assert_eq!(controller.to_string(), "AnimationController(▶ 0.100)");
    let _ = controller.tick(now + ms(30));
    assert_eq!(controller.to_string(), "AnimationController(▶ 0.200)");
    let _reverse = controller.reverse_on_next_tick();
    let _ = controller.tick(now + ms(40));
    let _ = controller.tick(now + ms(50));
    assert_eq!(controller.to_string(), "AnimationController(◀ 0.100)");
    controller.stop();
    assert_eq!(controller.to_string(), "AnimationController(◀ 0.100; paused)");
    controller.dispose();
    assert_eq!(controller.to_string(), "AnimationController(◀ 0.100; paused; DISPOSED)");
}

#[test]
fn all_original_ticker_operations_reject_a_disposed_controller() {
    use incular_animation::GravitySimulation;

    let controller = AnimationController::without_duration();
    let other_handle = controller.clone();
    controller.dispose();
    let now = Instant::now();
    let rejected = [
        panic_text(|| { let _ = other_handle.animate_back(now, 0.); }),
        panic_text(|| { let _ = other_handle.animate_to(now, 0.); }),
        panic_text(|| { let _ = other_handle.animate_with(GravitySimulation::new(0., 0., 0., 0.)); }),
        panic_text(|| other_handle.stop()),
        panic_text(|| { let _ = other_handle.forward(now); }),
        panic_text(|| { let _ = other_handle.reverse(now); }),
    ];
    assert!(rejected.iter().all(|message| message == "AnimationController used after dispose"));
    assert!(other_handle.is_disposed() && !other_handle.is_animating());
    assert_eq!(other_handle.value(), 0.);
}

#[test]
fn missing_duration_diagnostics_include_the_complete_pinned_remediation() {
    let controller = AnimationController::without_duration();
    let now = Instant::now();
    let cases = [
        (
            panic_text(|| { let _ = controller.forward(now); }),
            [
                "   AnimationController.forward() called with no default duration.",
                "   The \"duration\" property should be set, either in the constructor",
                "   or later, before calling the forward() function.",
            ].as_slice(),
        ),
        (
            panic_text(|| { let _ = controller.reverse(now); }),
            [
                "   AnimationController.reverse() called with no default duration or",
                "   reverseDuration.",
                "   The \"duration\" or \"reverseDuration\" property should be set,",
                "   either in the constructor or later, before calling the reverse()",
                "   function.",
            ].as_slice(),
        ),
        (
            panic_text(|| { let _ = controller.animate_to(now, 0.8); }),
            [
                "   AnimationController.animateTo() called with no explicit duration",
                "   and no default duration.",
                "   Either the \"duration\" argument to the animateTo() method should",
                "   be provided, or the \"duration\" property should be set, either in",
                "   the constructor or later, before calling the animateTo()",
                "   function.",
            ].as_slice(),
        ),
        (
            panic_text(|| { let _ = controller.animate_back(now, 0.8); }),
            [
                "   AnimationController.animateBack() called with no explicit",
                "   duration and no default duration or reverseDuration.",
                "   Either the \"duration\" argument to the animateBack() method should",
                "   be provided, or the \"duration\" or \"reverseDuration\" property",
                "   should be set, either in the constructor or later, before calling",
                "   the animateBack() function.",
            ].as_slice(),
        ),
    ];
    for (actual, expected) in cases {
        assert_eq!(actual, expected.join("\n") + "\n");
    }
    assert_eq!(controller.value(), 0.);
    assert!(!controller.is_animating());
    assert_eq!(controller.status(), AnimationStatus::Dismissed);
}
