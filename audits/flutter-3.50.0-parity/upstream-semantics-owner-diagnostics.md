# Retained semantics owner and diagnostics

Three pinned `semantics/semantics_test.dart` registrations pair.
The name of a framework tag appears in a debug dump even on an
unattached leaf (:964). Multiple tags print in discovery order
within the existing text budget, without appearing in the native
accessibility payload. The owner emits one matching create/dispose
pair (:1106), using the retained tree's owned lifetime rather than
an explicit disposal method. Release trees store no allocation token.

Pinned :1116 is paired. The native node's own string now includes its
explicit zero-based collection index, even for a detached node with no
graph parent. A retained dump prefers that same authored index. Without
one it falls back to the graph's current edge position and follows
reordering, without revisiting cycles or duplicate edges. The exact
Flutter case and 22 focused Rust semantics tests passed.

The two initial regressions failed before the implementation. The
focused semantics tests passed after the fix (37 distinct cases);
all five selected Flutter originals passed, including two existing
hit-test copy controls.
