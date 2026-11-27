# Foundation diagnostic messages and presence summaries

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
`packages/flutter/lib/src/foundation/diagnostics.dart`:
`DiagnosticsNode.message`, `MessageProperty`, and `FlagsSummary`.
The corresponding originals in `foundation/diagnostics_test.dart` are
`iterable flags property test` and `message test`.

Presence summaries preserve insertion order and retain the exact optional
values without invoking callbacks or serializing their payloads. An empty map
omits the wire list; a nonempty map of all absent entries emits an empty list.
No present entries means hidden unless an explicit empty label was provided.
Plain and named messages share text-only rendering and serialize a null value;
only the plain form suppresses the name. Tree truncation now uses the same
plain message path rather than building a parallel JSON payload.

Both pinned originals passed. Three native cases, the 15 detailed/compact
tree tests and 10 adjacent scalar property tests passed in focused runs.
General iterable properties and deep diagnostic text wrapping are not covered
by these two registrations.
