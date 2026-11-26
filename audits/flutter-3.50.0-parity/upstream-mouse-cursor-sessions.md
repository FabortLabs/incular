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
