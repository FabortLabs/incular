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

## Directional overflow placement

The pinned authored `constraints_direction_test.dart` and the retained
Rust fixture place a 200px child in a 100px clipped viewport: logical
top-start results in x=0 in LTR, x=-100 in RTL, and x=0 again when an
explicit LTR override is applied in an RTL subtree. The Rust node
retains its identity across direction changes. A logical alignment
with a mounted child requires a direction; the default physical center
does not. The authored check adds no upstream registration credit.

## UnconstrainedBox diagnostics and clip updates

Pinned widget cases :962, :980, and :991 passed. Rust's Display
now returns the same concise alignment, axis, and direction text
for an UnconstrainedBox. A mounted childless box keeps its renderer
when switched from the default clip to antialiasing. Mounting a
400×400 child in the same 200×200 box demonstrates the new clip in
the paint list, then restoring the default removes it.

Using a fresh centered 200×200 proxy per setting, the 400×400 child
produces a summary diagnostic only for the omitted mode and explicit
Clip.none. It reports 100 px on each edge. HardEdge, AntiAlias,
and AntiAliasWithSaveLayer produce no overflow report. Incular uses
its nonfatal debug diagnostic queue rather than a Dart error callback.
All 18 focused constraints and overflow tests passed.

The pinned ConstraintsTransformBox diagnostic at :1058 ran, but the
native type erases its callback; it has no stable transform label.
That diagnostic remains reference-only.
