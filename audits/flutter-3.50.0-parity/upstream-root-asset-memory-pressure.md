# Root asset cache memory pressure

Pinned original `services/binding_test.dart:101` pairs. The native
test drives the same `flutter/system` JSON memory-pressure
notification and observes two cached loads, two refetches after
the warning and no further backend reads once the cache is warm
again. A different channel, unknown message and malformed JSON
leave the caches alone.

The root bundle is now shared between the host service and
unscoped `DefaultAssetBundle` lookups. A host may install its
packaged bundle for the UI thread before mounting. The default is
still empty. A real mounted subtree confirms local bundle
overrides do not get cleared by a root warning. Structured root
assets are also invalidated. The native window host forwards
Winit's memory warning to the same application code.

Three focused runtime and five notices tests passed, along with
a Linux desktop host library check. Tests do not simulate an OS
memory warning on a mobile device or clear unrelated GPU caches.

```sh
cargo test --locked -j 1 -p incular-runtime --test flutter350_root_asset_pressure -p incular-assets --test flutter350_license_notices -- --test-threads=1
cargo check --locked -j 1 -p incular-desktop --no-default-features --lib
bash audits/flutter-3.50.0-parity/run-reference-linux.sh upstream services/binding_test.dart 'didHaveMemoryPressure clears asset caches'
```

## In-flight reads during invalidation

A retained asset can finish loading after a cache clear or eviction.
Previously it would repopulate Incular's cache. Reads and parses
now capture an invalidation generation: the original caller still
receives its result, but an old generation cannot insert that
result. Cache lookup and insertion stay short and no storage or
parser call happens under the lock. A single-key eviction leaves
already-cached unrelated entries alone and conservatively skips
any insertion that was already in flight.

Deterministic native races cover text and structured results after
both clear and eviction. The authored pinned Flutter test confirms
the same safe behavior for string futures, but Flutter currently
does re-insert a late parsed structured result. Incular deliberately
does not copy that stale-cache behavior. Ten focused native cases
and four authored Flutter checks passed. The authored checks do not
change the original-case count.
