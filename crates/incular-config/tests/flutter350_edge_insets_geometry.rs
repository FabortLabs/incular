//! Unresolved physical/directional EdgeInsetsGeometry arithmetic.

use incular_config::{
    EdgeInsets, EdgeInsetsDirectional, EdgeInsetsGeometry as G, MissingInsetsDirection,
    TextDirection,
};
use incular_core::Lerp;

const P: fn(f32, f32, f32, f32) -> G = G::only;
const D: fn(f32, f32, f32, f32) -> G = G::directional;

#[test]
fn equality_compares_all_six_unresolved_components() {
    let p = P(0., 5., 0., 7.);
    let d = D(0., 5., 0., 7.);
    assert_eq!(p, p);
    assert_eq!(d, d);
    assert_eq!(p, d);
    assert_eq!(d, p);

    let left = P(5., 0., 0., 0.);
    let start = D(5., 0., 0., 0.);
    let right = P(0., 0., 5., 0.);
    let end = D(0., 0., 5., 0.);
    assert_ne!(left, start);
    assert_ne!(start, left);
    assert_ne!(right, end);
    assert_ne!(end, right);
    assert_eq!(end + right, right + end);
    assert_ne!(end + right, end + left);
    let vertical = D(0., 1., 0., 0.) + P(0., 2., 0., 0.);
    assert_eq!(vertical, D(0., 3., 0., 0.));
    assert_eq!(vertical, P(0., 3., 0., 0.));
}

#[test]
fn mixed_lerp_does_not_resolve_start_and_end_early() {
    let lerp = G::lerp_optional;
    for (a, expected) in [
        (D(10., 0., 0., 0.), D(5., 0., 0., 0.)),
        (D(0., 10., 0., 0.), D(0., 5., 0., 0.)),
        (D(0., 0., 10., 0.), D(0., 0., 5., 0.)),
        (D(0., 0., 0., 10.), D(0., 0., 0., 5.)),
    ] {
        assert_eq!(lerp(Some(a), None, 0.5), Some(expected));
    }
    let directional = D(0., 0., 0., 10.);
    for zero in [D(0., 0., 0., 0.), G::ZERO] {
        assert_eq!(
            lerp(Some(directional), Some(zero), 0.5),
            Some(D(0., 0., 0., 5.))
        );
    }

    let crossed = D(10., 0., 0., 0.).lerp(&P(20., 0., 0., 0.), 0.5);
    assert_eq!(crossed, D(5., 0., 0., 0.) + P(10., 0., 0., 0.));
    assert_eq!(
        crossed.resolve(TextDirection::Ltr),
        EdgeInsets::only(15., 0., 0., 0.)
    );
    assert_eq!(
        crossed.resolve(TextDirection::Rtl),
        EdgeInsets::only(10., 0., 5., 0.)
    );
    assert_eq!(
        D(0., 0., 0., 1.).lerp(&(D(1., 0., 0., 1.) + P(0., 0., 2., 0.)), 0.5),
        D(0.5, 0., 0., 0.) + P(0., 0., 1., 1.)
    );
    assert_eq!(
        P(0., 0., 0., 1.).lerp(&(D(0., 0., 1., 1.) + P(0., 0., 2., 0.)), 0.5),
        D(0., 0., 0.5, 0.) + P(0., 0., 1., 1.)
    );
}

#[test]
fn lerp_preserves_physical_directional_and_mixed_components() {
    let sets = [
        (G::all(10.), G::all(20.), G::all(2.5), G::all(7.5)),
        (
            D(10., 0., 10., 0.),
            D(20., 0., 20., 0.),
            D(2.5, 0., 2.5, 0.),
            D(7.5, 0., 7.5, 0.),
        ),
        (
            D(10., 0., 10., 0.) + G::all(1.),
            D(20., 0., 20., 0.) + G::all(2.),
            D(2.5, 0., 2.5, 0.) + G::all(0.25),
            D(7.5, 0., 7.5, 0.) + G::all(0.75),
        ),
    ];
    for (a, b, extra, less) in sets {
        assert_eq!(G::lerp_optional(Some(a), Some(b), 0.25), Some(a * 1.25));
        assert_eq!(G::lerp_optional(Some(a), Some(b), 0.25), Some(b * 0.625));
        assert_eq!(G::lerp_optional(Some(a), Some(b), 0.25), Some(a + extra));
        assert_eq!(G::lerp_optional(Some(a), Some(b), 0.25), Some(b - less));
        assert_eq!(G::lerp_optional(None, None, 0.25), None);
        assert_eq!(G::lerp_optional(None, Some(b), 0.25), Some(b * 0.25));
        assert_eq!(G::lerp_optional(Some(a), None, 0.25), Some(a * 0.75));
    }
}

