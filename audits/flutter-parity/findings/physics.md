# physics: Simulation and scroll ballistics

## Summary

Eighteen deterministic frame twins cover tolerance defaults, all three spring damping regimes, friction, bounded friction, gravity, clamped simulations, direct scroll springs, and bouncing/clamping scroll simulations, plus four ballistic-dispatch edge cases added in this pass (clamping overscroll, bouncing overscroll at rest, clamping negative velocity, bouncing transfer clamp). `harness/run-frames.sh physics__` runs end to end: eight scenarios match and ten diverge. The four new dispatch scenarios all match, and the divergences are the same ten as before, concentrated in spring velocity (incular-animation only), stopping thresholds, and bound handling. No new divergence was found, so no finding was added. The incular-scroll ballistic spring is analytic, so physics-001 is scoped to the incular-animation spring types. Existing `crates/incular-scroll/tests/flutter_ballistics.rs` also covers the Android spline, bouncing friction and spring handoff, pixel-ratio tolerances, and fixed-extent ballistics.

`flutter analyze` and `cargo clippy -p incular-parity-harness --all-targets -- -D warnings` pass. The package-wide `cargo fmt -p incular-parity-harness` is deferred to the root agent while other scenario files are still being edited; this file was run through `rustfmt` directly.

## Coverage

| Flutter item | Scenario / evidence | Result |
| --- | --- | --- |
| `Tolerance` | `physics__tolerance__default`; used with explicit values in threshold probes | Default distance/time/velocity values match (`0.001` each). |
| `Simulation` | `physics__spring__damping_regimes`, `physics__friction__ordinary`, `physics__gravity__negative_acceleration_threshold`, and other direct simulations | Position, velocity, and completion are sampled. Incular's trait has no mutable `tolerance` member (physics-011). |
| `SpringDescription` | `physics__spring_description__damping_regimes`, `physics__spring_description__duration_bounce` | Valid positive damping ratios classify as under/critical/over on both sides. Raw invalid inputs are normalized by Incular (physics-006); sub-millisecond durations produce different constants (physics-007). Flutter's `duration` and `bounce` getters have no Incular equivalent (physics-008). |
| `SpringType` | `physics__spring_description__damping_regimes`, `physics__spring__damping_regimes` | All three normal regimes map to the same values. Negative ratio divergence follows the constructor normalization in physics-006. |
| `SpringSimulation` | `physics__spring__damping_regimes`, `physics__spring__tolerance_boundary` | Position and type match for the normal regimes. Velocity differs in all three from frame 0 (physics-001); exact-threshold `isDone` differs (physics-005). Flutter's `snapToEnd` option is not available (physics-009). |
| `ScrollSpringSimulation` | `physics__scroll_spring__snap_when_done`, `physics__bouncing_scroll__extent_handoff` | Position and completion follow the Flutter timeline; velocity shares the SpringSimulation derivative difference (physics-001). |
| `FrictionSimulation` | `physics__friction__ordinary`, `physics__friction__tolerance_boundary` | Ordinary friction samples match. Exact-threshold completion differs (physics-005). Flutter's constant-deceleration, `through`, `finalX`, and `timeAtX` surface is not implemented on Incular's animation simulation (physics-010). |
| `BoundedFrictionSimulation` | `physics__bounded_friction__position_and_velocity`, `physics__bounded_friction__distance_tolerance` | Position is clamped on both sides, but Incular zeroes velocity at the bound and misses Flutter's distance-tolerance completion (physics-003). |
| `GravitySimulation` | `physics__gravity__negative_acceleration_threshold` | Position and velocity formulas match; negative-acceleration completion diverges at frame 0 (physics-002). |
| `ClampedSimulation` | `physics__clamped_simulation__position_and_velocity` | Position clamping and completion forwarding match. Flutter's velocity bounds cannot be specified and the sampled velocity is not clamped (physics-004). |
| `BouncingScrollSimulation` | `physics__bouncing_scroll__extent_handoff`; also `flutter_ballistics.rs::bouncing_hands_off_to_a_spring_at_the_extent_it_crosses` | Frame values match through friction-to-spring handoff and settling. |
| `ClampingScrollSimulation` | `physics__clamping_scroll__spline`; also `flutter_ballistics.rs::clamping_fling_follows_the_android_spline` | Frame values match through the spline duration. |
| `createBallisticSimulation` (`scroll_physics.dart:754-771` bouncing, `:892-913` clamping) via `BouncingScrollSimulation` and `ClampingScrollSimulation` | `physics__ballistic_dispatch__clamping_overscroll`, `physics__ballistic_dispatch__bouncing_overscroll_at_rest`, `physics__ballistic_dispatch__clamping_negative_velocity`, `physics__bouncing_scroll__transfer_clamp` | Match. Dispatch with FixedScrollMetrics (DPR 1) returns the same spring-back, rest-overscroll, negative-velocity clamping and 5000 px/s transfer-clamp outputs through `ScrollPhysics::ballistic_simulation`. |
| `nearEqual`, `nearZero` | Source review | Flutter exports both helpers from `physics.dart`; no public Incular equivalent was found. Incular has only a private scroll-local `near_zero` (physics-012). |

