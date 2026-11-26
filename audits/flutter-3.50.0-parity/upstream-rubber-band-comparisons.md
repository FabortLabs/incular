# Partial upstream rubber-band return comparisons

Unmodified pinned Flutter
`packages/flutter/test/widgets/scroll_physics_test.dart:376,407`
passes both original tests. The committed
`flutter350_upstream_scroll_rubber_band` Rust target checks all eight
stationary positions and the moving-vs-stationary control after
equivalent dispatch. All eight positions differ beyond the original
0.01 px tolerance; both distinct-trajectory controls pass.

At t=0.2 s, starting x=-500 or 1500 with bounds [0,1000], DPR 1 and
zero velocity:
- Leading: Flutter -69.006285, Incular -134.145020.
- Trailing: Flutter 1069.006285, Incular 1134.145020.

Flutter also asserts its concrete simulation subtype; Rust's public
`may_overscroll` flag is broader and no equivalent exact model query
exists. Both originals are therefore **partial mismatches**, not
counted as complete paired registrations. The separately committed
`flutter350_bouncing_overscroll_at_rest` target and its pinned custom
Flutter reference compare return coordinates and actual completion
without relying on private subtype identity.

## Current result

Both pinned originals and their focused native counterparts now pass.
The earlier spring correction matches all eight sampled positions
within 0.01px. The native simulation's model query now verifies both
stationary and moving simulations are specifically Bouncing, not just
that they may overscroll. The moving and stationary trajectories remain
distinct. These are full behavioral pairs at the scroll-model boundary.
