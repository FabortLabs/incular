# AnimatedSwitcher interpolation and outgoing child lifetime

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

All five selected original registrations passed. Rust verifies
the three opacity samples [0.4,0.4,0.1] for an interrupted fade
(:9) and both prior snapshots. The original does not explicitly
assert a fixed eviction checkpoint after settling. A separate
pinned fixture does: after 166 ms from the last replacement,
Flutter retains only the current child. Under the same elapsed
time Incular retains the partly shown middle child too. Index
:9 as a partial match on sampled opacity with the independent
lifetime discrepancy, not a wrong fade value.

In the immediate back-to-back case (:53), Flutter discards a
never-shown middle child as soon as it is replaced. Incular
retains all three, confirmed by mounted key state rather than
pixel inference. Both default updates of an unkeyed same-type
child (:88) and add/remove/repeated null transitions (:116)
match. A custom Column layout (:175) mounts and retains the
single transition; this only compares the builder/returned subtree,
not Flutter-specific widget query mechanics.

See `flutter350_animated_switcher_upstream_parity` and
`animated_switcher_eviction_reference_test.dart`. No native
rendering, animation duration API or performance claim here.

## Configuration retained by an entry

Two authored pinned comparisons use durable
`animated_switcher_entry_configuration_test.dart` and the
same focused Rust test target:

- Original outgoing reverse duration is 200 ms, and the incoming
  descriptor specifies 200 or 800 ms. At elapsed 100 ms,
  Flutter's old-child alpha is [0.5,0.5]; Rust's is [0.5,0.875].
  The equal-settings control passes, isolating timing from the
  separate exit-eviction discrepancy. Rust overwrites the old
  controller duration from the incoming descriptor.
- A currently entering child at alpha 0.5 is rebuilt twice
  with the same key and zero elapsed time. The unchanged-curve
  control passes, but changing the descriptor curve to easeIn
  replaces the retained Rust alpha with 0.31640625. Flutter
  keeps 0.5 because the curve was captured by the existing entry.
  Rust's same child element stays mounted. Only the public
  default fade was compared, not every custom builder shape.

These are authored probes; the static upstream count is unchanged.
