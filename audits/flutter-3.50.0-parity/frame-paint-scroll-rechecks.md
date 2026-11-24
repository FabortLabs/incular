# Focused paint, transitions, and overscroll checks

Flutter 3.50.0 `ab2912035e` and Incular production `c647cb85`.
Scratch batches `20261124-160053-213`, `20261124-155855-653`,
and isolated `nav-observer-back-repair`.

| Fixture | Outcome | Frames |
| --- | --- | ---: |
| `paint__opacity__phase_values` | match | 5 |
| `paint__opacity__zero_keeps_layout` | match | 3 |
| `paint__decorated_box__solid_background` | match | 3 |
| `transitions__layout__size_clip` | match | 6 |
| `transitions__paint__decoration_matrix` | match | 6 |
| `navigation__navigator__observer_order` | match after repair | 9 |
| `navigation__pop_scope__route_back_blocked` | match after repair | 7 |
| `scroll__drag_overscroll__clamping` | mismatch | 37 |
| `scroll__drag_overscroll__bouncing` | mismatch | 37 |

The observer and blocked-back repairs preserve the scene and record
only relevant events/route retention. Each target also asserts the exact
observer log or handled, blocked back result and callback. The earlier
covered opacity was not a sound proxy for paint on either platform.

The scroll twins use the same pointer gesture, clock and scroll physics
category. At 48 ms, Flutter has emitted one UserScroll notification and
Incular none, for both modes. Clamping emits a second notification on
return to Idle at release, while bouncing remains ballistic through
the last common captured frame and keeps the count at one. Offset,
start, update, overscroll and end probes matched where recorded. Flutter
carries a direction explicitly (`widgets/scroll_position.dart:1073–1078`);
Incular's `ScrollNotification` has no direction field
(`crates/incular-scroll/src/notifications.rs:29–45`).
The two runtime fixtures arise from the same **one static registration**.
The exact runner admits just those two reviewed expansions and compares
them independently before this registration is closed as a mismatch.
