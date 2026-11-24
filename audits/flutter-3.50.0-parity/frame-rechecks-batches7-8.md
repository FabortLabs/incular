# Focused frame rechecks, batches seven and eight

Pinned Flutter 3.50.0 at `ab2912035e`, Incular implementation
`c647cb85`. Results from fresh strict frames; 10 new fixtures checked.
Combined: **45/253 existing frame registrations (17.79%), 41 match, 4
mismatch**. Source/fixture scope is not full behavior closure.

| Scenario | Outcome | Frames |
| --- | --- | ---: |
| `scroll__controller__jump_clamp` | mismatch | 6 |
| `scroll__controller__jump_activity_notifications` | match | 3 |
| `slivers__varied_extent_list__indices` | match | 3 |
| `slivers__fixed_prototype_extent_lists__indices` | match | 5 |
| `slivers__fill_remaining_no_scroll_body__shrink_after_preceding_growth` | match | 5 |
| `text__editable_home_end_shift_select` | match | 6 |
| `text__editable_undo_redo` | mismatch | 8 |
| `scroll__controller__animate_retarget` | match | 24 |
| `semantics__stale_action` | match | 4 |
| `navigation__navigator__observer_order` | mismatch | 9 |

First observed differences:

- Jump-clamp, frame 4 at 64 ms: offset 97856.2010603 vs 97856.203125
  (difference 0.002065, exceeds scalar tolerance 0.001). One frame
  exceeded tolerance. No user-visible effect established.
- Editable undo/redo, frame 6 at 680 ms: text length 2 vs 3, one
  frame differs. Investigate grouping/restoration across the native
  shortcut path before assigning root cause.
- Navigator observer order fixture, frame 3 at 48 ms: underlying root
  opacity 1 vs 0, four frames differ. The report does **not** show
  callback order divergence. The mismatch is the visual probe.

See the [runner](frame-rechecks-batch2.md#runner). Exact IDs and outcomes
are in [harness dispositions](harness-dispositions.tsv). Traces and reports
remain in audit scratch batches at UTC 15:20:07 and 15:20:59.
