# Focused frame rechecks, batch fifteen

Flutter 3.50.0 `ab2912035e` vs Incular `c647cb85`. Scratch batch
`20261124-154911-721`. All five new fixtures match.

| Scenario | Frames |
| --- | ---: |
| `anim_compose__curved_animation__unobserved_reversal` | 2 |
| `anim_core__controller_listener_notifications` | 22 |
| `anim_curves__curve2d__spline` | 1 |
| `misc__future_builder_keyed_reorder` | 4 |
| `misc__media_query_size_aspect` | 3 |

The ordinary listener fixture does not exercise removing a listener
mid-dispatch or equal-target notification, which have separate regression
tests. These outcomes do not certify the rest of each API.
