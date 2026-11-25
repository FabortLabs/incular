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
area without an exception (line 401). Line 373 is a mismatch: Flutter
asserts when a follower encloses its own target in paint order; Incular
handles that link cycle without an exception. This does not imply the
cycle produces a valid linked transform.

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
