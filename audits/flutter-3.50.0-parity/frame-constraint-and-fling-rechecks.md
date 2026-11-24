# Spring, bounds, and fling controls

Nine strict fixtures match pinned Flutter 3.50.0 `ab2912035e`.

| Scratch batch | Scenario | Frames |
| --- | --- | ---: |
| `20261124-163345-907` | `physics__spring_description__damping_regimes` | 3 |
| `20261124-163345-907` | `physics__spring_description__duration_bounce` | 3 |
| `20261124-163345-907` | `physics__spring__tolerance_boundary` | 3 |
| `20261124-163345-907` | `physics__friction__tolerance_boundary` | 3 |
| `20261124-163345-907` | `physics__clamped_simulation__position_and_velocity` | 31 |
| `20261124-163522-965` | `physics__clamping_scroll__spline` | 21 |
| `20261124-163522-965` | `physics__ballistic_dispatch__clamping_overscroll` | 91 |
| `20261124-163522-965` | `physics__ballistic_dispatch__clamping_negative_velocity` | 21 |
| `20261124-163522-965` | `anim_core__animate_with_fling` | 106 |

The zero-velocity bouncing overscroll dispatch diverges in the raw
fixture (x, dx and completion); equivalent default/selection inputs
are under review separately. The matched duration/bounce samples do
not supersede the dedicated critical damping classification case.
