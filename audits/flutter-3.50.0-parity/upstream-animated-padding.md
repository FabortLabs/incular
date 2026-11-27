# AnimatedPadding

All four pinned `widgets/animated_padding_test.dart` registrations passed on
Flutter and their native equivalents passed. The exact 800×600 RTL layout
checks include the 750→725→700 right edge, the original start size on the update
frame, and the `easeInOutBack` overshoot clamp to an 800-wide child at 128 ms.
The constrained text case remains zero-sized after settling. The Rust debug
description now reports the target padding, curve, and duration on one line.

A supplemental paired probe also verifies Flutter's optional child behavior:
childless padding occupies 10→20→30 px at 0, 100, and 200 ms and paints no
content. Incular now offers `AnimatedPadding::without_child` while preserving
its existing constructor. Negative targets are rejected at construction in
debug builds, distinct from valid animations whose curves briefly overshoot
and are clamped. Five focused native tests passed.
