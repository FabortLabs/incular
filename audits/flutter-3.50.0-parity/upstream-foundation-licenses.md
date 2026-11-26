# Foundation license paragraphs and collectors

All four pinned `foundation/licenses_test.dart` registrations pair.
The neutral foundation API does not need Material or any other design
system. It retains the notice text until a consumer asks for the
paragraphs. The tests compare all 25 original outputs, preserving
trailing spaces where the original expects them, LF/CRLF breaks,
form-feed page boundaries, paragraph indentation, centered lines
and bracketed preambles. Tabs count as eight spaces as in Flutter;
a separate case preserves UTF-8 text through wrapping.

The registry accepts async collector streams, invokes none before
a consumer polls, reads each in registration order and starts a
later collector only after the active one completes. Native and
Dart probes both deliberately pause a first collector and then check
that it resumes before the second starts. Re-reading creates fresh
streams rather than caching the materialized licenses. Native reset
clears future reads without mutating a stream already in progress.

Six focused Rust and two authored Dart cases pass. This scope does
not audit or change the Material license page or its pre-existing
registry, discover dependency notices, or load compressed NOTICES
bundles from services.

```sh
cargo test --locked -j 1 -p incular-core --test flutter350_licenses -- --test-threads=1
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe license_collectors_test.dart
```
