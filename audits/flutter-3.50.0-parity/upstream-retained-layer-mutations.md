# Mutating retained scene layers

Five originals in the pinned Flutter 3.50.0
`packages/flutter/test/rendering/layers_test.dart` passed. They clear the
dirty flag before changing the offset, rect/rrect/path clip geometry and
behavior, or backdrop blur, then verify the scene must be reissued.

The native tests use equivalent retained stages under a parent layer and
a real picture sibling. An unchanged stage reuses its previously flattened
buffer. Each property change creates a new buffer with the expected
picture placement, clip shape and coverage, or blur. Republishing the
same scene then reuses that new buffer and keeps the underlying picture
identity. Rectangular/rounded isolated coverage and the retained path's
exact rectangle and fill rule are checked in the render command stream.

The native compositor uses a scene buffer rather than Flutter's
per-layer engine flags. All five comparisons and 14 focused regressions
passed. The tests assert the requested renderer-neutral output, not a
live GPU raster comparison.

## Effect mutations

Four more exact pinned registrations passed: opacity alpha and offset,
color-filter color, shader-mask rectangle/blend/shader, and superellipse
geometry/coverage. The retained tests verify the changed output at every
step while keeping the source picture. Native opacity uses a translated
child stage for its offset. Native artistic color mode uses the per-pixel
`ColorFilter::blend` stage, not the matrix-only compatibility constructor.
The superellipse path was produced by the actual rounded-superellipse
builder; an additional rounded corner is verified outside the filled
shape after its geometry changes.

All nine mutation tests and 15 focused color, culling, and effect reuse
regressions passed without using a GPU.
