# Flutter 3.50 multidrag

Four registrations from pinned `gestures/multidrag_test.dart` match the
targeted Incular test `flutter350_multidrag`: motion beyond slop before the
hold expires never starts a drag (:16); movement within slop starts at 500 ms
and is replayed to the client (:43); touch-only delayed drag ignores a mouse
press and its moves even after expiration (:72); both an omitted button
filter and an explicit default accept primary and reject middle (:105).
Incular exposes explicit reset with `set_allowed_buttons_filter(None)`.
Flutter's Dart memory-event registration (:130) passed only on the pinned
reference; it is not counted as an Incular comparison.

The authored `multidrag_pointer_deadlines_test.dart` probe advances past
three overlapping timers in one pump. Flutter starts the earliest deadline
first, breaks equal deadlines by registration order, and keeps each
client's accumulated movement, later updates, and end/cancel independent.
The corresponding Rust case reproduced the inverse start order when
pointer IDs ran in reverse. Pending timers are now keyed by deadline and
creation order rather than pointer ID, and removed on rejection and
release. This authored comparison is not a fifth upstream registration.
