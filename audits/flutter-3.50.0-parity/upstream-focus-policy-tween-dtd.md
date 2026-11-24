# Focus policy, weighted tweens, and preview DTD

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular `c647cb85`.

- Four `focus_scope_test.dart` originals passed. Their Rust
  `flutter_focus_policy_upstream_parity` equivalents are partial,
  because the exact widget callback or scope-primary hooks are not
  available. Disabling a retained focused node clears host focus and
  observed state; excluding a subtree does not autofocus its sibling.
  A focused descendant does not make Incular's ancestor
  `has_focus()` true, and descendants that are blocked by ancestor
  policy can claim local focus even though the runtime target remains
  absent. Those are verified state gaps, not evidence of misrouted
  keyboard events in these cases.
- The three `animation/animations_test.dart` TweenSequence
  registrations match weighted joins, segment-local intervals, and
  single-segment endpoints. Rust target
  `flutter350_upstream_tween_sequences`. Separate malformed-input
  controls show Flutter debug asserts while Rust ignores invalid
  weights and clamps raw progress. Do not count that policy as an
  original test failure.
- Three original unbounded BoxConstraints checks pass on Flutter but
  construct infinite **minimums**, not representable in Incular.
  Reference-only. A separate pair with finite minima sees matching
  interpolation when both maxima are infinite, but Flutter debug
  rejects finite/infinite mixed maxima and Incular accepts both
  directions. See `flutter350_constraints_infinite_lerp` and
  `box_constraints_infinite_lerp_reference_test.dart`; no release
  exception claim.
- Four original preview DTD registrations pass on pinned Flutter
  offline (concurrent preview requests, wait for initialized event,
  already-ready LSP, and cancellation on failed subscription).
  Incular has no analogous public tool service. This exercised
  hermetic fake DTD and memory files only, not an actual daemon or
  network; four reference-only results.
