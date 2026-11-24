# Sliver viewport, header and visibility controls

Pinned Flutter 3.50.0 `ab2912035e`; all five strict comparisons matched
Incular production `c647cb85`. Scratch batch `20261124-162126-089`.

| Scenario | Frames |
| --- | ---: |
| `slivers__fill_viewport__pad_ends_fraction` | 3 |
| `slivers__opacity_offstage__visibility` | 3 |
| `slivers__pinned_header_sliver__child_extent` | 4 |
| `slivers__resizing_header__collapse_scroll` | 7 |
| `slivers__box_padding_fill_remaining__static_and_scroll` | 8 |

This does not settle the separate kept-alive child offscreen paint
or PageView retention comparisons.
