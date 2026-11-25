# Upstream animation controller lifecycle

Four exact original tests in pinned Flutter 3.50.0
`packages/flutter/test/animation/animation_controller_test.dart` passed.
The committed Rust target `flutter350_upstream_animation_controller`
ran the same public observable assertions using vsync and real
`Runtime::run_frame_at` calls.

| Registration | Outcome |
| --- | --- |
| 59, set value inside a status callback | Match: all three reentry flag pairs. |
| 92, forward/reverse callbacks | Mismatch: all ten status snapshots match, but Rust produces an additional unchanged 0.0 value notification at each reverse-at-lower-bound. |
| 415, elapsed duration | Match: 20 ms elapsed at the reported sample. |
| 726, setting value directly | Match: dismissed, forward, completed, forward, dismissed. |

The :92 notifications are the same causal family as C35, not an
independent new bug. Flutter gates the value callback on value change
(`animation_controller.dart:672–680`); Rust also considers status
(`crates/incular-animation/src/controller.rs:1036–1055`).
The zero-duration Rust fixture in :726 never performs timed animation;
nullable getters outside the measured active sample are not certified.

## Current zero-duration status/value recheck

The exact :92 registration passed again. The mounted Incular runtime target
now passes all four lifecycle cases: reversing at the lower bound no longer
emits an unchanged value notification. This is a current disposition for the
previously repaired zero-duration logic, not a claim about arbitrary listener
reentry beyond the measured cases.
