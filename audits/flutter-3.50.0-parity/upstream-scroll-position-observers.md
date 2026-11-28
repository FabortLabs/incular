# Scroll position listeners, alignment and deferred loading

Pinned `widgets/scroll_position_test.dart:142,173,249` passed.

A position listener is independent of the scroll notification bubble.
The native test mounts a list, jumps to 100 pixels, and sees one
position callback before Start, Update, and End, even when the
notification listener consumes the bubbling event. Jumping to the
same offset and pumping a frame do not call it again. Dropping the
subscription removes it. The native wheel control also confirms a
replacement activity opened by a position listener remains active.
The authored reference pins the order of the jump callbacks.

The seven ensureVisible calls in :173 match at the original 110-pixel
row heights. Keeping the partially visible row at its start leaves
offset zero; keeping it at its end advances to 60; revealing row zero
at its start returns to zero.

The deferral policy in :249 is partial by design: newly materialized
rows defer loading after a 50,000-pixel jump, then load immediately
after a 500-pixel move. Incular's fixed-extent index does not build
every intervening row, so it intentionally does not match Flutter's
total of 102 builder calls. All fifteen focused native tests passed.

## Wheel position and scrolling state

Pinned `widgets/scroll_controller_test.dart:382` passed. A position
listener sees scrolling already active during the original 300-pixel
wheel move. An authored 25-pixel reference probe confirms the full
sequence: scrolling becomes true at offset zero, position and all
three boundary callbacks see offset 25 while scrolling is true, then
the position reports idle again.

The native baseline reported false during the position and End
callbacks. The wheel now publishes the transient scrolling flag
separately from its notification bracket, so a reentrant listener
can still start a real replacement activity without the stale wheel
claiming or closing it. Tests cover takeover both before changing
pixels and from the position callback. All 42 focused native
activity, wheel, position and keyboard-dismissal tests passed.

## Reentrant programmatic jumps

An authored pinned test jumps from 0 to 100, then jumps to 200 from
the position listener. Each position is delivered once. The nested
update reports +100 at offset 200; the outer update reports +200
at offset 200, not a second stale +100. Incular now computes the
bubbling delta from the original offset after synchronous callbacks.
The native baseline failed the second delta; all ten focused native
checks passed after the repair. The authored case adds no upstream
registration.

## Scroll controller lifetime events

Pinned `widgets/scroll_controller_test.dart:412` passed. Each
debug-mode controller now publishes one creation and one final
disposal with the same identity. Cloning and dropping one of its
handles do not create or dispose a second object; a later controller
has a different identity. The tracking field is absent in release
builds. The native baseline had emitted no ScrollController event;
all 33 focused native allocation, attachment and activity checks
passed after adding lifetime tracking.
