# Physical pointer packet conversion

Nine exact pinned `gestures/gesture_binding_test.dart` registrations
passed (:278, :302, :379, :397, :443, :491, :552, :609, :721).
`incular_gestures::PointerEventConverter` now expands physical records
lazily into the existing logical pointer and pan/zoom payloads. It
retains the distinction between registration, hover, and contact motion.
Already-normalized native events keep their existing path.

The original five-record sequence tests assert the actual button masks:
touch, either stylus kind, and unknown devices get a primary fallback
only on otherwise buttonless down/contact move. Explicit buttons and
chords are never rewritten. Mouse and trackpad keep even a zero mask.
Separate cases check remove/re-add order, zero scroll as a real signal,
both axes' NaN/infinities, and a zero device pixel ratio. Each surviving
record keeps its input view, and the test applies ten different ratios
to verify independent coordinate conversion, including pan/zoom motion.

An additional native test verifies lazy view lookup, skipping a missing
view and unknown signal, physical-to-logical motion/radii, and preserving
opaque platform data only on contact move. Incular also rejects nonfinite
or negative view scales at the embedder boundary, rather than emitting
unusable coordinates. The converter does not replace a window dispatcher:
delivery to hit paths, canceled-pointer queues, and scroll response
callbacks are separate from these conversion comparisons. Ten converter
tests and 22 adjacent resampler, router and signal tests passed.

## Delivery through a retained view

Four further exact pinned `gesture_binding_test.dart` cases passed:
:165 (tap), :182 (move), :201 (hover) and :237 (cancel). A real
mounted Incular listener and the view's global router receive the
converted samples. Down/up reach the tap recognizer without erasing
the changed button, and move/up retain the down hit path even when
the pointer is outside. Measured logical motion and the synthesized
flag survive delivery. Device add/remove reach the global route
only; the three hover samples also reach the listener. Explicit
cancel ends the contact, drops a subsequent orphan up and lets the
same pointer tap on a fresh down.

The view-scoped packet entry also rejects foreign views and ignores
contact motion without a prior down rather than treating it as hover.
Hosts already supplying logical events still derive deltas as before.
Five delivery and ten adjacent boundary-metadata, raw-scale and
touch-resampling tests passed. Queued cancellation and raw pan/zoom
listener delivery are separate from these four comparisons.

## Cancel queued from an input callback

The pinned :254 “Can cancel pointers” registration passed. A mounted
native global route requests cancel while handling the down from a
two-record down/up packet. The request does not reenter the listener.
Cancel reaches the captured listener before the queued up, which is
discarded, and a later contact with the same identity taps normally.
An out-of-band cancellation is deferred to the driver's input
checkpoint; duplicates, unknown contacts, and handles whose view
has been dropped are harmless. Incular retains the contact's device
and last position so cancellation does not masquerade as a new
mouse location. This comparison uses immediate packet delivery;
cancellation requested from an opted-in resampled touch callback is
a separate clock-path case. Seven delivery and nine adjacent native
tests passed.
