//! Targeted Flutter 3.50.0 edge_insets_test.dart constructor/resolve contracts.

use incular_config::{EdgeInsets, EdgeInsetsDirectional as Directional, TextDirection};

#[test]
fn absolute_insets_constructors_store_physical_edges() {
    let uneven = EdgeInsets::from_ltrb(10., 20., 30., 40.);
    assert_eq!(
        (uneven.left, uneven.top, uneven.right, uneven.bottom),
        (10., 20., 30., 40.)
    );
    assert_eq!(
        EdgeInsets::all(10.),
        EdgeInsets::from_ltrb(10., 10., 10., 10.)
    );
    assert_eq!(EdgeInsets::only(10., 20., 30., 40.), uneven);
    assert_eq!(
        EdgeInsets::symmetric(10., 20.),
        EdgeInsets::from_ltrb(10., 20., 10., 20.)
    );
}

#[test]
fn directional_insets_constructors_resolve_in_both_directions() {
    let ltr = EdgeInsets::from_ltrb(10., 20., 30., 40.);
    let rtl = EdgeInsets::from_ltrb(30., 20., 10., 40.);
    for insets in [
        Directional::from_ste_b(10., 20., 30., 40.),
        Directional::only(10., 20., 30., 40.),
    ] {
        assert_eq!(insets.resolve(TextDirection::Ltr), ltr);
        assert_eq!(insets.resolve(TextDirection::Rtl), rtl);
    }
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        assert_eq!(
            Directional::all(10.).resolve(direction),
            EdgeInsets::from_ltrb(10., 10., 10., 10.)
        );
        assert_eq!(
            Directional::symmetric(10., 20.).resolve(direction),
            EdgeInsets::from_ltrb(10., 20., 10., 20.)
        );
    }
}

#[test]
fn directional_insets_resolve_one_edge_at_a_time() {
    assert_eq!(
        Directional::from_ste_b(10., 20., 30., 40.).resolve(TextDirection::Ltr),
        EdgeInsets::from_ltrb(10., 20., 30., 40.)
    );
    assert_eq!(
        Directional::from_ste_b(99., 98., 97., 96.).resolve(TextDirection::Rtl),
        EdgeInsets::from_ltrb(97., 98., 99., 96.)
    );
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        assert_eq!(
            Directional::all(50.).resolve(direction),
            EdgeInsets::from_ltrb(50., 50., 50., 50.)
        );
    }
    let mag = 963.25;
    let sides = [
        Directional::only(mag, 0., 0., 0.),
        Directional::only(0., mag, 0., 0.),
        Directional::only(0., 0., mag, 0.),
        Directional::only(0., 0., 0., mag),
    ];
    let [left, top, right, bottom] = [
        EdgeInsets::from_ltrb(mag, 0., 0., 0.),
        EdgeInsets::from_ltrb(0., mag, 0., 0.),
        EdgeInsets::from_ltrb(0., 0., mag, 0.),
        EdgeInsets::from_ltrb(0., 0., 0., mag),
    ];
    assert_eq!(
        sides.map(|a| a.resolve(TextDirection::Ltr)),
        [left, top, right, bottom]
    );
    assert_eq!(
        sides.map(|a| a.resolve(TextDirection::Rtl)),
        [right, top, left, bottom]
    );
    assert_eq!(Directional::only(0., 0., 0., 0.), Directional::default());
    assert_ne!(
        Directional::only(0., 1., 0., 0.),
        Directional::only(0., 0., 0., 1.)
    );
}
