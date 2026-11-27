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

## Failed exact asset resolution

Pinned `painting/image_provider_and_image_cache_test.dart:48` passed on
Linux. The native fixture resolves a missing name through `ExactAssetImage`
and observes the exact selected bundle key in the shared cache. The pending
count changes from zero to one, then back to zero before the stream exposes
the original not-found error. The failure leaves no retained entry.
A fresh resolution after correcting the fixture decodes once and is
retained under the same key. All eight focused bundle cache tests passed.

## Retention disabled at request start

The authored pinned `image_cache_retention_enable_test.dart` probe starts a
delayed resolution with either the entry count or the byte limit set to
zero, then enables retention before delivering the image. The listener
still receives the decoded frame, but neither case enters the retained
cache. Incular's blocking bundle path previously admitted the first image
after that transition. It now makes the enabled/disabled decision before
invoking the external loader, like its asynchronous path. A new resolve
after re-enabling the cache can still populate it. The regression failed
before the fix; all nine focused native bundle-cache tests now pass. This
authored comparison does not increase the upstream registration count.

## Per-key isolation during in-flight reads

Two authored Flutter probes verify that evicting a different or absent
key leaves an active decode cached when it completes, while evicting
the active key allows a replacement and does not let the old decode
replace it. These use Flutter's callback completer. The corresponding
native race was in the blocking loader: unrelated eviction invalidated
all in-flight blocking admissions, so the next lookup decoded again.

Incular now tracks blocking reads by exact decode key. Active reads
remain visible to pending queries, and eviction, clear, or a zero limit
detaches the corresponding reads without revoking their callers'
results. Native tests check original identity, decode counts, same-key
replacement after all three invalidation methods, and failed-read
cleanup. Twelve bundle-cache and 18 image-cache policy tests passed.
These authored checks do not increase the original registration count.
