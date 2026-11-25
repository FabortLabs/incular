use incular_core::{Offset, Rect, Size};
use incular_rendering::{CornerRadii, FillRule, Path};

#[test]
fn simple_rectangle_contains_exactly_its_closed_boundary_for_both_fill_rules() {
    let rect = Rect::from_origin_size(Offset::new(10.0, 20.0), Size::new(30.0, 40.0));
    let mut rounded = Path::builder();
    rounded.rounded_rect(rect, CornerRadii::ZERO);
    let mut reversed = Path::builder();
    reversed
        .move_to(Offset::new(40.0, 60.0))
        .line_to(Offset::new(40.0, 20.0))
        .line_to(Offset::new(10.0, 20.0))
        .line_to(Offset::new(10.0, 60.0))
        .close();
    for path in [rounded.build(), reversed.build()] {
        assert_eq!(path.bounds(), Some(rect));
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            for (x, y) in [
                (10.0, 20.0),
                (40.0, 60.0),
                (40.0, 30.0),
                (10.0, 35.0),
                (20.0, 60.0),
                (25.0, 40.0),
            ] {
                assert!(path.contains(Offset::new(x, y), rule), "missing ({x}, {y})");
            }
            for (x, y) in [
                (9.999, 30.0),
                (40.001, 30.0),
                (25.0, 19.999),
                (25.0, 60.001),
            ] {
                assert!(
                    !path.contains(Offset::new(x, y), rule),
                    "included ({x}, {y})"
                );
            }
        }
    }
}

#[test]
fn compound_path_still_applies_its_fill_rule_to_the_hole() {
    let outer = Rect::from_origin_size(Offset::ZERO, Size::new(20.0, 20.0));
    let inner = Rect::from_origin_size(Offset::new(5.0, 5.0), Size::new(10.0, 10.0));
    let mut path = Path::builder();
    path.rounded_rect(outer, CornerRadii::ZERO)
        .rounded_rect(inner, CornerRadii::ZERO);
    let path = path.build();
    let ring = Offset::new(4.0, 5.0);
    let center = Offset::new(10.0, 10.0);
    assert!(path.contains(ring, FillRule::EvenOdd));
    assert!(path.contains(center, FillRule::NonZero));
    assert!(!path.contains(center, FillRule::EvenOdd));
}
