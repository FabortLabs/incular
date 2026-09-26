# Store each text cache key once

Status: first implementation completed and measured. See [the result](04-text-cache-key-results.md). Borrowed-hit lookup remains a separate proposed step; 08 still depends on the final key representation.

## Finding and scope

`crates/incular-text/src/engine.rs` stores a HashMap of LayoutKey to Arc<TextLayout>
and a VecDeque of LayoutKey. `store` clones the key, including owned text, into the
queue. `LayoutKey::new` also builds an owned key before checking a cache hit.
Current eviction is FIFO; do not describe or silently change it to LRU.

## Implementation sequence

1. Measure key size, duplicated text/family bytes, both container capacities,
   occupancy and hit-path allocations. Include short labels and long paragraphs.
2. Prototype shared ownership of one key and compare against stable entry IDs in
   the queue. Include reference-count headers and any entry-index/table capacity.
3. Choose one authoritative key per entry. Preserve complete equality over text,
   font/fallback options, generation, width, wrapping and overflow. Hashes alone
   are never identities; collisions must still resolve by equality.
4. Preserve ordering during insertion/eviction. Handle duplicates, reset and failed
   lookup without stale IDs, orphan queue entries or unbounded bookkeeping.
5. Release all cache key ownership on eviction; external Arc<TextLayout> owners
   remain valid. Keep key lifetime separate from live layout lifetime.
6. Optimize borrowed lookup as a separate step if hit allocations matter. Prefer
   safe supported map APIs; justify new dependencies against bundle/architecture.

## Evidence, validation and acceptance

Measure total key/container bytes and allocation count per hit/miss. A shared-key
allocation can cost more for tiny entries, so use workload-weighted totals.

Extend `crates/incular-text/tests/engine.rs` for hits, all key distinctions, capacity
rollover, repeated insertion and font-generation reset. Test collisions if custom
hash storage is introduced, and retained layouts after eviction.

Accept less retained key memory with identical cache behavior and no material
lookup slowdown. Reject hash-only equality or memory merely moved into bookkeeping.
Deliver ownership comparison, isolated patch, targeted results and protocol-00
measurements. Roll back independently of byte-budget changes.
