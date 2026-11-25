# Container control paint and directional radii diagnostics

Two exact originals in pinned `widgets/container_test.dart` passed.

- :50 The simple Container path paints exactly three fills in world
  coordinates: green background at (5,5) size 53×78, yellow child
  at (26,43) size 25×33, and the semi-transparent blue foreground over
  the background bounds. The retained test rejects an extra border,
  clip, or paint command, and covers margin, padding, constraints and
  bottom-right alignment together.
- :744 Previously an invalid directional radius silently used LTR.
  The core builder prevalidation now reports a typed missing-ambient
  error before attempting to resolve it. The error names Container and
  includes the two original diagnostic fragments about missing text
  direction and the missing Directionality ancestor. A separate
  control covers DecoratedBox resolving valid RTL and not reading
  across a LookupBoundary.

The original expected Dart paint operations; Incular checks the retained
fill commands before wgpu. No pixel identity claim was made. Five
focused Rust targets passed (24 tests).
