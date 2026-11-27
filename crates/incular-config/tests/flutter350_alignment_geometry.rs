use std::hash::{DefaultHasher, Hash, Hasher};

use incular_config::{
    Alignment as A, AlignmentDirectional as D, AlignmentGeometry as G, TextDirection,
};
use incular_core::Lerp;

const DIRECTIONS: [TextDirection; 2] = [TextDirection::Ltr, TextDirection::Rtl];

fn hash(value: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn physical_alignment_value_hash_diagnostics_and_scalar_operators() {
    let alignment = A::new(0.5, 0.25);
    assert_eq!(format!("{alignment}"), "Alignment(0.5, 0.3)");
    assert_eq!(hash(alignment), hash(A::new(0.5, 0.25)));
    assert_eq!(alignment / 2., A::new(0.25, 0.125));
    assert_eq!(alignment.truncating_div(2.), A::CENTER);
    assert_eq!(alignment % 5., alignment);
    assert_eq!((-alignment) % 2., A::new(1.5, 1.75));
    assert_eq!(hash(A::new(-0.0, 0.0)), hash(A::CENTER));
}

#[test]
fn nullable_alignment_interpolation_scales_against_center_without_clamping() {
    let (a, b) = (A::TOP_LEFT, A::TOP_CENTER);
    assert_eq!(
        A::lerp_optional(Some(a), Some(b), 0.25),
        Some(A::new(-0.75, -1.))
    );
    assert_eq!(A::lerp_optional(None, None, 0.25), None);
    assert_eq!(
        A::lerp_optional(None, Some(b), 0.25),
        Some(A::new(0., -0.25))
    );
    assert_eq!(
        A::lerp_optional(Some(a), None, 0.25),
        Some(A::new(-0.75, -0.75))
    );
    assert_eq!(
        A::lerp_optional(Some(a), Some(b), 2.),
        Some(A::new(1., -1.))
    );

    let endpoint = D::TOP_START;
    assert_eq!(D::lerp_optional(None, None, 0.), None);
    assert_eq!(
        D::lerp_optional(Some(endpoint), Some(endpoint), 0.5),
        Some(endpoint)
    );
    for t in [-1., 0., 0.25, 1., 2.] {
        assert_eq!(
            G::lerp_optional(None, Some(G::TOP_START), t),
            Some(G::TOP_START * t)
        );
    }
}

#[test]
fn mixed_arithmetic_retains_components_and_satisfies_the_original_invariants() {
    let top_start = G::TOP_START;
    let top_end = G::TOP_END;
    let top_left = G::TOP_LEFT;
    let top_right = G::TOP_RIGHT;
    assert_eq!(top_end * 0. + top_right * 0., G::CENTER);
    assert_eq!((top_end + top_right) * 0., top_end * 0. + top_right * 0.);
    assert_eq!(top_start + top_left, top_left + top_start);
    for direction in DIRECTIONS {
        assert_eq!(
            (top_start + top_left).resolve(direction),
            top_start.resolve(direction) + A::TOP_LEFT
        );
        assert_eq!(
            (top_end * 0. + top_right * 0.).resolve(direction),
            A::CENTER
        );
        assert_eq!((top_end * 0. + top_left).resolve(direction), A::TOP_LEFT);
        assert_eq!((top_end * 0.).resolve(direction) + A::TOP_LEFT, A::TOP_LEFT);
    }
    assert_eq!(top_start.resolve(TextDirection::Ltr), A::TOP_LEFT);
    assert_eq!(top_start.resolve(TextDirection::Rtl), A::TOP_RIGHT);
    assert_eq!(top_end * 0., G::CENTER);
    assert_eq!(top_left * 0., G::CENTER);
    for item in [top_start, top_end, top_left, top_right] {
        assert_eq!(item * 1., item);
    }
    let numbers = [0., 1., -1., 2., 0.25, 0.5, 100., -999.75];
    for n in numbers {
        assert_eq!(top_start * n + top_start, top_start * (n + 1.));
        assert_eq!(top_end * n + top_end, top_end * (n + 1.));
        for m in numbers {
            assert_eq!(top_start * n + top_start * m, top_start * (n + m));
        }
    }
    assert_eq!(top_start + top_start + top_start, top_start * 3.);
    for value in [top_start, top_end] {
        assert_ne!(value, top_left);
        assert_ne!(value, top_right);
    }
    assert_ne!(top_start + top_left, top_left);
    assert_ne!(top_start + top_left, top_start);

    let directional = G::directional(1., 2.);
    let normal = G::xy(3., 5.);
    let combined = directional + normal;
    assert_eq!(combined.resolve(TextDirection::Ltr), A::new(4., 7.));
    assert_eq!(combined.resolve(TextDirection::Rtl), A::new(2., 7.));
    assert_eq!(normal + normal, normal * 2.);
    assert_eq!(directional + directional, directional * 2.);
    assert_eq!(combined - directional, normal);
}

fn approximate(actual: A, expected: A) {
    assert!(
        (actual.x - expected.x).abs() < 0.0002,
        "{actual} != {expected}"
    );
    assert!(
        (actual.y - expected.y).abs() < 0.0002,
        "{actual} != {expected}"
    );
}

#[test]
fn mixed_quarter_sample_resolves_each_preserved_horizontal_component() {
    let mixed = |x, y, start, dy| G::xy(x, y) + G::directional(start, dy);
    let first = mixed(10., 20., 30., 50.);
    let second = mixed(70., 110., 130., 170.);
    let quarter = mixed(25., 42.5, 55., 80.);
    for direction in DIRECTIONS {
        for (time, endpoint) in [(0., first), (1., second), (0.25, quarter)] {
            assert_eq!(
                first.lerp(&second, time).resolve(direction),
                endpoint.resolve(direction)
            );
        }
    }
    assert_eq!(first.lerp(&second, 0.25), quarter);
    assert_eq!(quarter.resolve(TextDirection::Ltr), A::new(80., 122.5));
    assert_eq!(quarter.resolve(TextDirection::Rtl), A::new(-30., 122.5));
}

#[test]
fn interpolation_commutes_with_direction_resolution_across_all_original_inputs() {
    let samples = [
        Some(G::TOP_LEFT),
        Some(G::TOP_CENTER),
        Some(G::TOP_RIGHT),
        Some(G::TOP_START),
        Some(D::TOP_CENTER.into()),
        Some(G::TOP_END),
        Some(G::CENTER_LEFT),
        Some(G::CENTER),
        Some(G::CENTER_RIGHT),
        Some(G::CENTER_START),
        Some(D::CENTER.into()),
        Some(G::CENTER_END),
        Some(G::BOTTOM_LEFT),
        Some(G::BOTTOM_CENTER),
        Some(G::BOTTOM_RIGHT),
        Some(G::BOTTOM_START),
        Some(D::BOTTOM_CENTER.into()),
        Some(G::BOTTOM_END),
        Some(G::xy(-1., 0.65)),
        Some(G::directional(-1., 0.45)),
        Some(G::directional(0.125, 0.625)),
        Some(G::xy(0.25, 0.875)),
        Some(G::xy(0.0625, 0.5625) + G::directional(0.1875, 0.6875)),
        Some(G::directional(2., 3.)),
        Some(G::xy(2., 3.)),
        Some(G::xy(2., 3.) + G::directional(5., 3.)),
        Some(G::xy(10., 20.) + G::directional(30., 50.)),
        Some(G::xy(70., 110.) + G::directional(130., 170.)),
        Some(G::xy(25., 42.5) + G::directional(55., 80.)),
        None,
    ];
    for direction in DIRECTIONS {
        let resolve = |value: Option<G>| value.unwrap_or(G::CENTER).resolve(direction);
        for a in samples {
            for b in samples {
                let (start, finish) = (resolve(a), resolve(b));
                approximate(start.lerp(&finish, 0.), start);
                approximate(start.lerp(&finish, 1.), finish);
                approximate(resolve(G::lerp_optional(a, b, 0.)), start);
                approximate(resolve(G::lerp_optional(a, b, 1.)), finish);
                for time in [0.25, 0.5, 0.75] {
                    let actual = resolve(G::lerp_optional(a, b, time));
                    approximate(actual, start.lerp(&finish, time));
                    let tolerance = 0.0002;
                    assert!(
                        actual.x >= start.x.min(finish.x) - tolerance
                            && actual.x <= start.x.max(finish.x) + tolerance
                    );
                    assert!(
                        actual.y >= start.y.min(finish.y) - tolerance
                            && actual.y <= start.y.max(finish.y) + tolerance
                    );
                }
            }
        }
    }
}

#[test]
fn physical_directional_and_mixed_value_operators_preserve_types() {
    let a = A::new(1., 2.);
    let b = D::new(1., 2.);
    assert_eq!(a * 2., A::new(2., 4.));
    assert_eq!(a / 2., A::new(0.5, 1.));
    assert_eq!(a % 2., A::CENTER_RIGHT);
    assert_eq!(a.truncating_div(2.), A::BOTTOM_CENTER);
    assert_eq!(b * 2., D::new(2., 4.));
    assert_eq!(b / 2., D::new(0.5, 1.));
    assert_eq!(b % 2., D::CENTER_END);
    assert_eq!(b.truncating_div(2.), D::BOTTOM_CENTER);
    for dir in DIRECTIONS {
        let b = G::from(b);
        assert_eq!(
            (G::CENTER + b * 2.).resolve(dir),
            D::new(2., 4.).resolve(dir)
        );
        assert_eq!(
            (G::CENTER + b / 2.).resolve(dir),
            D::new(0.5, 1.).resolve(dir)
        );
        assert_eq!(
            (G::CENTER + b % 2.).resolve(dir),
            D::CENTER_END.resolve(dir)
        );
        assert_eq!(
            (G::CENTER + b.truncating_div(2.)).resolve(dir),
            D::BOTTOM_CENTER.resolve(dir)
        );
    }
    assert_eq!(a + A::new(3., 5.), A::new(4., 7.));
    assert_eq!(a - A::new(3., 5.), A::new(-2., -3.));
    assert_eq!(b + D::new(3., 5.), D::new(4., 7.));
    assert_eq!(b - D::new(3., 5.), D::new(-2., -3.));
    assert_eq!(
        G::directional(-3., 5.).truncating_div(2.),
        G::directional(-1., 2.)
    );
    assert_eq!((G::directional(-3., -5.) % -2.), G::directional(1., 1.));
}

#[test]
fn formatting_geometry_matches_canonical_names_and_retains_the_y_component_once() {
    assert_eq!(A::new(1.0001, 2.0001).to_string(), "Alignment(1.0, 2.0)");
    assert_eq!(A::CENTER.to_string(), "Alignment.center");
    assert_eq!(
        (G::BOTTOM_LEFT + G::CENTER_END).to_string(),
        "Alignment.bottomLeft + AlignmentDirectional.centerEnd"
    );
    assert_eq!(A::new(0.0001, 0.0001).to_string(), "Alignment(0.0, 0.0)");
    assert_eq!(D::CENTER.to_string(), "AlignmentDirectional.center");
    assert_eq!(
        G::from(D::CENTER).to_string(),
        "AlignmentDirectional.center"
    );
    assert_eq!(
        (G::BOTTOM_RIGHT + G::BOTTOM_END).to_string(),
        "Alignment(1.0, 2.0) + AlignmentDirectional.centerEnd"
    );
}

#[test]
fn factories_and_constants_expose_both_coordinate_systems() {
    assert_eq!(G::xy(4., 5.), A::new(4., 5.).into());
    assert_eq!(G::directional(4., 5.), D::new(4., 5.).into());
    for (geometry, physical) in [
        (G::TOP_LEFT, A::TOP_LEFT),
        (G::TOP_CENTER, A::TOP_CENTER),
        (G::TOP_RIGHT, A::TOP_RIGHT),
        (G::CENTER_LEFT, A::CENTER_LEFT),
        (G::CENTER, A::CENTER),
        (G::CENTER_RIGHT, A::CENTER_RIGHT),
        (G::BOTTOM_LEFT, A::BOTTOM_LEFT),
        (G::BOTTOM_CENTER, A::BOTTOM_CENTER),
        (G::BOTTOM_RIGHT, A::BOTTOM_RIGHT),
    ] {
        assert!(matches!(geometry, G::Physical(value) if value == physical));
    }
    for (geometry, directional) in [
        (G::TOP_START, D::TOP_START),
        (G::TOP_END, D::TOP_END),
        (G::CENTER_START, D::CENTER_START),
        (G::CENTER_END, D::CENTER_END),
        (G::BOTTOM_START, D::BOTTOM_START),
        (G::BOTTOM_END, D::BOTTOM_END),
    ] {
        assert!(matches!(geometry, G::Directional(value) if value == directional));
    }
}

#[test]
fn direction_is_required_for_directional_and_mixed_values_even_for_zero_start() {
    let error = D::new(1., 2.).try_resolve(None).unwrap_err();
    let description = error.to_string();
    assert!(description.contains("No TextDirection found."));
    assert!(description.contains("without a Directionality ancestor"));
    let mixed = G::xy(5., 6.) + G::directional(15., 16.);
    assert_eq!(mixed.try_resolve(None), Err(error));
    assert_eq!(G::from(D::CENTER).try_resolve(None), Err(error));
    assert_eq!(G::CENTER.try_resolve(None), Ok(A::CENTER));
    assert_eq!(
        mixed.try_resolve(Some(TextDirection::Rtl)),
        Ok(A::new(-10., 22.))
    );
}
