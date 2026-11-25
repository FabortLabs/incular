//! Verified Flutter 3.50.0 regressions from the paired audit at ab2912035e.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_animation::{
    AnimationController, AnimationStatus, BoundedFrictionSimulation, ClampedSimulation, Curve,
    OptionalSizeTween, Simulation, SpringDescription, SpringType, TweenValue,
};
use incular_core::Size;

// C19: Flutter physics/spring_simulation.dart classifies zero bounce as critical.
#[test]
fn zero_bounce_is_critically_damped() {
    let duration = Duration::from_millis(500);
    let cases = [
        (-0.1, SpringType::OverDamped),
        (0.1, SpringType::UnderDamped),
        (0.0, SpringType::CriticallyDamped),
    ];
    assert_eq!(
        cases.map(|(bounce, _)| {
            SpringDescription::with_duration_and_bounce(duration, bounce).spring_type()
        }),
        cases.map(|(_, expected)| expected),
    );
}

// C20: Flutter physics/friction_simulation.dart checks the unrounded distance.
#[test]
fn bounded_friction_does_not_settle_before_reaching_distance_tolerance() {
    let simulation = BoundedFrictionSimulation::new(0.5, 8191.0, 1.0, 8190.0, 8192.0);
    let times = [1700, 1701, 1702].map(Duration::from_millis);
    let done = times.map(|time| simulation.is_done(time));
    let positions = times.map(|time| f64::from(simulation.position(time)));
    assert_eq!(done, [false, false, true], "positions={positions:?}");
}

// C21: Flutter Tween.transform returns the endpoint itself before interpolating.
#[test]
fn optional_size_tween_retains_an_absent_endpoint() {
    let size = Size::new(10.0, 20.0);
    let tween = OptionalSizeTween::new(None, Some(size));
    assert_eq!(tween.evaluate(0.5), Some(Size::new(5.0, 10.0)));
    assert_eq!(tween.evaluate(1.0), Some(size));
    assert_eq!(tween.evaluate(0.0), None);
}

// C22: Flutter IntTween rounds after interpolating, without narrowing inputs to f32.
#[test]
fn integer_tween_retains_midpoints_above_f32_exact_integer_range() {
    assert_eq!(TweenValue::new(2_i32, 4).evaluate(0.5), 3);
    let tween = TweenValue::new(16_777_217_i32, 16_777_219);
    assert_eq!(tween.evaluate(0.0), 16_777_217);
    assert_eq!(tween.evaluate(1.0), 16_777_219);
    assert_eq!(tween.evaluate(0.5), 16_777_218);
}

// C34: Flutter animation/listener_helpers.dart skips a listener removed before its turn.
#[test]
fn a_removed_value_listener_is_skipped_in_the_current_notification() {
    let controller = AnimationController::new(Duration::from_secs(1));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let remove_id = Rc::new(Cell::new(None));
    let result = Rc::new(Cell::new(false));
    let listener_a = controller.add_listener({
        let controller = controller.clone();
        let calls = Rc::clone(&calls);
        let remove_id = Rc::clone(&remove_id);
        let result = Rc::clone(&result);
        move |_| {
            calls.borrow_mut().push("A");
            if let Some(id) = remove_id.take() {
                result.set(controller.remove_listener(id));
            }
        }
    });
    let listener_b = controller.add_listener({
        let calls = Rc::clone(&calls);
        move |_| calls.borrow_mut().push("B")
    });
    remove_id.set(Some(listener_b));

    controller.set_value(0.5);
    let first = calls.take();
    controller.set_value(0.75);
    let second = calls.take();
    assert!(controller.remove_listener(listener_a));

    assert!(result.get(), "A removed B during the first notification");
    assert_eq!(second, ["A"], "B remains removed on later notifications");
    assert_eq!(first, ["A"], "B must not run after A removes it");
}

// C35: Flutter AnimationController._animateToInternal treats an unchanged value
// and a change in terminal status as separate notifications.
#[test]
fn animating_to_the_current_value_notifies_terminal_status_only() {
    let controller = AnimationController::new(Duration::from_secs(1));
    controller.set_value(0.5);
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

    let future = controller.animate_to(Instant::now(), 0.5);

    assert!(future.is_complete());
    assert_eq!(controller.value(), 0.5);
    assert_eq!(*statuses.borrow(), [AnimationStatus::Completed]);
    assert!(values.borrow().is_empty(), "values={:?}", values.borrow());
}

