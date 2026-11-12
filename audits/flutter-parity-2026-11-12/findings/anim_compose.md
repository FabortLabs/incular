# anim_compose: Composed animations

## Summary

TrainHoppingAnimation defers switching and its callback until a consumer reads the composed animation. Flutter switches immediately in response to an underlying animation notification. The remaining composition APIs need review.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| TrainHoppingAnimation crossing callback | incular_animation::TrainHoppingAnimation | scenario + reading | anim_compose__train_hopping__unobserved_crossing | divergent |
| CurvedAnimation, ReverseAnimation, ProxyAnimation, CompoundAnimation, AnimationMean/Max/Min, AlwaysStoppedAnimation, fixed animations, AnimationStyle and listener mixins | incular-animation composition APIs | not checked | none | not checked: package review remains incomplete |

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

## Pixel scene proposals

None.

## Harness requests

None.

## Open questions

Other train-hop ordering, disposal, and composed animation behaviors remain unchecked.
