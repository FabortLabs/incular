# Image key failures and cache-only provider queries

Reference: Flutter 3.50.0 at `ab2912035e160d9cad6306df307576da6128ebdb`.

`AssetImage.obtain_cache_status` resolves the bundle manifest, then reads
the exact selected key from the calling UI thread's cache without
starting pixel IO or touching its eviction order. `ExactAssetImage` can
check its already known key synchronously. The tests exercise a held
load, its pending/live flags, a completed scaled 8x8 image, listener
release, and the same physical asset name at a different scale. A cold
status lookup reads only the manifest.

A malformed manifest returns no frame and reaches the awaiting native
cache query and the stream's error listener without asking for pixels.
Flutter's original stream-listener test (34) matches that behavior.
The cache-location original (54) stays partial: Flutter reports through
a global error hook and completes with null; native callers receive a
typed error from the status query instead.

Both pinned originals, two authored pinned Flutter probes, and all 38
native bundle-image cache tests passed.
