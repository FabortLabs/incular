# Current recheck: clip hits, fitted layout and tap regions

The 16 named registrations below were rerun individually in the pinned Flutter
3.50.0 framework checkout, and the corresponding native fixtures passed on
Incular `ad3aa76c`. The older evidence remains useful as a record of the
original failures; these are current dispositions, not newly covered rows.

| Flutter registrations | Permanent focused native target and checked behavior |
| --- | --- |
| `widgets/clip_test.dart:155,178,200,225` | `incular-widgets::flutter350_clip_hit_upstream_parity`: custom path and rect boundaries, ellipse corners, and unchanged hit geometry for the transparent oval. |
| `widgets/fitted_box_test.dart:487` | `incular-widgets::flutter350_fittedbox_layout`: changing to and from `ScaleDown` invalidates layout and recomputes the child-dependent transform. |
| `widgets/tap_region_test.dart:405` | `incular-runtime::flutter_tap_region_upstream_parity`: empty defer/opaque/translucent hit regions distinguish outside from inside and propagate correctly. |
| `widgets/tap_region_test.dart:578,652,726,815,904,978` | `incular-runtime::flutter_tap_region_mouse_buttons_parity`: right and middle buttons classify inside/outside independently; enabled outside consumption prevents the underlying gesture. |
| `widgets/tap_region_test.dart:1133,1191` | `incular-runtime::flutter_tap_region_route_parity`: dismissed and background routes do not get outside events or consume a new route's button interactions. |
| `widgets/tap_region_test.dart:1262,1358` | `incular-runtime::flutter350_tap_region_semantic_actions`: semantic Tap and LongPress dispatch outside callbacks from the action target rather than fabricating a pointer event. |

The six native targets ran serially: 35 passed, none failed. These tests check
geometry, invalidation and input dispatch; they are not comparable GPU snapshots.
