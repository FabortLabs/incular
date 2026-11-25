//! Observable cases from Flutter 3.50.0 rendering/box_constraints_test.dart.

use incular_config::Constraints;
use incular_core::Size;

#[test]
fn normalize_raises_the_maximum_to_the_minimum() {
    let normalized = Constraints::try_normalized(3., 2., 11., 18.).unwrap();
    assert_eq!(normalized.min_width(), 3.);
    assert_eq!(normalized.max_width(), 3.);
    assert_eq!(normalized.min_height(), 11.);
    assert_eq!(normalized.max_height(), 18.);
}

#[test]
fn normalization_preserves_the_validated_layout_invariant() {
    assert_eq!(
        Constraints::try_normalized(-3., -2., 4., f32::INFINITY).unwrap(),
        Constraints::new(0., 0., 4., f32::INFINITY),
    );
    assert!(Constraints::try_normalized(0., f32::NAN, 0., 1.).is_err());
    assert!(Constraints::try_normalized(f32::INFINITY, f32::INFINITY, 0., 1.).is_err());
    assert!(Constraints::try_new(3., 2., 11., 18.).is_err());
}

#[test]
fn preserving_aspect_ratio_constrains_each_empty_dimension_without_dividing() {
    let constraints = Constraints::new(10., 20., 10., 20.);
    assert_eq!(
        constraints.constrain_preserving_aspect_ratio(Size::new(15., 0.)),
        Size::new(15., 10.)
    );
    assert_eq!(
        constraints.constrain_preserving_aspect_ratio(Size::new(0., 15.)),
        Size::new(10., 15.)
    );
}
