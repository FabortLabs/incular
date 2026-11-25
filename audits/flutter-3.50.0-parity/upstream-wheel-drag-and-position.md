# List-wheel drag and position

The pinned `widgets/list_wheel_scroll_view_test.dart:1415` and its
mounted Rust counterpart now match. A 49-pixel drag does not change
selection, one more pixel selects item 11, and reversing by 70 pixels
selects item 10 again. The viewport-owned controller and callback agree
at every checkpoint. Before the change, no touch movement reached the
wheel because the viewport was not an eligible scroll-arena member.

ListWheelScrollView now claims its viewport bounds and joins the
same axis-specific, lease-checked scroll route as other scroll views.
The lower-level ListWheelViewport remains a rendering-only primitive.
An authored paired fixture verifies 60 pixels of direct drag and
settling within one logical pixel of the 100-pixel item boundary at
device scale 1. Exact floating-point phase samples are not claimed.
Thirty-one focused Rust drag, input, and wheel-model checks passed.

## Fixed initial position after detachment

Pinned :1540 now matches: a controller set to item 10 at 1000 pixels
while attached starts again at initial item 3 (300 pixels) when a new
wheel position attaches. It does not reset merely because the same
position relayouts. An authored paired case mounts the same controller
at 80-pixel items, moves it to item 10, then unmounts and remounts it
at 120-pixel items. Both sides start at 240 and 360 pixels respectively.

The initial item is resolved in the checked metrics publication, not a
one-shot deferred jump. The real item extent beats a caller's fallback
extent, and an explicit deferred jump still takes precedence over the
initial item. Ordinary scroll controllers keep their existing behavior.
Fifty-five focused runtime, wheel-model, and controller tests passed.
Replacing a controller on a still-mounted wheel is a separate case.
