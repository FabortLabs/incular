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