## Findings

### physics-001 — Spring simulations estimate velocity with a forward difference

- Severity: P2
- Category: behavior
- Flutter: `packages/flutter/lib/src/physics/spring_simulation.dart:250-255,320-324,353-355,388-392` computes `dx` from each regime's analytic derivative.
- Incular: `crates/incular-animation/src/physics.rs:181-185` estimates it as `(position(t + 1 ms) - position(t)) / 1 ms`.
- Evidence: `physics__spring__damping_regimes` first diverges at frame 0 / 0 ms: Flutter reports initial velocity 30 for all regimes; Incular reports 34.8282 (under), 34.6680 (critical), and 34.3399 (over). `physics__scroll_spring__snap_when_done` also starts at 0 versus 4.9438. Position samples remain within the value tolerance.
- Impact: spring consumers receive a velocity that is one millisecond ahead of the requested instant. The same value feeds `isDone`, and scroll handoffs inherit the discrepancy.
- Likely cause: finite-difference velocity replaces Flutter's closed-form derivatives.
- Scope: the incular-scroll ballistic spring (crates/incular-scroll/src/ballistic.rs SpringMotion) computes analytic derivatives and matches in the bouncing and clamping dispatch scenarios; only the incular-animation SpringSimulation and ScrollSpringSimulation are affected.
- Fix direction: implement the analytic derivative for each damping regime and use it in spring completion checks.
- Confidence: high

### physics-002 — Negative acceleration marks gravity simulations done immediately

- Severity: P0
- Category: behavior
- Flutter: `packages/flutter/lib/src/physics/gravity_simulation.dart:90` finishes when `abs(x(time)) >= endDistance`, regardless of acceleration sign.
- Incular: `crates/incular-animation/src/physics.rs:343-349` compares `position <= end` whenever acceleration is negative.
- Evidence: `physics__gravity__negative_acceleration_threshold` first diverges at frame 0 / 0 ms: both positions are 0 with an end threshold of 100; Flutter reports `done = 0`, Incular reports `done = 1`.
- Impact: a negative-acceleration simulation starting below its positive threshold can be treated as finished before it moves.
- Likely cause: completion follows the acceleration direction instead of the absolute distance threshold defined by Flutter.
- Fix direction: compare `position.abs()` with the nonnegative end threshold for either acceleration sign.
- Confidence: high

### physics-003 — Bounded friction changes velocity and omits the bound-distance tolerance

- Severity: P2
- Category: behavior
- Flutter: `packages/flutter/lib/src/physics/friction_simulation.dart:166-195` clamps `x`, inherits the unbounded friction `dx`, and finishes when the clamped position is within `tolerance.distance` of either bound.
- Incular: `crates/incular-animation/src/physics.rs:292-307` zeroes velocity once raw position crosses a bound and only finishes on crossing or friction velocity tolerance.
- Evidence: `physics__bounded_friction__position_and_velocity` first diverges at frame 3 / 48 ms: Flutter reports `dx = 90.8356` at clamped `x = 4`; Incular reports `dx = 0`. In `physics__bounded_friction__distance_tolerance`, the start is 3.9995 with max 4 and default distance tolerance 0.001: Flutter reports done at frame 0, Incular does not; at frame 1 / 16 ms Flutter still reports `dx = 96.8468` while Incular reports 0.
- Impact: consumers lose the simulated velocity at a display bound, and a position already within Flutter's settling tolerance continues in Incular.
- Likely cause: Incular treats the bounds as a velocity stop and checks the unclamped position with exact inequalities.
- Fix direction: preserve the inner friction velocity and determine bound completion from the clamped position's distance to each bound using `tolerance.distance`.
- Confidence: high

### physics-004 — Clamped simulations cannot clamp velocity

- Severity: P2
- Category: missing-api
- Flutter: `packages/flutter/lib/src/physics/clamped_simulation.dart:33-40,52-65` accepts `dxMin`/`dxMax` and clamps the reported velocity.
- Incular: `crates/incular-animation/src/physics.rs:355-382` accepts only position bounds and forwards inner velocity unchanged.
- Evidence: source review; `physics__clamped_simulation__position_and_velocity` demonstrates the missing configuration with a diagnostic, non-equivalent setup. At frame 0 / 0 ms the same gravity simulation has raw velocity 200; Flutter configured with `dxMax = 50` reports 50, while Incular has no velocity-bound option and reports 200. This is not a same-configuration frame parity comparison.
- Impact: callers cannot use the Incular wrapper to cap velocity independently from position as Flutter allows.
- Likely cause: Incular's constructor models only position limits.
- Fix direction: add optional velocity bounds and clamp the forwarded velocity to them.
- Confidence: high

