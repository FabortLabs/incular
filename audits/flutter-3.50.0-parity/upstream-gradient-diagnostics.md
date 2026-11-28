# Gradient descriptions

The exact pinned `painting/gradient_test.dart:276` original passed. The
Rust descriptor now prints the gradient's two alignments, full RGBA
colors, clamp mode, and recorded rotation in the same order. It does not
print positions when they were inferred. Explicitly supplying the same
normalized stops retains the positions in diagnostics without changing
their renderer equality or cache identity. Opacity changes keep the
positions' declared status. The matching radial and sweep formatting
paths share the color and transform descriptions; their focused paint and
interpolation controls passed as well (13 native tests in total). Flutter's
rotated gradient golden images were not part of this comparison.
