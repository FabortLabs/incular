# AnimatedPositioned and AnimatedPositionedDirectional

Eight pinned registrations passed against real retained layout: the from-rect
constructor, interruption from the displayed midpoint, switching from an
explicit width to a trailing edge, and the zero-area directional case.
Interrupted and variable-switching cases run in physical, LTR, and RTL
coordinates. The native tests check retained identity, sizes and world-space
positions at every pinned sample, including the immediate 100→700 width
jump while the top edge keeps animating. The three remaining original basics
cases also assert Dart render-tree diagnostic snapshots and are not counted.

`AnimatedPositioned::from_rect` now copies all four inputs and leaves right
and bottom unset. One-line diagnostics and accessors expose the configured
values. Supplemental paired checks establish that three populated values on
one axis are invalid in debug builds and that an explicit direction on a Stack
does not provide an inherited Directionality to an animated descendant.
The native directional variant no longer silently assumes LTR there.

All eleven native comparisons and fourteen existing shared completion-timing
regressions passed in the named targets.
