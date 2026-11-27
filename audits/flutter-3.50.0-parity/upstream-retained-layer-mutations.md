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
