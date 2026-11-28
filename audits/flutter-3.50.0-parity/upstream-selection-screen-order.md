# Selection screen order

This authored differential isolates the positional rule in Flutter
3.50.0 `MultiSelectableSelectionContainerDelegate`. Two labels on
one line are placed at x=20 and x=140, then declared in reverse order.
The original declaration order is also run as a control. A mouse click
focuses the region; Ctrl+A and Ctrl+C must produce `LeftRight` in both
the selection callback and the clipboard.

The pinned Flutter probe passed. The initial native Runtime probe
returned `RightLeft` for the reversed layout. After deriving order
from transformed bounds it passes both cases. Each direct proxy is
ordered as one entry and expands to a contiguous child range, so
nested local ranges and custom formatting remain intact. The paint
path reuses the last command order instead of sorting independently
for every highlighted leaf. Numeric tests cover the strict 3 logical
pixel vertical threshold, containment, and stable ties.

The pinned authored control, two numeric tests and 23 targeted native
runtime/widget tests passed. The authored control does not count
toward the 8,856 original registrations. The distinct inline WidgetSpan
original at `selectable_region_test.dart:6603` needs its own matching
fixture and is not credited by this change.

## Inline WidgetSpan copy

The exact original at line 6603 is now paired. The focused native
Runtime fixture mounts the same single line with a real middle-aligned
inline Text child, clicks the paragraph, selects all, and copies
through the clipboard bridge. Before the fix it copied the U+FFFC
layout marker instead of the child. It now returns
`Hello my name is Dash.` and reports the corresponding UTF-16 content
range. Only actual, laid-out, selectable inline content contributes
plain text. This does not claim Web selection or independently
dragging a caret within the inline child.

The pinned original and 24 focused native selection/runtime tests
passed. Added one original to the paired ledger.
