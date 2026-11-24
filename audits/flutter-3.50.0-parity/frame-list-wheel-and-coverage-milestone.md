# ListWheel and frame audit milestone

The final frame registration has a valid controlled outcome. Pinned
Flutter 3.50.0 `ab2912035e` and Incular production `c647cb85` both
used explicit plain clamping, attached controllers, a 600 px viewport
and maximum offset 1950. The repaired case preserves all 66 gesture
and post-release frames, but no longer compares a projective Flutter
probe against an Incular affine probe.

At 48 ms, before release, Flutter has moved 66.6667 px, selected item 1
and emitted one selection callback. Incular records 0, item 0, and no
callback. At the final sample Flutter reaches offset 1950/item 39;
the result does not establish independent ballistic or projection
parity because Incular never entered the touch drag.

Separate paired tests confirm both the pre-release failure and the
matching programmatic jump to offset 100/item 2. Rust target
`flutter_list_wheel_touch_parity`; pinned Flutter
`list_wheel_touch_reference_test.dart`. The source localization is the
touch ancestor search excluding the distinct ListWheel render kind
(`crates/incular-widgets/src/tree/interaction.rs:434–481,965–1008`).
Flutter's default defers to platform physics; Incular's default is
clamping plus snapping. Those defaults were not assumed equal in
the controlled reproduction.

Scratch batch `list-wheel-controlled-frames-20261124-165147-485`.

## Frame coverage vs full parity

**All 253 static frame registration sites have comparable outcomes:
239 match and 14 differ.** This includes both named runtime expansions
of the one dynamic overscroll registration, counted once. These are
not 14 independent bugs: multiple registrations reproduce missing
UserScroll or the same undo grouping. The extreme jump fixture also
records a public f32 precision difference, not a confirmed visual
jump. This is not a claim that all rendering/device behavior or the
full upstream suite was verified. Continue the upstream audit and
retain design-system exclusion and wgpu. No implementation changes.
