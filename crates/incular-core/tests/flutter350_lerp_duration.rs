use incular_core::time::{NonFiniteDurationInterpolation, lerp_duration_micros};

const SECOND: i64 = 1_000_000;

#[test]
fn interpolates_between_positive_durations() {
    assert_eq!(lerp_duration_micros(SECOND, 2 * SECOND, 0.5), Ok(1_500_000));
}

#[test]
fn interpolates_between_negative_durations() {
    assert_eq!(
        lerp_duration_micros(-SECOND, -2 * SECOND, 0.5),
        Ok(-1_500_000)
    );
}

#[test]
fn interpolates_across_zero() {
    assert_eq!(lerp_duration_micros(-SECOND, 2 * SECOND, 0.5), Ok(500_000));
}

#[test]
fn factor_zero_starts_at_the_first_duration() {
    assert_eq!(lerp_duration_micros(SECOND, 2 * SECOND, 0.0), Ok(SECOND));
}

#[test]
fn factor_one_ends_at_the_second_duration() {
    assert_eq!(
        lerp_duration_micros(SECOND, 2 * SECOND, 1.0),
        Ok(2 * SECOND)
    );
}

#[test]
fn extrapolates_both_signs_without_clamping_the_factor() {
    assert_eq!(
        lerp_duration_micros(SECOND, 2 * SECOND, 5.0),
        Ok(6 * SECOND)
    );
    assert_eq!(
        lerp_duration_micros(-SECOND, -2 * SECOND, 5.0),
        Ok(-6 * SECOND)
    );
}

#[test]
fn rounds_final_microseconds_and_preserves_native_integer_edges() {
    assert_eq!(lerp_duration_micros(0, 1, 0.5), Ok(1));
    assert_eq!(lerp_duration_micros(0, -1, 0.5), Ok(-1));
    assert_eq!(lerp_duration_micros(i64::MIN, i64::MAX, 0.5), Ok(i64::MIN));
    assert_eq!(lerp_duration_micros(0, 1, 1e30), Ok(i64::MAX));
    assert_eq!(lerp_duration_micros(0, -1, 1e30), Ok(i64::MIN));
    for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            lerp_duration_micros(0, 1, t),
            Err(NonFiniteDurationInterpolation)
        );
    }
}
