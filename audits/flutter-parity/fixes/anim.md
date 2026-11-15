# anim fixes

Group `anim`: 12 findings from the anim_compose, anim_core, anim_curves and paint audits. Machine-readable records are in `anim.jsonl`.

## anim_compose-001 (P0): fixed
- Cause: TrainHoppingAnimation hopped only inside getters, so the switch and its callback waited for a read.
- Change: trains are subscribed through DependencySource; the hop runs when either train changes.
- Tests: `animation_graph.rs` train_hop_fires_on_the_crossing_not_on_a_read, train_hop_stops_listening_when_dropped.

## anim_compose-002 (P2): fixed (additive)
- Cause: AnimationSource is f32-specific.
- Change: `always_stopped<T>(value)` returns a forward-status DrivenAnimation<T>. AlwaysStoppedAnimation(f32) is kept.
- Test: always_stopped_carries_a_value_of_any_type.

## anim_compose-003 (P1): fixed
- Cause: CurvedAnimation latched its direction lazily on read.
- Change: it subscribes to the parent and latches on each change.
- Tests: curved_animation_keeps_its_direction_through_an_unobserved_reversal, plus the existing curved latch test.

## anim_core-001 (P2): fixed
- Cause: one change guard served both listener dispatch and reactive invalidation.
- Change: listeners run on every tick and set_value; reactive invalidation stays change-gated.
- Test: value_listeners_hear_every_tick_and_assignment.
- Divergence kept: start_run notifies value listeners only on a value change. Flutter does not notify at start; widgets depend on this.

## anim_core-002 (P2): fixed
- Cause: with_value seeded the reported-status baseline.
- Change: the baseline starts dismissed, as in Flutter.
- Test: first_forward_reports_status_from_an_interior_value.

## anim_core-004 (P2): fixed
- Cause: derived nodes had no subscription API.
- Change: AnimationSource::changes(); DoubleAnimation::observe / observe_status; DrivenAnimation::observe. Composite nodes forward parent changes.
- Limitation: reactive, change-gated semantics, not Flutter's per-notifyListeners cadence.
- Out-of-ownership edit: `crates/incular-widgets/src/explicit_animations.rs`, the `Clocks` impl gained `changes()`.
- Test: derived_nodes_observe_their_parents_changes.

## anim_curves-001 (P2): fixed
- Cause: Size tween and Size::lerp clamped extents to zero.
- Change: unclamped in the tween and in Lerp for Size. Rect follows.
- Limitation: AnimatedSize uses Size::new, which asserts non-negative, so it can still panic on overshoot. Pre-existing and outside this group.
- Tests: size_tween_extrapolates_negative_extents, size_lerp_extrapolates_negative_extents.

## anim_curves-002 (P2): fixed
- Cause: TweenSequence used an inclusive upper bound.
- Change: half-open intervals; the last item still takes progress 1.
- Test: tween_sequence_boundary_belongs_to_the_next_item.

## anim_curves-003 (P2): fixed (additive)
- Change: FlippedTweenSequence, TweenSequence<f32>::flipped, Animatable impls for TweenSequence<f32> and FlippedTweenSequence. f32 only; an empty sequence reads 0.
- Test: flipped_tween_sequence_inverts_the_reversed_timeline.

## anim_curves-004 (P2): deferred
- The finding is valid. Null endpoints need per-type semantics (colour alpha scaling, zero size and rect) and an additive API. No caller needs it yet.

## paint-020 (P2): fixed
- Cause: EdgeInsets, EdgeInsetsDirectional, Alignment, FractionalOffset, AlignmentDirectional and BorderSide clamped t.
- Change: t is unclamped. BorderSide width stays clamped at 0, since Flutter asserts it.
- Out-of-ownership: none. BorderSide is the owned impl in decoration.rs.
- Tests: `crates/incular-config/tests/lerp_extrapolation.rs` (EdgeInsets, Alignment). BorderSide has no unit test.

## paint-021 (P3): fixed
- Cause: HsvColor::new folded hue 360 to 0 before the lerp.
- Change: 360 is kept; the lerp takes the mixed hue modulo 360.
- Test: `crates/incular-core/tests/lerp_extrapolation.rs` hsv_hue_does_not_wrap_through_zero_before_360.

## Validation
- `cargo test -p incular-animation` for animation_graph, animation_behavior, reactive_observation, flutter_animation_parity, flutter_curves and public_api: all pass.
- `cargo test -p incular-config --test lerp_extrapolation` and `cargo test -p incular-core --test lerp_extrapolation`: pass.
- `cargo check -p incular-parity-harness --tests`: passes.
- `cargo clippy -p incular-animation -p incular-config -p incular-core --all-targets -- -D warnings`: passes.
- rustfmt --edition 2024 on each owned file.
- Scenario reruns: see the final report.

## Frame-order pass

- **anim_core-001 residual: fixed.** `start_run` notified value listeners on a status-only change, such as a reverse from the held value. It now notifies them only when the start moves the value; reactive readers and status listeners still hear the start.
  - Harness: `anim_core__controller_listener_notifications` matches (it was 7 against Flutter's 6 at frame 6).
  - Test: `reactive_observation.rs::a_reversal_from_the_current_value_does_not_notify_value_listeners`.
  - `animation_listener_can_mutate_after_notification_releases_state_borrow` now notifies through the first tick, because it encoded the extra start notification.
- anim_core-003 and anim_core-005 are recorded in `frame-order.jsonl`.
