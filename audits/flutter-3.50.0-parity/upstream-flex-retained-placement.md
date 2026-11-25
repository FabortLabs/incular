# RenderFlex spacing and retained child placement

Nine original registrations in pinned `rendering/flex_test.dart` passed.
Eight have matching retained behavior; the ninth only compares public
default values because the full Dart diagnostic tree is not portable.

| Registration | Compared behavior |
| --- | --- |
| :216 | Changing the second child's flex factor from zero to one keeps the same child and reassigns its width from 0px to 100px. The first child stays zero-sized. |
| :236 | Stretch sets the full 100px cross-axis size of both children, without changing which child gets the main-axis share. The retained children still receive the correct share after flipping to vertical. |
| :265 | With no fixed spacing, three 100px children use offsets [50,200,350] in a 500px row and [25,150,275] in a 400px column. |
| :303, :338, :377 | Start, end and center preserve 14px gaps in both axes and place the remaining free space in the expected outer positions. |
| :416, :455 | Space-evenly and space-around distribute the unused space while applying the 14px fixed spacing only between children. The fractional space-around positions agree with the pinned tolerance. |
| :194 | Horizontal, Start, Max, Center, Down and zero spacing match. This does not reproduce Dart's debug tree text. |

The Rust tests drive real layout and descriptor updates through Runtime,
not just a formula. The spacing cases assert both child dimensions and
positions and verify the keyed elements retain their identity when
the axis changes.

## Overflow-dependent clipping

Pinned `:62` verifies all Flex clip modes. Incular now records the
same default/no-clip, hard-edge, antialias and isolated save-layer
coverages at the actual 100×100 bounds. Tests inspect that the
child painting is inside the clip rather than only querying the
descriptor. The pinned debug warning and striped overflow
indicator are not claimed by this test, so the registration
remains partial.

An authored paired reference resizes one retained overflowing flex
from 100px to 250px and back. The clip disappears once content fits
and reappears when main-axis overflow returns. Rows and columns
also expose the same option in Incular. The existing placement
batch stays green.
