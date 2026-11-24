# Focused frame rechecks, batch nine

Flutter 3.50.0 at `ab2912035e` and Incular at `c647cb85`, fresh strict
comparison. Combined: **50/253 existing frame registrations (19.76%),
45 match and five differ**.

| Scenario | Result | Frames |
| --- | --- | ---: |
| `scroll__nested_scroll_view__drag_offsets` | match | 47 |
| `slivers__viewport_center__anchor` | mismatch | 5 |
| `navigation__overlay_portal__direct_anchor` | match | 3 |
| `text__editable_programmatic_controller_update` | match | 3 |
| `paint__raw_image__intrinsic_scale` | match | 3 |

Viewport center/anchor: the first frame's minimum scroll extent was
`-291.8181818181818` on Flutter versus `-303.0` on Incular. Three of five
frames differed on the minimum extent (max difference 18); no other
comparison field was reported. The observable contract mismatch is the
reported scrollable range; this run does not establish a gesture or pixel
difference. See the [focused runner](run-focused-frames.ps1) and the
fresh scratch batch `20261124-152359-349`.
