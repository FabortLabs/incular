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

## Modern KeyboardListener annotation

The separate authored Flutter 3.50.0 probe
`reference-probes/test/keyboard_listener_focus_annotation_test.dart` confirms
the modern listener exposes its supplied node to enclosing focus lookup and
publishes an unfocused/focused accessibility annotation with the Focus action.
Switching off its annotation on rebuild leaves the same node focused and
still delivers key down and up. The Incular native
`flutter350_keyboard_listener_parity` target checks those behaviors and
verifies that the Incular-only listener without a supplied node does not add
a focus stop to its child. This new probe is not an upstream-registration credit.
