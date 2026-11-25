use incular_core::{InvalidClampBounds, clamp_double, try_clamp_double};

#[test]
fn clamp_double_includes_the_flutter_infinity_and_nan_cases() {
    for (value, lower, upper, expected) in [
        (-1.0, 0.0, 1.0, 0.0),
        (2.0, 0.0, 1.0, 1.0),
        (f64::INFINITY, 0.0, 1.0, 1.0),
        (f64::NEG_INFINITY, 0.0, 1.0, 0.0),
        (f64::NAN, 0.0, f64::INFINITY, f64::INFINITY),
    ] {
        assert_eq!(clamp_double(value, lower, upper), expected);
    }
}

#[test]
fn clamp_preserves_signed_zero_and_rejects_invalid_intervals() {
    assert_eq!(clamp_double(-0.0, 0.0, 1.0).to_bits(), (-0.0_f64).to_bits());
    assert_eq!(clamp_double(f64::NAN, -1.0, 0.5), 0.5);
    assert_eq!(clamp_double(3.0, 1.0, 1.0), 1.0);
    assert_eq!(
        try_clamp_double(0.0, 1.0, -1.0),
        Err(InvalidClampBounds {
            lower: 1.0,
            upper: -1.0
        })
    );
    assert!(try_clamp_double(0.0, f64::NAN, 1.0).is_err());
    assert!(try_clamp_double(0.0, 0.0, f64::NAN).is_err());
}
