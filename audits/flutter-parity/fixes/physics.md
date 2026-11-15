# physics fixes

Group: physics. Owned files: `crates/incular-animation/src/physics.rs`, `crates/incular-animation/tests/flutter_physics.rs`. Shared-file edit: `harness/incular/tests/frames/physics.rs` (clamped and duration_bounce diagnostic twins only). Records: `physics.jsonl` in this folder.

Result: all 12 findings fixed. `harness/run-frames.sh physics__`: 18/18 scenarios match (was 8/18).

## physics-001 (P2): spring velocity
- Outcome: fixed. Confirmed before fixing (forward difference, frame 0 velocity 34.8 vs Flutter 30).
- Cause: finite-difference velocity.
- Change: analytic derivative per damping regime, computed in f64.
- Tests: `spring_velocity_is_analytic_across_regimes`, `spring_velocity_matches_central_difference`, `scroll_spring_starts_at_rest_with_zero_velocity`.

## physics-002 (P0): gravity negative acceleration
- Outcome: fixed. Confirmed (done=1 at frame 0).
- Cause: completion depended on acceleration sign.
- Change: `is_done` uses `abs(x(t)) >= end`.
- Tests: `gravity_negative_acceleration_completes_by_absolute_distance`.

## physics-003 (P2): bounded friction
- Outcome: fixed. Confirmed.
- Cause: velocity zeroed at bounds; completion on unclamped position.
- Change: friction velocity kept; completion on clamped position within `tolerance.distance` of a bound.
- Tests: `bounded_friction_keeps_velocity_at_the_bound`, `bounded_friction_completes_within_distance_tolerance_of_bound`.

## physics-004 (P2): clamped velocity bounds
- Outcome: fixed. Confirmed by source review.
- Change: `ClampedSimulation::with_velocity_bounds(dx_min, dx_max)`. The diagnostic twin now uses dxMin -50 / dxMax 50 to match the Flutter twin.
- Tests: `clamped_simulation_clamps_velocity_to_bounds`.

## physics-005 (P3): strict boundaries
- Outcome: fixed. Confirmed (both boundary twins done at frame 0).
- Change: strict `<` in friction and spring `nearZero` checks.
- Tests: `friction_completion_is_strict_at_velocity_tolerance`, `spring_completion_is_strict_at_tolerance_boundary`.

## physics-006 (P3): spring description inputs
- Outcome: fixed. Confirmed.
- Change: `SpringDescription::new` and `with_damping_ratio` store inputs unchanged. Behavior change: invalid inputs are no longer sanitized.
- Tests: `spring_description_preserves_raw_inputs`.

## physics-007 (P3): duration truncation
- Outcome: fixed. Confirmed.
- Change: duration truncated to whole milliseconds before computing stiffness. A zero duration is clamped to 1 ms (Flutter asserts).
- Tests: `duration_bounce_truncates_to_whole_milliseconds`.

## physics-008 (P2): duration and bounce getters
- Outcome: fixed.
- Change: `SpringDescription::duration()` and `bounce()`. The diagnostic twin now calls them.
- Tests: `spring_duration_and_bounce_getters_follow_flutter`.

## physics-009 (P2): snapToEnd
- Outcome: fixed.
- Change: `SpringSimulation::with_snap_to_end`, default false.
- Tests: `spring_snap_to_end_reports_exact_end_and_zero_velocity`.

## physics-010 (P2): friction helpers
- Outcome: fixed.
- Change: `with_constant_deceleration`, `FrictionSimulation::through`, `final_x`, `time_at_x`. The Newton-solved final time (10 iterations) matches Flutter. `time_at_x` returns `Option<Duration>`; `None` where Flutter returns infinity.
- Tests: `friction_final_x_is_the_asymptote_without_deceleration`, `friction_time_at_x_inverts_position`, `friction_through_passes_through_start_and_end_velocity`, `friction_with_constant_deceleration_stops_at_final_time`.

## physics-011 (P2): tolerance through the trait
- Outcome: fixed, with a limitation.
- Change: `Simulation::tolerance()` is a provided method defaulting to `Tolerance::DEFAULT`. A required setter would break `RepeatingSimulation` (incular-widgets) and `StepSimulation` (harness), so `set_tolerance` is inherent on each concrete type instead. `dyn Simulation` cannot be mutated.
- Tests: `tolerance_is_readable_through_trait_object`, `set_tolerance_changes_completion`.

## physics-012 (P3): nearEqual / nearZero
- Outcome: fixed.
- Change: public `near_equal` and `near_zero` in `incular_animation::physics`. `lib.rs` is unchanged. f32 signatures; no nullable variant.
- Tests: `near_helpers_are_strict_within_epsilon`.

## Validation
- `cargo test -p incular-animation --test flutter_physics`: 21 passed.
- `cargo clippy -p incular-animation --all-targets -- -D warnings`: clean.
- `harness/run-frames.sh physics__`: 18/18 match.
- `cargo test -p incular-scroll --test flutter_ballistics`: 14 passed. `--test physics_audit`: 13 passed, 1 failed (`controller_distinguishes_input_from_programmatic_movement`, offset assertion at physics_audit.rs:311). Scroll controller code is under active edit by another worker and scroll does not import the animation physics types; not caused by this change.
- `cargo check -p incular-scroll -p incular-widgets --tests`: fails in `incular-widgets/tests/scroll_pages_reorder.rs` (mismatched types) and in `incular-widgets` lib (`SizedBox`) from other workers' in-progress edits.
- `harness/run-frames.sh sample_list_fling`: 1/1 scenarios match (99 frames), run after the harness build recovered.
