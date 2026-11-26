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

Full native pixel equivalence remains open, especially for nine-sliced
or transformed images and fractional coverage. The independent fit-default fix is
[reported separately](upstream-tap-absorber-delta-and-raw-image.md#resolved-rawimage-fit-default).

## Tint, inversion, and fully transparent source coverage

The authored pinned Flutter pixel tests confirmed that fully inverting a
red SrcIn tint produces cyan, and that zero-opacity Src replaces pixels
inside the fitted source with transparent pixels without touching the
outside background. Zero-opacity SrcOver leaves the whole background
unchanged. Incular applies inversion after tint in one retained filter
rather than isolating twice. At exactly zero opacity the modes that erase
the destination still emit the fitted coverage with transparent color;
the ordinary SrcOver path and non-destructive controls do not emit the
raster. The native checks execute the captured filters and verify effect
bounds for those modes. Forty-six native cases passed; these two authored
reference cases do not change the upstream registration total.

## Nine-slice and direction-aware raster geometry

Nine-slicing and ambient mirroring now record on the existing image
render object rather than through an extra builder, stack and custom
paint subtree. Source-replacement scope comes from the actual nine
patches, in their transformed coordinates, bounded by the repeat
clip. This keeps the unpainted space around an explicitly scaled-down
slice or an edge-aligned RTL image out of the blend. The native tests
check full-fit and smaller slice bounds, physical left/right alignment,
LTR → RTL → LTR with a stable render identity, and the required
Directionality error. The pinned authored Flutter pixels show the first
source column changing red to blue while the physical left edge and the
outside green pixels stay put.

The four pinned source-effect controls and 51 focused native image,
painting and neighboring decoration tests passed. This verifies native
rendered commands and source color math; offscreen wgpu pixel readback
and transformed fractional coverage remain unverified.

## Retaining the source effects with the render object

Tint, inversion, opacity, and destination blending now update a
single retained image. The default image owns no effect stages.
Adding or removing one selected stage does not replace the other
stages, remeasure image layout, or re-record its picture. Clearing
the image removes all its effect stages without replacing the image
render object. The focused test checks stable IDs, evaluated tint
colors, stage order, and the tree's layer-ownership invariants.

Flutter's original `widgets/basic_test.dart:24` is a partial match:
the Flutter case checks exact public properties. The native probe
runs its default-to-replacement configuration and matches identity,
image, dimensions, scale, placement, repetition, slice, and invisible
source-over paint. Flutter's diagnostic label and specific sampler
quality, antialiasing, and animation-backed opacity are not matched.

Flutter's `rendering/image_test.dart:210` is also partial: native
paint applies independent tint modes to the actual source image and
does not treat that field as the destination blend. The Dart setter
getter round-trip is not part of the exposed Rust renderer API.

Both selected upstream cases and 53 focused native tests passed.

## Logical alignment is independent of mirroring

RawImage and the image-provider façade accept physical, directional,
or mixed alignment geometry. A logical alignment reads Directionality
even when the image itself is not mirrored. In the applied Flutter
pixel checks, top-start changes edges under RTL whether mirroring is
off or on; mirroring changes the order of the source columns, not the
resolved edge. The native image keeps its render identity through an
LTR → RTL → LTR change and applies destination blending only in the
resolved raster bounds. It also tests mixed physical and directional
alignment. Five authored Flutter probes and 54 native cases passed.
