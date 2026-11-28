# Mounted text-selection gesture callbacks

The 12 pinned `widgets/text_selection_test.dart` cases from :144 to
:470 run the neutral widget's real pointer arena. The native detector
pairs:

- The six-tap down count, seven rapid single/double/triple phases, and a
  second press held down without an extra single-up or long-press start.
- The 20 ms and 120 ms diagonal swipes: one down and no single-up,
  cancel, double-down or long-press callback at the asserted boundaries.
- Four force presses with a reused pointer ID; and a normal tap followed
  by a force press that wins without a double tap.
- A two-second touch hold versus a mouse hold, and the complete
  start/update/end sequence of touch, mouse, and delayed mouse drags.

This is a mounted RawGestureDetector, not a synthetic callback trace.
The callbacks receive real event details from the retained underlying
recognizers. Tests also verify the Apple/Windows tap-count policies and
that rebuilding callbacks preserves an ongoing tap series. A stale series
resets before its next tracked press, not in an idle frame.

The force capability is decided at Down, but the raw range is read from
each later sample. Previously a supported press that switched to a valid
0–1 movement range could not start a force press; the native tests now
use the original event ranges. No hardware pressure or pointer timing
is inferred from the Linux host.

An authored reference probe also checks pressure on the initial Down.
With a force-capable range, a sample already over the threshold wins
before Up, then reports force end without firing the competing tap.
An unsupported pressure range does not eagerly enter force recognition
and still allows the normal tap. The mounted native regression covers
both branches. The raw arena remembers the first eager acceptance while
other Down contenders join, then settles it before later samples. This
probe adds behavior coverage without incrementing the upstream case count.

## Long press in a mounted editor

Pinned registrations :497, :520, :538, and :558 passed all eight Flutter
variants. In the mounted native counterpart, focused iOS and macOS select
the precise caret; an unfocused iOS editor and the Android, Fuchsia,
Linux, and Windows variants select the word. The test waits across the
500 ms recognition boundary, observes the selection cause, and verifies
that the toolbar opens on release, not while the finger is still down.
Desktop tap-down's prior selection is distinct from the hold. The macOS
case moves that caret before the recognition deadline so the long-press
selection is observable even when the original press coordinate does not move.

For :558, a touch Down remains below the deadline while the scroll
controller jumps to 8,000 logical pixels. Both the first row and its
editor are disposed. Crossing the former deadline and releasing the
contact leaves focus, the editing value, and the old toolbar unchanged.
The comparison covers framework behavior, not OS toolbar presentation.
