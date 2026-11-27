# Retained box proxy property updates

Five exact Flutter 3.50.0 originals in
`rendering/proxy_getters_and_setters_test.dart` passed. The native
fixtures read the same defaults and changed values, then exercise
their effects on an already mounted render box:

- Replacing a tight height with a tight width changes 0×10 to 10×0.
- LimitedBox's infinite defaults allow the child to choose its size;
  changing its maxima to 0 and 1 limits only the unbounded layout.
- Aspect ratios 1, 0.2 and 1.2 resolve the next frame's expected width.
- Intrinsic width and height steps start absent and independently
  round 13×17 to 20×17 and then 20×20.
- The retained opacity controller begins at 1 and setting it to 0
  suppresses paint without changing the child's layout or identity.

The layout proxies expose Rust readbacks and descriptor updates instead
of mutable Dart render properties. The tests retain each render object.
All 16 focused native constraint and opacity cases passed.
