//! Numeric contracts shared with Flutter's foundation library.

/// The lower bound exceeded the upper bound, or a bound was NaN.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InvalidClampBounds {
    pub lower: f64,
    pub upper: f64,
}

impl std::fmt::Display for InvalidClampBounds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid clamp interval {}..={}", self.lower, self.upper)
    }
}

impl std::error::Error for InvalidClampBounds {}

/// Flutter's `clampDouble`: clips a number to an inclusive interval, using
/// the upper bound for NaN. Values already inside keep their representation
/// (including signed zero).
///
/// # Panics
/// Panics on unordered or NaN bounds. Use [`try_clamp_double`] to validate
/// externally supplied bounds.
#[must_use]
pub fn clamp_double(value: f64, lower: f64, upper: f64) -> f64 {
    try_clamp_double(value, lower, upper).unwrap_or_else(|error| panic!("{error}"))
}

/// [`clamp_double`] with a structured error for invalid bounds.
///
/// # Errors
/// Returns [`InvalidClampBounds`] for unordered or NaN bounds.
pub fn try_clamp_double(value: f64, lower: f64, upper: f64) -> Result<f64, InvalidClampBounds> {
    if lower <= upper {
        Ok(if value < lower {
            lower
        } else if value > upper || value.is_nan() {
            upper
        } else {
            value
        })
    } else {
        Err(InvalidClampBounds { lower, upper })
    }
}
