# RenderStack parent data, sizing and clipping

Five pinned `rendering/stack_test.dart` original registrations passed
(13, 64, 101, 110, 233). Native comparisons exercise every parent-data
mutation, all four zero offsets, an empty tight stack, an empty stack
under a tight flex, and the exact default and explicit clip coverage
recorded when a positioned child overflows.

Positioned children and standalone layout now share one constraint
calculation. Opposing edges win over an explicit dimension. A finite
negative explicit dimension is tight zero, not an unset natural-size
axis. The authored Flutter `positioned_negative_extent_test.dart`
confirms this for a real widget, changing the native regression from
25 pixels to zero. Non-finite options retain Incular's existing finite
layout policy. The empty pure stack takes the biggest size only when
both axes are bounded; mounted and pure layout agree under a singly
unbounded axis.

The existing clip-only update test expected a relayout even though its
comment and the current runtime require none. The same counter failure
was reproduced on the unchanged parent commit; the assertion now
checks that toggling a clip changes painting without changing layout
work or child geometry.

## IndexedStack semantics and inspection

The three pinned `RenderIndexedStack` cases (148, 176, 204) passed,
as did the native retained-tree equivalents. A three-child stack only
exposes the selected second child's semantics. Diagnostics enumerate
all three children as `child 1` through `child 3`; only the selection
is sparse rather than offstage. A null selection exposes no child
semantics and marks every child offstage. The native tests verify each
reported render identifier and that all children remain laid out at
100 by 100, including when hidden.
