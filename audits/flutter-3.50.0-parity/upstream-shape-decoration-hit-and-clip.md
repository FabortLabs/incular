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
passed. This does not count the pinned primitive-matrix registration,
which also requires superellipse and heterogeneous border interpolation.
