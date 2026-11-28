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

## Isolated image crossfades

An authored reference widget produces RGBA (64, 64, 127, 255) when two
half-opacity images crossfade over opaque blue at the midpoint. The
native baseline only drew the second image. Both box and shape
decorations now paint both images with their independent fit and crop,
add their weighted premultiplied colors over transparency, and combine
the group with the original background once. The wgpu source-over fast
path still bypasses the group for ordinary drawing, but retains it
when a child blend uses the group's destination.

The native compositor test verifies both ordered image IDs, their
effective 0.25 opacities, and that Plus sits inside a separate
source-over group. Ten native image, shape and nullable controls
passed, as did the wgpu library check. Incular GPU framebuffer and
Flutter's three image-lerp goldens have not been compared and are not
counted as paired.

## Box-to-shape conversion

Pinned `painting/shape_decoration_test.dart:16` passes. The native
counterpart verifies equivalent shape values by comparing both
directions' inner and outer paths, insets and drawing, rather than
requiring equality on arbitrary Rust trait objects. A circle receives
the uniform box side, logical corner radii remain logical, and a
sharp rectangle keeps the original physical or directional border.
The native constructor also rejects combining a color and a gradient.

A separate mounted check confirms that conversion preserves images,
shadows and box-relative shaders. The same logical gradient's actual
paint coordinates reverse after an ambient direction change, with
the correct box dimensions. Three constructor and 17 regression
cases passed (13 widget cases and four concrete pointer routes).

## Mixed box and shape transitions

An authored Flutter probe verifies exact endpoint types and continuous
fill colors at 25%, 50% and 75%, with a 150-pixel wide midpoint
contour when a 200 × 100 circular box transitions to a rectangle.
The native counterpart previously returned a box at 25%; it now
checks exact colors, outlines and paint in both directions. The
mounted transition test now asserts interpolation rather than the
previously incorrect midpoint switch. Nineteen focused native tests
passed. The authored probe does not increase the registration count.

When a crossfade is retargeted before completion, a blended image
encountered as an endpoint restores its own group with source-over.
It must not be added to the outer group merely because it is the
second endpoint. The authored Flutter pixel probe checks both nesting
orders over black: red followed by a green/blue group produces
(64, 64, 64, 255), while that group followed by red produces
(128, 64, 64, 255). The native test now checks both group orders and
each exact leaf opacity. The earlier implementation failed by
emitting Plus *around* the nested group. Ten focused image and
transition tests passed. No additional registration was counted.

## Mounted shape paint order and image clip

Five pinned originals in `widgets/shape_decoration_test.dart` passed:
:24, :46, :72, :88, and :108. Mounted fills and images paint before the
outer then inner strokes of a compound border. A custom border receives the
filled box and inherited direction, with outer-path then paint for a color
and inner-path then paint for an image. The old native image painter asked
for the outer path and painted beneath both strokes. The corrected image
clip excludes their full combined width. The public decoration-wide clip
and shape hit contour still use the outer path. The directional radial
gradient case also verifies the emitted focal point, radii, and stops.
All nine focused native paint, image, and crossfade tests passed; these
inspect the retained display list, not framebuffer goldens.

## Unresolved directional padding

The original `widgets/shape_decoration_test.dart:64` passed. An inner
three-pixel start stroke exposes directional padding before resolution,
not physical padding silently resolved to LTR. It resolves to the left in
LTR and to the right in RTL. The native test also verifies mixed physical
and directional compound insets and zero padding for an unpainted border.
All eight focused tests passed.

## Negative shadow spread across the center

The authored pinned Flutter pixel probe draws box and shape decoration
shadows for rectangles, circles, and rounded rectangles. When negative
spread crosses an axis from positive to negative size, the painted edges
are sorted; at exactly zero extent, there is no shadow. On a 20 × 16
rectangle, spread -15 paints a centered 10 × 14 rectangle, or a fitted
10 × 10 circle. Both native painters previously panicked for inverted
bounds. They now share centered, normalized bounds and skip empty draws.
Native tests check the emitted bounds around zero, including translated
shadows, and the mounted shape regressions passed (seven focused tests).
No original registration count was changed by this authored probe.

## Shape decoration value comparison

The pinned original `widgets/shape_decoration_test.dart:132` passed.
Two separately allocated default physical borders produce equal white
decorations with the same shadow and matching hashes. The native test
also checks unequal fill or stroke values, independently allocated
compound borders, and shared-instance comparison for opaque custom
borders. Built-in shapes compare their paint descriptors without
resolving direction. Nine focused native tests passed.
