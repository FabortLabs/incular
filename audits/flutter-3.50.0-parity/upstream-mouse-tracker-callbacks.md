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

## Rebuilding, remounting and reparenting a hovered region

Pinned `widgets/mouse_region_test.dart:882,930,1029,1818` passed.
Removing a hovered region emits no exit on a dead annotation.
Mounting a replacement under the still pointer produces one enter
after paint, then its reactive text changes next frame. Rebuilding
the same keyed region produces neither enter nor exit. A real
movement from ten pixels outside to ten inside invokes its closure
once with local coordinates and changes the mounted text.

The reparent port uses the original 50/50 then 30/70 row widths and
the stationary pointer at (40,5). The MouseRegion changes parent
without changing its element or renderer, emitting no enter, exit,
or host cursor activation. Its existing Click cursor remains
active. All 29 focused mouse tracker and cursor session tests
passed, including the final enter-count assertion.

### Stylus crossing a mounted MouseRegion

Pinned `widgets/mouse_region_test.dart:2031` and the mounted native
fixture start a stylus outside a 10×10 top-left region at (20,20).
On motion to (5,5) the same stylus produces Enter then Hover; motion
back to (20,20) adds one Exit. The first outside sample emits no
callback. The native fixture checks the actual callback order and both
global and local coordinates, not tablet-driver integration.

### Dragging after the original hovered child detaches

Pinned `widgets/mouse_region_test.dart:1992` passed. The native
fixture presses a centered Draggable with a mouse, holds for two
seconds, then moves (10,10) to start a drag with feedback. Replacing
the original gesture/hover child used to cancel the whole pointer
stream even though that child had lost the arena. The accepted drag
source kept its live route after the fix, the actual feedback moves
by the second (10,10), and the detached child receives no hover
or tap. Cancel restores the original child and removes feedback;
removing the source itself still cancels exactly once.

A separate pinned authored comparison confirms child replacement,
continued drag updates and feedback, no stale hover, and cleanup.
All 33 focused native tracker, feedback, drop-path and drag option
tests passed; the authored case adds no registration credit.
