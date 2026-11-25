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
