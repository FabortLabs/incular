# Text units, remaining animation controls, and bounce dispatch

Pinned Flutter 3.50.0 `ab2912035e` vs Incular production `c647cb85`.

| Scenario | Outcome | Frames |
| --- | --- | ---: |
| `text__text_scaler_linear` | match | 2 |
| `text__strut_style_min_height` | match | 2 |
| `text__text_height_behavior_first_ascent` | match | 2 |
| `anim_compose__train_hopping__unobserved_crossing` | match | 3 |
| `anim_core__fling_negative_and_animate_back` | match | 77 |
| `text__editing_selection_offsets` | match | 2 |
| `physics__ballistic_dispatch__bouncing_overscroll_at_rest` | mismatch | 91 |

The repaired selection fixture keeps the same `😀x` text and end
caret, compares four scalars using checked units, and rejects malformed
boundaries. Both frameworks have 3 UTF-16 units and 5 UTF-8 bytes for
length and caret. Dart's raw public value remains UTF-16; Rust's
remains UTF-8. The old raw 3-vs-5 scalar is therefore not a wrong
caret position.

Equivalent normal bouncing is different when already outside the
bounds at rest. Both fixtures start x=110 with bounds 0–100, velocity
0, DPR 1, distance tolerance 1 and velocity tolerance 20. At 16 ms
Flutter x=108.5348813 and dx=-84.5132704, Incular x=109.7825775
and dx=-24.9937477. Flutter finishes at the 240 ms sample; Incular
has not. Flutter uses a separate rubber-band spring when outside the
range with speed below tolerance (`widgets/scroll_physics.dart:879–895`),
while Incular always passes the normal spring
(`crates/incular-scroll/src/physics.rs:627–641`).
This is not the earlier finite scroll f32 narrowing case.

Scratch: `20261124-163621-795`,
`editing-selection-offsets-20261124-163647-164`, `20261124-163522-965`.

## Stationary rubber-band return

Pinned Flutter selects a separate overdamped spring only when an
out-of-range bouncing release is slower than the tolerance velocity.
The visible root has a 70ms half-life. Incular now uses those exact
parameters for that case, independently of the fast/normal moving
fling spring. Both extents and the exact velocity threshold are
covered at device ratio 2; the existing 16ms position/velocity and
240ms snap checks remain at ratio 1.

The strict fresh stationary trace matches all 91 frames, including
the earlier snap and the derivative after snapping. Both touch
overscroll variants still match their 37-frame references and the
ListWheel fling matches its 66-frame trace. All 17 focused ballistic
and 11 neighboring native touch/fill checks passed.
