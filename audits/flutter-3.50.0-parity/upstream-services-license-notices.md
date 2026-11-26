# Packaged service license notices

Pinned `services/binding_test.dart:73` passed on the Flutter VM.
The Incular root-bundle collector reads real gzip `NOTICES.Z`
bytes off the native caller thread and supplies both entries to
the foundation registry. It preserves all three package names
and three paragraphs for each entry, in the original order.
No request occurs at registration or until a stream is polled.

The plain `NOTICES` path uses UTF-8 and bypasses cached text.
Tests show a new raw notice supersedes an older cached string
without removing the old cache entry. Missing files, bad UTF-8,
invalid compression and an expanded-size limit have explicit
asset errors; one broken collector does not prevent subsequent
collector output. The five target tests passed, alongside
the asset and foundation regression targets.

The `wasm32-unknown-unknown` asset crate compiles. The unchanged
Flutter test compiled inside an isolated web test package, but
Chromium could not start in this environment (`socket() failed:
Operation not permitted`), so live Flutter browser execution is
not credited. The host supplies its root asset bundle explicitly;
the Rust collector is not automatically installed by a Flutter
Binding or a global platform channel. The original remains partial.

```sh
cargo test --locked -j 1 -p incular-assets --test flutter350_license_notices --test public_api -p incular-core --test flutter350_licenses -- --test-threads=1
cargo check --locked -j 1 -p incular-assets --target wasm32-unknown-unknown
bash audits/flutter-3.50.0-parity/run-reference-linux.sh upstream services/binding_test.dart 'Adds rootBundle LICENSES to LicenseRegistry'
```
