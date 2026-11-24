# Focused frame rechecks, batches four through six

Pinned Flutter 3.50.0 `ab2912035e`, Incular `c647cb85`.
Fifteen additional exact fixtures ran with fresh captures in strict mode
and matched the harness's measured fields and tolerances. See
[runner](frame-rechecks-batch2.md#runner) and the
[dispositions](harness-dispositions.tsv). Combined: 35/253 (13.83%),
34 matches and one fade-pop mismatch. This is the existing custom frame
harness, not the upstream registration or original probe count.

| Exact fixture | Frames |
| --- | ---: |
| `anim_core__scheduler_callbacks_time_dilation` | 11 |
| `anim_core__controller_set_value_mid_run` | 13 |
| `physics__bouncing_scroll__extent_handoff` | 122 |
| `semantics__opacity_zero` | 2 |
| `navigation__overlay_portal__content_outside_anchor_clip` | 5 |
| `focus_input__focus_scope_default_traversal` | 3 |
| `focus_input__tap_region_group_outside` | 4 |
| `text__editable_formatter_length_then_digits` | 4 |
| `text__editable_read_only_typing` | 4 |
| `paint__data__hsv_lerp_hue_wrap` | 3 |
| `navigation__overlay_portal__show_hide` | 7 |
| `layout__wrap__alignment_and_run_axes` | 2 |
| `gestures__dismissible_threshold` | 45 |
| `semantics__roles_flags` | 2 |
| `anim_core__animate_to_repeat` | 22 |

The successful read-only keyboard fixture is not a UIKit/IME result;
the formatter fixture has a different edit sequence from C29/C30; and the
existing HSV fixture does not exercise the 350°→10° longer-arc discrepancy
in the source audit. Matches apply only to the exact fixture.

Fresh logs and reports are in `.tmp/flutter350-full-audit/frame-batches/`,
batches starting at UTC 15:12:45, 15:14:51 and 15:16:34 on 2026-11-24.
