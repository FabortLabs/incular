# Basic widgets inside a zero-area view

The five pinned `widgets/basic_test.dart` cases for LimitedBox, Padding,
Offstage, childless RawImage, and Builder passed. Each native counterpart
mounts the same widget or placeholder arrangement in LTR Center, builds a
real frame at exactly zero logical width and height, and checks the
mounted render bounds. Padding has five logical pixels on each edge, so
it also proves those insets do not turn a zero-sized view into a positive
box. Native: `incular-runtime --test flutter350_basic_zero_area` (5 passed).
The separate StatefulBuilder and ColoredBox cases are not counted here.
