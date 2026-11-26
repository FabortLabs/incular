# Structured binary asset caching

Six pinned `services/asset_bundle_test.dart` controls pair:
:82, :116, :147, :189, :210 and :277. The :116 and :147 rows
replace their earlier partial comparisons.

Binary structured loads hand the byte slice to the parser
without a UTF-8 round trip. The test decodes the actual
StandardMessageCodec bytes for the Flutter manifest
`{"one":[]}` using Incular's platform codec. A warm binary
read returns the existing value synchronously and never
calls a replacement parser. Parser failures report the
source asset and can be retried.

Cached text, parsed text and parsed binary are independent,
even for the same key. A complete clear removes all three;
targeted eviction invalidates only the selected key. The
binary path also obeys the in-flight generation guard added
for memory pressure, so old results cannot refill a cleared
cache. The erased asset trait has a binary parser helper as
well; like the older erased text helper it does not cache
typed parse results. The concrete caching bundle supplies the
typed cache for Rust consumers that want it.

Sixteen focused native checks passed across binary assets,
controlled invalidation, root pressure and packaged notices.

```sh
cargo test --locked -j 1 -p incular-runtime --test flutter350_structured_binary_assets --test flutter350_root_asset_pressure -p incular-assets --test flutter350_asset_cache_generation --test flutter350_license_notices -- --test-threads=1
```
