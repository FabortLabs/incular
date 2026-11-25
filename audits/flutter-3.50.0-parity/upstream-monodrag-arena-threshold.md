# Monodrag arena ownership and callback thresholds

Pinned `gestures/monodrag_test.dart` :16, :71 and :102 ran
and have complete paired Rust assertions.

- An initial Down records its original timestamp until the
  drag starts. Acceptance clears that pending timestamp; a
  later rejection and duplicate accept leave it clear and do
  not replay onStart. Before the repair, Incular returned a
  null source timestamp for a drag that won at the press.
- With `only_accept_drag_on_threshold` enabled, the same
  uncontested arena can resolve at the press without starting
  the drag. Down→Up with no movement emits no start, update, or
  end. With the flag off, the exact same press emits start and
  end, with no update.

Both authored Flutter and Rust probes then move 17, 18, and
19px with the flag enabled and an already-owned arena. Start
fires only at 19px, with the crossing coordinate and
timestamp; the crossing move is not replayed as an update
under Start behavior. The Rust control also reuses one
recognizer on a new press to ensure the previous arena win
does not bypass slop in a later stream.

## Start, Down, and event timestamps

Seven exact `gestures/drag_test.dart` registrations are paired.
:85 and :107 use the winning movement at (30,0) for default
and explicit Start behavior, with no replayed update. :460
holds (10,10) as the start under Down behavior and projects
the first update to (30,10) with delta (20,0).

:137, :1603, and :1665 have the same exact reporter name;
all three were selected and run together. A sole horizontal
drag emits one start, then the original movement and a
stationary zero-delta event as distinct updates, then an end
on release. Diagonal movement retains its true world position
while each update drops the y component of its delta. :423
anchors the monotonic test clock and observes the original
100/200/300ms timestamps, including the stationary sample.

## A pan wins after its only sibling tap rejects

The pinned `gestures/drag_test.dart:14` keeps the pan pending
through the press and 14px diagonal move. At 22px, the tap
has rejected and the pan becomes the default winner, despite
remaining below normal pan slop. A subsequent move produces
(0,-5), and release sends exactly one end without a tap.

Before the repair the combined Incular detector waited for
pan slop and did not start. The new default extends the same
sole-member fallback used by the prior axis drag repair. It
still does not override an explicit slop requirement or steal
a claim from another detector in the same pointer arena.

## Button changes terminate the current drag

Four pinned `gestures/drag_test.dart` registrations (:1806,
:1824, :1857, :1880) check a real button-mask change on a
mouse move. Before winning, a changed press reports cancel
immediately, even if that event would have crossed slop.
After winning, it emits end without applying the changed
move, and later moves of that pointer produce nothing.
The reused recognizer still starts and ends the following
unmodified pointer normally.

The core routes the changed sample through the same terminal
path as a cancelled drag; prior state decides cancel versus
end. It keeps the prior drag position and Flutter's ordering
of velocity sampling before the button check. Ordinary tap
and long-press button controls remain green.

## Supported device filtering

Pinned `gestures/drag_test.dart:189,241,1699` verifies that touch-only
horizontal and vertical recognizers ignore a mouse sequence and that a
mouse-only recognizer ignores touch before handling a normal mouse drag.
Incular now filters on pointer down before entering the gesture arena. An
omitted device set accepts every device; an explicitly empty set accepts none.
Subsequent events for an accepted pointer retain its original device.

The three matching assertions run in `flutter350_drag_supported_devices`.
The separate retained-tree test `flutter350_gesture_device_allowlist` verifies
that an excluded inner detector does not delay an eligible ancestor, and that
the touch-only child still wins for touch. Existing button-change, transformed
drag, device slop and tap/drag targets also passed.
