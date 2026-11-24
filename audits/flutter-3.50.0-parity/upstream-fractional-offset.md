# FractionalOffset and alignment origins

Reference: pinned Flutter 3.50.0, `packages/flutter/test/painting/fractional_offset_test.dart`. All five original registrations passed.

- Lines 26, 42 and 50: fully paired in `flutter350_fractional_offset`. Interpolation of one absent endpoint uses the center; two absent endpoints remain absent. Conversions preserve out-of-range fractions and subtract the rect origin before normalization.
- Line 36: equal values interpolate correctly; Rust values have no observable Dart object identity.
- Line 9: Flutter reference only; the original arithmetic and diagnostic cluster is not fully covered by the Incular public API.

`Alignment::along_size` now uses Flutter's top-left origin. All eight focused configuration tests pass. The retained follower test with unequal rectangles, distinct anchors and a nonzero offset now passes when unscaled. The scaled case still does not inherit the leader's scale (actual 30 x 20; Flutter expects 60 x 40); that is separate and remains open.
