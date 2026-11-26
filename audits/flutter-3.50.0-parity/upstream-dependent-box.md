# Positioning a measured box relative to a target

Pinned `painting/geometry_test.dart:9` passes. All ten native
coordinates match: an overwide child is centered even when the
target is on an edge; otherwise margin constraints clamp the
centered position only when there is room for both margins.

A pinned authored control checks above/below fallback, preference
when neither side fits, clamping the near edge, and the horizontal
NaN target selecting the upper clamp bound. Both implementations
passed those seven variants. Fourteen focused native checks passed.
This registers the geometry helper only, not tooltip placement or
a design-system tooltip.

## Mounted RawTooltip placement

Eight pinned `widgets/raw_tooltip_test.dart` originals passed:
`:24, :80, :138, :203, :278, :336, :399, :2660`. The mounted
native tooltip places the measured bubble in owning-overlay
coordinates, using the target center and Flutter's default ten-pixel
horizontal margin. The center, top-left, preferred above/below and
both right-edge cases have the original bounds. A long press also
opens custom positioning with the original target and tooltip
sizes, and the delegate's offset determines painted layout.

The default path uses the actual overlay viewport, not the anchor's
local constraints. Explicit Incular anchor/link options still use
the original composited follower; a native control verifies its
offset. The position callback is held until measured geometry is
available. The 14 focused native checks include Escape, hover, tap,
long press, and all placement variants. This is RawTooltip rather
than a Material or Cupertino tooltip.
