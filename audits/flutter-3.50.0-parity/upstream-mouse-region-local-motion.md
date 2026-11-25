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
