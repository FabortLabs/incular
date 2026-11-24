# Focused frame rechecks, batch thirteen

Flutter 3.50.0 `ab2912035e` vs Incular `c647cb85`, fresh strict frames.
The five fixtures below match. Result as of this batch: 60 valid matches.

| Scenario | Result | Frames |
| --- | --- | ---: |
| `focus_input__reading_order_band_overlap` | match | 4 |
| `focus_input__ordered_unordered_reading_order` | match | 5 |
| `focus_input__arrow_directional_focus` | match | 5 |
| `focus_input__key_event_bubbles_past_ignored` | match | 3 |
| `focus_input__mouse_region_nested_enter_order` | match | 3 |

These validate the fixture conditions, not all focus or mouse routing
behavior. Scratch batch `20261124-153441-530`;
[focused runner](run-focused-frames.ps1).
