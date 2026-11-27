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
not claimed. The compact iterative export and the two Element-backed cases
are separate verification work.
