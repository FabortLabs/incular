# incular-core

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Geometry, identity, input, interpolation and lower-level context values. |
| API class | application; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Available; shared dependency sources and scoped subscriptions. |

Platform-independent foundation types for Incular. This crate is intentionally
free of windowing and renderer dependencies.

Owns dependency-free geometry, colors, normalized input events, phase dirty
flags, and the generational `Arena`. It does not own tree semantics or layout
policy. Arena IDs are stale after removal, even if the slot is reused.

Wheel input uses `InputEvent::ScrollAt` when the platform reports a position
with the delta. Both values are in logical pixels.

## Opt-in restoration boundary

`RestorationKey`, `RestorationScope`, and `RestorationBackend` form the small
renderer-independent boundary for persisted declarative state. The runtime owns
the scope hierarchy, duplicate-ID checks, snapshot format, diagnostics, and
asynchronous storage. Lower-level crates can read, replace, or remove raw
`serde_json::Value` values through a supplied scope without depending on the
runtime or serializing framework objects.

A key is one stable path segment. Paths remain structured key vectors, so
arbitrary key text is never ambiguously concatenated into a storage path.

### Dependency observer failures

In an unwinding build, `DependencySource::notify` calls remaining live
observers before resuming the first panic. With
`notify_collecting_panics`, callers receive all original payloads in
listener order and decide how to report them.

### Foundation values

`ObserverList` iterates every registration in order. `HashedObserverList`
iterates distinct values in first-registration order while counting
duplicates until each registration has been removed. Both are passive
collections, separate from reactive dependency tracking.

`clamp_double(value, lower, upper)` matches Flutter's inclusive numeric
clamp, including mapping NaN to the upper bound. Use `try_clamp_double`
when the interval might have unordered or NaN bounds.

`CachingIterable` consumes its source lazily and caches values for independent
cursors. Mapping, filtering, expansion, take and skip return derived caches
that share the source. Rust cursors clone values as they yield them.
