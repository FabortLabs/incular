//! Flutter 3.50.0 painting/fractional_offset_test.dart.
use incular_config::FractionalOffset as Fraction;
use incular_core::{Offset, Rect, Size};

#[test]
fn absolute_offset_is_normalized_by_size() {
    assert_eq!(
        Fraction::from_offset_and_size(Offset::new(100., 100.), Size::new(200., 400.)),
        Fraction::new(0.5, 0.25)
    );
}

#[test]
fn rectangle_origin_is_removed_before_normalizing() {
    let rect = Rect::from_origin_size(Offset::new(50., 20.), Size::new(200., 400.));
    assert_eq!(
        Fraction::from_offset_and_rect(Offset::new(150., 120.), rect),
        Fraction::new(0.5, 0.25)
    );
    // Authored control: a point outside the box is not snapped to an edge.
    assert_eq!(
        Fraction::from_offset_and_rect(Offset::new(-50., 520.), rect),
        Fraction::new(-0.5, 1.25)
    );
}

#[test]
fn missing_interpolation_endpoints_are_center_except_when_both_are_missing() {
    let (a, b) = (Some(Fraction::TOP_LEFT), Some(Fraction::TOP_CENTER));
    for (from, to, want) in [
        (a, b, Some(Fraction::new(0.125, 0.))),
        (None, None, None),
        (None, b, Some(Fraction::new(0.5, 0.375))),
        (a, None, Some(Fraction::new(0.125, 0.125))),
    ] {
        assert_eq!(Fraction::lerp_optional(from, to, 0.25), want);
    }
}

#[test]
fn equal_operands_preserve_value_without_requiring_object_identity() {
    let value = Some(Fraction::new(1., 2.));
    assert_eq!(Fraction::lerp_optional(None, None, 0.), None);
    assert_eq!(Fraction::lerp_optional(value, value, 0.5), value);
}
