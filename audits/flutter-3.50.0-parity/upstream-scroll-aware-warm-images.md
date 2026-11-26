# Scroll-aware images with pending and warm cache entries

Eight pinned registrations compare reactivity under no scrollable,
idle, slow, and fast motion, disposal of a deferred source, and
provider equality by both context and underlying provider. The native
tests check that immediate cases start I/O even though their first frame
is pending, pending work enters the image cache before completion, cold
fast-scroll requests wait, and a disposed cold request never starts.

An authored Flutter VM control and the native warm-cache test both
show that repeated warm requests do no I/O, even during fast scroll
and after the former context is disposed. They still promote the
resource in the LRU so a different entry is evicted. A cold request
that becomes warm can attach on the next scheduled frame without
interrupting scrolling. An already-pending cache entry joins the
original bytes request instead of starting another one.

Pinned registrations 435 and 496 remain partial. The relative timing,
absence of a second load and LRU behavior are represented, but Flutter's
direct stream-completer assignment and identity assertions are not the
same surface as Incular's shared decoded handle and callback-backed
stream. A non-caching asset bundle also cannot shortcut a resolution
that would need to read an unretained manifest.

```sh
cargo test --locked -j 1 -p incular-widgets --test scroll_aware_image --test flutter350_async_bundled_image -p incular-image --test flutter350_resize_image --test flutter350_bundle_image_cache -- --test-threads=1
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe scroll_warm_image_test.dart
```
