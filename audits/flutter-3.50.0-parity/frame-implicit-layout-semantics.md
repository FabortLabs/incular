# Implicit animation, layout and semantics controls

Twenty strict fixture outcomes match pinned Flutter 3.50.0 `ab2912035e`.

| Scratch batch | Scenario | Frames |
| --- | --- | ---: |
| `20261124-163840-902` | `implicit__padding__edge_insets` | 27 |
| `20261124-163840-902` | `implicit__align__alignment_and_factors` | 27 |
| `20261124-163840-902` | `implicit__positioned__edges_and_size` | 27 |
| `20261124-163840-902` | `implicit__scale__alignment` | 27 |
| `20261124-163840-902` | `implicit__rotation__turns_and_alignment` | 27 |
| `20261124-163946-751` | `implicit__slide__fractional_offset` | 27 |
| `20261124-163946-751` | `implicit__opacity__opacity` | 27 |
| `20261124-163946-751` | `implicit__sliver_opacity__opacity` | 27 |
| `20261124-163946-751` | `implicit__physical_model__color_elevation_radius` | 27 |
| `20261124-163946-751` | `implicit__fractional_sized_box__factors` | 27 |
| `20261124-164014-610` | `layout__container__composition_order` | 2 |
| `20261124-164014-610` | `layout__intrinsic_width__steps_and_flex_child` | 2 |
| `20261124-164014-610` | `layout__box_constraints__helper_values` | 2 |
| `20261124-164014-610` | `misc__rotated_box_layout_size` | 2 |
| `20261124-164014-610` | `misc__spacer_flex_shares` | 2 |
| `20261124-164054-030` | `semantics__value_hint_tooltip` | 2 |
| `20261124-164054-030` | `semantics__increase_decrease` | 3 |
| `20261124-164054-030` | `semantics__gesture_tap` | 3 |
| `20261124-164054-030` | `semantics__horizontal_scroll` | 4 |
| `20261124-164054-030` | `navigation__pop_scope__blocked_root_route` | 3 |

All static registrations in the implicit, layout, semantics and
navigation frame files now have valid outcomes. The tested probes
do not imply parity for every parameter, visual backend or screen
reader. Dedicated regressions for those libraries remain intentionally red.
