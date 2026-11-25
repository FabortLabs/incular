# TapRegion, AbsorbPointer, editing deltas, and RawImage

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

## Pointer and semantic routing

- Eight exact Flutter TapRegion originals passed. The normalized
  primary mouse matches outside/inside mouse-up (:113, :339),
  suppression with consumption (:179), and updating a mounted
  region's group (:487). The focused Rust Runtime target is
  `flutter_tap_region_upstream_parity`.
- Outside/inside down callbacks on both right and middle mouse
  (:578, :652, :904, :978) do not fire in Incular. The same mounted
  raw listeners receive each button phase with the correct held and
  changed bits, and the primary control passes. The shared early
  filter precedes classification in `tree/raw_input.rs:736–747`.
  `flutter_tap_region_mouse_buttons_parity` checks callback membership
  per contact, not same-phase ordering, pointer add/remove,
  navigation scope, or secondary consumption.
- All three original AbsorbPointer registrations passed. Incular
  preserves delivery to a disjoint sibling (:11). The semantic
  tests (:26, :50) compare Flutter's test-local button to Incular's
  neutral retained action surface, so are partial rather than
  tests of identical focus adapters. With absorption off, the
  label, role, enabled/focusable flags, Activate and Focus match.
  With it on, the flags still match and Activate is suppressed,
  but only Incular advertises Focus. This is a tree difference,
  not a native screen reader or keyboard-focus claim. See
  `flutter350_absorb_pointer_upstream_parity`.

## TapRegion callback order

The separate painted-mouse callback-order probe
`tap_region_callback_order_test.dart` confirms outside callbacks run
before inside callbacks, on both down and up. Incular initially sent
inside first in both phases. The retained dispatch now matches
`A.outsideDown, B.insideDown, A.outsideUp, B.insideUp`. All eight
focused primary and navigation Runtime cases pass. This is an authored
ordering check, not a new original registration or a claim about order
between multiple callbacks in the same classification.

## Editing and raster defaults

- Three original TextEditingDelta checks match the observable
  applied text, caret and composing payload for deletion (:138),
  shorter replacement (:235) and selection/composition-only
  update (:336). Incular cannot represent Flutter's separate
  stored oldText/JSON variants, so these are partial.
  The independently adapted invalid insertion (:68) also exposes
  a policy difference: Flutter debug rejects the mismatch, while
  Incular clamps the out-of-range offset against its input and
  inserts a character. It is not a release assertion comparison or
  a wire-format test. See `flutter350_services_text_delta_parity`.
- A separate authored pair, not a static upstream count, measures
  RawImage on actual Windows wgpu. With omitted fit and a center
  slice, Flutter fills 100×60, Incular paints only a centered
  30×30 image. Explicit Fill, explicit ScaleDown, and no-slice
  controls match. Flutter defers the missing fit to paintImage;
  Incular hardcodes ScaleDown before the correct slice-aware
  fallback. See `flutter350_raw_image_nine_slice` and
  `raw_image_nine_slice_test.dart`.
