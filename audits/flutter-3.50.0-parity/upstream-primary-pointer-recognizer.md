# Primary pointer recognizer lifecycle

Four pinned registrations in `gestures/recognizer_test.dart` passed
(:60, :93, :133, and :168, under `PrimaryPointerGestureRecognizer`).
The native base is driven by the real explicit gesture arena and
pointer router. It starts ready, records the initial global and local
points on the primary down, and retains the last primary ID after
clearing the sequence. A winner stays possible until its last route
finishes. A rejected sequence becomes defunct, ignores later primary
callbacks, and returns to ready when the final contact is released.

The same recognizer can be reused after a completed sequence. Its
pre-accept tolerance uses 18 logical pixels by default, reads the
current device override, or uses the explicit finite or unlimited
policy. It compares distance in global coordinates even through a
scaled route. Eight native cases also check post-accept cancellation,
single deadline delivery, multiple contacts, cleanup on disposal, and
late arena decisions that must not affect a new sequence. The
adjacent arena and pointer router cases brought the focused total
to 29. These are direct recognizer comparisons; they do not replace
the retained widgets' existing live gesture arbitration.
