# Focused frame rechecks, batch fourteen

Flutter 3.50.0 `ab2912035e` versus Incular `c647cb85`. Scratch batch
`20261124-153715-618`.

| Scenario | Outcome | Frames |
| --- | --- | ---: |
| `focus_input__shortcuts_action_intent` | Match | 3 |
| `focus_input__keyboard_page_scroll` | Mismatch | 12 |
| `focus_input__listener_pointer_lifecycle` | Match | 4 |
| `focus_input__ignore_pointer_falls_through` | Match | 2 |
| `focus_input__absorb_pointer_blocks_sibling` | Match | 2 |

The scroll offset matched, but after the initially focused row left
the cache Flutter reports no focus on its external FocusNode and Incular's
node still reports focus. The first divergent sample is frame 6, 96 ms;
the field differs in six frames. This is an observable stale node state,
not a difference in whether PageDown moves the scroll position.

A fresh pinned Linux recheck of `focus_input__keyboard_page_scroll`
matches all 12 frames, including the scroll offset, held row focus and
retained interactive geometry. This replaces the original mismatch
after the keyboard scroll routing fixes.
