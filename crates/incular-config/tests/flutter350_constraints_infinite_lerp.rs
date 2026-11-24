//! Flutter 3.50.0 box-constraint interpolation with finite minima.

use incular_config::Constraints;
use incular_core::Lerp;

#[test]
fn matching_unbounded_maxima_preserve_infinity_and_interpolate_other_bounds() {
    let a = Constraints::new(0., f32::INFINITY, 10., 20.);
    let b = Constraints::new(0., f32::INFINITY, 20., 30.);

    assert_eq!(
        a.lerp(&b, 0.5),
        Constraints::new(0., f32::INFINITY, 15., 25.)
    );
}

#[test]
#[cfg(debug_assertions)]
fn mixing_finite_and_unbounded_maxima_is_rejected_in_both_directions() {
    let finite = Constraints::new(0., 100., 10., 20.);
    let unbounded = Constraints::new(0., f32::INFINITY, 20., 30.);
    let rejected = [
        std::panic::catch_unwind(|| finite.lerp(&unbounded, 0.5)).is_err(),
        std::panic::catch_unwind(|| unbounded.lerp(&finite, 0.5)).is_err(),
    ];

    assert_eq!(rejected, [true, true]);
}
