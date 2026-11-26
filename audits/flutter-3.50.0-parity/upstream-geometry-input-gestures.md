# Alignment, insets, text state, and gesture ports

Pinned Flutter 3.50.0 `ab2912035e`; Incular production remains
`c647cb85`. The exact upstream runner selects named originals
in the pinned checkout and uses only an offline scratch package.

## Geometry and composited followers

`painting/alignment_test.dart:117` matches all directional resolution
and value equality expectations. Factories/static members at 338/343
match the resulting values, but Rust exposes two separate types instead
of Flutter's shared public constructor/constant namespace. Both are
partial matches. `painting/edge_insets_test.dart:41,134` match all
directional constructor and resolve cases, including asymmetric
individual edges. At line 12 the physical constructor values match;
Rust has no separate absolute-insets resolve call, recorded as partial.

Independent paired regression: `Alignment::along_size(100×60)`
should place top-left at (0,0), center at (50,30), bottom-right at
(100,60). Incular instead returns center-relative offsets; the
off-range alignment (1.5,-2) also differs by exactly (50,30).
See `flutter350_alignment_contracts` and
`alignment_along_size_reference_test.dart`.

This helper is **not** the cause of a linked follower's scale gap:
linked widgets use a different `LayerAnchor` with correct top-left
math. With a 100×60 leader at (100,90), 30×20 follower, bottom-right
target anchor, top-center follower anchor and offset (7,11), both
paint (192,161,30,20) at scale 1. At leader scale 2, Flutter paints
(284,232,60,40), while Incular's composited stream has
(299,232,30,20). Flutter's pinned reference verifies actual pixels;
Incular checks its flattened paint and bounds, not GPU readback
for this new fixture. See `flutter_follower_transform_parity` and
`follower_transform_reference_test.dart`.

### Linked affine repair

Flutter's FollowerLayer multiplies the complete leader transform by the
leader-space anchor delta, then expresses that result relative to the
follower's parent. Matching only the world anchor left the follower with
the wrong size and angle. Incular now shares the same world transform
between compositing and widget geometry, input and semantics. The focused
runtime fixture measures the corrected (284,232,60,40) scaled rectangle;
both follower scale directions are also covered by the widget tests.

The six original widget registrations at lines 123, 131, 139, 186, 197
and 208 of `composited_transform_test.dart` pass in both implementations.
Incular checks the actual composited point, not an independently rebuilt
transform. The three offset-only coordinates are exact; the rotation
pairs agree within 0.001 px in Rust's single-precision coordinates.
An additional test assertion checks both axes follow the leader when
the unlinked follower parent is itself rotated by a different angle.
The authored scaled Flutter fixture checks raster pixels; the Rust
follower regression checks the flattened display list, not native pixels.

The three nested layouts at lines 270, 281 and 292 also pass. These add
padding, an unlinked outer follower and skew before the linked inner
follower. Both sides also lay out a linked target and follower at zero
area without an exception (line 401). Line 373 now also pairs: debug
runtime frames report the paint-order error when a follower encloses
its own target. A retained update correcting the hierarchy restores
publication. Incular's acyclic forward links between sibling layers
remain supported; the diagnostic only rejects the nested self-cycle.

## Text state

`services/text_input_test.dart:55` matches reversed-selection deletion.
At :78 and :94 replacing ASCII before or after the reversed selection
produces the right text but collapses the selection in Incular.
Flutter preserves (14,6) or (13,5), Incular reports (6,6) or (15,15).
The `text_editing_delta_test.dart:40` composing insertion payload
and apply round trip match, but no Flutter JSON/subclass/oldText
decoding counterpart was asserted. See
`flutter350_services_text_state_parity`.

## Gestures

Three original standalone velocity-tracker checks passed on Flutter.
Incular's raw tracker and estimate are private, so those remain open
rather than paired. Authored public touch-pan probes match at both
40 ms and 41 ms sample/idle boundaries with aligned receipt/event
clocks, checking velocity vectors (not counting the originals).
See `flutter350_pan_velocity_sample_windows`.

