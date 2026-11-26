# Diagnostic double precision

Both `foundation/double_precision_test.dart` registrations passed at the
pinned revision. Default formatting uses one fixed decimal place; absent
values print `null`. Setting precision switches to significant digits,
and resetting it restores the default. Like Dart's isolate-local debug
setting, the Rust setting is isolated to the calling UI thread.

The authored Flutter probe covers signed zero, NaN, infinity, exponent
cutoffs, both signs of exact decimal ties, and a tie at precision 21.
Native rounding operates on the exact decimal midpoint: advancing the
input by one binary ULP would skip several decimal places at high
precision. PolynomialFit uses the same conversion with its own fixed
precision rather than inheriting the global debug override.

Native: five focused core formatting tests and eight least-squares
regression tests passed.
