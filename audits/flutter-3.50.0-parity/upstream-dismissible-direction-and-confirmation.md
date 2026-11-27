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

## Pending confirmation now matches

Pinned `:781`, `:843`, and `:869` now pair. The native fixture uses
the original nine fixed-height list rows, padding and cross-axis slide.
A completed drag and a 1000-pixel-per-second fling each ask exactly
once. Removing the row while the decision is pending makes either
answer a no-op, even if an application still owns a copy of the
immutable descriptor. A second drag cannot move the row or ask again
while pending. False and null both return the child to its exact
original position. An accepted native control collapses it and calls
dismissal once.

`confirm_dismiss_async` uses the existing UI-thread future and
completer. The mounted subscription detaches on removal; the pending
decision does not spin frames, and completion wakes the next
transition. The synchronous confirmation API remains available.

The three selected Flutter references and 24 focused Rust tests passed.

## Disabled rows do not capture scroll drags

Pinned `:1037`, `:1053`, and `:1127` pair. With direction None, none
of the four drag directions dismisses the item. The scrolling variant
stays at zero for the two horizontal gestures and the downward drag,
then moves exactly 100 pixels for the upward drag. The native baseline
remained at zero because the disabled row still put a vertical
recognizer in the gesture arena. The disabled shell now contributes
no drag callbacks and defers hit testing to its child. Switching from
horizontal to None keeps the same child element and render object.
All three references and ten focused Rust runtime tests passed.

## Opaque gesture surfaces stop raw sibling hits

Pinned `:976` pairs. In the original overlapping stack, the rear raw
Listener receives the touch for DeferToChild and Translucent over
the empty foreground child, but not the default or explicit Opaque
behavior. Previously the retained gesture route chose the opaque
foreground while the shared raw hit path still reached the sibling.
The raw sibling walk now stops at that opaque gesture surface.
The exact pinned case and 28 focused Rust tests passed, including
shared hit reuse, mouse regions and scroll overlays.
