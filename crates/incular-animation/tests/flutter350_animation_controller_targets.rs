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
