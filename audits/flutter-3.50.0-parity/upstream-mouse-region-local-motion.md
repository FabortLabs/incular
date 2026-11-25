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
