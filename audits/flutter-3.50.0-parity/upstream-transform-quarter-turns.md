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

### Uniform and independent scales

Both pinned `widgets/transform_test.dart` scale registrations (814 and
843) passed. The 150×100 child retains its layout size and its world
center at (400, 300). A uniform 1.5 scale produces 225×150 paint bounds;
different 1.5 and 1.2 scales produce 225×120. Native checks use a small
logical-pixel tolerance for the f32 scale representation. The two
constructors are separate in Rust, so the invalid option combinations
tested in Dart are not claimed as runtime assertion equivalents.
Eighteen focused transform tests and two focus geometry tests passed.

### Filtered transforms in the retained renderer

Five pinned `widgets/transform_test.dart` registrations (560, 592, 624,
656, 693) passed. Four have complete native counterparts in
`flutter350_transform_filters`. Each fixture uses the same viewport and
one low-quality linear matrix resampling stage. The tests project the
native local matrix through its published stage transform to compare
the effective world-space matrix: (25, 25) translation; 3.14159
centered scale; 45-degree rotation; and the same rotation around the
world pivot when the child is offset inside Center. They verify
coefficients against the pinned numbers and count actual resampling
commands rather than generic transform wrappers.

Case 693 remains partial: removing and restoring sampling drops and
reinstates the native stage, but Rust's Transform takes a required child
and the Dart absent-child intermediate state was not compared. These
fixtures verify retained command semantics with the wgpu renderer
unchanged, not an external filter golden or native GPU pixels.

## Default affine matrices in the original viewport

Pinned `widgets/transform_test.dart:207,231,277,291` passed. With an
800×600 retained child, a default quarter-turn gives the exact affine
coefficients [0,1,-1,0,700,-100] and a default double scale gives
[2,0,0,2,-400,-300]. The native compositor applies each matrix
to the actual child picture and keeps the (400,300) center fixed for
the scale. These two cases pair.

The (100,50) translation moves the native child origin without a
resampling pass. In the 400×300 centered clipped case, plain half scale
maps the painted child to a 200×150 rectangle at (200,150); the
offset adjustment relative to scaling the outer origin is (100,75),
as in Flutter. These are partial: Incular's retained layers keep
translation bookkeeping and flatten the local adjustment into a
world matrix, rather than allocating Flutter's particular layer
arrangement. All four focused native checks passed.
