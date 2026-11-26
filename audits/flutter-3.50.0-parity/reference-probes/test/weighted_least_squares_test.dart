import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('weights are squared when fitting and finding confidence', () {
    final fit = LeastSquaresSolver(
      <double>[0, 0, 1, 1],
      <double>[0, 10, 2, 12],
      <double>[1, 2, 1, 2],
    ).solve(1)!;
    expect(fit.coefficients[0], closeTo(8, 1e-6));
    expect(fit.coefficients[1], closeTo(2, 1e-6));
    expect(fit.confidence, closeTo(5 / 13, 1e-6));
  });

  test('rank deficiency and insufficient data return no fit', () {
    expect(
      LeastSquaresSolver(<double>[1, 1, 1], <double>[1, 2, 3], <double>[
        1,
        1,
        1,
      ]).solve(1),
      isNull,
    );
    expect(
      LeastSquaresSolver(<double>[1, 2], <double>[1, 2], <double>[
        1,
        1,
      ]).solve(2),
      isNull,
    );
    expect(
      LeastSquaresSolver(<double>[1, 2], <double>[1, 2], <double>[
        0,
        0,
      ]).solve(0),
      isNull,
    );
  });

  test('diagnostics handle decimal midpoints, exponents, and signed zero', () {
    final fit = PolynomialFit(7);
    fit.coefficients.setAll(0, <double>[
      1.125,
      -1.125,
      1e-8,
      1e3,
      1e-6,
      -0.0,
      double.infinity,
      double.nan,
    ]);
    fit.confidence = 0.0625;
    expect(
      fit.toString(),
      'PolynomialFit([1.13, -1.13, 1.00e-8, 1.00e+3, 0.00000100, -0.00, Infinity, NaN], confidence: 0.063)',
    );
  });
}
