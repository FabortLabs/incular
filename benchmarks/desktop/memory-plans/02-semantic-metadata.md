# Make default widget metadata cheap

Status: optional metadata implemented and measured. See [the result](02-semantic-metadata-results.md). The occupancy census and further representation choices remain open.

## Finding and ownership

`WidgetNode` in `crates/incular-widgets/src/tree/widget/mod.rs` embeds
`SemanticProperties`. In `tree/specs.rs` these contain strings, inline optional
`ExplicitSemantics`, focus/editing options and five optional Rc callbacks.
Measure default/custom occupancy rather than assuming every node uses defaults.

## Implementation

1. Count unique descriptors, populated fields/callbacks, explicit semantics and
   owned string bytes. Separate metadata from the derived semantic tree.
2. Prototype absent metadata meaning immutable defaults, with optional Box or Rc
   storage for customized properties. Compare deep bytes and descriptor clone cost.
3. Centralize default reads and first-write allocation. If sharing populated
   metadata, use copy-on-write so changing a cloned descriptor cannot mutate a
   mounted sibling. Avoid allocating defaults on read.
4. Consider a separate callback block only if measurements justify another
   allocation; do not create several tiny allocations for common custom metadata.
5. Preserve absent versus explicitly empty labels, focus flags, undo limits, input
   hints and callback identity/equality. Keep the public ExplicitSemantics builder
   contract unchanged where possible.
6. Audit all semantic traversal and builder call sites before removing inline
   fields. Accessibility remains fully functional in every build that supports it.

## Measurement

Compare unique WidgetNode bytes and populated metadata allocations, including Rc
headers and allocator bins. Measure default-heavy and heavily annotated workloads
separately. Shared Widget handles must not multiply the descriptor count.

## Validation and acceptance

Use relevant `semantics`, `semantic_roots` and `tree_editing_semantics_gestures`
tests. Add cloned-descriptor isolation, explicit-empty/default behavior and callback
lifetime cases. Compare semantic trees and action routing, including focused input;
unchanged screenshots alone do not establish accessibility correctness.

Accept a net reduction with identical default/custom behavior and no material
annotated-widget regression. Reject disabled/deferred accessibility as a saving.
Deliver occupancy census, storage choice rationale, focused validation and paired
measurements. Revert independently if ownership or behavior changes.
