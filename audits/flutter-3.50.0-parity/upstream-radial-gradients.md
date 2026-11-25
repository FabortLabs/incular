# Flutter 3.50 aligned radial gradients

Pinned `painting/gradient_test.dart:406,482,510,542,573,591,618,638,
690,712,736,1010` and the linear direction case :308 passed. Both
shader kinds reject missing logical direction at shader creation;
physical alignments do not require it. The radial tests reproduce
default and changing radii/centers, nullable focal points, the stop
union, opacity replacement versus scaling, shader recoloring, and
discrete rotation. The radius and focal radius lower using the actual
shortest side, not average size or one fixed dimension. The original
value matcher allows sub-byte channel error, so the Rust checks
allow 8-bit quantization but require the geometry and stop offsets
to agree exactly.

The physical renderer already supports focal sampling. The new
neutral box descriptor resolves into that real radial brush in the
retained painter and shares the existing shader cache. A linear to
radial transition fades the old gradient out then the new one in; the
midpoint selects the radial shape at zero alpha. A Rust fixture checks
the emitted brush center and 1200px radius at the pinned radius 20
on a 100×60 box. No renderer change is required for this tranche.

The pinned radial identity registration :536 is partial: the Rust
value retains the same GPU stop identity and missing/missing stays
absent, rather than guaranteeing Dart heap-object identity.
