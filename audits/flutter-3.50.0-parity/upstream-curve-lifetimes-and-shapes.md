# Curve lifetimes and bounded shape checks

Five pinned `animation/curves_test.dart` controls pair:
one-line diagnostics, flipped ease, elastic and back overshoot,
and deceleration. The native suite repeats the original checkpoints
from 0 to 1, including Flutter's different in/out overshoot edges.
Flipping keeps the endpoints and evaluates the reflected curve;
the decelerating curve stays within range and moves farther early.

Previously a newly sampled Catmull-Rom curve or a general flipped
curve was interned permanently on the heap. Native timing curves
now own those values through shared references. A bounded cache
holds weak references to the 32 recently used sampled curves, so
live identical descriptors reuse work but discarded descriptors
do not keep their samples. Validation and sampling run outside
the cache lock. General flips have no process-wide cache.

Tests inspect the actual shared data while both curves are alive
and verify no live reference remains after the last owner drops.
The old direct enum variants accepted a static reference; use
`Curve::flipped()` and `Curve::catmull_rom()` or pass a shared
reference to the corresponding enum variant. Twenty-one focused
native cases passed, including the previously pinned numeric
samples and invalid-spline controls.

```sh
cargo test --locked -j 1 -p incular-animation --test flutter350_curve_storage --test flutter350_upstream_curve_geometry --test flutter350_upstream_curve_samples -- --test-threads=1
```