Three original arena cases match Rust terminal winner/loser selection
after both members register; Flutter's explicit close/callback lifecycle
has no direct Rust counterpart, so count partial matches only.
A separate direct public API probe reproduces late rejection rewriting
the winning Rust entry to rejected. Flutter emits the original
terminal callbacks and ignores later resolutions. This was not
reproduced through routed widget input. See
`flutter350_arena_winner_decisions` and its pinned reference.

## Standalone velocity tracker comparisons

Six exact Flutter registrations at
`gestures/velocity_tracker_test.dart:64,77,138,158,177,196` passed.
Rust in-crate tests exercise the actual estimators, not a copy: the
pinned interrupted trace's 48 ms gap leaves only the later samples
and matches (649.5,3890.3) within the original 0.1% tolerance. Empty
base history is zero; after the original ten positions and an idle
interval the base, iOS and macOS estimators all return zero. The iOS
weighting also reports exactly (0,−1) once four samples of the
constant trajectory exist. Explicit timestamps replace the Dart test
clock. All four focused Rust tests passed. This is behavior coverage,
not equivalence of public estimator types or host touch events.

## Recorded drag velocity and scratch allocation

The exact `velocity_tracker_test.dart:38` replay passed. Incular uses
all 306 numerical events from the pinned original fixture, preserving
history across its 13 releases, and all 26 velocity components agree
within the reference's 0.1% tolerance. Releases estimate immediately
after the last delivered sample as in Flutter's fast replay. The same
vectors held before and after replacing the three heap scratch buffers
with bounded stack arrays. Ten focused tracker and pan tests passed;
the separate, already documented synthesized scale-move mismatch
still reproduced in a broader run and is not addressed here.

## Terminal arena resolution

The authored late-rejection probe now matches Flutter. A winner's late
rejection previously rewrote both entries to rejected; it now leaves
the original winner and loser unchanged. A still-pending simultaneous
member can reject separately, and canceling the live stream still
clears it. Eight focused Rust checks and the pinned authored probe
passed. Flutter's separate close/sweep/hold lifecycle remains outside
this authored repair, so this does not change the upstream case count.

### Scale contact eligibility and callback details

Pinned scale_test.dart :278/:312 compare with the standalone scale
recognizer's supported-device set. A rejected mouse cannot participate
before a touch starts or add, move, end, or change the pointer count of
an active touch scale. The sequence checks start at (0,0), first movement
to (20,30), and the final genuine touch release. Rejected device events
also leave the last valid event timestamp unchanged.

Pinned :722 pairs start/update/end pointer counts after changing from
one contact to two, back to one, and finally zero. Adding a finger sends
an End with the new count; its first move starts a new segment. This
uses the recognizer's actual `accept` API for the sole arena member.

Pinned :1506 verifies source timestamps on the first of one, two, and
three contact segments, including repeat timestamps and continued
one-finger panning. The native trace checks the same start, update, end,
span and focal delta checkpoints. The mounted tap-competition check
below verifies the same callback timing. Pinned :1832 is partial: native
start details report touch, mouse, stylus, inverted stylus, and unknown.
Flutter also uses a raw pan/zoom trackpad contact; Incular's aggregate
trackpad route remains distinct from this scale recognizer.

### Scale contact admission after the first win

Pinned scale_test.dart :380 and the mounted nested scale/drag scene
produce the same event trace: the 40 px vertical move wins scale;
adding a second finger ends that segment; its 20 px horizontal move
must restart and update scale, not start the competing horizontal drag.
After both lift, the first-in-route scale again wins a fast 70 px
horizontal move. Before this fix, the second move emitted scale-start
followed by drag-start and silently dropped the scale update. Pending
contacts now resolve in the existing scale's arena before dispatch, with
no additional arena walk during updates that did not add a contact.

Pinned :1506 also pairs with the mounted tap competitor: the first
start reports the original 10 ms down stamp, the subsequent starts
report their 40, 90, 130 and 150 ms restart motions, and repeated 140 ms
updates keep their shared timestamp. There are no taps. The standalone
test independently checks focal geometry, scales and deltas on the
same path.

