# Stack layout, direction and clipping

Pinned Flutter 3.50.0 passed all 18 original registrations in widgets/stack_test.dart and the authored stack_default_direction_test.dart. The focused Incular target compiled: eight tests pass and three intentionally fail.

| Registrations | Evidence |
| --- | --- |
| :20, :24 | Empty Stack mounts with and without Center. |
| :33, :76, :203 | Moving left to right clears prior insets and repositions the child; removing Positioned resets actual placement; explicit 11×12 and left/right offsets are honored. Partial for Flutter's direct StackParentData access; the Rust checks use the retained Positioned wrapper and actual child geometry. |
| :111, :157 | Physical center placement gives the same offsets in both directions. The directional bottomEnd portion is not represented by Incular's physical Alignment-only Stack API. |
| :269, :278 | Flutter-only: retained default Clip property inspection and describeApproximatePaintClip are not represented by these public-behavior tests. |
| :308 | The translated oversized Positioned child emits a clip with HardEdge, and then no clip with None. This records the rendered display list, not Flutter's internal painting-context method order. |
| :349, :379 | The real LayoutBuilder sees the corresponding min/max constraints for default loose and all three explicit fits. |
| :422, :459, :494 | Pinned Flutter-only; no corresponding directional positioned/explicit Stack direction surface in Incular. |
| :500 | Partial mismatch. In the absolute subset, a left-only child under center alignment is at (0,0) instead of the Flutter (0,250); the unspecified vertical axis loses center alignment. Directional Positioned rows were not ported. |
| :726 | Partial mismatch: the original Flutter rejects missing Directionality for the default Stack and checks specific diagnostics. Incular accepts it; explicit physical center is a passing Rust control. |
| :760 | When laid out at zero area with a Placeholder, the stack size is zero and it doesn't crash. This does not resize a real desktop window. |

Separate authored LTR → RTL → LTR reference: the 10px child in a 20px Stack should have relative x = [0,10,0], but Incular produces [0,0,0]. The larger sibling and stack size match. This verifies the default resolves to a physical left rather than ambient start; it is not an exact pairing for the directional bottomEnd original.

Run the named Flutter originals with run-upstream-reference.ps1 (groups of five maximum), the authored sample with run-reference-probe.ps1 -Test stack_default_direction_test.dart, and cargo test --locked -p incular-widgets --test flutter350_stack_layout_parity -- --nocapture.

## Windows wgpu: default clipping distinguishes geometry from paint

An authored pinned Flutter test and an actual hidden-window wgpu readback compare the same opaque 100×100 scene at 1×. A centered 40×40 Stack paints a 20×20 red child after a 30px right shift. One case puts the shift in the child's layout with Positioned, the other uses only a paint transform.

At (75,40), outside the Stack's right edge but inside the red child, HardEdge with a paint-only shift is red in Flutter, black in Incular. The (65,40) interior red control and the adjacent black samples match. The directly Positioned overflowing child clips in both, and Clip.none exposes both samples for both variants. This distinguishes an unnecessary clip from a wrong child transform or wrong surface size. Flutter guards with a layout-overflow flag before stacking a clip; Incular has a clip layer for any non-None Stack.

Reference: run-reference-probe.ps1 -Test stack_overflow_raster_test.dart. Incular: set INCULAR_DESKTOP_LIVE_TESTS=1 and run cargo test --locked -p incular-desktop --test flutter350_stack_overflow_raster -- --nocapture.

## Positioned geometry repair

Both retained stacks now measure and place Positioned children with the
same layout path. An axis with no leading or trailing inset uses the
configured alignment and the child's final measured size, even when
that size was explicit. The absolute subset at :500 now passes, along
with IndexedStack's explicit (10,20,30,40) geometry control. The focused
layout and existing widget stack suites passed 3 and 23 tests. The
directional default and missing-Directionality tests are still open;
this fix does not introduce directional positioning APIs.

## Layout-dependent clipping repair

Stack and IndexedStack now attach the clip stage only when a positioned
child's layout crosses an edge and clipping is enabled. Paint-only
transforms do not turn it on. The overflow decision is retained when
changing the clip option without remeasurement. The pinned Flutter probe
and native Windows wgpu test agree on all four pixels across six retained
frames, including re-enabling HardEdge on unchanged overflowing geometry.
The separate IndexedStack wgpu selection test and the focused stack/clip
widget suites pass.

## Ambient start alignment

Stack and IndexedStack now resolve the default top-start alignment
using the nearest Directionality. An explicit physical top-left
does not mirror. The pinned probe verifies retained LTR/RTL/LTR and
explicit vs default alignment on both widgets. The default alignment
and 34 other named widget checks pass; DevTools also compiles with
the actual resolved values. An absent Directionality still falls back
to LTR in Incular, so the existing Flutter assertion probe stays open.

## Directional positioning and empty parent data

Pinned `widgets/stack_test.dart:111,157,422,459,494,500` and
`widgets/positioned_test.dart:12,133` pass. The Incular ports assert preserved
child identity with direction updates, physical center and directional
bottom-end in LTR/RTL, and all 36 final child rectangles under four
combinations of direction and alignment. A stack's explicit direction
overrides its own alignment without changing inherited Directionality.
The physical rectangle and relative-inset constructors expose all six
expected optional fields. The zero-area Positioned placeholder mounts.

The authored `stack_positioned_empty_insets_test.dart` exposed a separate
failure: a single 40×20 child wrapped in an empty Positioned made a loose
stack fill 800×600. Pinned Flutter reports 40×20 → 800×600 → 40×20 as
an inset is added and cleared, preserving the child. The retained path and
the pure layout function now classify children by actual configured
positioning fields rather than the presence of the wrapper.

Focused layout/stack controls passed with the existing
missing-Directionality diagnostic mismatch left open. A separate
IgnoreBaseline regression was discovered in the same regression run
and is tracked independently.
