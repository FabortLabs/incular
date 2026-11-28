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

## Ordinary callback lifetime

Exact `widgets/semantics_test.dart:706,1552` also passed on both sides.
The box-only `flutter350_box_semantic_handlers` fixture swaps the active
tap closure on the same node and receives no new accessibility projection
or semantics revision; dispatch immediately calls the replacement. Adding
or removing long-press changes the projection while retaining node ID and
bounds. The second fixture verifies the complete original add/remove
sequence, including the three clipboard actions, the selection payload and
both accessibility focus notifications. No stale disabled handler can run.

## IgnorePointer and AbsorbPointer over a sibling

Pinned `widgets/basic_test.dart:1414,1512,1537,1611,1635` passed.
The exact mouse sequence moves from (200,200) to (50,50), presses and
releases, toggles the pointer wrapper without moving the device,
presses and releases again, then restores the wrapper. IgnorePointer
passes through to the overlapping sibling. AbsorbPointer blocks that
sibling but not its listener ancestor. The enter, exit and down
callbacks match Flutter in order.

Before the fix, an ordinary Listener erased the sibling-blocking
decision of the opaque MouseRegion it wrapped. A stationary pointer
then left both overlapping regions hovered or missed the expected
exit on the covered sibling. Raw input proxies now preserve the
child's existing hit decision. MouseRegion's deliberate non-opaque
override still allows hits through. The semantic checks additionally
verify three stable nested nodes across four toggle states and a
button retaining label, role, focusability and enabled state while
its tap and accessibility focus actions are suppressed. An actual
dispatch verifies no blocked tap invokes the callback.

All 42 native checks in the pointer blocker, mouse hit policy, mouse
tracker and pointer input targets passed.
