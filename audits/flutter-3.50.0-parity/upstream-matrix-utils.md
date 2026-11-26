# Double-precision matrix utilities

All ten pinned `painting/matrix_utils_test.dart` registrations passed.
The Incular counterpart exposes column-major `Matrix4`, `Point64` and
`Rect64` through `incular::painting`.

- **12, 19, 30:** very large finite edges stay finite under translation;
  in-place multiplication changes only the right-hand destination;
  extracting a 2D translation rejects a 3D offset, rotation, scale or
  zero matrix.
- **63, 73, 83, 93:** cylindrical projection preserves the identity at
  zero angle, reduces to rotation with zero radius/perspective, and
  produces the pinned numeric matrix at the default perspective.
- **119, 139, 158:** a forced point ignores incoming x/y, affine
  rectangle bounds handle identity, scale and rotation, and 9,999
  perspective rectangles match an independent analytic projection of
  all four corners. Additional native checks cover uniform-scale
  extraction and the horizontal cylinder orientation.

Native: 12 matrix tests and six preexisting affine geometry tests
passed. The geometry utility does not enable 4×4 perspective on the
ordinary retained widget, compositor, or hit-test paths. The renderer
remains wgpu, with its current retained affine path unchanged.
