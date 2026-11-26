# Decoded bundle-image retention and memory pressure

The bundled-image cache keys actual retained resources by bundle identity,
physical path, and scale. Independent widgets resolving the same file no
longer reread or decode it. Count and decoded-byte limits evict by least
recent use; hits promote, exact-fit images are admitted, and an oversized
decode cannot evict other work. A zero budget disables retention.
Different bundles and scales stay separate even when the file bytes are
identical. This leaves Incular's application-owned byte-payload cache intact.

The exact Flutter reference controls cover the LRU sequence, full and
single-key eviction, oversized images, completed-key visibility, pending
eviction, and both count and byte reconfiguration. Two other exact
reference controls are partial: pending-key visibility and coalescing of
requests agree, but the Rust callback path shares a physical decode and
result rather than exposing Flutter's original ImageStreamCompleter
object. Flutter's separate live-listener index and animated frame
completers are not represented by this cache.

The runtime's real `flutter/system` memory pressure message now drops
decoded cache ownership as well as the root asset cache. The held image
in the native check stays readable and a subsequent cache miss repopulates
the new cache. A pending request detached by clear/evict still resolves
for its existing waiters, but cannot overwrite a newer cache entry.
No widget state is mutated from a loader thread.

```sh
cargo test --locked -j 1 -p incular-image --test flutter350_bundle_image_cache --test flutter350_asset_image_keys -p incular-runtime --test flutter350_root_asset_pressure -p incular-widgets --test flutter350_async_bundled_image -- --test-threads=1
```