#[test]
fn operators_preserve_the_mixed_representation() {
    let a = D(1., 2., 3., 5.) + G::ZERO;
    assert!(matches!(a, G::Mixed { .. }));
    assert_eq!(a * 2., D(2., 4., 6., 10.));
    assert_eq!(a / 2., D(0.5, 1., 1.5, 2.5));
    assert_eq!(a % 2., D(1., 0., 1., 1.));
    assert_eq!(a.truncating_div(2.), D(0., 1., 1., 2.));
    assert_eq!(a + a, a * 2.);
    assert_eq!(a - a, G::ZERO);
    assert_eq!(a - a, D(0., 0., 0., 0.));
}

#[test]
fn display_formats_physical_directional_and_mixed_values() {
    assert_eq!(EdgeInsets::ZERO.to_string(), "EdgeInsets.zero");
    assert_eq!(EdgeInsets::all(1.01).to_string(), "EdgeInsets.all(1.0)");
    assert_eq!(
        EdgeInsetsDirectional::all(1.01).to_string(),
        "EdgeInsetsDirectional(1.0, 1.0, 1.0, 1.0)"
    );
    assert_eq!(
        (D(4., 0., 0., 0.) + P(0., 3., 0., 0.)).to_string(),
        "EdgeInsetsDirectional(4.0, 3.0, 0.0, 0.0)"
    );
    assert_eq!(
        (D(0., 4., 0., 0.) + P(0., 0., 3., 0.)).to_string(),
        "EdgeInsets(0.0, 4.0, 3.0, 0.0)"
    );
    assert_eq!(
        (D(4., 0., 0., 0.) + P(3., 0., 0., 0.)).to_string(),
        "EdgeInsets(3.0, 0.0, 0.0, 0.0) + EdgeInsetsDirectional(4.0, 0.0, 0.0, 0.0)"
    );
}

#[test]
fn geometry_factories_preserve_concrete_kind_and_convert_view_padding() {
    assert_eq!(G::all(10.), EdgeInsets::all(10.).into());
    assert_eq!(
        G::only(10., 20., 30., 40.),
        EdgeInsets::only(10., 20., 30., 40.).into()
    );
    assert_eq!(
        G::directional(10., 20., 30., 40.),
        EdgeInsetsDirectional::only(10., 20., 30., 40.).into()
    );
    assert_eq!(
        G::symmetric(10., 20.),
        EdgeInsets::symmetric(10., 20.).into()
    );
    assert_eq!(
        G::from_ltrb(10., 20., 30., 40.),
        EdgeInsets::from_ltrb(10., 20., 30., 40.).into()
    );
    assert_eq!(
        G::from_steb(10., 20., 20., 40.),
        EdgeInsetsDirectional::from_steb(10., 20., 20., 40.).into()
    );
    assert_eq!(G::ZERO, EdgeInsets::ZERO.into());
    assert_eq!(
        G::from_view_padding(EdgeInsets::ZERO, 10.),
        EdgeInsets::from_view_padding(EdgeInsets::ZERO, 10.).into()
    );
    assert_eq!(
        G::from_view_padding(EdgeInsets::only(100., 20., 50., 16.), 2.),
        P(50., 10., 25., 8.)
    );
}

#[test]
fn only_physical_edges_resolve_without_direction_and_error_names_the_missing_ancestor() {
    let direct = EdgeInsetsDirectional::all(10.);
    assert_eq!(direct.try_resolve(None), Err(MissingInsetsDirection));
    assert_eq!(
        direct.try_resolve(Some(TextDirection::Rtl)),
        Ok(EdgeInsets::all(10.))
    );
    let physical = P(5., 5., 0., 0.);
    let directional = D(15., 15., 0., 0.);
    assert_eq!(
        physical.try_resolve(None),
        Ok(EdgeInsets::only(5., 5., 0., 0.))
    );
    for geometry in [
        D(10., 10., 10., 10.),
        physical + directional,
        directional - physical,
    ] {
        let error = geometry.try_resolve(None).unwrap_err();
        assert_eq!(error, MissingInsetsDirection);
        assert!(error.to_string().contains("No TextDirection found."));
        assert!(
            error
                .to_string()
                .contains("without a Directionality ancestor")
        );
    }
}

#[test]
fn identical_optional_endpoints_keep_their_value_without_interpolating() {
    let physical = EdgeInsets::ZERO;
    let directional = EdgeInsetsDirectional::ZERO;
    let geometry = G::ZERO;
    assert_eq!(EdgeInsets::lerp_optional(None, None, 0.), None);
    assert_eq!(EdgeInsetsDirectional::lerp_optional(None, None, 0.), None);
    assert_eq!(G::lerp_optional(None, None, 0.), None);
    assert_eq!(
        EdgeInsets::lerp_optional(Some(physical), Some(physical), 0.5),
        Some(physical)
    );
    assert_eq!(
        EdgeInsetsDirectional::lerp_optional(Some(directional), Some(directional), 0.5),
        Some(directional)
    );
    assert_eq!(
        G::lerp_optional(Some(geometry), Some(geometry), 0.5),
        Some(geometry)
    );
    let infinite = EdgeInsets::all(f32::INFINITY);
    assert_eq!(
        EdgeInsets::lerp_optional(Some(infinite), Some(infinite), 0.5),
        Some(infinite)
    );
}
