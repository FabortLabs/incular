use incular_core::{debug_double_precision, debug_format_double, set_debug_double_precision};

#[test]
fn default_diagnostics_format_doubles() {
    assert_eq!(debug_double_precision(), None);
    assert_eq!(debug_format_double(Some(1.0)), "1.0");
    assert_eq!(debug_format_double(Some(1.1)), "1.1");
    assert_eq!(debug_format_double(None), "null");
}

#[test]
fn double_precision_override_uses_significant_digits_then_restores_default() {
    assert_eq!(set_debug_double_precision(Some(3)), None);
    assert_eq!(debug_format_double(Some(1.0)), "1.00");
    assert_eq!(set_debug_double_precision(None), Some(3));
    assert_eq!(debug_format_double(Some(1.0)), "1.0");
}

#[test]
fn diagnostic_precision_does_not_leak_into_another_thread() {
    set_debug_double_precision(Some(4));
    let result = std::thread::spawn(|| (debug_double_precision(), debug_format_double(Some(2.0))))
        .join()
        .unwrap();
    assert_eq!(result, (None, "2.0".into()));
    assert_eq!(debug_format_double(Some(2.0)), "2.000");
    set_debug_double_precision(None);
}

#[test]
fn decimal_ties_exponent_cutoff_and_signed_zero_match_the_reference() {
    assert_eq!(debug_format_double(Some(1.25)), "1.3");
    assert_eq!(debug_format_double(Some(-1.25)), "-1.3");
    assert_eq!(debug_format_double(Some(-0.0)), "-0.0");
    assert_eq!(debug_format_double(Some(1e21)), "1e+21");
    assert_eq!(debug_format_double(Some(f64::INFINITY)), "Infinity");
    assert_eq!(debug_format_double(Some(f64::NAN)), "NaN");

    set_debug_double_precision(Some(3));
    assert_eq!(debug_format_double(Some(1.125)), "1.13");
    assert_eq!(debug_format_double(Some(-1.125)), "-1.13");
    assert_eq!(debug_format_double(Some(1e-6)), "0.00000100");
    assert_eq!(debug_format_double(Some(1e-7)), "1.00e-7");
    assert_eq!(debug_format_double(Some(1e3)), "1.00e+3");
    assert_eq!(debug_format_double(Some(-0.0)), "-0.00");
    set_debug_double_precision(None);
}

#[test]
fn highest_precision_rounds_only_one_decimal_position_at_an_exact_tie() {
    let value = 1. + 1. / (1_u32 << 21) as f64;
    set_debug_double_precision(Some(21));
    assert_eq!(debug_format_double(Some(value)), "1.00000047683715820313");
    assert_eq!(
        incular_core::debug_double::format_double_fixed(value, 20),
        "1.00000047683715820313"
    );
    set_debug_double_precision(None);
}
