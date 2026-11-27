# Rendering mouse tracker callbacks

Reference: `packages/flutter/test/rendering/mouse_tracker_test.dart` on
Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
Native comparison: `incular-runtime/tests/flutter350_mouse_tracker_reference.rs`
through the normalized platform event and a laid-out widget tree.

The hover connection subscription now reports only first-device and
last-device transitions, in order after mouse-region enter/exit callbacks.
Both dynamic variants of the upstream registration (mouse and stylus)
exercise positioned device arrivals, hover, removal, and reconnection.
Button contact can update the tracked position but does not emit hover
while pressed. Touch presence does not connect a hover device or request
an extra frame.

The remaining paired cases verify real nested and disjoint regions, callbacks
that are absent on one region, and transformed enter/hover/exit coordinates,
including exit against the old hit transform when the region moves away from
a stationary pointer. A stationary update never invents a hover sample.
These are normalized runtime tests, not OS-specific stylus event generation.
The upstream synthetic finder disappearance case is separate from real
MouseRegion unmount: a removed widget target does not receive a stale exit.

The nested case exposed an Incular-only off-by-one: a point at the exact
bottom or right edge remained hovered. Flutter's `RenderMouseRegion` uses
the half-open `Size.contains` through `RenderProxyBoxWithHitTestBehavior`.
Incular now applies those bounds in both raw hit collection and the
retained-target path. The authored Flutter and native probes cover top/left
inclusion, an interior subpixel, and bottom/right exclusion. The authored
case earns no original-registration credit.

Nine original registration sites passed on the pinned reference (ten actual
executions). Ten focused native tracker checks and five adjacent per-device
checks passed.
