# Specialize retained element state

Status: scrolling state implemented and measured in [the first result](01-scrolling-sidecar-results.md). Notification, editing and layout-builder state remain proposed. Depends on [00](00-measurement.md); coordinate with 02/03.

## Finding and ownership

`Element` in `crates/incular-widgets/src/tree.rs` embeds sliver identity vectors,
overlay IDs, advanced-child keys, notification/editing subscriptions, revision
counters and layout-builder state. Ordinary nodes pay for unused inline fields.
Inspect all field accesses in `crates/incular-widgets/src/tree/` before changing it.

## Implementation

1. Group fields by coherent lifetime: sliver/advanced scrolling, notifications,
   editing, layout builder. Count live owners of each group in the fixed tracker
   and representative editing/scrolling workloads.
2. Measure Element size/alignment, allocated arena slots and deep heap bytes.
3. Prototype private records behind `Option<Box<State>>`; compare a keyed side
   arena only if its lookup/capacity overhead can be justified. Keep common identity,
   parent/children, widget/render IDs and invalidation in the common record.
4. Do not put independent behaviors into mutually exclusive variants: some nodes
   need multiple state groups. Allocate each only when needed.
5. Centralize initialization/access/teardown. Preserve same-kind state during
   reconciliation; release it on incompatible kind transitions and unmount.
6. Preserve RAII subscriptions and callback ordering. Keep `build_context` and
   `render_context` distinct; their separate dependency ownership is intentional.
7. Migrate one family at a time, starting with least frequently populated fields.

## Measurement

Compare old/new element and arena-capacity bytes, plus all specialized records,
allocation rounding and side-table overhead. Include peak overlap during rebuild.
Report common-node and specialized-node costs separately; no MB saving is assumed.

## Validation and acceptance

Select relevant cases from `tree_reconciliation`, `tree_scrolling_slivers`,
`advanced_scrolling_retained_integration`, `editable_text_interaction` and
`inherited_context_contracts`. Add focused kind-transition, unmount/subscription
and reentrant-callback regressions under widget tests.

Accept lower net retained memory without identity changes, dropped subscriptions,
new idle work or material reconciliation/layout slowdown. All behaviors remain
available. Deliver field census, ownership diagram, independently revertible patch,
targeted results and paired measurements under protocol 00.
