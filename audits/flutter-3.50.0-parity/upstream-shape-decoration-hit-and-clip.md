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
