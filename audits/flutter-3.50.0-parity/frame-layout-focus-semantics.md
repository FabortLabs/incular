# Layout, focus and semantics frame checks

All ten strict fixtures matched pinned Flutter 3.50.0 `ab2912035e`.

| Scratch batch | Scenario | Frames |
| --- | --- | ---: |
| `20261124-161415-762` | `layout__table__default_flex_columns` | 2 |
| `20261124-161415-762` | `layout__baseline__without_baseline_child` | 2 |
| `20261124-161415-762` | `layout__layout_builder__rebuild_triggers` | 7 |
| `20261124-161415-762` | `layout__visibility__maintain_flags` | 2 |
| `20261124-161415-762` | `layout__flex__direction_reversal` | 2 |
| `20261124-161629-162` | `focus_input__mouse_region_enter_hover_exit` | 4 |
| `20261124-161629-162` | `focus_input__childless_sizedbox_passes_hits` | 2 |
| `20261124-161629-162` | `semantics__offstage_visibility` | 2 |
| `20261124-161629-162` | `semantics__set_text` | 3 |
| `20261124-161629-162` | `semantics__custom_paint_regions` | 2 |

These fixtures are specific to their constraints, lifecycle and sampled
fields. They do not supersede the dedicated loose Table/Baseline
regressions or the stale external focus flag after cache eviction.
All 18 registrations in `focus_input_test.dart` now have valid
outcomes: 17 matched, one differed.
