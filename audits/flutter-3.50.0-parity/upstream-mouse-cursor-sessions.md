# Rendering mouse-cursor sessions

### Activation and stationary pointers

Five pinned `rendering/mouse_tracker_cursor_test.dart` registrations
(89, 150, 212, 255, 287) passed. Native tests use actual retained
MouseRegion routes and positioned platform events, recording the
runtime's cursor activations. A pointer added outside activates Basic,
entering changes to Grabbing, further moves inside activate nothing,
and departing restores Basic. Adding inside activates Grabbing right
away; removing a device sends no activation. The first non-deferring
annotation wins; preceding Defer annotations do not consume the search.

A cursor-only widget update now refreshes a stationary device, even
without new pointer coordinates. An unchanged frame does not activate
again. The runtime shares the input/layout hover hit test with cursor
selection instead of asking the host to hit-test the tree again. Hosts
that expose one window cursor use the latest hover device; hosts that
can expose more can subscribe to per-device activations. No physical
cursor driver was asserted in this batch.

### Independent devices and absent hosts

The other four pinned cases (71, 319, 363, 398) passed on both sides.
Without a host callback, selecting a cursor remains safe. Explicit
Basic is the same selection as no annotation, so crossing the boundary
does not re-activate it. Removing and adding a device opens a fresh
session and publishes Basic again. Two devices get independent
activation histories and keep their own current cursor; the most
recently positioned surviving device controls the single-window view.
An authored native control also verifies that a resize handle changing
direction under a stationary mouse refreshes its cursor. The existing
multi-device hover and compositor-driven follower regressions passed.
