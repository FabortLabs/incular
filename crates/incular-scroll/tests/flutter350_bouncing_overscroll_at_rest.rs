//! Flutter 3.50 stationary out-of-range ballistic dispatch, at DPR 1.

use incular_scroll::{FlingSimulation, ScrollPhysics};

const VALUE_TOLERANCE: f64 = 0.001;

fn close_to(actual: f32, expected: f64) {
    assert!(
        (f64::from(actual) - expected).abs() <= VALUE_TOLERANCE,
        "expected {expected} ± {VALUE_TOLERANCE}, got {actual}",
    );
}

fn simulation() -> FlingSimulation {
    let physics = ScrollPhysics::BOUNCING.with_device_pixel_ratio(1.0);
    assert_eq!(physics.tolerance().distance, 1.0);
    assert_eq!(physics.tolerance().velocity, 20.0);
    assert!(
        physics
            .ballistic_simulation(90.0, 0.0, 0.0, 100.0)
            .is_none()
    );
    physics
        .ballistic_simulation(110.0, 0.0, 0.0, 100.0)
        .expect("an out-of-range position returns to its extent at zero release velocity")
}

#[test]
fn stationary_overscroll_follows_flutter_rubber_band_decay() {
    let motion = simulation();
    close_to(motion.x(0.0), 110.0);
    close_to(motion.dx(0.0), 0.0);
    // First 16 ms sample from the pinned Flutter frame fixture.
    let (x, dx) = (motion.x(0.016), motion.dx(0.016));
    assert!(
        (f64::from(x) - 108.534_881_326_147_2).abs() <= VALUE_TOLERANCE
            && (f64::from(dx) + 84.513_270_394_693_52).abs() <= VALUE_TOLERANCE,
        "at 16 ms, expected (x, dx) near (108.5348813261472, -84.51327039469352), got ({x}, {dx})",
    );
}

#[test]
fn stationary_overscroll_finishes_and_snaps_by_240_ms() {
    let motion = simulation();
    assert!(!motion.is_done(0.224));
    assert!(motion.is_done(0.320));
    assert!(
        motion.is_done(0.240) && motion.x(0.240) == 100.0,
        "expected done and snapped to 100 at 240 ms, got done={} x={}",
        motion.is_done(0.240),
        motion.x(0.240),
    );
    // The scroll spring snaps position, but dx still samples its derivative.
    close_to(motion.dx(0.240), -9.196_634_393_526_46);
}
