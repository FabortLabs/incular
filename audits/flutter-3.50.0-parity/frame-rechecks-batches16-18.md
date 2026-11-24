# Focused batches sixteen through eighteen and corrected Hero checks

Pinned Flutter 3.50.0 `ab2912035e` and Incular production `c647cb85`.
Scratch batches: `20261124-155323-997`, `20261124-155511-929`,
`20261124-155705-520` and `hero-shuttle-repair`.

| Scenario | Disposition | Frames |
| --- | --- | ---: |
| `semantics__exclude` | match | 2 |
| `semantics__indexed` | match | 2 |
| `semantics__scrollable` | match | 4 |
| `semantics__container_label_only` | match | 2 |
| `focus_input__shift_tab_reverse_traversal` | match | 5 |
| `layout__row__main_and_cross_alignment` | match | 2 |
| `layout__indexed_stack__active_child` | fixture-not-equivalent | 2 |
| `layout__wrap__multiple_runs` | match | 2 |
| `layout__aspect_constraints_and_fraction` | match | 2 |
| `layout__padding_and_overflow_boxes` | match | 2 |
| `implicit__container__fields` | match | 27 |
| `implicit__retarget__current_value_and_on_end` | match | 34 |
| `implicit__zero_duration__completion` | match | 7 |
| `implicit__disabled_animations__reduced_motion` | match | 11 |
| `switchers__animated_list_remove_shifts_items` | match | 24 |
| `navigation__hero__pop_flight_navigator` | mismatch | 47 |
| `navigation__hero__push_flight_navigator` | match, repaired fixture | 25 |

IndexedStack matched both children’s retained geometry and colors. The
original divergence was the inactive child's recorded opacity, 1 vs 0.
Flutter paints and hit-tests only the selected child
(`rendering/stack.dart:847–870`), while the recorder walks all attached
elements and its ancestor opacity calculation does not handle
RenderIndexedStack (`harness/flutter/test/frames/recorder.dart:216–242,272–296`).
Incular's paint tree also emits only its selected compositor child
(`crates/incular-widgets/src/tree/painting.rs:1029–1034`). Exclude this
probe from comparable frame outcomes; a painted-pixel test is preferable.

The repaired Hero harness configures matching controllers and a
separately tagged shuttle built only in the flying overlay. Push matches
all 25 frames. Pop matches push and setup, but the reference shuttle is
present and Incular's missing for 18 reverse frames, starting at t=416 ms.
This is an actual absent return shuttle in the compared traces. The
original [comparability review](navigation-fixture-comparability.md)
applies only to the unrepaired flight setup.

The default-enabled semantics mismatch also has a committed direct test:
`cargo test --locked -p incular-runtime --test flutter_semantics_enabled_parity`.
Flutter distinguished unset, true and false; Incular's true/unset
states were equal. Three new restoration controls passed and the
same-turn ownership handoff intentionally failed in
`flutter350_restoration_bucket_parity`. A lease drop is not the
equivalent of Flutter bucket disposal; explicit removal is. These
restoration checks are bounded observations, not complete translations
of the upstream test registrations.
