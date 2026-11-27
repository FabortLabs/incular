# Offscreen semantics in an ordinary scroll box

Pinned Flutter `widgets/semantics_test.dart:2460` passed: Label text 10
exists while offscreen and becomes visible after a 500 px scroll.
The matching Incular fixture `flutter350_box_scroll_semantics_reveal`
mounts the same 20 labels separated by 50 px boxes, in a
`SingleChildScrollView` with explicit child semantics. It verifies the
label's retained ID, exact translated bounds and hidden state at offset
0, 500, and back at 0. The desktop AccessKit projection tracks the same
identity and hidden-state round trip. No physical screen reader is
required for this projection check.
