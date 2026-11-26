# Root asset cache memory pressure

Pinned original `services/binding_test.dart:99` pairs. The native
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
