# Mounted tap, long press, and scale coordinate spaces

All seven selected registrations passed on pinned Flutter and against the
retained Incular runtime. The comparisons drive timestamped touch
events through hit testing, arena resolution, and the real callbacks.

- `transformed_tap_test.dart:12,48,108`: unchanged local and global
  down/up coordinates at identity, and a global 18-pixel slop under both
  2× and 0.5× scaling. A 17-pixel move taps and reports the scaled local
  delta; a 19-pixel move cancels exactly once without a tap-up.
- `transformed_long_press_test.dart:12,53,123`: local and global start
  and end positions match; motion before the deadline emits no move
  update, and motion after recognition reports both scaled local and
  original global displacement, even outside the detector.
- `transformed_scale_test.dart:9`: two contacts report both start
  focal points and the single update, in local and global coordinates.

Both scaled tap cases failed before the correction. The slop calculation
already used global coordinates, but the retained arena postponed TapDown
even when the tap was the only member. It now delivers that lone tap's
arena acceptance at close, so later slop rejection delivers TapCancel.
Tap plus drag, long press, double tap, scale or raw contenders are not
treated as a lone tap; earlier double-tap competition stays intact.

Native: `flutter350_transformed_recognizer_coordinates`,
`flutter350_double_tap_competition`, `flutter350_transform_hit_tests`,
and `flutter350_upstream_long_press` (20 passed).
