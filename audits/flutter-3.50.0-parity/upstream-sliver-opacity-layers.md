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
