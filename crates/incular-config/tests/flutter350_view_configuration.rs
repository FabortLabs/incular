use incular_config::{Constraints, ViewConfiguration, ViewConstraints};
use incular_core::{Matrix4, Size};
use std::hash::{DefaultHasher, Hash, Hasher};

fn digest(configuration: ViewConfiguration) -> u64 {
    let mut state = DefaultHasher::new();
    configuration.hash(&mut state);
    state.finish()
}

#[test]
fn equality_and_hash_include_constraints_and_device_pixel_ratio() {
    let make = |physical, ratio| {
        ViewConfiguration::new(
            Constraints::tight(Size::new(20., 20.)),
            Constraints::tight(Size::new(physical, physical)),
            ratio,
        )
        .unwrap()
    };
    let a = make(40., 2.);
    let b = make(40., 2.);
    let c = make(60., 3.);
    assert_eq!(a, b);
    assert_eq!(digest(a), digest(b));
    assert_ne!(a, c);
    assert_ne!(digest(a), digest(c));
    assert_eq!(a.to_matrix(), Matrix4::scale(2., 2., 1.));
    assert!(!a.should_update_matrix(make(60., 2.)));
    assert!(a.should_update_matrix(c));
}

#[test]
fn sizing_uses_double_scale_then_clamps_to_physical_constraints() {
    let configuration =
        ViewConfiguration::from_view_constraints(ViewConstraints::new(40., 80., 10., 50.), 1.25)
            .unwrap();
    assert_eq!(
        configuration.logical_constraints(),
        Constraints::new(32., 64., 8., 40.)
    );
    assert_eq!(
        configuration.to_physical_size(Size::new(1., 300.)),
        Some(Size::new(40., 50.))
    );
    assert_eq!(
        configuration.to_physical_size(Size::new(50., 18.)),
        Some(Size::new(62.5, 22.5))
    );
    assert!(ViewConfiguration::from_view_constraints(ViewConstraints::default(), 0.).is_err());
    assert!(
        ViewConfiguration::from_view_constraints(ViewConstraints::default(), f64::NAN).is_err()
    );
    let default = ViewConfiguration::default();
    assert_eq!(
        default.logical_constraints(),
        Constraints::tight(Size::ZERO)
    );
    assert_eq!(
        default.physical_constraints(),
        Constraints::tight(Size::ZERO)
    );
    assert_eq!(default.device_pixel_ratio(), 1.);
}
