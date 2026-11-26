//! Decimal formatting used by Flutter-compatible diagnostic properties.

use std::cell::Cell;

thread_local! {
    static DEBUG_PRECISION: Cell<Option<u8>> = const { Cell::new(None) };
}

/// Returns the optional significant-digit override for the current thread.
#[must_use]
pub fn debug_double_precision() -> Option<u8> {
    DEBUG_PRECISION.with(Cell::get)
}

/// Changes the current thread's diagnostic precision override and returns
/// its previous value. `None` restores fixed one-place formatting.
///
/// # Panics
///
/// Panics if a supplied precision is not in `1..=21`.
pub fn set_debug_double_precision(precision: Option<u8>) -> Option<u8> {
    assert!(
        precision.is_none_or(|digits| (1..=21).contains(&digits)),
        "precision must be in 1..=21"
    );
    DEBUG_PRECISION.with(|current| current.replace(precision))
}

/// Formats a diagnostic double or the literal `null` for an absent value.
#[must_use]
pub fn debug_format_double(value: Option<f64>) -> String {
    let Some(value) = value else {
        return "null".to_owned();
    };
    if let Some(precision) = debug_double_precision() {
        format_double_precision(value, precision)
    } else {
        format_double_fixed(value, 1)
    }
}

fn nonfinite(value: f64) -> Option<&'static str> {
    if value.is_nan() {
        Some("NaN")
    } else if value == f64::INFINITY {
        Some("Infinity")
    } else if value == f64::NEG_INFINITY {
        Some("-Infinity")
    } else {
        None
    }
}

/// Decimal halfway values are dyadic rationals. Test their exact bit
/// representation rather than multiplying by a floating-point power of ten
/// (which would falsely round values just to one side of a tie).
fn decimal_halfway(abs: f64, power: i32) -> Option<u128> {
    const MANTISSA_MASK: u64 = (1_u64 << 52) - 1;
    let bits = abs.to_bits();
    let biased_exp = ((bits >> 52) & 0x7ff) as i32;
    let (mantissa, exponent) = if biased_exp == 0 {
        (bits & MANTISSA_MASK, -1074)
    } else {
        (
            (bits & MANTISSA_MASK) | (1_u64 << 52),
            biased_exp - 1023 - 52,
        )
    };
    if mantissa == 0 {
        return None;
    }
    let zeros = mantissa.trailing_zeros();
    // 2 * value / 10^power must be an odd integer.
    if exponent + 1 - power + zeros as i32 != 0 {
        return None;
    }
    let odd = u128::from(mantissa >> zeros);
    let fives = 5_u128.checked_pow(power.unsigned_abs())?;
    if power >= 0 {
        odd.is_multiple_of(fives).then_some(odd / fives)
    } else {
        odd.checked_mul(fives)
    }
}

/// `point` counts digits before the decimal, including an implicit leading
/// zero when the magnitude is fractional.
fn place_decimal(digits: &str, point: i32) -> String {
    if point <= 0 {
        let zeros = "0".repeat((-point) as usize);
        format!("0.{zeros}{digits}")
    } else {
        let point = point as usize;
        if point == digits.len() {
            digits.to_owned()
        } else {
            format!("{}.{}", &digits[..point], &digits[point..])
        }
    }
}

/// Decimal significant-digit formatting for Flutter-compatible diagnostics.
#[doc(hidden)]
#[must_use]
pub fn format_double_precision(value: f64, precision: u8) -> String {
    assert!((1..=21).contains(&precision), "precision must be in 1..=21");
    let digits = usize::from(precision - 1);
    if let Some(label) = nonfinite(value) {
        return label.to_owned();
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let abs = value.abs();
    let formatted = format!("{abs:.digits$e}");
    let (significand, exp) = formatted.split_once('e').unwrap();
    let mut exp: i32 = exp.parse().unwrap();
    let power = exp - digits as i32;
    // Rust uses ties-to-even. Dart rounds exact ties away from zero.
    // Construct the rounded decimal directly: at high precision, advancing
    // the float by one ULP may skip many decimal positions.
    let decimal = if let Some(odd) = decimal_halfway(abs, power) {
        let mut rounded = (odd / 2 + 1).to_string();
        exp = power + rounded.len() as i32 - 1;
        rounded.truncate(usize::from(precision));
        rounded
    } else {
        significand.replace('.', "")
    };
    if exp < -6 || exp >= i32::from(precision) {
        return format!("{sign}{}e{exp:+}", place_decimal(&decimal, 1));
    }
    format!("{sign}{}", place_decimal(&decimal, exp + 1))
}

/// Decimal fixed-point formatting for Flutter-compatible diagnostics.
#[doc(hidden)]
#[must_use]
pub fn format_double_fixed(value: f64, digits: u8) -> String {
    assert!(digits <= 20, "digits must be in 0..=20");
    if let Some(label) = nonfinite(value) {
        return label.to_owned();
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let abs = value.abs();
    if abs >= 1e21 {
        let shortest = format!("{abs:e}");
        let (mantissa, exponent) = shortest.split_once('e').unwrap();
        let exponent: i32 = exponent.parse().unwrap();
        return format!("{sign}{mantissa}e{exponent:+}");
    }
    if let Some(odd) = decimal_halfway(abs, -i32::from(digits)) {
        let rounded = (odd / 2 + 1).to_string();
        return format!(
            "{sign}{}",
            place_decimal(&rounded, rounded.len() as i32 - i32::from(digits))
        );
    }
    let digits = usize::from(digits);
    format!("{sign}{abs:.digits$}")
}
