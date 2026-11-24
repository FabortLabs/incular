# Pinned rechecks of existing frame scenarios

Flutter framework `ab2912035e` (3.50.0); Incular framework `c647cb85`.
Ran only the named, unchanged harness twins under strict mode. Frame matching
here means equal under the harness tolerances (0.01 logical px, 0.0001
transform, 0.002 opacity, one color value, and 0.001 scalar), not all API or
pixel behavior of a component.

**11/253 existing frame fixtures checked (4.35%), 10 matched, one diverged.**
The 253 frame entries are separate from the eight value/widget registrations,
the upstream catalog, and original C01–C27 comparisons.

| Fixture | Result | Frames |
| --- | --- | ---: |
| `layout__flex__expanded_and_spacing` | match | 2 |
| `layout__baseline__cross_axis` | match | 2 |
| `layout__fitted_box__contain_alignment` | match | 2 |
| `layout__box_constraints__limited_overflow_intrinsic` | match | 2 |
| `layout__flex__fit_loose_and_tight` | match | 2 |
| `layout__fitted_box__all_fits` | match | 2 |
| `navigation__navigator__push_pop_can_pop` | match | 6 |
| `navigation__navigator__fade_pop_timeline` | mismatch | 47 |
| `semantics__block_previous` | match | 2 |
| `semantics__merge` | match | 2 |
| `gestures__scale__two_pointer_pinch` | match | 7 |

## Fade-pop mismatch

First top opacity divergence: frame 3 at 48 ms, Flutter 0 vs Incular
0.012038, 19 divergent frames. First below-route opacity divergence: frame 4
at 64 ms, Flutter 0.973333 vs Incular 1, 40 divergent frames and max
absolute difference 0.5. These are current measured fixture differences,
not a finding that every route transition differs. Old 3.41.7 records are
comparison leads; do not reuse older traces or assume the same first frame.

The direct BlockSemantics and merge label frame fixtures matched alongside
the separate custom/nested/action failures. All six layout frames matched
alongside the newly found boundary failures: the input cases differ.

## Reproduction

Copy `harness/flutter/pubspec.yaml`, `fonts/`, and `test/` into a scratch
package under `.tmp`. Use `.tmp/flutter/bin/flutter.bat`, a fresh
`PARITY_FRAMES_OUT` at `$out/frames/flutter`, and
`flutter test test/frames/<family>_test.dart --plain-name <exact-scenario>`
once per selected fixture. The Flutter fixture registrations all ran.
Then set `INCULAR_PARITY=1`, `INCULAR_PARITY_DIR=$out`,
`PARITY_STRICT=1` and `PARITY_SCENARIO=<exact-scenario>` and run only
`cargo test --locked -q -p incular-parity-harness --test frames`.
Raw traces and strict reports for this batch are under
`.tmp/flutter350-full-audit/retained-frame-comparisons/`.
