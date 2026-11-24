# Implicit transform and opacity samples

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

Five exact originals and their Rust counterparts pass. At
0, 500, 750, and 1000 ms, the same four samples match the
fractional slide (:218), scale (:276), rotation including
the 1.5-turn full path (:334), and ordinary and sliver
opacity (:392, :473). The Rust transforms are measured
from the actual recorded paint commands for a fixed-size
child; alpha comes from the mounted layer node.

Both sources count exactly one parent build on mount,
no parent rebuild before a frame, and exactly one more
for the property change. Subsequent effect ticks do not
rebuild the caller. The Rust adapter uses an explicit
builder revision rather than the Dart fixture's tap.
See `flutter350_implicit_transition_samples`.
