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

## ShaderMask configuration and delayed shader creation

The exact `RenderShaderMask getters and setters` original passed at line 54.
The mounted Rust fixture checks the default mode, descriptor readbacks,
callback replacement, and the published blend mode and shader. A second
pinned probe and the Rust regression verify that replacing the callback
does not invoke it until the next paint. Before the fix, the callback
ran early while updating the retained layer. Changing only the blend mode
continues to reuse the already painted shader in Incular.

An independent pinned probe calibrated the transformed clip: translating
a 40×40 shader surface by (10, 5) within a 40×40 clip leaves a 30×35
published semantic rect. The native test now distinguishes that rect from
the unchanged 40×40 paint effect bound. The previous semantic expectation
failed on the unchanged parent commit as well. All 44 focused native
shader, layer reuse, observer and topology cases passed; no GPU-readback
claim is made.
