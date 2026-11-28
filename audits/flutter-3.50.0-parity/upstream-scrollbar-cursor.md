# RawScrollbar cursor scope and drag lifecycle

Nine exact pinned `widgets/scrollbar_test.dart:3767–4303`
registrations passed. The native runtime checks the installed
per-device cursor at the original track and content coordinates.
With no resolver, an ancestor I-beam remains active even above
the thumb. A configured cursor is limited to the track/thumb or
an active drag, and current hovered/dragged/disabled state is
available to the resolver without adding a design-system
dependency.

Changing the property over a stationary mouse activates the
new cursor. In a drag, the dragged cursor survives leaving the
track; release off-track hands the cursor back to the ancestor.
Disabling interaction or cancelling the drag clears dragged
state. A touch drag updates a stationary or moving mouse in the
same viewport without requiring mouse ownership. Shrinking the
scroll range clears dragging but preserves the existing hover
until the mouse moves away, as in the pinned original. Adjacent
thumb input, track taps and hover-to-reveal checks also passed.
