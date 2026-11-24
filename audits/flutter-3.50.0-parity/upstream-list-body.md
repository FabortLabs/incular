# ListBody directional geometry and layout errors

All six originals in widgets/list_body_test.dart passed on the pinned Flutter 3.50.0 checkout. The Incular checks use the public descriptor, keyed boxes of the same requested 200x150 size, and an 800x600 test surface. The targeted Rust run compiled and produced five expected red tests.

| Registration | Result |
| --- | --- |
| :31 down | Children remain 200px wide at x=300 in Incular instead of filling 800px at x=0. All four y positions and 150px heights match. |
| :47 up | Reference only. The Rust descriptor has no reverse option. Reordering the input changes element identity order rather than checking the reverse layout contract. |
| :63 right | The four LTR x positions and 200px widths match. Incular keeps height 150 at y=225; Flutter forces height 600 at y=0. |
| :85 left | All mirrored RTL x positions match. The same cross-axis stretch mismatch remains. |
| :107 limited main axis | The translated bounded composition finishes quietly instead of reporting a layout failure. This probe asserts rejection, not the exact FlutterError text. |
| :139 nested unbounded cross axis | The translated horizontal ListBody containing a vertical Flex and ListBody finishes layout, instead of reporting a failure. The exact Flutter diagnostic text is not paired. |

The Rust descriptor delegates directly to Row/Column, preserving the basic order but inheriting different child constraints. Run: cargo test --locked -p incular-widgets --test flutter350_list_body_parity -- --nocapture.
