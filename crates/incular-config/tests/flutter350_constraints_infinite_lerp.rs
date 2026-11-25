//! Flutter 3.50.0 box-constraint interpolation at the finite layout boundary.

use incular_config::{ConstraintError, Constraints};
use incular_core::Lerp;

#[test]
fn matching_unbounded_maxima_preserve_infinity_and_interpolate_other_bounds() {
    let horizontal = Constraints::new(0., f32::INFINITY, 10., 20.);
    let later_horizontal = Constraints::new(0., f32::INFINITY, 20., 30.);
    let mixed_horizontal = Constraints::new(0., f32::INFINITY, 15., 25.);
    assert_eq!(horizontal.lerp(&later_horizontal, 0.5), mixed_horizontal);
    assert_eq!(
        horizontal.try_lerp(later_horizontal, 0.5),
        Ok(mixed_horizontal)
    );

    let vertical = Constraints::new(10., 20., 0., f32::INFINITY);
    let later_vertical = Constraints::new(20., 30., 0., f32::INFINITY);
    let mixed_vertical = Constraints::new(15., 25., 0., f32::INFINITY);
    assert_eq!(vertical.lerp(&later_vertical, 0.5), mixed_vertical);
    assert_eq!(vertical.try_lerp(later_vertical, 0.5), Ok(mixed_vertical));
}

#[test]
fn checked_interpolation_rejects_dimension_changes_without_ever_poisoning_layout() {
    let either = Constraints::default();
    let height = Constraints::new(20., 30., 0., f32::INFINITY);
    let width = Constraints::new(0., f32::INFINITY, 20., 30.);

    for (begin, end) in [(either, height), (either, width), (height, width)] {
        assert_eq!(begin.try_lerp(end, 0.5), Err(ConstraintError::Invalid));
        assert_eq!(end.try_lerp(begin, 0.5), Err(ConstraintError::Invalid));
        assert_eq!(begin.lerp(&end, 0.49), begin);
        assert_eq!(begin.lerp(&end, 0.5), end);
    }
}
