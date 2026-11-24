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

Additional observable check: fresh Rust mounts for indices 0, 1 and 2 each return the correct selected-color rectangle. In a retained 0 → 1 → 2 transition with the same children, Runtime::run_frame returns the rectangle for index 0 on all three frames, even though input tests show hit testing follows the selection. This assertion observes the runtime display list, not a native GPU pixel readback.

Run: cargo test --locked -p incular-runtime --test flutter350_indexed_stack_parity -- --nocapture.
