# AnimatedSwitcher builder replacement, repeated keys, and zero area

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

All three original registrations pass on Flutter. Changing the
transition builder (:319) replaces all three mounted fades with
transforms in Incular; this is a scoped partial match because
the Rust marker is a scalar transform rather than the Dart
animation-valued ScaleTransition and KeyedSubtree checks.
Initial empty, add keyed X, and replace with keyed Y in a zero-size
host (:420) all keep measured size zero.

For the A→B→A reentry (:400), Flutter finds exactly one instance of
A after its 1000 ms pump. Incular still has two mounted A render
leaves with the same logical child key; the marker matches the
child type and color, not just a hit or pixel. This is a further
instance of the outgoing-entry cleanup gap. See
`flutter350_animated_switcher_upstream_parity`.
