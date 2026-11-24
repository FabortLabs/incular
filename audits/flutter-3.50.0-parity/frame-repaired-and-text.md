# Repaired fixture outcomes and text/paint checks

Pinned Flutter 3.50.0 `ab2912035e` vs Incular production `c647cb85`.

| Scenario | Result | Frames |
| --- | --- | ---: |
| `layout__indexed_stack__active_child` | match with actual paint assertions | 2 |
| `navigation__navigator__fade_push_timeline` | match with equivalent transitions | 25 |
| `navigation__navigator__fade_pop_timeline` | match with equivalent transitions | 47 |
| `paint__data__alignment_lerp_extrapolated` | match | 3 |
| `paint__data__border_radius_lerp_extrapolated` | match | 3 |
| `text__widget_span_layout` | match | 2 |
| `text__default_text_style_max_lines` | match | 2 |
| `text__editable_long_press_word_selection` | match | 4 |

IndexedStack fixture now checks inactive layout separately from the
actual paint: Flutter active orange and inactive-only transparent
pixels vs Incular active rectangle only in its compositor output.
The separate inherited-RTL case checks directional default placement
after the wider inactive child establishes an 80 px stack. Both
Flutter directions passed, but Rust reports x=0 instead of x=60
in RTL (`flutter_indexed_stack_parity`).

Both fade fixtures author the same 300 ms primary and secondary
transitions and inject both clocks. The top is intentionally
non-opaque on both sides so the below route actually paints. At 192 ms
the below/top values are 0.76/0.48 on both within tolerance; after pop
the top is absent and the below opacity returns to 1. This confirms
the focused non-opaque transitions, not general opaque compositor parity.

Scratch: `indexed-stack-paint-20261124-160101-939`,
`nav-fade-repair-01`, `20261124-160335-038`.
