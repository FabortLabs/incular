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
  Those were reference-only at the time. They now pair at the
  connected transport boundary; see
  [preview IDE readiness](upstream-widget-previews-runtime.md#shared-readiness-on-an-existing-ide-connection).

## Mounted Focus callbacks and permissions

Four exact Flutter originals at `focus_scope_test.dart:1233,1254,1294,1562`
now pair with mounted Incular Focus widgets. An owned but disabled
node rejects requests without notifying; disabling an already focused
node reports one loss. A non-focusable parent reports membership when
its child takes focus, and excluding descendants denies both child
requests without reporting a grant. Unlike the earlier partial ports,
these assert the widget callback and the actual owned focus nodes.

Three authored controls pair as well: switching between children does
not cause a false/true notification on the common parent, an explicit
permission can reenable an external node without autofocusing, and
removing a widget detaches its callback before that node is reused.
Sixteen focused Rust tests and all seven pinned Flutter cases passed.
