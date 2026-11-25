# Decoration image paint

## Original drawing and nine-slice comparisons

Four exact-name registrations in pinned `painting/paint_image_test.dart`
passed. Incular's `flutter350_decoration_image_paint` target covers:

- :35 Cover and align: decoded image identity, centered 300×150 pixel crop
  starting at (0,75), and translated 200×100 destination at (50,75).
- :185 Uneven scale: a 208×142 bitmap at scale 3 is drawn without throwing.
  The same decoded image backs all nine draws. The pixel-space center and
  the logical fixed corners and destination are also checked.
- :206 Smaller than the slice borders: with the same 208×142 bitmap and a
  100×46 destination, the decoration still emits draws using the bitmap.
  The old code panicked on a negative inner extent and separately on
  rounded slice edges crossing. An empty center now feeds box-fit as zero;
  a collapsed center also uses a single shared boundary to keep both
  adjacent slices' coordinates in order.

Flutter records a single `drawImageNine`; Incular emits multiple bounded
image draws. Tests check the observable paint requirements, not the command
encoding, and do not assert GPU pixels.

The fourth registration (:165) supplies a slice whose scaled right and
bottom exceed the 300×300 bitmap. It tests arguments logged by a mock
canvas, not native drawing. Incular does not forward an unchecked
nine-slice draw command and this is not counted as paired.

Four focused Rust targets passed (32 tests).

## Ordinary image repetition

The authored `image_scale_paint_reference_test.dart` now also records
repeated image draws. Both axes and each single axis place the same
20×20 tiles as Flutter in a 50×50 box. Every tile is drawn under one
bounds clip. When a single tile fills the box, both implementations
skip the clip and emit just one draw. The first Rust run failed because
ordinary retained images emitted partial edge tiles with no clip.
The three focused Rust targets passed (25 tests), and both authored
reference cases passed. This adds no original registration credit.

## Flutter-specific paint diagnostics

The remaining five originals passed in the focused reference run.
:55 and :135 check oversized-decode warnings against the view DPR,
the specific debug color inversion, and the smaller-image threshold.
:230, :269, and :312 check the paint-image report hook and its
cross-frame deduplication, changed sizes, and unnamed source behavior.
Incular exposes decoded handles and renderer diagnostics, but does not
expose these oversize-inversion or per-frame image-size callbacks.
These are reference-only; paint draw count is not a substitute for
a reported event or warning.
