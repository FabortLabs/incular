# LayoutBuilder: retained constraints and public harness boundaries

All six named originals below passed with the pinned Flutter 3.50.0 snapshot via the exact-name upstream runner. The Incular comparisons use the current public APIs.

| Flutter registration | Incular verification |
| --- | --- |
| :452 | Reference only. The Flutter SliverLayoutBuilder callback reads MediaQuery through its build context, despite the test title. The Rust callback only receives SliverConstraints. |
| :490 | Reference only. Flutter nests sliver builders and the inner result remains a sliver. The Rust callback returns a box Widget, so it cannot reproduce that configuration. |
| :698 | Partial match. Resizing the host ancestor from 800x600 to 700x500 with identical inner 10x10 constraints does not rerun the builder. Replacing the descriptor reruns it even with identical constraints, and a later 20x20 size updates the measured geometry. This does not reproduce manually marking the Flutter render box or count the low-level child layout/resize hooks. |
| :779 | Reference only. The Flutter build callback inspects the previously laid-out child RenderBox size; the Rust public build context does not expose equivalent access. |
| :864 | Reference only. The first-layout smoke test requires keeping an overlay entry mounted while omitting it from layout behind an opaque entry. No corresponding public overlay contract was found; replacing it with IndexedStack would not test skipped layout. |
| :1007 | Partial match. A host pointer tap first gives a real keyed EditableText focus; repumping its preceding sliver item from 60 to 6000 logical pixels moves the editor offscreen without crashing. The controller text remains intact. This does not assert Flutter GlobalKey semantics or the mechanism of AutomaticKeepAlive. |

Focused Incular targets: incular-widgets / flutter350_layoutbuilder_dependencies (9 passed); incular-runtime / flutter350_sliver_layoutbuilder_parity (6 passed). No framework changes.
