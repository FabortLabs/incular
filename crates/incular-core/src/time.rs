//! Monotonic and wall-clock time values that work on every Incular target.
//!
//! On native targets these are exactly [`std::time`]'s types, so values
//! created through either path are interchangeable. On `wasm32` browser
//! targets, where `std::time::Instant::now` panics, they are backed by the
//! browser's `performance.now()` and `Date.now()` clocks.

pub use web_time::{Duration, Instant, SystemTime, SystemTimeError, UNIX_EPOCH};

/// A signed duration could not be interpolated because its result was not
/// finite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonFiniteDurationInterpolation;

impl std::fmt::Display for NonFiniteDurationInterpolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("signed duration interpolation produced a non-finite value")
    }
}

impl std::error::Error for NonFiniteDurationInterpolation {}

/// Flutter's `lerpDuration` for signed microseconds. The factor is not
/// clamped. Half-microseconds are rounded away from zero, and finite results
/// beyond the signed range saturate. The endpoint difference wraps as it
/// does with native Dart's signed integers.
///
/// # Errors
/// Returns [`NonFiniteDurationInterpolation`] when the intermediate value
/// is NaN or infinite.
pub fn lerp_duration_micros(
    start_micros: i64,
    end_micros: i64,
    factor: f64,
) -> Result<i64, NonFiniteDurationInterpolation> {
    let delta_micros = end_micros.wrapping_sub(start_micros);
    let micros = start_micros as f64 + delta_micros as f64 * factor;
    if micros.is_finite() {
        Ok(micros.round() as i64)
    } else {
        Err(NonFiniteDurationInterpolation)
    }
}
