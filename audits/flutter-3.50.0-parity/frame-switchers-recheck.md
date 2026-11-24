# Focused switcher and animated-child checks

Flutter `ab2912035e` vs Incular production `c647cb85`, strict scratch
batch `20261124-160641-159`. All five match.

| Scenario | Frames |
| --- | ---: |
| `switchers__switcher_overlapping_children` | 30 |
| `switchers__cross_fade_reversed_different_sizes` | 33 |
| `switchers__animated_size_retarget` | 33 |
| `switchers__animated_list_insert_size_transition` | 24 |
| `switchers__switcher_same_key_no_transition` | 11 |

These cover overlapping switcher children, reversed unequal-size cross
fade, animated size retargeting, list insertion, and same-key reuse.
