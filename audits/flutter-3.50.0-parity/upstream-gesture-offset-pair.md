# Paired local and global gesture coordinates

The pinned `gestures/recognizer_test.dart:42` passed. The native public
OffsetPair compares the same starting local and global offsets, and
adds/subtracts both spaces component by component. A native event with
a translated, nonuniform reflected local space verifies both the
position and displacement constructors; only the position incorporates
the translation. The tap-and-drag recognizer now uses the same public
pair and delta addition rather than maintaining a private duplicate.
Two focused coordinate tests and 17 tap-and-drag tests passed.
