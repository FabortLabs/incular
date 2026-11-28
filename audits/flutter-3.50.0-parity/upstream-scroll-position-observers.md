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
