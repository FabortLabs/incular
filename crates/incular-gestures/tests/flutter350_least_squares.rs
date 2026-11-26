use incular_gestures::{LeastSquaresSolver, PolynomialFit};

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}

fn line(degree: usize, slope: f64) {
    let x = [0., 1., 2., 3., 4., 5., 6., 7., 8.];
    let y = x.map(|x| 1. + slope * x);
    let result = LeastSquaresSolver::new(&x, &y, &[1.; 9])
        .solve(degree)
        .unwrap();
    assert_eq!(result.coefficients.len(), degree + 1);
    close(result.coefficients[0], 1.);
    close(result.coefficients[1], slope);
    if degree == 2 {
        close(result.coefficients[2], 0.);
    }
    close(result.confidence, 1.);
}

#[test]
fn linear_polynomial_fits_a_horizontal_line() {
    line(1, 0.);
}

#[test]
fn linear_polynomial_fits_an_ascending_line() {
    line(1, 1.);
}

#[test]
fn quadratic_polynomial_fits_a_horizontal_line() {
    line(2, 0.);
}

#[test]
fn quadratic_polynomial_recovers_the_linear_term_of_a_sloped_line() {
    line(2, 1.);
}

#[test]
fn diagnostic_string_uses_three_significant_digits_and_fixed_confidence() {
    let mut fit = PolynomialFit::new(2);
    fit.coefficients = vec![123.45, 54.321, 1.3579];
    fit.confidence = 0.9876;
    assert_eq!(
        fit.to_string(),
        "PolynomialFit([123, 54.3, 1.36], confidence: 0.988)"
    );
}

#[test]
fn weights_are_squared_in_the_fit_and_confidence() {
    let x = [0., 0., 1., 1.];
    let y = [0., 10., 2., 12.];
    let w = [1., 2., 1., 2.];
    let fit = LeastSquaresSolver::new(&x, &y, &w).solve(1).unwrap();
    close(fit.coefficients[0], 8.);
    close(fit.coefficients[1], 2.);
    close(fit.confidence, 5. / 13.);
}

#[test]
fn rank_deficiency_and_insufficient_data_return_no_fit() {
    assert!(
        LeastSquaresSolver::new(&[1., 1., 1.], &[1., 2., 3.], &[1.; 3])
            .solve(1)
            .is_none()
    );
    assert!(
        LeastSquaresSolver::new(&[1., 2.], &[1., 2.], &[1.; 2])
            .solve(2)
            .is_none()
    );
    assert!(
        LeastSquaresSolver::new(&[1., 2.], &[1., 2.], &[0.; 2])
            .solve(0)
            .is_none()
    );
}

#[test]
fn decimal_midpoints_exponents_and_signed_zero_match_flutter() {
    let fit = PolynomialFit {
        coefficients: vec![
            1.125,
            -1.125,
            1e-8,
            1e3,
            1e-6,
            -0.0,
            f64::INFINITY,
            f64::NAN,
        ],
        confidence: 0.0625,
    };
    assert_eq!(
        fit.to_string(),
        "PolynomialFit([1.13, -1.13, 1.00e-8, 1.00e+3, 0.00000100, -0.00, Infinity, NaN], confidence: 0.063)"
    );
}
