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
