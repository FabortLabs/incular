# FractionalTranslation and shifted input or accessibility bounds

Five exact `widgets/basic_test.dart` registrations passed on the pinned
Flutter checkout:

- **164, 191, 218:** the listener receives a down at the translated child
  center when it is wholly inside, partly inside, or entirely outside
  the untranslated box. The native runtime uses actual pointer down/up
  dispatch and the listener receives local position (50, 50) in all
  three cases.
- **245:** on a display with scale 3, tapping the fractional translation's
  unshifted center updates the translation from (0.4, 0.4) to (0.8, 0.8).
  The same mounted semantic label's physical origin moves from
  (1170, 870) to (1290, 990); its logical size stays (100, 100). Incular
  stores world logical bounds, not Flutter's per-node matrix.
- **327:** a zero-area view does not crash and the translation remains
  zero size. A shrink child is the native equivalent of Flutter's
  omitted child in this fixture.

Native: `flutter350_fractional_translation` and
`flutter350_proxy_semantic_geometry` in `incular-runtime` (nine passed).
No production changes were required.
