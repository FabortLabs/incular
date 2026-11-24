# IndexedStack: selection, focus, positioning and layout policies

All 16 originals in widgets/indexed_stack_test.dart passed on the pinned Flutter 3.50.0 checkout. The focused Rust runtime target runs 12 tests: six pass and six intentionally fail.

| Registration | Result |
| --- | --- |
| :12, :18, :368 | Empty stack mounts without exceptions with or without a Center. |
| :27 | Partial mismatch. All three child elements remain mounted, but each Incular test frame invokes every custom painter, vs just the selected painter in Flutter. The Rust test queries mounted keys, not Flutter's skipOffstage finder. |
| :73 | On overlapping children, a tap goes solely to the selected index before and after changing selection from 0 to 2. |
| :104 | Reference only. No equivalent public inherited Visibility.of query was identified. |
| :138, :167, :378 | Reference only. No public null index. A positive invalid index is not a safe substitute because pinned Flutter rejects it. Rust has no public Flutter-compatible skipOffstage finder contract. |
| :196 | Mismatch through real Focus widgets with external nodes. A hidden node reports focus after the direct request while the active node also still reports focus. A second test using KeyboardListener verifies that delivered keys still go only to the selected branch: [0,0,1,1]. Do not confuse erroneous focus flags with misrouted keystrokes. |
| :242 | The hidden overlapping gesture child receives no tap; the same location activates it when selected. |
| :264 | Partial mismatch. Positioned child (10,20,30,40) instead lays out at (0,0,0,0); ordinary Stack with the same child is a passing control. Flutter-specific StackParentData and inherited sibling Visibility assertions are not mapped. |
| :307 | The same retained stack render object changes from default HardEdge to AntiAlias. This checks property propagation, not raster quality. |
| :325 | Loose, Expand and Passthrough transmit exactly the expected constraint tuples to the actual layout builder, including min sizes. |
| :389 | Reference only. Negative selection has no representation in the Rust usize API. |
| :400 | Rust completes layout with index 2 and two children, where Flutter asserts. |

The existing Incular `stack_layout.rs` tests explicitly expect an out-of-range index to hide all children and to allow a later valid selection. Treat the stricter Flutter assertion as a contract decision, not a tested break in an existing Incular caller. That earlier suite also proves index changes update the hit target, but does not compare outgoing pixels. The independent `flutter_indexed_stack_parity.rs` test already covers W04's default ambient-RTL geometry.

Additional observable check: fresh Rust mounts for indices 0, 1 and 2 each return the correct selected-color rectangle. In a retained 0 → 1 → 2 transition with the same children, Runtime::run_frame returns the rectangle for index 0 on all three frames, even though input tests show hit testing follows the selection. This assertion observes the runtime display list, not a native GPU pixel readback.

Run: cargo test --locked -p incular-runtime --test flutter350_indexed_stack_parity -- --nocapture.

## Windows wgpu readback

The separate Flutter 3.50.0 raster fixture repumps the same IndexedStack with selections 0, 1, 2, 0 and passes. The focused Windows target uses WgpuRenderer to present to a hidden native window and captures RGBA at device ratio 1. The expected green/blue interior still reads red after changing to index 1/2. Three interior samples differ on each of those frames; all four immediately adjacent outside samples stay black. Each fresh Rust mount correctly presents its requested color. This confirms the display-list failure reaches raster output in the tested runtime-to-wgpu path.

References: run-reference-probe.ps1 -Test indexed_stack_raster_test.dart. Set INCULAR_DESKTOP_LIVE_TESTS=1 and run cargo test --locked -p incular-desktop --test flutter350_indexed_stack_raster -- --nocapture.

## Retained selection and painting repair

On a selection-only update the retained compositor swaps the attached child
layer, even when the element order is unchanged. Only the current selection
records pictures; a hidden painter keeps its pending work and records the
latest descriptor only when selected. An unchanged selected picture can
be reused after hiding and showing it again. The runtime regression cases
for retained color and selected-only paint now pass, as does the additional
hidden-update control. The Windows wgpu test passes all seven pixel samples
on each retained selection (0, 1, 2, 0) and the three cold mounts.

The focused runtime suite at this point passes nine tests; four separate
checks for focus state, Positioned geometry, and positive invalid index
still fail. The repair does not change those behaviors.

## Explicit no selection

The pinned originals at lines 138, 167 and 378 were rerun successfully.
Lines 138 and 378 now pair: a stack with no selection keeps the 200×200 child
layout without accepting its tap, and two shrink children mount without an
error. A separate retained check proves the selected child's element stays
mounted while its paint, focus and hit target are excluded. The line 167
Flutter finder itself has no Rust equivalent; the retained check only
partially covers that registration.

The pinned Flutter raster fixture and the native Windows wgpu target now
agree at all seven sampled pixels over six retained frames: 0 → none → 1 →
2 → none → 0. They check inside, adjacent outside and edge pixels; both
unselected frames are entirely background at the sampled positions. The
three original cold-mount color controls still pass.

Across the focused IndexedStack runtime and Stack widget targets, 39 Rust
checks passed. The sole red check is still the deliberate divergence for a
positive out-of-range index. No-selection is explicit and does not depend
on the out-of-range behavior.
