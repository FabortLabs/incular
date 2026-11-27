# Semantic heading/transform, ballistic clamping, and scheduler dilation

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

- Five exact originals in `widgets/semantics_test.dart` passed.
  The Incular `flutter350_semantics_heading_geometry` target matches
  tooltip-only metadata (:100), and replacing child semantics while
  keeping the parent's label (:1525). Rust doesn't expose the
  same semantic directionality, and the latter uses an explicit
  neutral exclusion wrapper; these are partial comparisons.
  Below a singular FittedBox (:1876), Incular retains an otherwise
  empty ScrollView node with untransformed 266×55 bounds. The
  labeled control disappears from Flutter's actual semantics tree
  and remains published by Incular as well. Flutter's root
  debug transform accessor is not compared.
- Heading default and 1–6 are correct, but Rust accepts 0/7/8/9
  and advertises a Heading role with no valid level (:2020).
  Flutter debug rejects the values at construction; -1 is not
  representable by the Rust u8 setter. The nine cases of :2039
  compare direct Flutter configuration absorption with a separately
  verified retained MergeSemantics fixture. Both preserve a
  parent's existing level. Only Flutter inherits a child's 1 or 2
  when the parent has no level. Authored retained probes:
  `semantics_heading_merge_test.dart`. No OS announcement claim.
- All five exact clamping simulation originals passed. Incular
  matches monotonic deceleration (:29), continuous smooth stop
  (:43), and the acceleration formula (:101). Flutter's absolute
  1e-10 initial-condition tolerance (:11) is finer than the Rust
  f32 public boundary: Rust preserves rounded values, not the
  original doubles. At :65, restarting repeatedly at 90 Hz
  accumulates about 0.00146 px at terminal position; greatest
  sampled position difference about 0.00195 px. The restart
  comparison uses public launch thresholds adapted to accept
  every nonzero sample, and terminal None at exact zero. Both
  implementations use the same model equations. These are
  partial precision differences, not a wrong friction law.
  See `flutter350_upstream_clamping_simulation`.
- Default 1.0, half-speed elapsed 5 ms, and double-speed elapsed
  20 ms match the original 10+10 ms frame cases in
  `flutter350_scheduler_dilation_exact`. Original animation :21
  cancellation also passed on Flutter. The prior B06 Rust
  regression reproduces the same missed cancellation in a
  partially adapted fixture with an extra later control and no
  comparison of Flutter's relative epoch origins. It is not
  another independent bug.

## Exact queued callback recheck

Pinned `scheduler/animation_test.dart:21` and the permanent Rust
port pass. The first callback observes zero elapsed time relative
to the first frame, cancels the second callback from that same
snapshot, and neither executes on the next frame. The Rust API
exposes an absolute Instant, so the port checks it equals the
first raw timestamp that defines the epoch. This completes the
earlier partial comparison.

## Heading level remediation

Pinned `semantics_test.dart:2020` and its Rust ports agree on
the absent default, all six valid values and all five signed invalid
values from the original. The public setter now rejects invalid
headings in debug, and does not advertise them in release. This
completes the earlier partial comparison.

The separately adapted :2039 checks all nine parent/child combinations
in the retained semantics tree. The previous repair kept explicit
parent levels but lost levels from children when the parent had no
level. The merged node now takes the first descendant's level in
tree order only when a parent level is absent. A separate test updates
and clears these fields without replacing the node. Both authored
Flutter controls and all five targeted heading regressions pass.
The direct Flutter SemanticsConfiguration.absorb API is still distinct
from the retained Rust wrapper and is not counted as another full pair.

## Empty fitted content

The pinned :1876 test and the authored labeled control still pass.
Incular previously substituted the identity transform when FittedBox
had no area, exposing its 266 by 55 scroll node and a labeled control
in the semantics tree. It now represents the fit as singular,
removes the descendant semantics, and keeps the root unaffected.
The retained test also checks a visible, empty, visible transition
with and without merging. The draw command disappears while empty
and returns when area is restored without replacing the render object.
All 44 selected layout, semantics and custom-paint tests pass.
The Flutter root's debug transform accessor remains a distinct surface,
so this recheck does not add an original pair.

## Double-precision clamping simulation

The pinned initial-condition and ballistic-restart cases now pair using
the public ClampingScrollSimulation. It reuses the runtime's f64 motion
equations without first rounding the caller's inputs or samples to f32.
All nine initial positions and velocities, and every restarted position
and velocity along the 90Hz trajectory, agree within 1e-10. The runtime's
existing f32 scrolling/layout boundary is unchanged. Native controls
also verify that a zero-velocity simulation is finite and increased
friction shortens travel.

## Published heading levels and validation precedence

Exact pinned `widgets/semantics_test.dart:2105,2137,2175` all passed.
The native `flutter350_box_semantics_metadata` suite mounts six labeled
headings and ordinary text inside a ListView. It verifies that both the
retained graph and desktop AccessKit projection expose the six levels as
headings and do not reinterpret adjacent plain text.

The direct annotation check compares unspecified, valid, and invalid in
the retained declaration and the resulting mounted node. For ordinary
nested annotations the nine original combinations use invalid over
valid over unspecified. Before the repair an outer invalid annotation
could leave the displayed inner label marked valid. Compatible labels and
their validity now reach the same node; an explicit semantic boundary
stops the merge. That boundary was also exercised by an authored pinned
Flutter test. Native tests confirm that the projected invalid flag clears
when the active result becomes valid. All 38 focused runtime checks in
this batch passed.
