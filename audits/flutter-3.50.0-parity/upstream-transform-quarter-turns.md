# Widget rotation matrices and padding

### Exact quarter-turn coefficients

The pinned `widgets/transform_test.dart:356` registration passed.
The native widget uses the same 800×600 viewport and centered repaint
boundary, verifying the exact 2D affine coefficients and translation
at 0, 90, 180 and 270 degrees, plus a negative quarter turn. It checks
the actual transform applied to the drawable as well as the retained
geometry. Before the change the 90-degree case contained off-axis
coefficients of about −4.37e−8 and a pivot near (700.0000175, −99.9999869)
instead of (700, −100). A near-but-not-exact angle still uses continuous
rotation. The zero-angle drawable's transform matches the unwrapped
repaint boundary.

The original remains partial because Flutter also asserts the number and
types of physical layers. Incular combines the affine into published
picture transforms rather than emitting matching Flutter TransformLayer
and OffsetLayer objects. The fix does not change the general affine
constructor, the animated rotation API, or the renderer.

### Paint origin through padding

The exact pinned `widgets/transform_test.dart:264` passed. The native
800×600 fixture pads the placeholder left 30, top 20, right 50, and
bottom 70 then applies a 2× scale about the placeholder's local origin.
The widget's measured location and actual stroked outline both start
at (30, 20), not at twice that offset, while the 720×510 local box
paints at 1440×1020. Thirteen native transform tests and the rotated
drag regression passed.

### Mirrored hits and zero area

Both original `widgets/transform_test.dart` cases (874 and 958) passed.
The native widget uses the same 2×2 arrangement of 40×40 boxes and
sends pointer presses/releases at all four quadrant centers. A red
target is reached only at (60, 20) with X flipped, (20, 60) with Y
flipped, and (60, 60) with both. The unflipped control reaches (20, 20).
A centered zero-area placeholder flipped on Y also completes layout
and paint without growing. Fifteen focused transform tests and the
existing rotated-drag test passed.

### Nested translation hit testing

The pinned `widgets/transform_test.dart:500` passed. Its retained native
counterpart sends a raw pointer press at the black child's translated
center, (1500, 1350). The listener reports the correct local (400, 300)
and the original untransformed center is not a hit. Before the fix an
intermediate layout bounds check suppressed that actual callback.
Widget transforms now forward to their children without treating that
rectangle as a clip. Actual clip nodes and bounded specialized
render objects keep their own hit policies.

The authored `nested_transform_hit_opt_out_test.dart` additionally checks
that disabling hit conversion for the outer transform still allows
the nested transform to move the target, while a surrounding ClipRect
rejects the same outside hit. Native regressions cover both raw and
direct retained hit paths (50 targeted tests passed).
