# Raw keyboard listener accessibility

Both pinned `widgets/focus_traversal_test.dart:3651,3674` controls
passed. A mounted RawKeyboardListener with an otherwise silent child
publishes a single unfocused, focusable node with only the Focus
action by default. Opting out publishes no semantic node. That does
not remove the live focus node: the native control requests keyboard
focus and observes both normalized key phases while the semantic
tree remains empty.

The compatibility observer never consumes keys and does not emulate
obsolete platform-specific raw event formats. Five targeted keyboard
listener tests and fourteen neighboring semantics checks passed.
