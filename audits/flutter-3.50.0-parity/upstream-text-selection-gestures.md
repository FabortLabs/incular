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

## Drag device and competing renderer

Pinned :871, :896, :959, :1015 and :1086 passed nine exact Flutter variants.
The mounted native regressions verify that mouse selection does not open
touch handles or the toolbar (including on a read-only field); mouse and
stylus select from offsets 4 to 10, while Android touch and unknown-device
drags move a collapsed caret to offset 10. The stylus variants cover
Android, Fuchsia, Linux, macOS and Windows.

A narrower neutral probe demonstrates why a rapid tap followed by a drag
doesn't necessarily select a word range: when the renderer participates,
its tap wins the completed first contact and the outer builder's drag
restarts at count 1. When the renderer ignores input, the builder can
carry its own tap into a count-2 drag and select the word range 2 to 14.
The mounted Incular builder now preserves that distinction on the same
editor, without adding a new retained layout node or changing callers
that install no builder. Without it the rapid Android finger drag
incorrectly selected 2 to 14 when the expected value was a caret at 10.

Builder-level force press remains a separate comparison. The generic
detector's pressure recognition does not establish the builder's editor
selection and toolbar behavior.

## Drag anchor during editor scroll

Pinned :1145 checks a vertical viewport-offset change from 20 to 150
while a mouse drag is captured; the original global anchor adjusts by
exactly 130 pixels but the end follows the mouse. The native mounted
editor verifies the equivalent stable document anchor, its projected
point after scrolling, and the actual selection extent at the release.

A second, neutral reference fixture verifies the same contract during
a word-wise double-press drag. The prior native code remapped the original
screen coordinate against the new viewport, moving the anchored word
from offset 303 to 503 while its extent was already correct at 799.
The runtime now captures the initial point in content coordinates once
the word drag starts and resolves that point against live text layout.
This preserves the anchor without freezing the current pointer, and
without caching a byte offset that could be invalidated by text changes.

## Selection overlay hit area and menu

The exact iOS variant at :1246 and the non-web context menu case :2051
pass on the pin. The mounted native regressions exercise hit routing, not
just visual geometry. A focused iOS editor shows one collapsed handle;
its 48 × 48 logical-pixel hit region fits inside the field dimensions.
A physical touch on the knob triggers its callback, while an outside touch
reaches the sibling's tap handler without invoking the handle again.

On Android, the caller's actual context-menu widget is absent before
the click at text offset 0, then mounted in the overlay after a real
secondary-button tap. Its bounded child has the expected dimensions
and the editor remains focused. This does not compare the browser's
native context menu or any design-system appearance.

## Mouse selection scrolls to the document edge

Pinned :1843 and :1917 both pass. The mounted native test starts with an
unfocused editor and unset selection inside a 64 logical-pixel outer
viewport. A finger tap focuses the editor and places the caret precisely at
offset 4. A mouse drag then extends to the document end and moves the
outer scroll controller to its maximum. The uncapped editor does not
scroll privately; a six-line cap requires both outer and inner scrolling.

Before the fix, the outer viewport eagerly won the ordinary pointer
arena on a stationary press. That consumed the release, so the editor
never received its confirmed tap and stayed unfocused. The retained
scroll recognizer now keeps competing when a hit editor also has a
runtime tap, rather than treating itself as the only contender. A drag
can still take ownership after its normal movement threshold.
