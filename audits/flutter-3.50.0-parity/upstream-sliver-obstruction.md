# Rendering: sliver scroll obstruction

The Flutter 3.50.0 originals
`rendering/sliver_persistent_header_test.dart:14` and `:29` compare
`SliverGeometry.maxScrollObstructionExtent`. A purely floating
header reports zero, and a floating pinned header reports its
100-pixel minimum, not its 200-pixel maximum, geometric visibility,
or currently painted extent.

The Incular field is independent of paint and layout extent.
Collapsed, partially painted, and fully clipped native cases keep the
same obstruction for floating-pinned headers. Fixed and natural pinned
headers use their measured/pinned extent. Flowing geometry and the
pinned header when wrapped by a Flutter-style SliverPadding keep the
default on the padding wrapper itself. Twenty-six adjacent layout,
constraint, padding, and viewport checks passed. This slice verifies
the producer geometry; scroll-to-reveal consumption is separate.

## Scrollable.ensureVisible

The neutral Flutter probe uses 200 pixels of header scroll extent, a
100-pixel pinned minimum, a 300-pixel spacer, and a 100-pixel target.
In a 600-pixel viewport, start/center/end alignment yields offsets
`[400, 200, 0]` with a floating pinned header and `[500, 250, 0]`
with a merely floating header. Before the fix Incular put the target
at offset 500 behind the pinned strip.

The retained viewport now records the obstruction before each
top-level sliver during layout, and explicit reveal subtracts that
space from the alignment viewport. Both axes and reversed axis
directions pass the same Flutter and native observations. Eight
existing ensure-visible tests remain executable and pass alongside
the centered-anchor and scroll-focus neighbors. This verifies
`Scrollable.ensure_visible` for following slivers, not automatic
caret reveal or nested-group-specific propagation.