#[test]
fn zero_duration_value_listener_observes_preterminal_status_in_both_directions() {
    let controller = AnimationController::new(Duration::from_secs(1));
    controller.set_value(0.5);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let value_id = controller.add_listener({
        let controller = controller.clone();
        let calls = calls.clone();
        move |value| {
            calls
                .borrow_mut()
                .push(("value", value, controller.status()))
        }
    });
    let status_id = controller.add_status_listener({
        let controller = controller.clone();
        let calls = calls.clone();
        move |status| {
            calls
                .borrow_mut()
                .push(("status", controller.value(), status))
        }
    });
    let now = Instant::now();
    assert!(
        controller
            .animate_to_curved(now, 0.75, Some(Duration::ZERO), Curve::Linear)
            .is_complete()
    );
    assert!(
        controller
            .animate_back_curved(now, 0.25, Some(Duration::ZERO), Curve::Linear)
            .is_complete()
    );
    assert_eq!(
        *calls.borrow(),
        [
            ("value", 0.75, AnimationStatus::Forward),
            ("status", 0.75, AnimationStatus::Completed),
            ("value", 0.25, AnimationStatus::Completed),
            ("status", 0.25, AnimationStatus::Dismissed),
        ]
    );
    assert!(controller.remove_listener(value_id));
    assert!(controller.remove_status_listener(status_id));
}

#[test]
fn status_listener_removal_and_addition_take_effect_at_the_correct_boundary() {
    let controller = AnimationController::new(Duration::from_secs(1));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let old_id = Rc::new(Cell::new(None));
    let first_id = controller.add_status_listener({
        let controller = controller.clone();
        let calls = calls.clone();
        let old_id = old_id.clone();
        move |status| {
            calls.borrow_mut().push(('A', status));
            if let Some(id) = old_id.take() {
                assert!(controller.remove_status_listener(id));
                let calls = calls.clone();
                controller
                    .add_status_listener(move |status| calls.borrow_mut().push(('C', status)));
            }
        }
    });
    old_id.set(Some(controller.add_status_listener({
        let calls = calls.clone();
        move |status| calls.borrow_mut().push(('B', status))
    })));
    controller.set_value(0.5);
    controller.set_value(1.);
    assert_eq!(
        *calls.borrow(),
        [
            ('A', AnimationStatus::Forward),
            ('A', AnimationStatus::Completed),
            ('C', AnimationStatus::Completed),
        ]
    );
    assert!(controller.remove_status_listener(first_id));
}

struct ConstantSimulation {
    position: f32,
    velocity: f32,
}

impl Simulation for ConstantSimulation {
    fn position(&self, _: Duration) -> f32 {
        self.position
    }

    fn velocity(&self, _: Duration) -> f32 {
        self.velocity
    }

    fn is_done(&self, _: Duration) -> bool {
        false
    }
}

// C38: Flutter engine/lib/ui/math.dart maps NaN to the upper bound.
#[test]
fn clamped_simulation_maps_nan_to_the_upper_bounds() {
    let bounded = |position, velocity| {
        ClampedSimulation::new(ConstantSimulation { position, velocity }, 0.0, 5.0)
            .with_velocity_bounds(0.0, 7.0)
    };
    let finite = bounded(-1.0, 100.0);
    let time = Duration::ZERO;
    assert_eq!([finite.position(time), finite.velocity(time)], [0.0, 7.0]);
    assert!(!finite.is_done(time));

    let nan = bounded(f32::NAN, f32::NAN);
    assert!(!nan.is_done(time), "clamping preserves inner completion");
    assert_eq!([nan.position(time), nan.velocity(time)], [5.0, 7.0]);
}

// C39: Flutter physics/clamped_simulation.dart asserts ordered bounds in debug.
#[cfg(debug_assertions)]
#[test]
fn clamped_simulation_rejects_inverted_bounds_in_debug() {
    use std::panic::catch_unwind;

    let inner = || ConstantSimulation {
        position: 0.0,
        velocity: 0.0,
    };
    assert!(
        catch_unwind(|| {
            ClampedSimulation::new(inner(), 0.0, 5.0).with_velocity_bounds(0.0, 7.0)
        })
        .is_ok(),
        "valid position and velocity bounds are accepted",
    );
    let position_rejected = catch_unwind(|| ClampedSimulation::new(inner(), 5.0, 0.0)).is_err();
    let velocity_rejected =
        catch_unwind(|| ClampedSimulation::new(inner(), 0.0, 5.0).with_velocity_bounds(7.0, 0.0))
            .is_err();
    assert_eq!([position_rejected, velocity_rejected], [true, true]);
}