The authored retained-input control uses two independent device IDs both
reporting pointer 1. Their initial additions and later lifts now keep
the count sequence 1, 2, 1, 0, with focal points and scale derived from
both contacts. Previously adding the second device replaced the first
contact. The arena also removes a rejected contact immediately, before
a different device's motion can use its position. This host normalization
check does not add an upstream registration.

### Local scale deltas and first rotation

The authored Flutter and native mounted probes scale the same 100×100
gesture surface by 2. A (40,20) world move keeps focalPoint in world
coordinates but reports focalPointDelta (20,10) and the new focal point
in local coordinates. The previous native update incorrectly reported
(40,20) as the delta. Slop and span still use global positions.

Pinned scale_test.dart :694 now pairs: the first two-finger move from
the line (0,0)–(10,10) to (0,0)–(0,10) reports +π/4, without an update
on the second pointer's down. The native tolerance is 10⁻⁶ radians.

### Trackpad scale input remains separate

Pinned :774/:890/:1092/:1186/:1293/:1447 passed upstream. Those cases
require a single retained pan/zoom contact with correlated translation,
scale, rotation and timestamps. They respectively exercise raw scale
details, simultaneous touch interaction, competition with drag, the
start-geometry reset, optional scroll-to-scale conversion and the final
scale velocity. Incular's native aggregate pinch/rotation events use
an independent retained callback and do not enter the touch-scale
recognizer as correlated pan/zoom contacts. These are reference-only
rather than equivalences with native aggregate callbacks.

#### Direct absolute pan/zoom events

Pinned :774/:890/:1092/:1186/:1293 were rerun on the Linux reference.
The recognizer now takes absolute trackpad pan, multiplicative zoom and
rotation as one stream. A trackpad contributes two to pointer count but
one to the global centroid; touch spans are measured about the touch-only
centroid. Zoom multiplies those spans, rotation adds to the touch angle,
and `DragStartBehavior::Start` rebases both. Native tests reproduce the
scale-side geometry of :774/:890/:1186; those registrations remain
partial until the horizontal drag participates in the same arena.

Pinned :1293 is paired using the actual managed arena's deferred sole
winner: no start on close, start without an update after the routed
begin, anchored focal point, count two, and both default vertical gain
and a custom horizontal gain across two lifecycles. This is the direct
input protocol. The OS aggregate trackpad callback and hardware ingress
are separate and not verified by this run.

#### Direct pan/zoom ending velocity

Pinned :1447 pairs at the absolute recognizer boundary. The managed arena
settles the sole contender after Start; 100 vertical pan samples at 25 ms
intervals increase scale exponentially while the focal point remains
anchored. Native and reference ending scale velocity agree within 0.0001
(Flutter: 281.41454098027765). No translational fling is reported by the
native scroll-to-scale path.

#### Trackpad scale versus drag

Pinned :774, :1092 and :1186 now pair. Direct pan/zoom scale and an actual
horizontal trackpad drag recognizer are registered in the same ordered
managed arena. Pinch and sufficiently large pan win for scale routed
first; a second trackpad joins a scale already in progress, ending and
restarting at the right boundaries. The drag is never started on those
winning scale streams. The start-behavior case measures zoom and rotation
from the accepting sample.

The authored Flutter and native probes also take the losing-scale path:
exactly 18 horizontal pixels is still pending, 20 wins for drag, and a
later zoom cannot steal the gesture. Moving the reported origin changes
the callback's position, not its pan delta; the next update is (50, 0)
at (102, 45). Only drag ends. Native controls exercise projected vertical
slop at 2× and panning based on path length rather than net displacement.

The absolute drag recognizer is single-stream and the aggregate native
callback is unchanged. Mixed touch-and-trackpad geometry (:890) remains
partial pending competing touch recognition on all the corresponding
mounted routes; no physical trackpad is claimed as tested.

#### Mounted mixed touch and trackpad

