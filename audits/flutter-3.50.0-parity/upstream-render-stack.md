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
