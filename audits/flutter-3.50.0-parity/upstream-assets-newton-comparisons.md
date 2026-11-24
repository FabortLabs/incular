# Additional upstream assets and Newton physics pairs

Pinned Flutter 3.50.0 `ab2912035e`, Incular `c647cb85`. The official
Flutter tests ran via exact name filters; the observable Rust equivalents
are now committed.

- `asset_bundle_test.dart:59,178,197,234`: four matches. Covers raw/text
  load counts, cached synchronous structured values, parser errors, and
  injected HTTP 400. Rust uses a synchronous result or typed error rather
  than Dart's Future subtype or diagnostic tree; the HTTP test sends no
  real network request. Run
  `cargo test --locked -p incular-assets --test flutter_asset_bundle_parity flutter_asset_`.
- `newton_test.dart:12,32,76,90,120,163,192,219`: six matches; two Rust
  assertions fail against the exact reference. Velocity samples differ
  slightly (`120.00001`, `36.000004`, `10.800001` vs
  `120`, `36`, `10.8`), and positive friction-through is not done just
  after 1 s while it is in Flutter. Duration uses the first nanosecond
  after 1 s; Flutter samples 0.1 ns after. That translation is less strict,
  not an excuse to round the sample down; velocity differences remain
  independently observable. Run
  `cargo test --locked -p incular-animation --test flutter_upstream_physics flutter_`.
  Three original escaping-bounds Newton tests remain open.
