# Logical BoxDecoration borders

An authored Flutter 3.50.0 widget confirms that a 40 × 20 container
with a 6-pixel start border and 2-pixel end border gives its child a
32 × 20 content box. The content starts at x=6 in LTR, x=2 in RTL.

The native mounted test checks the same sizes on a direction-only
update, plus the actual red and blue paths on the correct sides.
The retained painter and the container's padding now resolve the same
logical border. A new interpolation case verifies that the physical
horizontal sides fade out before logical sides fade in, including the
exact midpoint and the reversed transition. Two native cases and 11
existing container/gradient/radius controls passed.

The authored mount case is separate from the pinned registration
counter. BoxDecoration-to-ShapeDecoration conversion is a separate
case; this change preserves the original logical border to enable it.
