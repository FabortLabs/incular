# Sliver opacity composition and retained layers

All seven exact pinned `rendering/proxy_sliver_test.dart`
registrations passed. The native fixture mounts the matching one-box
sliver in a real viewport and verifies no source draw or generation
for zero opacity, a visible byte-128 source for half opacity, and
visible byte 255 for the opaque case. Changing a child's paint in
a live viewport updates its source generation without replacing
the static or animated half-opacity layer.

These registrations contain one sliver box each. A sliver producing
several retained boxes currently wraps the boxes individually;
group compositing when they visually overlap is outside this claim.

## Overlapping static sliver children

An authored pinned Flutter raster probe measured alpha 128 at a
half-opacity sliver's isolated and overlapping pixels alike.
Incular originally submitted two separate half-opacity groups, which
could darken the overlap. It now supplies one retained scope for
consecutive rows, with clip and opacity scopes nested in source order.
A zero-opacity scope still follows the appropriate semantics policy;
revealing it retains the existing children.

The narrow-viewport regression also exposed staging measurements
temporarily reparenting pictures out of their retained scopes. Final
layout now assigns those parents once, so clipping cannot cull a
newly revealed picture on stale bounds. Forty targeted native tests
passed, including the overlap, nested order, and reveal at viewport
sizes 10 and 100. This fixes static sliver opacity; the animated
siblings' multi-row scope remains open. The authored result adds
no original registration credit.

The Flutter `SliverFadeTransition` animation and
`SliverAnimatedOpacity` implicit tween now also submit a shared
scope instead of attenuating each row. The pinned raster probe
runs both control paths and gets alpha 128 in each overlap.
The native animation test changes half opacity to three quarters
under the same scope and source generation. The public
Incular-specific direct opacity controller keeps its existing
per-row update path. No extra original registration is credited.
