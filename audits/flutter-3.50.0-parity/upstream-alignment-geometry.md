# Alignment arithmetic and resolution

Exact pinned Flutter 3.50.0 `painting/alignment_test.dart` cases:
- :17, :28 cover physical scalar operators, one-line diagnostics, consistent
  equal-value hashes, and interpolation to and from a center-valued null.
- :56, :170, :200, :271 cover zero/one invariants, mixed component
  arithmetic, the exact quarter interpolation, and resolving in either
  direction. The native test replays the original 30-by-30 ordered pairs
  (including null), both directions, both endpoints and all three interior
  times. A tolerance of 0.0002 accounts for Incular's f32 layout values.
- :280 and :309 cover typed arithmetic: physical and directional values
  keep their own coordinate system, and mixed values postpone resolution.
  Signed remainder and truncation controls distinguish Dart's nonnegative
  remainder from Rust's default signed float remainder.
- :322, :338, :343 cover canonical diagnostics, constructors and all 15
  shared constants. Mixed diagnostics print the vertical component once.
  Direct pinned Dart fixed formatting also confirms half-way rounding.
  The factories and constants upgrade the prior partial comparisons.
- :361 and :376 cover rejection of missing text direction. Native fallible
  resolution reports the same diagnostic theme; physical geometry still
  resolves with no ambient direction, while even a zero relative component
  retains the requirement of its original directional type.

The three original identity cases :38, :44, :50 also ran. Null and numeric
endpoints compare, but Rust's copyable value types intentionally have no
observable wrapper object identity. They remain partial. The original
:117 directional-coordinate comparison remains credited separately.
Nine focused geometry tests and fifteen mounted widget controls passed.
