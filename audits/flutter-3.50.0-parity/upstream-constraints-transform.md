# Retained ConstraintsTransformBox

Pinned `rendering/box_test.dart:407,482,499,563,584` passed by
exact name. The previous LayoutBuilder lowered the transformed
constraints through a ConstrainedBox, which re-intersected them with
the original limits. The retained proxy now passes the transformed
limits directly to its child, sizes itself under the original limits,
and aligns the measured child. The width-only transform and fully
unconstrained transform retain the same 200×200 renderer. A typed
invalid transform returns a layout error before measuring the child.

With a 200×200 requested child and a 100×100 viewport, removing only
the width maximum gives a 200×100 child at (-50,0). Clipping uses the
chosen coverage only on overflow. Unclipped layout records a debug
overflow without rewriting paint; :499 is partial because Flutter
uses its diagnostic error callback and a double.maxFinite fixture.

The same 100-letter, 30px flow case now wraps in a default paragraph
and stays on one line under the max-width transform. The separate
[engine comparison](upstream-long-word-wrap.md) verifies the
unbreakable-word fallback. :563 remains partial because the Flutter
original also calls the native intrinsic-measurement API. A direct
painter still runs at zero area; :584 is partial because the original
uses an empty paragraph's draw call, not a custom painter.

## Childless and zero-area constraints proxies

The pinned `widgets/basic_test.dart:1044` and :1078 originals
passed. A zero-area UnconstrainedBox with a direction-aware placeholder
and a genuinely childless ConstraintsTransformBox both finish layout
and paint at zero size. In the native retained check the childless
proxy does not allocate a fake child or invoke its constraint callback;
mounting and removing a real child keeps the original proxy renderer
and invokes the callback only while the child is present.
