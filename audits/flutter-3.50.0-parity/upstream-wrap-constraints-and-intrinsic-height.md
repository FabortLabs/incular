# Wrap constraints, one-pixel overflow, and intrinsic height

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

All five exact Flutter originals and focused Rust cases passed.
Shrink-wrapping both arrangements and cross-axis end alignment
(:644) match, as do one pixel pushing the last child into a
new run (:898) and a child exactly as wide as the host (:925).
The horizontal intrinsic height with two 16-pixel text runs
and a 40-pixel Row (:973) matches at 72.

The default, non-overflowing Wrap case (:787) records no
clip in Incular. Only this part is ported. Flutter's second
half explicitly requests HardEdge and then observes a clip
on overflow. Incular has no public Wrap clipBehavior
option; substituting another widget's clip would not test
the same API. See `flutter350_wrap_constraints_parity`.
