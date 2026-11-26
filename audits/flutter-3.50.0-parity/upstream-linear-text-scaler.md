# Linear text scaler clamping

Five pinned originals at `painting/text_scaler_test.dart` passed on
Flutter 3.50.0. The two linear cases pair. Four independent operations
produce equal 3× policies; neutral scaling equals linear(1). Capping a
5× policy at 3, flooring neutral scaling at 3, and setting a 3–3 range
all produce exactly 3×.

Rust rejects the invalid bounds with structured errors. The equivalent
inverted range, NaN maximum, negative minimum and infinite minimum
all reject; the pinned ClampedScaler case is partial because its source
is a platform scaler. The system equality and reclamping cases remain
reference-only: Incular has a linear host factor, not Flutter's
platform size-dependent scaling and clamped system wrapper.

The authored `linear_scaler_layout_test.dart` uses the resulting policy
in a real mounted paragraph. A clamp from 5× to 3× measures the same
as directly selecting 3×. Reclamping with minimum 4 gives exactly the
direct 4× size and increases both dimensions. The native control also
retains the render object. This is a size comparison within each
engine, not a cross-engine glyph pixel claim.
