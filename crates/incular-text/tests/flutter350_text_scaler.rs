//! Flutter 3.50.0 numerical linear scaler and clamp validation.

use incular_text::{TextScaleBoundsError, TextScaler};

#[test]
fn equal_linear_scalers_can_be_created_by_different_clamp_histories() {
    let scaler = TextScaler::no_scaling();
    let all = [
        TextScaler::linear(3.),
        scaler.try_clamp(3., f32::INFINITY).unwrap(),
        TextScaler::linear(3.),
        scaler.try_clamp(1., 5.).unwrap().try_clamp(3., 6.).unwrap(),
    ];
    for lhs in all {
        assert!(all.iter().all(|rhs| lhs == *rhs));
        assert_eq!(lhs.scale(16.), 48.);
    }
    assert_eq!(scaler, TextScaler::linear(1.));
    assert!(TextScaler::linear(1.).is_no_scaling());
}

#[test]
fn linear_clamping_pins_the_scale_before_paragraph_layout() {
    for (raw, min, max) in [(1., 3., f32::INFINITY), (5., 0., 3.), (5., 3., 3.)] {
        let result = TextScaler::linear(raw).try_clamp(min, max).unwrap();
        assert_eq!(result, TextScaler::linear(3.));
        assert_eq!(result.scale(11.), 33.);
    }
    assert_eq!(
        TextScaler::no_scaling().try_clamp(5., 4.),
        Err(TextScaleBoundsError::InvalidMaximum)
    );
}

#[test]
fn clamp_rejects_nonfinite_minima_negative_minima_and_nan_maxima() {
    let clamped = TextScaler::linear(4.).try_clamp(1., 5.).unwrap();
    for (min, max, error) in [
        (3., 2., TextScaleBoundsError::InvalidMaximum),
        (3., f32::NAN, TextScaleBoundsError::InvalidMaximum),
        (-1., 2., TextScaleBoundsError::NegativeMinimum),
        (
            f32::INFINITY,
            f32::INFINITY,
            TextScaleBoundsError::NonFiniteMinimum,
        ),
    ] {
        assert_eq!(clamped.try_clamp(min, max), Err(error));
    }
}
