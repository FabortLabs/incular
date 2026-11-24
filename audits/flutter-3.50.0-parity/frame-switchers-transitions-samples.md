# Transition, switcher and sample frame checks

Twenty-four strict fixture outcomes match pinned Flutter 3.50.0
`ab2912035e` and unchanged Incular production `c647cb85`.

| Scratch batch | Scenario | Frames |
| --- | --- | ---: |
| `20261124-164154-553` | `transitions__layout__align` | 6 |
| `20261124-164154-553` | `transitions__positioned__relative_rect` | 6 |
| `20261124-164154-553` | `transitions__builders__cached_child` | 4 |
| `20261124-164154-553` | `transitions__text__default_style` | 6 |
| `20261124-164154-553` | `transitions__sliver_fade__opacity` | 3 |
| `20261124-164258-462` | `switchers__animated_grid_insert_size_transition` | 24 |
| `20261124-164258-462` | `switchers__sliver_animated_list_initial_layout` | 7 |
| `20261124-164258-462` | `switchers__sliver_animated_grid_insert_size_transition` | 24 |
| `20261124-164258-462` | `switchers__animated_modal_barrier_color_and_dismiss` | 27 |
| `20261124-164258-462` | `switchers__switcher_reverse_duration` | 33 |
| `20261124-164414-759` | `switchers__switcher_layout_builder_top_left` | 25 |
| `20261124-164414-759` | `switchers__animated_size_top_left_alignment` | 25 |
| `20261124-164414-759` | `gestures__reorderable_list_long_press` | 6 |
| `20261124-164414-759` | `misc__expansible_height_animation` | 17 |
| `20261124-164509-536` | `misc__inherited_model_aspects` | 4 |
| `20261124-164509-536` | `misc__form_save_invalid_field` | 4 |
| `20261124-164509-536` | `misc__repeating_builder_reverse_pause` | 23 |
| `20261124-164509-536` | `misc__orientation_builder_constraints` | 5 |
| `20261124-164509-536` | `misc__placeholder_fallback_size` | 2 |
| `20261124-164607-639` | `sample_animated_container` | 28 |
| `20261124-164607-639` | `sample_animated_opacity` | 21 |
| `20261124-164607-639` | `sample_animated_positioned` | 33 |
| `20261124-164607-639` | `sample_animated_rotation` | 28 |
| `20261124-164607-639` | `sample_list_fling` | 99 |

The ListWheel raw trace also diverged in batch `20261124-164414-759`,
but is not included in this tally pending a controlled gesture/default
physics comparison. No WGPU performance or all-platform rendering
result is claimed from these scalar and geometry fixtures.
