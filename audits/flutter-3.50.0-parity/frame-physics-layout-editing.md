# Physics, layout and editing event confirmations

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

| Scratch batch | Scenario | Outcome | Frames |
| --- | --- | --- | ---: |
| `20261124-162701-401` | `physics__friction__ordinary` | match | 41 |
| `20261124-162701-401` | `physics__gravity__negative_acceleration_threshold` | match | 31 |
| `20261124-162701-401` | `physics__bounded_friction__position_and_velocity` | match | 13 |
| `20261124-162701-401` | `physics__scroll_spring__snap_when_done` | match | 101 |
| `20261124-162701-401` | `physics__bouncing_scroll__transfer_clamp` | match | 121 |
| `20261124-162901-737` | `layout__visibility_offstage_and_transform` | match | 2 |
| `20261124-162901-737` | `layout__flex__main_alignment_variants` | match | 2 |
| `20261124-162901-737` | `layout__stack__fit_alignment_positioned` | match | 2 |
| `20261124-162901-737` | `layout__stack__empty_and_positioned_only` | match | 2 |
| `20261124-162901-737` | `layout__aspect_ratio__constraint_cases` | match | 2 |
| `20261124-162504-359` | `text__editable_enter_callback_order` | mismatch | 5 |

Remediation: the raw Enter path on Android and iOS now leaves the editing
value and active composition alone. The native Done action still completes
before submitting. Both pinned mobile variants passed, as did 28 focused
platform-text, shortcut and Enter tests. The Windows keyboard fallback remains
an explicit desktop behavior and is not claimed as a paired Flutter result.

Native completion actions also distinguish notification from editing. The
pinned traversal probe covers Next and Previous with and without a custom
completion callback. Default completion clears the composing range and
moves focus; a custom callback runs before submission and does neither
automatically. A multiline Newline notification leaves the editing value
already supplied by the IME alone, while Windows raw Enter still inserts a
newline. All 28 focused Rust tests across the four action/input targets pass.
| `20261124-162504-359` | `text__editable_undo_no_programmatic_selection` | mismatch | 9 |

Raw Enter on a neutral Android-profile EditableText did not call
completion or submission in Flutter; Incular called both. The pinned
custom Flutter reference and the dedicated Rust integration target
(`flutter_editable_enter_ime_parity`) distinguish normalized hardware
key handling from an actual IME Done event. The latter completes then
submits on both. The result is not a live Android keyboard claim.
Incular's raw Enter fallback is intended for desktop Winit
(`crates/incular-runtime/src/frame/text_actions.rs:69–80`) but is not
profile-gated on the observed path.

Using physical End instead of a programmatic selection produces the
same early undo mismatch already isolated in the 500 ms grouping test.
This is a second frame registration, not a second underlying bug.

Five ordinary physics fixtures match; they do not supersede the
narrow precision, NaN, bounds-validation and classification regressions.

The fresh strict Linux run of `text__editable_enter_callback_order`
matches all five frames and callback counters. The raw Enter key and
the platform editing action remain separate; the desktop fallback is
not being compared in this fixture.
