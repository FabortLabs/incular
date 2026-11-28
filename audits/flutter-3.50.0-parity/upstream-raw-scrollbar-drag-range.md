# RawScrollbar drag across signed or reversed offsets

Reference: Flutter 3.50.0 at `ab2912035e160d9cad6306df307576da6128ebdb`.

A mounted reversed vertical view starts with a 90-pixel thumb at y=510.
Dragging it upward by 10 pixels puts it at y=500 and increases the
controller offset. The centered viewport tests exercise a real sliver
center with range [-600, 600] and verify that the 200-pixel thumb moves from
y=200 to y=190 while going to a negative logical offset. Those cases ran
under all six modeled platform settings; these are not separate OS runs.

An additional matched authored test varies the range to [-600, 0].
Incular previously hid the thumb because its maximum was zero. The fade
now checks the full span; the thumb is visible at y=300, and a real
10-pixel thumb drag moves it to y=290 and the controller to -20.

The selected pinned originals (1605, and every platform variant of 2170
and 2244), one authored Flutter probe, and all 13 targeted native tests
passed.
