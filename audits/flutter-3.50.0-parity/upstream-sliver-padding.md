# SliverPadding geometry

## Padding only, signed overlap, and leading edges

Six selected originals in pinned `widgets/slivers_padding_test.dart`
and their Rust comparisons passed. A childless padding takes scroll
extent, moves the following sliver to (0,200) without a dummy mounted
child, and reports paint extent 200 then 150 after a jump to 50
(:292, :314). Padding before a fill-remaining child reduces its
600-pixel viewport height to 570 (:557). Direct render probes compare
positive overlap reduced by visible leading padding (:589), negative
overlap passed through (:618), and preservation of the child's paint
origin of 10 (:646).

The authored asymmetric padding probe checks all four axis directions
and their actual leading edges. The first Rust run sent 20 pixels
(top) to an upward-growing child where Flutter sent 40 (bottom). Both
axes now resolve their leading edge correctly. A separate Rust control
shows an actual child offset correction is forwarded unchanged with
no extra padding geometry. Four focused Rust targets passed (49 tests).

## Retained list correction under a growing leading row

Pinned :505 and the authored `sliver_padding_correction_reference_test.dart`
pass. The authored version uses controller jumps instead of drag
recognition to isolate the layout; both reference paths place row 2
at (0,100) before growth. Returning toward the start after row 0
doubles in height places row 0 at y=-200 in Flutter but y=0 in
Incular. The dedicated red `flutter350_sliver_padding_correction`
target documents this partial mismatch.

The direct padding control proves it forwards a correction already
emitted by its child. The variable-list or viewport path must still
produce, apply, and lay out the correction in the same frame. Passing
the direct wrapper check does not close the retained regression.

## Applying explicit corrections before paint

The authored `sliver_viewport_correction_reference_test.dart` verifies
a viewport retries immediately, in either direction, when its sliver
returns an offset correction instead of render geometry. Rust initially
tried [0, 0, 0] and never left offset zero, because the correction was
applied after publishing the empty correction range. Now the viewport
accepts the correction without clamping to old content extents and
retries before it publishes or positions children. A custom correction
that never converges gets a bounded typed error. The direct cases now
paint the same child at y=-200 on the same frame in both engines.

Four focused Rust targets passed (46 tests). This is authored coverage,
not an extra original registration. The changing-leading-row regression
above remains open: its ordinary variable-extent list has to produce the
correction before the viewport can apply it.

## Retained anchor repair and touch-input comparison

Original :505 now pairs through Runtime with the same 20-pixel touch
slop waypoint and 300-pixel drags, rebuilt list sizes, and first-frame
rectangles. Both sides place row 2 at (0,100) with height 200, then
place row 0 at (0,-200) with height 400 on the first frame after
returning toward the start. Before the fix, Incular placed row 0 at y=0.

Two issues were distinct. The implicit CustomScrollView lost its
position on rebuild; it now hands the mounted viewport position to
the replacement implicit controller. A variable list also now holds
an identity-checked overlapping anchor while it measures re-entering
leading rows and issues a correction if their new total would place
the beginning before zero. This does not retain an anchor for a
nonoverlapping jump or for a replacement explicit controller.

The authored Flutter and Rust controls also cover shrinking and
unchanged preceding sizes and a later resize after the rewind has
settled. These never issue the growth correction. The explicit
replacement keeps its own initial position. Eight focused Rust
targets passed (160 tests); all three authored Flutter cases passed.

## Replacing padding and changing direction

The leftward Viewport comparison (:348) now exercises the public
`reverse(true)` option. Replacing the empty sliver's left/right
padding moves the following 201-pixel child from x=399 to x=409.
The debug comparison at :394 first lays out zero padding, then
verifies that replacing it with negative edges triggers the specific
SliverPadding assertion rather than silently clamping.

The direct padding render control for :419 compares the trailing
extent after switching through all four directions, then again
at a scroll offset of 99,999.9. It is partial because Incular derives
the result from live geometry and child constraints rather than
exposing the original Flutter RenderSliver getter. Three originals
and four focused Rust targets (23 tests) passed.
