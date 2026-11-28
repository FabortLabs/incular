# Drag callback detail descriptions

The exact Flutter 3.50.0 `gestures/drag_test.dart:1596` registration
passed. The authored pinned probe additionally checks the ordered text of
down, start, update and end details rather than only their line counts.
It omits Flutter's ephemeral object identity suffix: the Rust values
are copyable and have no equivalent object identity. A populated source
timestamp is a Rust monotonic timestamp, not a framework Duration.

The native descriptions read the actual positions, device, delta and
velocity. An accessibility drag start has no originating device (rather
than an Unknown device); physical drag starts preserve their reported
kind. The touch-specific draggable path still recognizes physical
touch. All ten focused description, drag timing and mounted accessibility
checks passed. This does not mark the all-gesture diagnostics test in
`gestures/debug_test.dart` paired.
