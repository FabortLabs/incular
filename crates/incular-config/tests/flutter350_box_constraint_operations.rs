use incular_config::{ConstraintError, Constraints, ViewConstraints};
use incular_core::Size;

#[test]
fn box_constraints_arithmetic_uses_each_axis_bound() {
    let start = Constraints::new(3., 7., 11., 17.);
    let doubled = start * 2.;
    assert_eq!(doubled, Constraints::new(6., 14., 22., 34.));
    assert_eq!(doubled / 2., start);
    assert_eq!(start.truncating_div(2.), Constraints::new(1., 3., 5., 8.));
    assert_eq!(start % 3., Constraints::new(0., 1., 2., 2.));
    assert!(std::panic::catch_unwind(|| Constraints::default().truncating_div(2.)).is_err());
    assert!(std::panic::catch_unwind(|| start / 0.).is_err());
}

#[test]
fn host_view_bounds_become_validated_layout_constraints() {
    assert_eq!(
        Constraints::from_view_constraints(ViewConstraints::default()),
        Constraints::default(),
    );
    let view = ViewConstraints::new(1., 2., 3., 4.);
    let constraints = Constraints::from_view_constraints(view);
    assert_eq!(constraints, Constraints::new(1., 2., 3., 4.));
    assert_eq!(ViewConstraints::from(constraints), view);
    assert_eq!(constraints.constrain(Size::new(50., 1.)), Size::new(2., 3.));
    assert!(view.is_satisfied_by(Size::new(2., 3.)));
    assert!(!view.is_satisfied_by(Size::new(3., 3.)));
    assert!(ViewConstraints::tight(Size::new(3., 4.)).is_tight());
    assert_eq!(
        ViewConstraints::new(2., 4., 6., f32::INFINITY) / 2.,
        ViewConstraints::new(1., 2., 3., f32::INFINITY),
    );
    assert_eq!(
        Constraints::try_from(ViewConstraints::new(f32::INFINITY, f32::INFINITY, 0., 1.)),
        Err(ConstraintError::Invalid),
    );
}

#[test]
fn copying_constraint_bounds_only_replaces_the_requested_values() {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
    let hash = |value| BuildHasherDefault::<DefaultHasher>::default().hash_one(value);
    let start = Constraints::new(3., 7., 11., 17.);
    let unchanged = start.copy_with(None, None, None, None);
    assert_eq!(unchanged, start);
    assert_eq!(hash(unchanged), hash(start));
    let modified = start.copy_with(Some(13.), Some(17.), Some(111.), Some(117.));
    assert_eq!(modified, Constraints::new(13., 17., 111., 117.));
    assert_ne!(modified, start);
    assert_ne!(hash(modified), hash(start));
    assert_eq!(
        hash(Constraints::new(-0., 7., -0., f32::INFINITY)),
        hash(Constraints::new(0., 7., 0., f32::INFINITY)),
    );
    assert_eq!(
        start.copy_with(None, Some(9.), None, None),
        Constraints::new(3., 9., 11., 17.),
    );
}
