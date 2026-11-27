# FractionalOffset and alignment origins

Reference: pinned Flutter 3.50.0, `packages/flutter/test/painting/fractional_offset_test.dart`. All five original registrations passed.

- Lines 26, 42 and 50: fully paired in `flutter350_fractional_offset`. Interpolation of one absent endpoint uses the center; two absent endpoints remain absent. Conversions preserve out-of-range fractions and subtract the rect origin before normalization.
- Line 36: equal values interpolate correctly; Rust values have no observable Dart object identity.
- Line 9: fully paired. The focused fixture covers unary minus, add/subtract, multiply, divide, truncating division, Euclidean remainder, equal-value hashes, and the one-line diagnostic with a one-decimal half-up magnitude. A signed-zero hash and negative remainder are additional native controls.

`Alignment::along_size` now uses Flutter's top-left origin. All eight focused configuration tests pass. The retained follower test with unequal rectangles, distinct anchors and a nonzero offset now passes when unscaled. The later [linked affine repair](upstream-geometry-input-gestures.md#linked-affine-repair) independently closed the scale inheritance gap.
