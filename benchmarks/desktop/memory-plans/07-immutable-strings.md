# Right-size and share immutable text

Status: proposed after census 00. Coordinate ownership changes with 04/06.

## Finding and scope

Widget text, render configuration, labels and example data contain Strings that
may remain immutable. Other strings are edited and need growth. Inspect widget
`tree/specs.rs`, `tree.rs`, text style/editing ownership, and Issue/data/detail
handling in `examples/issue_tracker/main.rs`. Font bytes and many layout arrays
already share storage; do not claim those completed improvements again.

## Implementation sequence

1. Trace text from data model to descriptor, render state, cache key and semantics.
   Count actual copies, capacities, literals and unique shared allocations.
2. Prefer Box<str> for uniquely owned immutable data. Use Rc/Arc only when actual
   sharing and thread contracts justify their metadata; retain growable editing
   buffers. Keep application string builders ergonomic.
3. Convert once at the ownership boundary. Eliminate shared-to-String conversions
   downstream that would recreate copies. Never borrow from a shorter-lived owner.
4. Preserve immutable descriptor semantics and editing through explicit ownership
   or copy-on-write, without introducing shared mutable text or another reactive
   owner. Audit public field compatibility before type changes.
5. Consider bounded interning of repeated project/status/owner values separately
   as an application optimization. Preserve every record, exact string and fixed
   workload; report app-only savings separately from framework savings.
6. Evaluate small-string containers only against measured length distributions,
   including their impact on containing enums and dependency/bundle size.

## Evidence, validation and acceptance

Measure unique text bytes, unused capacity, allocation count and metadata across
all owners. Sharing a short uniquely owned string may increase total memory.

Use affected style/text, reconciliation and editing tests; preserve exact dataset
roundtrip, Unicode and semantic labels. Cover edit commit/cancel, filtering,
descriptor cloning and unmount. Compare the existing five visual states.

Accept net ownership/capacity savings without changing content or editing. Reject
truncation, record removal or compression that repeatedly decodes hot-path text.
Deliver ownership census, API rationale, isolated patch and protocol-00 results;
avoid double-counting text already removed by 04/06.
