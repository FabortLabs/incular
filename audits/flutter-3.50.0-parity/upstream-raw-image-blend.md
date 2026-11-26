# RawImage destination blending

Pinned Flutter 3.50.0: rendering/image_test.dart :217 and :222,
widgets/image_test.dart :3145 and :3155. Flutter's RenderImage
defaults to source-over, accepts Plus, and updates to Multiply
on the existing render object. Incular's ordinary RawImage retains
the same image/render identity and carries the same default and
updates through to its compositor. The implicit and explicit
source-over states do not create a blend stage. A mode-only change
does not remeasure the box or re-record its image.

Destination blending uses the already-painted destination and is
separate from the optional image tint. The retained command test
verifies that the tint and opacity finish first. An absent image
does not create any destination blend. A fitted image does not
blend outside its raster; repeat-x and repeat-y extend only along
the repeated axis. The pinned authored pixel test samples both edges
of a centered source-replacement image and checks that the red
background outside it stays red. The Incular counterpart verifies
the actual native display-list destination and group bounds; no
headless wgpu pixel readback was available for this pair.

Full per-draw pixel equivalence is open for nine-sliced or transformed
images, globally transparent sources with destructive blend modes,
and combinations of color inversion and tint. This does not change
the existing no-slice vs. nine-slice fit-default discrepancy.
