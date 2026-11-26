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
