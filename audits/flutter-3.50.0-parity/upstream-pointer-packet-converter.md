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
chords are never rewritten. A mouse contact keeps even a zero mask.
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

## Raw pan and zoom listener delivery

The exact pinned :668 “Pointer pan/zoom events” case passed. A
mounted `Listener` now observes absolute trackpad start, update
and end separately from aggregate native gestures. Start captures
the path and its transforms, an update outside the view follows
that path, and orphan updates and repeated ends do not fire.
An authored pinned probe uses a 2.5× pixel ratio and a centered 2×
widget transform. The native counterpart agrees on global and
local position, total pan, measured pan delta, scale, rotation,
synthesized state and device/pointer identity. Eight delivery and
12 adjacent scale/transform tests passed. The authored probe
adds no original registration credit.

## Native platform-view hit requests

The pinned :105, :120, :133 and :149 registrations passed. The retained
view now answers using its actual hit path, not just the frontmost
target or the platform view's layout bounds. An empty path and an
ordinary-only path both answer false. With two mounted targets, the
platform surface answers true at either position in the path. An
additional check keeps a transparent platform surface out of the
answer. A query forwards no pointer command. Five focused and nine
adjacent runtime tests passed.

## Failed host dispatch

The pinned :687 “Error handling” registration passed. In Rust the
host pre-dispatch callback uses a typed failure instead of throwing.
A failure on the first down reports the failed event and packet
context exactly once, without calling the handler on the following up
or notifying mounted listeners. When a later packet pumps the queue,
the remaining valid samples resume in arrival order ahead of new
input. The failed down is not replayed and its orphan up reaches no
widget. Shutdown discards deferred packets and cancellation requests.
Ten focused and five adjacent runtime tests passed. This boundary is
distinct from a widget callback throwing after delivery has started.


## Trackpad signals are not contacts

Both original Flutter registrations in gestures/events_test.dart:829
and :839 passed. The packet converter preserves trackpad hover, scroll,
inertia cancellation, and pan/zoom with the original device and view
identity. A signal takes priority over any accompanying contact tag.
Trackpad Down, Move, and Up tagged as contacts now produce no event,
instead of sending a fictitious contact to the retained tree. The old
mouse mask test also exercised those invalid trackpad records; the
mouse coverage is unchanged and the trackpad expectation now checks
the reference boundary.

The allowed types pair, but the rejection case is partial: Rust drops
invalid engine packets, while Flutter additionally asserts on its
individual pointer constructors. Incular's coarse already-logical
RawPointerEvent deliberately remains usable by native adapters.
Eleven packet-converter and twelve mounted delivery tests passed.

## Small pan updates after large cumulative displacement

The pinned Flutter probe `precise_local_pan_delta_test.dart` projects a
large cumulative pan and a separately measured subpixel delta through a
translated scale. The mounted Rust listener used to report a zero local
delta while its global pan delta was still `(0.5, -0.5)`: it subtracted
already-rounded absolute coordinates. The listener now maps the delta
with the linear part of its affine transform. Position and absolute
local pan still use the full transform, including translation. Retained
packet delivery now reports Flutter's `(0.25, -0.25)` local delta.
This authored probe does not add an upstream registration.

## Native input at fractional device pixel ratios

The pinned `fractional_pointer_ratio_test.dart` probe feeds physical
`(10, -10.0000003)` at 1.3× into Flutter's pointer and scroll converter.
The native Winit adapter agrees bit for bit at Incular's f32 output,
with the expected sign reversal for content scroll. Keeping the
coordinates and scale in f64 through division prevents one-ULP cursor
or wheel drift. WindowMetrics shares that implementation and preserves
host precision in the inverse direction too. The desktop shell's
existing full-precision cursor path now uses the same conversion.
All 28 focused adapter and platform tests passed. This authored
probe does not add upstream registration credit.
