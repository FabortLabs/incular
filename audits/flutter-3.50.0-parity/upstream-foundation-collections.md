# Foundation collection helpers

All seven exact `foundation/collections_test.dart` registrations
passed against pinned Flutter 3.50.0. Seven focused native counterparts
pass through the public `incular::core::collections` API.

- Nullable lists compare length and elements in order; nullable maps
  and sets ignore iteration order. Absent is not empty. Values' own
  equality still applies, and shared collection references compare equal.
- Binary search uses the same matching midpoint, including for repeated
  keys. Rust represents the missing result as `None` rather than -1.
- The stable sort checks randomized input for every length 0–249,
  stable ties at 8, 50 and 511–513, and reverse sorting a middle range
  without moving either exterior segment. The all-equal, one-outlier,
  and reverse-order cases preserve every original element. The native
  stable slice algorithm is intentional; comparator call order is not
  an asserted part of the public behavior.

Native command: `cargo test --locked -p incular-core -j 1 --test
flutter350_foundation_collections -- --test-threads=1`.
