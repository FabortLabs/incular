# Weighted least-squares fitting and pointer velocity

All five pinned `gestures/lsq_solver_test.dart` cases (14, 28, 42, 57, 72)
passed. Incular had a private unit-weight quadratic velocity fit, but
no public weighted solver or polynomial fit.

`LeastSquaresSolver` now uses weighted QR factorization, returns the
coefficients in ascending power order, returns no result for a
rank-deficient design, and computes Dart's weighted coefficient of
determination. `PolynomialFit` prints three significant digits per
coefficient and three fractional digits for confidence, including Dart's
signed-zero and exact decimal tie behavior for these precisions.
The three authored Flutter probes exercise uneven weights, ranks and
formatting boundaries. All eight native public tests pass.

The pointer velocity path still uses fixed stack scratch. It reuses a
single factorization for the two axes, instead of factoring the same
timestamps twice. All five focused velocity tests passed, including
all thirteen pinned Flutter release traces. A local optimized spot
check of the two-axis kernel at 20 samples took roughly 9.9 ms instead
of 16.0 ms per 25,000 fits (medians of alternating samples). This is
kernel-only, not a frame-rate benchmark.
