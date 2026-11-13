# anim_compose: Composed animations

## Summary

TrainHoppingAnimation defers switching and its callback until a consumer reads the composed animation. Flutter switches immediately in response to an underlying animation notification. The remaining composition APIs need review.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| TrainHoppingAnimation crossing callback | incular_animation::TrainHoppingAnimation | scenario + reading | anim_compose__train_hopping__unobserved_crossing | divergent |
| AlwaysStoppedAnimation generic value | incular_animation::AlwaysStoppedAnimation | reading | none | partial: fixed f32 value and forward status match; non-f32 values unavailable |
| AnimationStyle fields, no-animation constant, merge and interpolation | incular_animation::AnimationStyle | reading | none | source-aligned for values and midpoint choice; widget integration not checked here |
| kAlwaysCompleteAnimation and kAlwaysDismissedAnimation values/status | `always_complete`, `always_dismissed` | reading | none | values and status source-aligned; listener API differences are covered by anim_core-004 |
| ReverseAnimation value/status | incular_animation::ReverseAnimation | reading | none | `1-parent` and all four reversed statuses source-aligned; listener API covered by anim_core-004 |
| CompoundAnimation and AnimationMean/Max/Min value/status | `CompoundAnimation`, `animation_mean/max/min` | reading | none | finite scalar value formulas and status choice align; generic types, edge values and notifications remain unchecked |
| ProxyAnimation parent detach and scalar value/status | incular_animation::ProxyAnimation | reading | none | defaults and detached cached values align in source; listener API covered by anim_core-004 |
| CurvedAnimation direction latch | incular_animation::CurvedAnimation | scenario + reading | anim_compose__curved_animation__unobserved_reversal | divergent when direction changes before the first read; other lifecycle cases unchecked |
| Listener mixins | incular-animation composition APIs | not checked | none | not checked: package review remains incomplete |

## Findings

### anim_compose-001 — Train hopping callback waits for a getter read

- Severity: P0
- Category: lifecycle
- Flutter: `animation/animations.dart:520` subscribes to both train values; `:556` handles their notifications and `:581` invokes `onSwitchedTrain` after a crossing.
- Incular: `crates/incular-animation/src/animations.rs:606` checks for crossing only in `hop`, which is called by the getter paths at `:601` and `:628`.
- Evidence: scenario `anim_compose__train_hopping__unobserved_crossing`, frame 1 (16 ms), `value.switches`: Flutter 1 vs Incular 0; one frame differs, maximum difference 1. Both start with current=0.2, next=0.8 and the same zero callback count; setting next to 0.1 crosses the current train. A subsequent `currentTrain`/`current_train` read makes Incular emit the callback, matching again at frame 2. Independently rerun with `harness/run-frames.sh anim_compose__`.
- Impact: if a consumer depends on the switch callback but does not read the composed animation, that callback never fires and dependent state can remain stale. If it reads later, the callback occurs late and during the getter call.
- Likely cause: the Rust implementation samples the parents lazily instead of subscribing to their changes.
- Fix direction: observe parent value changes and perform the hop and callback when the crossing occurs, respecting callback reentrancy and disposal.
- Confidence: high

### anim_compose-002 — AlwaysStoppedAnimation accepts only scalar values

- Severity: P2
- Category: missing-api
- Flutter: `animation/animations.dart:91-104` defines `AlwaysStoppedAnimation<T>` with a value of arbitrary type `T`.
- Incular: `crates/incular-animation/src/animations.rs:337-345` defines `AlwaysStoppedAnimation(pub f32)` and returns `f32` through `AnimationSource`.
- Evidence: source comparison. `crates/incular-animation/tests/animation_graph.rs:63-65` checks the scalar value and forward status; it does not exercise a non-scalar value.
- Impact: a Flutter caller can supply a fixed color, size, or other object as an animation value; the Incular type cannot represent that animation directly.
- Likely cause: the composition source abstraction is specialized to `f32`.
- Fix direction: provide a typed fixed animation and compatible typed animation interface for non-scalar values.
- Confidence: high

### anim_compose-003 — CurvedAnimation can latch the wrong curve before its first running read

- Severity: P1
- Category: behavior
- Flutter: `animation/animations.dart:380-384,420-429` subscribes to the parent's status at construction and latches the first running direction until an endpoint status clears it.
- Incular: `crates/incular-animation/src/animations.rs:221-260,263-278` updates its direction lazily when its value is read, using rest counts to notice endpoint transitions.
- Evidence: `anim_compose__curved_animation__unobserved_reversal`. Frame 0 matches at 0. Both controllers then start forward, set their value to 0.5, and reverse without reading the curved value. At frame 1 (16 ms) Flutter reports 0.31640625 using ease-in and Incular reports 0.68359375 using ease-out; one frame differs, maximum difference 0.3671875. The filtered suite was independently run. The existing `crates/incular-animation/tests/flutter_animation_parity.rs:213` covers a direction latch with sampled values but not this unobserved sequence.
- Impact: the next observed animated value can jump to the reverse curve after a forward run is reversed before the curved value was sampled, instead of remaining on the forward curve.
- Likely cause: the lazy update observes only the current reverse status and missed the earlier forward status event; no endpoint was reached between them.
- Fix direction: track status changes when they occur, including unobserved direction changes, and preserve the initial direction until the parent reaches an endpoint.
- Confidence: high

## Revalidation and validation (HEAD a19bf487)

- Revalidated anim_compose-001, -002 and -003 against HEAD. `crates/incular-animation/src/animations.rs` is unchanged since a425aa2e, so the cited lines still apply. Both scenarios reproduce: `anim_compose__train_hopping__unobserved_crossing` (frame 1, switches 1 vs 0) and `anim_compose__curved_animation__unobserved_reversal` (frame 1, 0.3164 vs 0.6836).
- The coverage file `audits/flutter-parity/coverage/anim_compose.jsonl` has concrete statuses for all 13 rows. AnimationWithParentMixin is MISSING (no counterpart; parent subscription is inside ProxyAnimation, ReverseAnimation and CurvedAnimation).
- `harness/run-frames.sh anim_compose__`: Flutter twins pass; Incular report 0 of 2 match, both as expected.
## Pixel scene proposals

None.

## Harness requests

None.

## Open questions

Other train-hop ordering, disposal, and composed animation behaviors remain unchecked.
