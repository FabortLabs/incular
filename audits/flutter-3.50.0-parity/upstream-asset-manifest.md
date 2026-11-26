# Packaged asset manifests

The three pinned `services/asset_manifest_test.dart` registrations at
lines 56, 80 and 85 paired on the Flutter VM and Incular native: logical
keys (not resolution paths), exact variant metadata, an absent key,
and the content-hashed primary variant including a path with spaces.
The Rust tests consume bytes emitted by the pinned Flutter codec, not
data re-encoded by the Rust parser under test.

The authored Flutter probe additionally checks lazy conversion and
query order, empty/unlisted/null/malformed rows, exact lowercase
eight-hex hashes, source-map extension exceptions, competing hashes,
Unicode, overlapping cached loads and a failed load's retry. Incular
shares successful materialization across handles, uses one async byte
read for concurrent requests, and calls completion outside locks.
Evict/clear detach an old in-flight request without cancelling its
waiters or allowing it to refill a newer cache. That generation guard is
intentional (Flutter still permits late structured values to refill).

The web JSON/base64 envelope and asynchronous bundle path are exercised
with the same wire assets in the native Rust tests; the assets crate
compiles for `wasm32-unknown-unknown`. This is not a live browser result.
This batch covers manifest parsing/metadata, not image-provider
selection, image decode, or GPU sampling.

```sh
cargo test --locked -j 1 -p incular-assets --test flutter350_asset_manifest --test flutter350_asset_cache_generation --test network_bundle -- --test-threads=1
cargo check --locked -j 1 -p incular-assets --target wasm32-unknown-unknown
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe asset_manifest_behavior_test.dart
```
