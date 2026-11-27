# Foundation diagnostics tree JSON

Baseline: Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
Source: `packages/flutter/lib/src/foundation/diagnostics.dart`, notably
`DiagnosticsNode.toJsonMap`, `toJsonList`, `DiagnosticsProperty.toJsonMap`,
and the default serialization delegate. Original fixture:
`packages/flutter/test/foundation/diagnostics_json_test.dart`.

`incular-core::DiagnosticsNode` serializes heterogeneous, ordered snapshots
using the existing typed scalar property wire data. With default options only
the node details are sent. A positive depth emits children, including an
explicit empty list for reached leaves above the depth boundary. Opting in to
properties includes hidden and null entries: visibility in a diagnostic
string is not a filter on structured JSON. Additional keys and independent
per-node options apply to both relations. Filtering can replace properties or
promote children and preserves returned order. Truncation happens after
filtering and appends a marked sentinel without changing the real nodes.

The paired native target `flutter350_diagnostic_tree` mirrors the original
tree and checks exact key/relationship presence, order, and sentinel behavior.
All 12 original detailed serialization cases passed on the pinned Linux
reference, as did 12 native cases and 10 adjacent scalar diagnostic tests.
The property adapters represent snapshots, not Flutter's lazy diagnostic
builder; expansion of live object values and full inspector integration are
not claimed.

`DiagnosticsNode::to_json_iterative` implements the separate compact export.
It walks breadth first with no recursive serialization, returns the essential
fields, never emits properties, and treats positive depth as enabling all
descendants. It omits leaf child arrays and sets the truncation flag on the
owner of a truncated list, not its sentinel. The pinned compact original and
15 native diagnostics tests passed, including delegate ordering and the
quirk that depth is not decremented in this mode.

The live widget-tree adapter reads retained elements, rather than guessing
a widget's type from the paint/layout object that it produced. In particular
the Placeholder keeps its own identity while using the same limited-box
lowering for size and geometry. The public StatefulBuilder owns retained
local state before its first layout; a plain builder does not. A stale
element ID gives no snapshot, and children are returned in retained order.
Both pinned Element originals passed, as did three native identity/lifecycle
tests and seven native Placeholder size, paint, and pointer regressions.
This is not an exhaustive per-widget audit of Flutter's StatefulWidget
hierarchy or a change to the existing DevTools transport.
