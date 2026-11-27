# Placeholder sizing, paint, and child

### Bounded axes and a real child

The pinned `widgets/placeholder_test.dart` fixtures cover fill at the
800×600 viewport, fill inside Center, fallback only on each unbounded
axis of a positioned child, independently customized 200×300 fallbacks,
paint defaults and overrides, a mounted text child, and zero-area layout.
Native counterparts use mounted widgets and assert bounds and actual
stroke commands, including the default blue-grey and 2 logical pixels,
the green override, and the 10 logical pixel override.

Before the fix an Incular placeholder inside a Center measured 400×400
despite room for 800×600. This widget now follows Flutter's LimitedBox
around CustomPaint: the fallback applies only to an unbounded axis, and
empty paint otherwise takes the largest finite size. An optional child
determines the box's size under those constraints; the default color
and stroke and the two diagonals do not change.

### Child hits and layout updates

In both authored pinned tests the outline alone never takes a hit; an
underlying button can receive a tap, and a 20×30 interactive child remains
both 20×30 and tappable when wrapped. Native tests use actual gesture
routing, not a simulation of the hit outcome. Making the painter itself
transparent, rather than disabling the entire widget subtree, preserves
the child's routing.

The pinned `rendering/repaint_boundary_2_test.dart:9` switches an outer
box from loose viewport limits to tight 100×100 constraints and rebuilds
two placeholders with 10×10 bounds rather than 20×20. The native fixture
flushes both layouts and paint, verifies what constraints the builder
received, and checks both current outline sizes on both frames.
