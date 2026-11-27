# MouseRegion local motion

The pinned `widgets/mouse_region_test.dart:168,207,241,788` registrations
passed. Before this repair Incular sent window coordinates as the local
coordinates to region enter, hover and exit callbacks. The Rust ports now
assert local (50,50) at global (400,300) and exit local (-349,-249) at
global (1,1). Holding a mouse button still allows enter/exit, without a
hover callback. An unpressed move delivers enter before hover and only
exit when moving outside.

A 2x scale produces a 300x200 hit area for a 150x100 child. The tested
edge crossing, 198-pixel global move, 99-pixel local delta and outside
position all agree. No device-added event is modeled as a window enter;
these cases start with a real positioned sample outside the region.

## Stationary pointer

Both existing original cases :393 and :433 were rerun. Moving the widget
under a still mouse uses its new inverse transform for enter; moving it
away uses the previous hit transform for exit. Unmount still emits no
stale exit. The 19 focused lifecycle and pointer tests passed. The
separate cursor-repaint assertions :1665 and :1726 remain red on the
recorded paint count even though the effective cursor updates.

## Positioned device presence

Pinned :274, :299 and :336 are paired at the normalized host boundary.
An added mouse enters the region at its known position with no ordinary
hover. Removal emits one localized exit, without hover/re-entry on the
next frame. A newly mounted region under a still pointer gets one enter
and no hover. The earlier app could not represent a positioned device
arrival independently of Winit's positionless window entry.

The new portable event can be routed to a retained window or offset
from a transient surface. Native CursorEntered/CursorLeft behavior is
unchanged; no particular OS device plug/unplug notification is claimed.
Twenty-two focused Rust tests and a serial desktop crate check passed.

## Independent device positions

Both pinned `rendering/mouse_tracker_test.dart:168,177` originals pass.
Removing an unknown device first is inert. Two mice add, move, and
leave independently, with hover still active until the last one leaves.

The paired authored checks also demonstrate two baseline defects:
a touch in a different region changed the active mouse cursor and
subsequent layout synthesized an exit and enter at the touch position;
remeasuring a scene with two stationary mice refreshed only the last one.
Positioned hover samples now stay separate from contact input. Relayout
uses each device's own last position in connection order, and a new
pointer sequence on the same physical device does not re-enter. When
the last active device is removed, the single native-window cursor
falls back to the most recent remaining sample.

The existing two list-wheel regressions reproduced with and without
this patch and remain tracked separately. No OS plug/unplug, cursor
selection across different windows, or cross-sequence delta is claimed.

## Hit policy and touch hover

Five more pinned originals pass at :70, :96, :136, :473 and :505.
A childless deferred region is not targeted, but switching it to
default opacity now enters under the same stationary pointer.
A non-opaque deferred region over a hit child and an empty translucent
region both let the lower mouse region participate. The Rust Default
implementation now matches the public no-child constructor.

A buttonless touch-hover sample invokes onHover in local coordinates
without changing mouse tracking. Active touch motion, including a
normalized contact move without button bits, is not such a hover.
Nested motion enters both regions and only exits the child when moving
into the parent's padding. Two authored probes also verify active touch
motion and toggling opacity without re-entering the top region.
Thirty-five focused Rust checks passed.

## Overlapping regions and absorber boundaries

Four originals from `widgets/mouse_region_test.dart:1362,1408,1454,1475`
pair. The native scene replays every position in the three-region
fixture with the exact enter, hover, and exit order, including the
default-opaque variant. A childless opaque region still blocks the
underlying region over its own size.

Five authored Flutter checks and 25 focused native checks pass.
A MouseRegion that defers to a child now requires an accepted child
hit, not merely an enclosing layout box. If that child ignores
pointers, the underlying region receives both hover and taps.
An accepting opaque MouseRegion blocks the siblings behind it,
while non-opaque or no-child translucent annotations can observe
hover without stealing the retained tap. Both sides enter two
overlapping regions in back-to-front order.

The old combined hit-test failure had a second, independent cause:
the AbsorbPointer at y=60 incorrectly claimed a hit at y=25 because
the absorber returned before checking its own bounds. The raw and
retained paths now constrain absorption to its layout box, including
the top/left edges but excluding the bottom/right edges. Both
probes exercise real taps inside, outside, and at the edges.
The existing regression still asserts the underlying ActionId(1);
its hover expectation now matches the pinned Flutter deferring
region instead of treating a rejected child as a hit.

```sh
cargo test --locked -j 1 -p incular-widgets --test flutter350_mouse_region_overlap --test pointer_hit_test --test pointer_hover --test pointer_event_fields --test flutter350_absorb_pointer_upstream_parity -- --test-threads=1
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe mouse_region_overlap_test.dart
```

Strict existing frame rechecks after the overlap repair also match:
`focus_input__listener_pointer_lifecycle` (4 frames),
`focus_input__absorb_pointer_blocks_sibling` (2), and
`focus_input__ignore_pointer_falls_through` (2). These are
rechecks of previously accounted registrations.

## Hover transfer and feedback across frames

Five exact pinned cases and their native counterparts pass:
`widgets/mouse_region_test.dart:580,982,1102,1152,1203`.

- Between adjacent 100px centered regions, the mouse exits the old
  region before entering and hovering the new one. The exit uses the
  new global cursor position in the previous region's local coordinates.
  Moving outside and removing both regions adds no duplicate events.
- Unplugging at (400,300) emits exactly one exit at that position,
  local (50,50), with zero motion delta and no additional hover.
- Mounting or moving a region under a still pointer records one enter
  but leaves both rendered feedback widgets unchanged in that frame.
  The next frame rebuilds both and settles without another request.
  Unmounting while still hovered leaves the outer feedback unchanged,
  sends no synthetic exit, and queues no extra frame.

Five new mounted runtime tests and 26 adjacent checks passed. These
compare visible feedback and scheduling, not the internal phase in
which a callback can affect an unrelated custom painter.

## Callback changes and paint ordering

Pinned `widgets/mouse_region_test.dart:1050,1074,1513,1609` all pass.
The actual retained painter runs exactly once with or without a
connected mouse. Replacing only callbacks does not repaint, and the
next physical motion uses only the replacement hover callback.

Changing opacity from blocking to nonblocking must repaint before
the stationary pointer enters the newly uncovered region. The native
fixture used to log `hover-enter` before `paint`. The runtime now uses
the final hit route once, applies native button styling before paint,
then delivers raw enter/exit and cursor changes after paint. A single
device's saved hit stays inline. Physical pointer motion continues to
deliver its enter, exit and hover callbacks immediately.

Three direct native fixtures and 60 adjacent pointer, cursor and input checks passed.
