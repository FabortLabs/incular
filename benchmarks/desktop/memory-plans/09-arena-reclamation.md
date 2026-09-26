# Reclaim vacant tree storage safely

Status: proposed, higher-risk core work after 00 and simpler ownership wins.

## Finding and scope

`crates/incular-core/src/arena.rs` uses Vec<Slot<T>>, a free-index vector and
generation-tagged IDs. Removal releases T but retains slot storage/capacity. This
can preserve the footprint of a historical large tree. Inspect all core arena
consumers, not only widget/render trees, before changing the shared type.

## Implementation sequence

1. Measure live/allocated slots, vector capacity, free entries and retained bytes
   after building a large tree then returning to a small one. Check existing APIs
   for any reclamation already present.
2. Compare modest reservation changes, a stable identity directory with dense
   payload storage, and reclaimable payload chunks with persistent generations.
3. Preserve stale-ID rejection. Never truncate generation history and recreate
   a removed slot with generation zero; that can resurrect an old ID.
4. For dense payload storage, maintain ID-to-index mappings and repair them after
   swap removal. For chunks, track occupancy and free only empty payload chunks
   while preserving identity tombstones/generation information.
5. Preserve checked index bounds and generation-overflow behavior. Do not narrow
   IDs to u16, wrap generations, or reclaim metadata needed for identity safety.
6. Reclaim at justified lifecycle/pressure thresholds, not on every frame/removal.
   Keep enough capacity to avoid oscillation in ordinary navigation.
7. Preserve promised iteration behavior and borrow safety. Assess the effect of
   mapping indirection on every arena lookup before accepting the representation.

## Evidence, validation and acceptance

Measure slot/payload storage plus persistent generation/mapping overhead. Report
large-to-small recovery separately from small steady state and measure insert,
lookup, iteration and removal throughput. Remeasure after node compaction 01/03.

Add focused stale-ID, reuse, generation-boundary, empty-chunk and live-ID cases
under core tests. Exercise mixed operations against a reference model. Run affected
tree reconciliation and navigation lifecycle tests after integration.

Accept actual reclaimable payload reduction without identity changes or material
steady-state regressions. Deliver representation/invariant design, churn evidence,
isolated implementation and protocol-00 results. Revert if safety requires keeping
enough extra metadata to erase the benefit.
