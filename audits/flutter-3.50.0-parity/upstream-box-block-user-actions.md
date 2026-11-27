# Action blocking and merge compatibility in ordinary semantics

Verified three original `widgets/semantics_test.dart` registrations (:1905,
:1947, :1980) on pinned Flutter 3.50.0. The mounted Incular
`flutter350_block_user_actions` fixtures check the same labeled children
and compare both the exposed action lists and real dispatch:

- A blocking parent with explicit children preserves both child labels
  and hides both tap actions.
- A blocked tap in one child and an unblocked long-press in another are
  compatible; the owner exposes the joined label and the long-press only.
  Before the fix Incular published two independent child nodes.
- Two declared tap handlers conflict even if one is blocked, so the
  children remain separate and only the unblocked handler can run.

The fix shares the existing compatible-label/action rules for slivers with
ordinary box containers; only compatible simple annotation fragments absorb.
The authored Flutter and Incular controls switch from merge to conflict and
back on a mounted tree, retaining the owner ID and rejecting retired child
actions. Focused sliver, pointer blocker, binding, and retained semantics
regressions passed. Accessibility action blocking is not a pointer hit-test
or keyboard-focus blocker.
