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

## Current targets, immediate completion, and reset

Ten pinned originals pair. A target equal to the current value finishes
synchronously at either bound or in the interior. Midflight, that
request cancels the existing future and reports the terminal forward
status without starting another ticker. The 333 ms controls demonstrate
the subtle case where Rust's public f32 value differs from its retained
double-precision animation value. It is still a no-op, not a short
restart or a snap to rounded precision. Explicit endpoints still land
on their real bounds.

Zero duration cancels outstanding ticker work. Reset reaches the lower
bound from idle, running in either direction, and completed, with the
exact status sequences. An `animate_to` run reports Forward then
Completed regardless of numeric direction or which operation preceded
it. Ten original and two authored pinned Flutter cases passed,
alongside 33 focused Rust cases.