### physics-005 — Completion is inclusive at Flutter's strict tolerance boundary

- Severity: P3
- Category: behavior
- Flutter: `friction_simulation.dart:158-160` uses `abs(dx) < tolerance.velocity`; `spring_simulation.dart:259-261` uses strict `nearZero` checks.
- Incular: `crates/incular-animation/src/physics.rs:188-191,267-269` uses `<=` for both distance and velocity checks.
- Evidence: `physics__friction__tolerance_boundary` uses initial speed and velocity tolerance both equal to 0.001; `done` is 0 in Flutter and 1 in Incular at frame 0. `physics__spring__tolerance_boundary` starts exactly one distance tolerance from equilibrium; Flutter reports not done and Incular reports done at frame 0.
- Impact: a simulation exactly on the tolerance edge can stop one evaluation earlier in Incular.
- Likely cause: inclusive comparisons replace Flutter's strict comparisons.
- Fix direction: use strict comparisons for the same boundary cases, including the `nearZero` checks used by springs.
- Confidence: high

### physics-006 — Spring descriptions normalize inputs Flutter preserves

- Severity: P3
- Category: behavior
- Flutter: `packages/flutter/lib/src/physics/spring_simulation.dart:27-44` stores constructor inputs and computes damping ratio directly without clamping.
- Incular: `crates/incular-animation/src/physics.rs:52-71` substitutes defaults for nonpositive mass/stiffness, negative damping, and a negative ratio.
- Evidence: `physics__spring_description__damping_regimes` first diverges at frame 0. A raw description `(mass: 0, stiffness: 0, damping: -1)` remains `(0, 0, -1)` in Flutter and becomes `(1, 100, 10)` in Incular. A damping ratio of -0.25 yields damping -5 / underdamped in Flutter and damping 20 / critically damped in Incular.
- Impact: callers using the same spring parameters can get a different solver regime and motion; invalid or unusual values do not retain their Flutter meaning.
- Likely cause: Incular adds fallback validation absent from Flutter's public constructors.
- Fix direction: preserve constructor parameters and let documented validation or the caller govern invalid physical values.
- Confidence: high

### physics-007 — Duration and bounce spring creation retains sub-millisecond precision

- Severity: P3
- Category: behavior
- Flutter: `packages/flutter/lib/src/physics/spring_simulation.dart:70-80` derives stiffness from `duration.inMilliseconds`.
- Incular: `crates/incular-animation/src/physics.rs:76-91` derives stiffness from `Duration::as_secs_f32()`, retaining microseconds.
- Evidence: `physics__spring_description__duration_bounce` passes 300,500 µs and bounce 0.3. At frame 0 Flutter reports stiffness 438.6491, damping 29.3215, and derived duration 300 ms; Incular reports stiffness 437.1906, damping 29.2727, and the test-derived duration 301 ms.
- Impact: sub-millisecond duration values produce slightly different spring timing and damping.
- Likely cause: the conversion uses fractional seconds rather than Flutter's millisecond truncation.
- Fix direction: convert the duration to whole milliseconds before computing stiffness if exact Flutter parity is intended.
- Confidence: high

### physics-008 — Spring description duration and bounce getters are missing

- Severity: P2
- Category: missing-api
- Flutter: `packages/flutter/lib/src/physics/spring_simulation.dart:119-155` exposes `duration` and `bounce` derived getters.
- Incular: `crates/incular-animation/src/physics.rs:46-105` exposes mass, stiffness, damping, constructors, and spring classification, but no derived getters.
- Evidence: source review. The duration/bounce frame twin computes the Flutter getters and reconstructs those values in test code from Incular's fields because the Incular type has no corresponding methods.
- Impact: callers cannot query the perceptual duration or bounce value of an existing spring description through Incular's public API.
- Likely cause: only physical coefficients were modeled in the Rust type.
- Fix direction: add derived `duration` and `bounce` accessors with Flutter's formulas and rounding.
- Confidence: high

### physics-009 — SpringSimulation lacks the `snapToEnd` option

