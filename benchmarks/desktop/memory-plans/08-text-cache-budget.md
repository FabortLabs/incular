# Bound text caching by bytes

Status: proposed. Finalize after 04; revise accounting after 05/06/07.

## Finding and scope

TextEngine in `crates/incular-text/src/engine.rs` caps the cache at 2,048 entries.
Entries can have very different deep sizes. Eviction drops a cache reference;
widgets may still retain the layout. Inspect text diagnostics and runtime profiling
snapshots without moving cache ownership into runtime or DevTools.

## Implementation sequence

1. Define cache accounting scope and distinguish cache references/estimated bytes
   from globally live layout bytes. Arc strong counts cannot establish exact
   ownership or unique memory totals.
2. Account for keys, lines/runs, glyphs, carets/clusters and diagnostic payloads.
   Deduplicate backing allocations within the scope; never charge an entire
   shared font blob to every layout. Document approximation explicitly.
3. Add byte and entry limits at the text owner, with defaults justified by actual
   workloads. Permit oversized layouts to be returned without mandatory admission.
4. Preserve FIFO initially; changing eviction policy is a separate measured change.
   Evict until both limits hold and keep bookkeeping bounded.
5. Update accounting consistently on insert, replacement, eviction and generation
   reset. External layouts remain valid and retain resources until their owners drop.
6. Avoid shape/evict loops using sensible admission and budgets. Native memory
   pressure integration, if needed, must use existing platform/runtime boundaries
   while text remains portable; implement pressure policy separately.

## Evidence, validation and acceptance

Measure cache estimates, actual heap, hit/miss rate, shaping time and peak/idle
memory. Exercise alternating large documents, unique labels, scrolling and multiple
windows; an idle-only benchmark cannot reveal cache thrashing.

Add targeted tests for zero/tiny budgets, oversized entries, replacement, ordering,
generation reset, external owners and bounded bookkeeping. Verify accounting after
release, then run relevant existing engine/cache regressions.

Accept bounded growth and lower stressed-workload memory without material repeated
shaping cost. Report evicted bytes separately from process bytes actually released.
Deliver accounting specification, chosen-budget evidence, isolated patch and paired
protocol-00 results. Revert policies that trade a small memory win for CPU churn.
