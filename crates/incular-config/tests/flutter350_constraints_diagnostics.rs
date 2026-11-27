use std::borrow::Cow;

use incular_config::{Constraints, ViewConstraints};

#[test]
fn displays_the_pinned_bound_descriptions_without_admitting_invalid_layout() {
    let biggest = ViewConstraints::new(f32::INFINITY, f32::INFINITY, f32::INFINITY, f32::INFINITY);
    assert_eq!(biggest.to_string(), "BoxConstraints(biggest)");
    assert!(Constraints::try_from(biggest).is_err());
    assert_eq!(
        Constraints::default().to_string(),
        "BoxConstraints(unconstrained)"
    );
    assert_eq!(
        Constraints::new(50., 50., 0., f32::INFINITY).to_string(),
        "BoxConstraints(w=50.0, 0.0<=h<=Infinity)"
    );
    assert_eq!(
        ViewConstraints::new(8., 4., 0., 20.).to_string(),
        "BoxConstraints(8.0<=w<=4.0, 0.0<=h<=20.0; NOT NORMALIZED)"
    );
    assert_eq!(
        Constraints::new(1.25, 1.25, 2.25, 2.25).to_string(),
        "BoxConstraints(w=1.3, h=2.3)"
    );
}

#[test]
fn absent_or_identical_endpoints_do_not_allocate_a_new_constraint() {
    assert!(Constraints::lerp_ref(None, None, 0.).is_none());
    let original = Constraints::default();
    let Some(Cow::Borrowed(value)) = Constraints::lerp_ref(Some(&original), Some(&original), 0.5)
    else {
        panic!("identical endpoints should be borrowed");
    };
    assert!(std::ptr::eq(&original, value));
    let equal = Box::new(original);
    let Some(Cow::Owned(value)) = Constraints::lerp_ref(Some(&original), Some(&equal), 0.5) else {
        panic!("equal but distinct endpoints are not identical");
    };
    assert_eq!(value, original);
}
