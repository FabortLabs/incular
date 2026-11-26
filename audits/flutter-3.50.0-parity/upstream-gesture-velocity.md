
## Public velocity values

Pinned `gestures/velocity_tracker_test.dart:52` pairs. The velocity
already used by drag callbacks now supports addition, subtraction,
negation, equality-compatible hashing and a single-line, one-decimal
diagnostic. Signed zero compares and hashes the same. Clamping scales
the vector without changing its direction and leaves in-range values
alone. The pinned original and 14 focused native checks passed,
including adjacent drag and trackpad controls.
