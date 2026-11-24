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
