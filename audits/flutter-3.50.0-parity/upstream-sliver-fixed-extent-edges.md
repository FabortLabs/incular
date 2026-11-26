# Fixed-extent sliver edges

All 18 registrations in pinned `rendering/sliver_fixed_extent_layout_test.dart`
passed. The focused Incular target `flutter350_sliver_fixed_extent_edges`
compares materialization, retained paint, and the real layout protocol.

- Lines 13 and 116: with a 600 px viewport and no cache, pages 0, 1, and 2
  replace one another at offsets 0, 600, and 1200; only the selected page
  is attached and painted. Flutter's 1199.999999999998 double rounds to
  1200 in Incular's coordinate type. A separate representable one-ULP
  predecessor of 1200 now also drops page 1, even when coming from page 1.
  Neither test silently rounds the retained controller to the edge.
- Lines 49, 60, 65, 81, 89: an extent of zero materializes only index zero.
  The exact 600 px end selects index 0, 601 selects 1, 2700 selects 4, and
  six exact 414 px items select index 5. The mounted layout verifies
  offsets and sizes from the configured extent.
- Lines 55, 73, 98, 107: the Dart calculator returns index zero even at
  zero-length input, or uses sub-ULP double deltas not representable by
  the native f32 coordinates. Incular compares the retained half-open
  interval directly, including one and two representable steps above
  an edge; these are not the same direct method or f64 inputs.
- Lines 150, 194, 238, 283: the retained slivers use their own 600 px or
  30 px extent at both initial offsets, with exact layout positions
  and 1800 px or 90 px total extent. The deprecated redundant extent
  arguments do not exist in Incular.
- Lines 328, 349, 381: the direct garbage-count query and a custom
  adaptor overriding the total extent (invalid nonmultiple and valid
  near-multiple) remain reference-only.

The old viewport-sized layout used plain division for its cache
boundaries, unlike the fixed-size list. Both now share the same
constant-time, ULP-aware interval calculation.
