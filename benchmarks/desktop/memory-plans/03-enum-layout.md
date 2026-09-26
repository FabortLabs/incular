# Compact large enum payloads

Status: first private render-feature implementation measured in [the result](03-render-feature-results.md). WidgetKind and public RenderKind remain proposed. Depends on census 00; remeasure after 01/02.

## Finding and scope

Inspect `WidgetKind` in widget `tree/specs.rs`, `RenderKind` in `tree.rs`, and
`RenderFeatureState`, `RenderObjectPayload`, `RenderNode` in
`render_object/mod.rs`. Different variants have different payload sizes, but every
instance reserves space for the largest inline payload. Actual dominant variants
and their frequency are not yet measured.

## Implementation sequence

1. Collect enum/payload/container size and alignment plus a variant histogram,
   including None and simple layout nodes. Count allocated slots, not only live ones.
2. Compare current storage against selective boxing and typed side arenas. Include
   padding, allocator bins, indirection, metadata and reserved capacity.
3. Keep small frequent variants inline; box only large uncommon payloads that
   materially increase common size. Remeasure after each change because another
   variant can become the size limit. Do not box every variant indiscriminately.
4. Preserve feature-state reconciliation, controller lifetimes and one behavioral
   owner. Move representation without moving lifecycle responsibilities.
5. Start with private enums. RenderKind is public: classify its API, audit external
   constructors/pattern matches and document migration before changing its shape.
6. Set justified target-specific layout ceilings where useful. Do not assume a
   universal Rust enum layout or use packed/unsafe pointer representations.

## Evidence, validation and acceptance

Estimate common-size reduction times allocated instances, minus boxed payloads,
allocation overhead and capacity. Measure peak rebuild overlap and allocation
count, not only final idle size. Compare rebuild/layout CPU and frame latency.

Run focused reconciliation and tests for every affected feature family, covering
transitions into/out of boxed variants. Include applicable public API/architecture
checks and visual states after input/resize. Place new regressions under tests.

Accept a measured net reduction with unchanged lifecycle/rendering and acceptable
CPU cost. Reject unplanned application API breaks and ownership cycles. Deliver
variant histogram, weighted size comparison, isolated patch and before/after
results using protocol 00; revert rejected variants individually.
