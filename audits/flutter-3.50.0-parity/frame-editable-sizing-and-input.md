# Text sizing and input frame controls

Nine strict comparisons matched pinned Flutter 3.50.0 `ab2912035e`.

| Scratch batch | Scenario | Frames |
| --- | --- | ---: |
| `20261124-163032-581` | `text__text_max_lines_ellipsis` | 2 |
| `20261124-163032-581` | `text__editable_max_lines_sizing` | 2 |
| `20261124-163032-581` | `text__editable_min_lines_sizing` | 2 |
| `20261124-163032-581` | `text__editable_expands_sizing` | 2 |
| `20261124-163032-581` | `text__text_soft_wrap_false` | 2 |
| `20261124-163207-471` | `text__rich_span_style_metrics` | 2 |
| `20261124-163207-471` | `text__editable_word_arrow_selection` | 7 |
| `20261124-163207-471` | `text__editable_backspace_delete` | 6 |
| `20261124-163207-471` | `text__editable_formatter_digits_then_length` | 4 |

The raw Unicode selection-offset trace differed numerically and needs
unit-aware fixture review before a behavior verdict. Matching controls
do not resolve the independently verified caret-remapping, undo
grouping, and raw Enter differences.
