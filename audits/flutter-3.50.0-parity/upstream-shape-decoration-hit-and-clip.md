# ShapeDecoration hit testing and clip outlines

Five pinned `painting/shape_decoration_test.dart` registrations passed.

- **102:** a border without a custom hit test evaluates its outer path,
  not the inset stroke path.
- **112, 177:** a decoration delegates to its shape's custom hit test,
  and a compound border delegates to its outer border without allocating
  a fallback path.
- **272, 282:** the clip is the actual centered circle or full-width
  ellipse, not the enclosing rectangle, at all pinned sample points.

Native: five focused geometry/dispatch tests and seven preexisting
shape-decoration widget tests passed. This slice checks the public
decoration and border API; mounted hit routing is a separate comparison.

## Mounted hit routing

Four authored Flutter probes and their retained Incular callbacks match:
a transparent corner falls through to the sibling behind it, a real
child can still accept a point outside the shape, a custom border
override can reject a painted point, and changing LTR to RTL moves a
rounded start corner. Before the fix, the background paint layers
claimed their rectangular bounds and hid the underlying target.

The paint-only layers now ignore pointer input, and the decoration
tests its true border only after its real child misses. Paint and hit
testing share the inherited text direction. Shape-only hit updates also
invalidate stationary pointer hit testing. Native: four mounted tests,
nine ColoredBox controls and twelve focused shape API/widget tests
passed. Authored cases are not counted as upstream registrations.

## Color-to-gradient transitions

Pinned `painting/shape_decoration_test.dart:188` passes. Incular used to
hold a solid fill to the midpoint, then switch to a gradient. The
intermediate keeps only a gradient now, using the existing gradient's
geometry and a uniform version of the solid endpoint. The original
decoration is returned unchanged at each exact endpoint.

Two native tests cover seven forward and reverse fractions, independently
calculated channel values rounded to Incular's RGBA8, near-solid stops,
the actual gradient paint command, and preservation of radial and sweep
geometry. Fourteen transition and seven ShapeDecoration controls also
passed. Interpolation between two distinct gradient geometries, image
interpolation, and heterogeneous border morphs remain separate gaps.

## Exact primitive boundaries

Three authored Flutter probes and their native counterparts match. Box,
circle, oval, stadium and rounded rectangle hit tests now use analytic
containment without constructing a path. Right and bottom bounds are
excluded, while left and top can be included. Curved boundaries use
the normalized ellipse, including eccentric circles and directional
rounded rectangles whose adjacent radii require a shared reduction.

The old box and circle hit tests wrongly included the right edge in the
paired fixtures. The native tests also cover an ellipse boundary and a
nearby outside point, degenerate frames and the same scaled logical
radius in both text directions. Thirty-four prior shape regressions
passed. The full pinned primitive matrix is covered separately below.

## Directional decoration images

Pinned `painting/shape_decoration_test.dart:228` passes. In the mounted
RTL case, the 100 × 200 source is recorded at destination (20, 760)
with its full size. Previously it was centered at (470, 360). Logical
alignment now resolves against ambient direction; it reports a missing
direction instead of silently centering. The LTR path also records source
and destination rectangles, and both cases retain the circular clip.

Both decoration types now share their image painter. Shape decoration
also honors RTL mirroring and nine-slice patches. Three mounted native
cases passed and check actual compositor commands, exact crop and
destination, missing-direction diagnostics, and all nine patches. The
three box image paint cases, seven shape controls and 15 paint option
controls passed. Incular accepts an already-decoded image handle here;
the upstream synchronous test provider makes no async callbacks. No
provider lifecycle or GPU pixel equivalence is claimed.

## Morphing hit and clip outlines

Pinned `painting/shape_decoration_test.dart:76` passes. The decoration
transition now uses the existing fitted-outline interpolation for
rounded rectangles, rounded superellipses, circles and ovals. It
preserves exact endpoint identities. For a 200 × 100 box, the midpoint
between a circle and a rectangle now has bounds (25, 0, 150, 100).
Previously the endpoint switch widened it to the full box immediately.

Both native counterparts passed. They check every pinned sample point,
the reverse morph, equality of paint and clip contours, and the
intermediate side width. Four existing nested outline cases, two
gradient transition cases and 14 mounted transition cases also passed.
The separate primitive matrix below adds the stadium morphs. Incompatible
families still use the endpoint fallback.

## Primitive shape matrix and stadium morphs

Pinned `painting/shape_decoration_test.dart:120` passes. The port checks
all 112 shape/point combinations both against the expected Flutter
truth table recorded by the authored probe and against Incular's
actual outer path. The missing stadium-to-circle transition previously
rejected the pinned point (11, 35) by switching straight to the circle.

A stadium now morphs to a fitted circle or ellipse by changing its
frame, and to a rounded rectangle by changing its radii without
resizing its frame. Hit testing, painting and clipping evaluate the
same fitted shape. Both Flutter and Incular show a stadium-to-rectangle
midpoint whose edge is distinct from either endpoint. Five primitive
native cases and 28 existing shape and outline cases passed.

## Nullable and identical decorations

Pinned `painting/shape_decoration_test.dart:62` and `:68` pass. The
nullable Rust API returns no value for two absent decorations and
borrows the exact original for an identical input or a selected
endpoint when both are present. A single present decoration uses the
nullable scaling rules instead of jumping to an empty shape at halfway.

An authored Flutter case and its native counterpart verify continuous
changes to color opacity, rounded corners, stroke width and shadow
geometry in both directions. A native probe also checks fading a
single image and gradient while preserving image/shader geometry. Four
nullable cases and 18 focused transition controls passed. Two distinct
images still need a true crossfade, and BoxDecoration-to-ShapeDecoration
conversion remains separate.
