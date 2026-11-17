//! Springs, transitions and velocity-preserving motion values.

use std::time::Duration;

use incular_animation::{
    Channel, Curve, Keyframes, MotionValue, Spring, Transition, TransitionKind, ms, secs,
};
use incular_core::time::Instant;

fn run(value: &MotionValue, start: Instant, frames: u32) -> Instant {
    let mut now = start;
    for _ in 0..frames {
        now += ms(16);
        value.tick(now);
    }
    now
}

#[test]
fn duration_helpers() {
    assert_eq!(ms(250), Duration::from_millis(250));
    assert_eq!(secs(0.5), Duration::from_millis(500));
    assert_eq!(secs(-1.), Duration::ZERO);
    assert_eq!(secs(f32::NAN), Duration::ZERO);
}

#[test]
fn transition_conversions_and_builders() {
    let t: Transition = ms(200).into();
    assert_eq!(
        t.kind(),
        TransitionKind::Ease {
            duration: ms(200),
            curve: Curve::EaseOut
        }
    );
    let t = Transition::ease(ms(100))
        .curve(Curve::EaseInOut)
        .delay(ms(20));
    assert_eq!(t.delay_duration(), ms(20));
    let t: Transition = Spring::bouncy().into();
    assert!(matches!(t.kind(), TransitionKind::Spring(_)));
    assert!(Transition::instant().is_instant());
}

#[test]
fn opacity_never_springs_unless_overridden() {
    let spring = Transition::spring(Spring::bouncy()).delay(ms(30));
    let fade = spring.for_channel(Channel::Opacity);
    assert!(matches!(fade.kind(), TransitionKind::Ease { .. }));
    assert_eq!(fade.delay_duration(), ms(30));
    assert!(matches!(
        spring.for_channel(Channel::Geometry).kind(),
        TransitionKind::Spring(_)
    ));
    let custom = spring.opacity(Transition::linear(ms(80)));
    assert_eq!(
        custom.for_channel(Channel::Opacity).kind(),
        TransitionKind::Ease {
            duration: ms(80),
            curve: Curve::Linear
        }
    );
}

#[test]
fn ease_reaches_target_and_starts_on_first_tick() {
    let value = MotionValue::new(0.);
    assert!(value.animate_to(100., &Transition::linear(ms(100))));
    let start = Instant::now();
    // The first tick fixes the start time: nothing moves yet.
    value.tick(start);
    assert_eq!(value.get(), 0.);
    value.tick(start + ms(50));
    assert!((value.get() - 50.).abs() < 0.01);
    value.tick(start + ms(100));
    assert_eq!(value.get(), 100.);
    assert!(!value.is_animating());
}

#[test]
fn ease_reversal_is_shortened() {
    let value = MotionValue::new(0.);
    value.animate_to(100., &Transition::linear(ms(100)));
    let start = Instant::now();
    value.tick(start);
    value.tick(start + ms(25));
    assert!((value.get() - 25.).abs() < 0.01);
    value.animate_to(0., &Transition::linear(ms(100)));
    let back = start + ms(25);
    value.tick(back);
    // A quarter of the way back took a quarter of the duration.
    value.tick(back + ms(25));
    assert_eq!(value.get(), 0.);
}

#[test]
fn spring_retarget_keeps_velocity() {
    let value = MotionValue::new(0.);
    value.animate_to(100., &Transition::spring(Spring::snappy()));
    let start = Instant::now();
    value.tick(start);
    let now = run(&value, start, 5);
    let velocity = value.velocity();
    let position = value.get();
    assert!(velocity > 0., "moving toward the target");
    value.animate_to(-100., &Transition::spring(Spring::snappy()));
    value.tick(now);
    // No jump: the value continues from where it was with the same velocity.
    assert!((value.get() - position).abs() < 1e-3);
    value.tick(now + ms(1));
    assert!(value.get() > position, "momentum carries it forward first");
    run(&value, now, 200);
    assert!((value.get() + 100.).abs() < 0.01);
    assert!(!value.is_animating());
}

#[test]
fn spring_settles_exactly() {
    let value = MotionValue::new(1.);
    value.animate_to(1.2, &Spring::bouncy().into());
    let start = Instant::now();
    value.tick(start);
    let mut overshot = false;
    let mut now = start;
    for _ in 0..200 {
        now += ms(16);
        value.tick(now);
        overshot |= value.get() > 1.2;
    }
    assert!(overshot, "a bouncy spring overshoots");
    assert_eq!(value.get(), 1.2);
}

#[test]
fn delay_holds_then_moves() {
    let value = MotionValue::new(0.);
    value.animate_to(10., &Transition::linear(ms(100)).delay(ms(50)));
    let start = Instant::now();
    value.tick(start);
    value.tick(start + ms(40));
    assert_eq!(value.get(), 0.);
    value.tick(start + ms(100));
    assert!((value.get() - 5.).abs() < 0.01);
}

#[test]
fn instant_jumps_and_set_stops() {
    let value = MotionValue::new(0.);
    assert!(value.animate_to(5., &Transition::instant()));
    assert_eq!(value.get(), 5.);
    value.animate_to(10., &Transition::linear(ms(100)));
    value.set(3.);
    assert!(!value.is_animating());
    assert_eq!(value.get(), 3.);
    assert!(!value.animate_to(3., &Transition::linear(ms(100))));
}

#[test]
fn derived_values_follow_their_source() {
    let x = MotionValue::new(10.);
    let half = x.map(|v| v / 2.);
    assert_eq!(half.get(), 5.);
    x.set(40.);
    assert_eq!(half.get(), 20.);
    assert!(!half.set(1.));
    assert!(half.is_derived());
    x.animate_to(100., &Transition::linear(ms(100)));
    assert!(half.is_animating());
    assert_eq!(half.target(), 50.);
}

#[test]
fn keyframes_sample_segments() {
    let track = Keyframes::new([1., 1.2, 1.], ms(100)).curve(Curve::Linear);
    assert_eq!(track.sample(0.), 1.);
    assert!((track.sample(0.25) - 1.1).abs() < 1e-5);
    assert!((track.sample(0.5) - 1.2).abs() < 1e-5);
    assert_eq!(track.sample(1.), 1.);
    let timed = Keyframes::new([0., 10., 0.], ms(100))
        .times([0., 0.2, 1.])
        .curve(Curve::Linear);
    assert!((timed.sample(0.1) - 5.).abs() < 1e-4);

    let value = MotionValue::new(1.);
    value.animate_keys(track, Duration::ZERO);
    let start = Instant::now();
    value.tick(start);
    value.tick(start + ms(50));
    assert!((value.get() - 1.2).abs() < 1e-4);
    value.tick(start + ms(100));
    assert_eq!(value.get(), 1.);
    assert!(!value.is_animating());
}
