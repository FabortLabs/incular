# Retained semantics owner and diagnostics

Two pinned `semantics/semantics_test.dart` registrations pair.
The name of a framework tag appears in a debug dump even on an
unattached leaf (:964). Multiple tags print in discovery order
within the existing text budget, without appearing in the native
accessibility payload. The owner emits one matching create/dispose
pair (:1106), using the retained tree's owned lifetime rather than
an explicit disposal method. Release trees store no allocation token.

The dump now prints a child's current index in its parent and updates
it after a reorder. This is partial at :1116: Flutter's standalone
SemanticsNode can set its index directly, whereas Incular derives it
from the graph's edge; a root has no index. This also prevents malformed
duplicate edges or cycles from redisplaying a node or exhausting a
valid tree's diagnostic node budget.

The two initial regressions failed before the implementation. The
focused semantics tests passed after the fix (37 distinct cases);
all five selected Flutter originals passed, including two existing
hit-test copy controls.
