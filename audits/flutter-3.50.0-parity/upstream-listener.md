# Listener routing and coordinates

All eight original registrations in pinned Flutter `widgets/listener_test.dart`
passed. The focused Rust target `flutter350_listener_parity` verifies:

- Line 15: a touch down bubbles bottom → middle → top through a decorated subtree.
- Line 45: a buttonless touch move, after adding the pointer, emits exactly one hover.
- Lines 70, 136, 207 and 280: touch down/move/up and a fresh wheel signal
  report the expected global/local positions and deltas under translation,
  scaling, their combination, and rotation. These remain partial because
  Incular's public raw event does not expose the conversion matrix itself.

The two Flutter-specific RenderPointerListener diagnostic registrations at
lines 353 and 376 remain reference-only. The separate
`listener_captured_transform_test.dart` pair checks transforms across a
retained reposition during an active press (G02 in the behavior audit).