Pinned :890 pairs. Native input uses one mounted scale recognizer plus a
competing horizontal drag on both hit paths. Starting a touch ends the
existing bracket; movement starts a new one. Scale counts 2 → 3 → 4,
does not give the trackpad twice the centroid weight, and combines the
signed rotation and scale factors. The complete teardown sequence emits
no drag or extra scale callbacks. An additional control reuses identical
numeric device and pointer IDs across the two protocols. Window-local
routing and cancel-all teardown are checked separately.

These are correlated events passed to the widget tree. The platform
aggregate path still does not manufacture absolute streams from
independent native pinch/rotation/pan notifications.

#### Portable runtime ingress for absolute streams

An embedder can send `InputEvent::PointerPanZoom` through the ordinary
window platform event route. The runtime uses the real position and
timestamp carried by the stream. The integration test needs no mouse
cursor or hover fallback, does not call the native aggregate callback,
does not dispatch an update outside a stream, and reports Start/Update/
End with the right focal points, count, and scale. The existing
aggregate input test still requires an observed hover position. This adds
no claims about device drivers that do not supply correlated input.

#### Mounted scroll-to-scale settings and drag admission

Three authored Flutter probes pair with the mounted consumer: an enabled
horizontal gain maps 125 px to e while keeping the original focal point
and zero pan delta; scale accepts absolute trackpad input even when
`supportedDevices` restricts ordinary input to touch, while drag does not;
and a losing horizontal drag delivers one Down and one Cancel before
ScaleStart, with no DragStart or duplicate cancellation. These authored
probes are separate from the pinned registration count.

#### Trackpad pan and mixed drag tenure

Pinned drag_test.dart :2522 pairs on the mounted correlated-input route
with two real pan recognizers: the winning route stays pending after
28.28 px, starts on the step taking travel to 42.43 px, then delivers only
the incremental (0, −5) move and one end. No outer pan starts.
The two mixed touch/trackpad drag-tenure cases :2583 and :2671 passed on
the reference and are recorded separately from mixed **scale** support.

Pinned :2583 and :2671 now pair at the direct mono-drag recognizer
boundary, using two real competitors and separate managed arena
registrations for each physical or absolute contact. Touch can join
trackpad drag, and trackpad can join touch drag without waiting for
slop again; the combined interaction has one start and ends only when
the last contact leaves. The previous active pointer resumes from its
own last sample if the latest contact lifts. This direct evidence does
not by itself verify mixed drag on the mounted widget routes.

Mounted application pan-only detectors now share that same tenure across
the two routes. The nested two-detector replay of :2583 and :2671 checks
both join orders, callback delta and device kind, and final-only End.
This also retains the previous transformed drag and tap-versus-pan
controls. Scroll brackets, payload drag-and-drop and mixed callback
detectors retain their specialized routes; they are not covered by the
mounted shared-pan claim.

#### Full scale lifecycle and device kind

Pinned scale_test.dart :15 and :1832 now pair in the mounted tree. The
first replays all 14 moves with one, two and three active contacts, an
actual competing tap, axis-specific scaling and focal deltas, five start
and end brackets, and no update on contact reconfiguration or tap leak.
The second reuses the mounted recognizer across touch, mouse, stylus,
inverted stylus, unknown and true absolute trackpad streams; each
publishes the initiating kind and terminates before the next begins.

All 15 registrations in the pinned scale test file now have paired
evidence at their applicable recognizer or widget boundaries. This is
a file-scoped checkpoint, not a claim about OS hardware or the rest of
the gestures library.

### Drag before scale and signed rotations

The authored Flutter and native mounted probes also reverse the route
order. A horizontal drag ahead of the scale wins the second contact's
20 px move. That event must not report a scale restart or update; the
surviving first contact then restarts on its own. Incular previously
reported both a losing two-contact scale restart/update and a drag
start on the same move. The pending scale now observes no event when
an earlier exclusive claim will reject that contact.

Pinned :472 pairs: the mounted scale competes with a tap throughout
a 1→2→3→2 contact sequence. It reports rotation +π after crossing the
first two-contact line, rebases after a contact is replaced, and reports
−π crossing the new line in the opposite direction. The focal points,
deltas, callback count, and end sequence match; no tap is delivered.
