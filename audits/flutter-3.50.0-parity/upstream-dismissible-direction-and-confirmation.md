# Dismissible direction, thresholds, and confirmation lifecycle

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

All seven selected original Dismissible registrations passed.
The Rust target `flutter350_dismissible_direction_threshold` has
four passing scoped tests through actual Runtime pointer routing.

- Direction EndToStart accepts a physical left drag under LTR
  (:351), and right under RTL (:383). A reverse-direction drag
  never dismisses, followed by an accepted control in the same
  retained scene.
- A threshold of 1 disables only StartToEnd; an opposite drag
  still succeeds (:551).
- With no resize duration, a full-extent drag calls Dismiss
  exactly once by the first release frame and never calls Resize
  (:891), on both frameworks.

The first three compare neutral single-control callback behavior
rather than Flutter's list removal and multi-item fixture and are
partial. No resize is paired on both observable callbacks and the
first frame after a complete drag.

Pending-confirmation removal, re-drag suppression, and a later
false result pass in Flutter (:781, :843, :869). Incular's public
`confirm_dismiss` takes `Fn(DismissDirection) -> bool` and consumes
it synchronously on slide completion, with no pending Future
or public equivalent. These are reference-only rather than tests
that simulate asynchronous state by calling a synchronous hook.
