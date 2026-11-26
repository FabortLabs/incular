# Physical and directional painting insets

Seven exact upstream cases passed on the pinned Flutter 3.50.0 reference
(`painting/edge_insets_test.dart:12,80,115,228,358,370,423`). Incular now
explicitly resolves physical edges unchanged in either direction, preserves
unspecified edges on a copy, interpolates an absent endpoint from zero,
and performs modulo and integer division independently on all four edges.
The negative controls distinguish Euclidean remainder from signed Rust
remainder and truncation from flooring.

The numeric control also checks all four signed corners, opposite-side
flipping, both totals, and exact rectangle and size inflation and deflation.
It is partial against Flutter's broader diagnostic/hash test: Incular
exposes valid render sizes only, returns None for a negative or non-finite
extent, and does not emulate Dart diagnostic/hash output. No existing
layout normalization or rendering behavior was changed. Six focused
Rust inset cases and two interpolation controls passed.

```sh
cargo test --locked --offline -j1 -p incular-config --test flutter350_edge_insets_parity --test lerp_extrapolation -- --test-threads=1
```
