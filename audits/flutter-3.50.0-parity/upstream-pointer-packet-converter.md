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
