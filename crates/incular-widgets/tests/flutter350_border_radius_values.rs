//! Flutter 3.50.0 physical and directional radii are value arithmetic,
//! separate from the size-dependent normalization of their painted paths.

use incular_config::TextDirection::{Ltr, Rtl};
use incular_core::{Offset, Rect, Size};
use incular_widgets::{BorderRadius, BorderRadiusDirectional, BorderRadiusGeometry, Radius};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    ops::{Add, Sub},
};

#[test]
fn physical_and_directional_factories_form_the_same_raw_rounded_rects() {
    let rect = Rect::from_origin_size(Offset::new(19., 23.), Size::new(10., 8.));
    let a = Radius::elliptical(89., 87.);
    let b = Radius::elliptical(103., 107.);
    let z = Radius::ZERO;
    let cases = [
        (
            BorderRadius::all(Radius::elliptical(5., 7.)),
            BorderRadiusDirectional::all(Radius::elliptical(5., 7.)),
            [Radius::elliptical(5., 7.); 4],
            [Radius::elliptical(5., 7.); 4],
        ),
        (
            BorderRadius::circular(3.),
            BorderRadiusDirectional::circular(3.),
            [Radius::circular(3.); 4],
            [Radius::circular(3.); 4],
        ),
        (
            BorderRadius::vertical(a, b),
            BorderRadiusDirectional::vertical(a, b),
            [a, a, b, b],
            [a, a, b, b],
        ),
        (
            BorderRadius::horizontal(a, b),
            BorderRadiusDirectional::horizontal(a, b),
            [a, b, b, a],
            [b, a, a, b],
        ),
        (
            BorderRadius::ZERO,
            BorderRadiusDirectional::ZERO,
            [z; 4],
            [z; 4],
        ),
        (
            BorderRadius::only(z, a, b, z),
            BorderRadiusDirectional::only(z, a, b, z),
            [z, a, b, z],
            [a, z, z, b],
        ),
    ];
    for (physical, directional, ltr, rtl) in cases {
        assert_eq!(format!("{physical:?}").lines().count(), 1);
        assert_eq!(format!("{directional:?}").lines().count(), 1);
        let raw = |r: BorderRadius| {
            let shape = r.to_rounded_rect(rect);
            assert_eq!(shape.rect, rect);
            let r = shape.radii;
            [r.top_left, r.top_right, r.bottom_right, r.bottom_left]
        };
        assert_eq!(raw(physical), ltr);
        assert_eq!(raw(directional.resolve(Ltr)), ltr);
        assert_eq!(raw(directional.resolve(Rtl)), rtl);
        assert_eq!(
            [
                directional.top_start,
                directional.top_end,
                directional.bottom_end,
                directional.bottom_start
            ],
            ltr
        );
    }
}

