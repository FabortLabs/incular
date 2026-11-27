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
The special infinite shrink-wrapping paint-clip test is a separate case.

Three direct and 15 adjacent native viewport and semantics checks passed.