- Severity: P2
- Category: missing-api
- Flutter: `packages/flutter/lib/src/physics/spring_simulation.dart:215-228,241-255` supports `snapToEnd`, which snaps both position to the endpoint and velocity to zero after completion.
- Incular: `crates/incular-animation/src/physics.rs:119-145,146-191` has no constructor option; `ScrollSpringSimulation` snaps position only at `:217-231`.
- Evidence: source review. The direct frame scenarios use the default `snapToEnd = false`; there is no equivalent Incular `SpringSimulation` configuration to twin Flutter's true case.
- Impact: callers cannot request Flutter's exact endpoint-and-zero-velocity behavior on a general spring simulation.
- Likely cause: endpoint snapping is split into a separate scroll-oriented wrapper.
- Fix direction: add a `snap_to_end` option that snaps both outputs once `is_done` is true.
- Confidence: high

### physics-010 — FrictionSimulation omits Flutter's desktop and endpoint helpers

- Severity: P2
- Category: missing-api
- Flutter: `packages/flutter/lib/src/physics/friction_simulation.dart:40-80,137-155` provides `constantDeceleration`, `FrictionSimulation.through`, `finalX`, and `timeAtX`.
- Incular: `crates/incular-animation/src/physics.rs:236-270` exposes only construction and the `Simulation` methods.
- Evidence: source review. `physics__friction__ordinary` verifies the shared zero-constant-deceleration curve; the additional Flutter constructor and endpoint methods have no Incular counterpart to exercise.
- Impact: callers cannot reproduce Flutter's constant-deceleration desktop friction or query/tune motion through known endpoints using the public Incular animation type.
- Likely cause: Incular implements the basic exponential decay only; similarly named endpoint behavior exists privately inside scroll ballistics.
- Fix direction: expose constant deceleration, a through constructor, final position, and time-at-position on the animation friction type.
- Confidence: high

### physics-011 — Simulation tolerance cannot be read or changed through the trait

- Severity: P2
- Category: missing-api
- Flutter: `packages/flutter/lib/src/physics/simulation.dart:36-56` gives every simulation a default tolerance and a mutable public `tolerance` field.
- Incular: `crates/incular-animation/src/physics.rs:108-115` defines only position, velocity, and completion on `Simulation`.
- Evidence: source review. Incular constructors accept a tolerance only on selected concrete types; generic callers cannot inspect or update it through `dyn Simulation`.
- Impact: code that changes tolerance after construction or handles heterogeneous simulations through the base interface cannot be ported directly.
- Likely cause: the trait contains evaluation methods but omits shared simulation configuration.
- Fix direction: expose tolerance through the simulation abstraction, with a documented mutable or builder-based update path.
- Confidence: high

### physics-012 — nearEqual and nearZero have no public Incular equivalent

- Severity: P3
- Category: missing-api
- Flutter: `packages/flutter/lib/src/physics/utils.dart:10-21` exports both scalar tolerance helpers from `package:flutter/physics.dart`.
- Incular: `crates/incular-animation/src/physics.rs` has no public equivalents; `crates/incular-scroll/src/ballistic.rs:58-60` contains only a private `near_zero` helper.
- Evidence: source review; both inventory entries were marked `UNRESOLVED` before this audit.
- Impact: callers cannot reuse Flutter's null-aware near-equality and near-zero checks through an Incular physics utility.
- Likely cause: the helper was kept local to scroll spring completion rather than exposed as a shared physics utility.
- Fix direction: add public helpers in the animation physics module if parity with Flutter's exported utility surface is required.
- Confidence: high

## Pixel scene proposals

None. The differences are scalar simulation outputs and completion decisions; frame value sampling is sufficient.

## Harness requests

None. The current recorder captures every physics output needed for this package.

## Open questions

- Whether Incular intends to preserve Flutter's permissive behavior for physically invalid spring inputs or deliberately sanitize them.
- Whether the partial `FrictionSimulation` and `Simulation` APIs are intentional Rust design choices or parity gaps to fill.

## Revalidation and validation (HEAD a19bf487)

- Revalidated all 12 findings against HEAD. `git diff a425aa2e HEAD -- crates/incular-animation` is empty and `crates/incular-scroll/src/ballistic.rs` and `physics.rs` are unchanged, so the cited lines are current. physics-001 through physics-012 reproduce with the same first divergences.
- Added four scenarios: `physics__ballistic_dispatch__clamping_overscroll`, `physics__ballistic_dispatch__bouncing_overscroll_at_rest`, `physics__ballistic_dispatch__clamping_negative_velocity`, `physics__bouncing_scroll__transfer_clamp`. They use Flutter `createBallisticSimulation` with `FixedScrollMetrics` (DPR 1) on the Flutter side and `ScrollPhysics::ballistic_simulation` on the Incular side. All four match.
- `harness/run-frames.sh physics__`: Flutter tests pass (18 scenarios); Incular report: 8 match, 10 diverge, the same ten as before.
- Coverage rows in `audits/flutter-parity/coverage/physics.jsonl` now carry concrete statuses; no row is `unchecked`.
- Findings JSONL normalized to the shared schema; all 12 are `accepted` (listed in SUMMARY.md).