#[test]
fn signed_radius_arithmetic_preserves_raw_values_in_each_family() {
    let z = Radius::ZERO;
    let a = Radius::elliptical(1., 2.);
    let b = Radius::elliptical(3., 5.);
    let differences = Radius::elliptical(-2., -3.);
    let physical = |first, second| BorderRadius::only(first, second, z, z);
    let directional = |first, second| BorderRadiusDirectional::only(first, second, z, z);
    assert_eq!(physical(a, z) - physical(b, z), physical(differences, z));
    assert_eq!(
        Sub::sub(physical(a, z), physical(b, z)),
        physical(differences, z)
    );
    assert_eq!(physical(z, a) + physical(b, z), physical(b, a));
    assert_eq!(Add::add(physical(z, a), physical(b, z)), physical(b, a));
    assert_eq!(-physical(a, z), physical(Radius::elliptical(-1., -2.), z));
    assert_eq!(BorderRadius::only(a, b, a, b) * 0., BorderRadius::ZERO);
    assert_eq!(
        BorderRadius::circular(15.) / 10.,
        BorderRadius::circular(1.5)
    );
    assert_eq!(
        BorderRadius::circular(15.).trunc_div(10.),
        BorderRadius::circular(1.)
    );
    assert_eq!(
        BorderRadius::circular(15.) % 10.,
        BorderRadius::circular(5.)
    );

    assert_eq!(
        directional(a, z) - directional(b, z),
        directional(differences, z)
    );
    assert_eq!(
        Sub::sub(directional(a, z), directional(b, z)),
        directional(differences, z)
    );
    assert_eq!(directional(z, a) + directional(b, z), directional(b, a));
    assert_eq!(
        Add::add(directional(z, a), directional(b, z)),
        directional(b, a)
    );
    assert_eq!(
        -directional(a, z),
        directional(Radius::elliptical(-1., -2.), z)
    );
    assert_eq!(
        BorderRadiusDirectional::only(a, b, a, b) * 0.,
        BorderRadiusDirectional::ZERO
    );
    assert_eq!(
        BorderRadiusDirectional::circular(15.) / 10.,
        BorderRadiusDirectional::circular(1.5)
    );
    assert_eq!(
        BorderRadiusDirectional::circular(15.).trunc_div(10.),
        BorderRadiusDirectional::circular(1.)
    );
    assert_eq!(
        BorderRadiusDirectional::circular(15.) % 10.,
        BorderRadiusDirectional::circular(5.)
    );

    let mixed = BorderRadiusGeometry::from(BorderRadius::circular(17.))
        + BorderRadiusGeometry::from(BorderRadiusDirectional::circular(-15.));
    assert_eq!(
        (mixed / 10.).resolve(Rtl),
        BorderRadius::circular(0.20000005)
    );
    assert_eq!(mixed.trunc_div(10.).resolve(Rtl), BorderRadius::ZERO);
    assert_eq!((mixed % 10.).resolve(Rtl), BorderRadius::circular(12.));
    assert_eq!((-mixed).resolve(Rtl), BorderRadius::circular(-2.));
    assert_eq!(
        Radius::elliptical(-15., 15.).trunc_div(10.),
        Radius::elliptical(-1., 1.)
    );
}

#[test]
fn geometry_factories_preserve_the_variant_and_reject_ambiguous_horizontal_sides() {
    use BorderRadiusGeometry as Geometry;
    let [a, b, c, d] = [5., 10., 15., 20.].map(Radius::circular);
    assert_eq!(Geometry::all(b), BorderRadius::all(b).into());
    assert_eq!(Geometry::circular(10.), BorderRadius::circular(10.).into());
    assert_eq!(Geometry::ZERO, BorderRadius::ZERO.into());
    assert_eq!(
        Geometry::horizontal(Some(a), Some(b), None, None),
        BorderRadius::horizontal(a, b).into()
    );
    assert_eq!(
        Geometry::horizontal(None, None, Some(a), Some(b)),
        BorderRadiusDirectional::horizontal(a, b).into()
    );
    assert_eq!(
        Geometry::horizontal(None, None, None, Some(b)),
        BorderRadiusDirectional::horizontal(Radius::ZERO, b).into()
    );
    for (left, right, start, end) in [
        (Some(b), None, Some(a), None),
        (None, Some(b), None, Some(a)),
        (Some(b), Some(b), Some(a), Some(a)),
    ] {
        assert!(
            std::panic::catch_unwind(|| Geometry::horizontal(left, right, start, end)).is_err()
        );
    }
    assert_eq!(
        Geometry::only(a, b, d, c),
        BorderRadius::only(a, b, d, c).into()
    );
    assert_eq!(
        Geometry::directional(a, b, d, c),
        BorderRadiusDirectional::only(a, b, d, c).into()
    );
    assert_eq!(
        Geometry::vertical(a, b),
        BorderRadius::vertical(a, b).into()
    );
}

#[test]
fn copying_with_no_corner_overrides_preserves_value_and_hash() {
    let first = BorderRadius::circular(5.);
    let second = first.copy_with(None, None, None, None);
    let hash = |radius: BorderRadius| {
        let mut hasher = DefaultHasher::new();
        radius.hash(&mut hasher);
        hasher.finish()
    };
    assert_eq!(first, second);
    assert_eq!(hash(first), hash(second));
}
