# Viewport semantics cache clips

Seven pinned `rendering/viewport_caching_test.dart` numeric cases pair.
For an 800×600 vertical viewport, the default extends the semantic
clip by 250 logical pixels before and after. Explicit pixel extents
0 and 500 are unchanged by size. Viewport factors 0, 0.5, 1 and 2.5
expand by exactly 0, 300, 600 and 1500 pixels on each end. The cross
axis does not expand. A horizontal control resizes the main-axis
viewport from 800 to 600 and the 0.5 cache shrinks from 400 to 300.

The shared clip is used for real semantic tree culling. In the retained
control, the offscreen label inside the cache is present, a farther
label is excluded, and setting the cache to zero removes the cached
label without scrolling. Existing pixel configuration on CustomScrollView
continues to work alongside the typed pixel-or-viewport policy.

Flutter's separate null factor in viewport mode throws. A Rust
`ScrollCacheExtent::Viewport` must contain a numeric factor; leaving the
property unset chooses the normal pixel default. Because the public Rust
API cannot express that nullable input, it is not counted as paired.

Three direct and 15 adjacent native viewport and semantics checks passed.

## Unbounded shrink-wrapping paint clip

The separate pinned `rendering/viewport_caching_test.dart:128` case
passes: a shrink-wrapping viewport reports the full finite 800×600
paint clip when the main-axis constraint is infinite, even with one
pixel of preceding overlap. A mounted reference probe supplies that
overlap through a preceding sliver and checks all four axis directions.
It switches from bounded to unbounded and back at the same final size:
only the finite layout insets the physical leading edge by one pixel.

The mounted native test initially reproduced the one-pixel crop under
unbounded constraints. The retained clip query and the accessibility
projection now share the same rule. It reads the incoming limit, not
the finite working extent that native shrink-wrap uses to lay out children.
The fix does not change sliver sizing, actual viewport overflow layers,
or scroll metrics.
