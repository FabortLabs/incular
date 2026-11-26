# Catmull-Rom precomputation and invalid inputs

Five pinned `animation/curves_test.dart` originals now pair:
:318, :381, :408, :491 and :576. The explicit precompute methods
produce the same spline points with custom endpoints and the
same sampled timing values at the original 1e-6 tolerance. Both
regular and precomputed construction reject too few control
points, out-of-range tension, non-finite coordinates and invalid
endpoint handles. The timing curve rejects nonmonotonic or
multivalued points as in the original.

The previous native spline accepted non-finite coefficients.
Its adaptive sampler would never take the flat-span branch for
a NaN and could continue subdividing without useful output.
The native spline now returns a typed error before subdivision,
and the shared sampler catches invalid ranges, tolerances and
non-finite samples from user implementations. Adjacent floating
point parameters end a span when no subdivision is representable.
The valid deterministic sample path is unchanged.

Fourteen focused timing/geometry tests passed. The Rust spline
has always built eagerly and rejects invalid inputs during
construction rather than waiting for its first sample. The new
precompute entry points are explicit eager constructors. A
degenerate adjacent segment is rejected rather than returning
invalid geometry.

```sh
cargo test --locked -j 1 -p incular-animation --test flutter350_upstream_curve_geometry --test flutter350_upstream_curve_samples -- --test-threads=1
```
