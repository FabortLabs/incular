# Nested and moved repaint boundaries

Both pinned `rendering/repaint_boundary_test.dart` cases (13 and 32)
passed. The native empty-content counterpart of case 13 mounts two
nested retained repaint boundaries alternating with three opacity
nodes. It flushes layout, paint and semantics through the three
updates at 0.9, 0.8 and 0.7. Both boundary elements and render IDs
stay stable. A separate painted control also verifies the order
and number of changing opacity scopes in the published display list.

Case 32 stays partial. The native fixture invalidates paint under a
boundary, moves it from a top-level Stack to a 50-pixel padding wrapper
before the next layout, and verifies the original render ID, new origin,
and new paint rather than stale content. This covers retained invalidation
and reparenting, not Flutter's explicit render-object compositing-bits
invalidation method. The remaining diagnostics on this file depend on
custom RenderBox layer ownership and are not credited here.
