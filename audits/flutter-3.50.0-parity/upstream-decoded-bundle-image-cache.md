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

## Pending stream identity and membership

Two exact pinned registrations now pair (:211 and :311 in
`painting/image_cache_test.dart`). A pending exact bundle key is
already in the cache; a second resolution returns the same live
stream and issues no second provider read. When its only decode
finishes, both callers see the same frame and the cache no longer
counts a pending entry.

The native UI memo is weak and keyed by the actual pending
job, not only by filename. Clearing the cache creates a new
stream for the same key; old callers still receive their own
result, which cannot satisfy or replace the new pending job.
ExactAssetImage uses this path and keeps its decoded-key label.
Equivalent sharing across different UI threads, the completed
live-listener registry, and Flutter's separate clearLiveImages
policy are not claimed. All 44 focused native tests passed.

## Live listener ownership, eviction and LRU re-admission

Thirteen exact pinned registrations passed: :232, :344, :381,
:398, :415, :441, :455, :471, :488, :504, :525, :561, and :691.

The native cache now distinguishes a pending decode, a retained
decoded frame, and an actively observed stream. An observer keeps
its stream alive after the first frame, without making a decoded
cache entry or an expired stream inherently live. A pending key is
live while cache-owned work is tracked. Clearing it removes only
that ownership. Explicit clearLiveImages drops the UI index without
canceling consumers, and releasing an old observer cannot erase
tracking for a replacement at the same key.

Default eviction removes all three applicable states. Passing
includeLive=false preserves only an externally observed stream;
a subsequent lookup reuses that stream without repeating the I/O.
A live image can lose the only retained LRU slot and stay live.
Re-admitting a live 8 × 8 image restores exactly 256 decoded bytes.
Six unobserved decodes leave zero live images with enabled or
disabled retention. These comparisons use exact bundle keys on
one UI thread; cross-thread stream identity is not claimed. All
65 focused native cache, listener, resize, and frame tests passed.
