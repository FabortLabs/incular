# anim_curves: Curves and tweens

## Summary

Six deterministic frame scenarios cover the 43 Flutter `Curves` constants, the concrete 1D and 2D curve classes, typed tween classes, and weighted sequences. Four scenarios match. `SizeTween`/`RectTween` extrapolation and exact sequence boundaries diverge. Source review found no direct `FlippedTweenSequence` or nullable tween endpoints in Incular. Flutter scenario tests pass; the filtered runner reports the two expected divergences.

## Coverage

| Flutter item | Scenario / evidence | Result |
| --- | --- | --- |
| `ParametricCurve<T>` and `Curve` | `anim_curves__curves__interior`, `anim_curves__curve_classes__edges`, `anim_curves__curve2d__spline` | Abstract bases exercised through concrete implementations. Every sampled curve maps 0 and 1 to the expected endpoints. |
| `SawTooth`, `Interval`, `Split`, `Threshold`, `Cubic`, `ThreePointCubic` | `anim_curves__curve_classes__edges` | Match at interior samples and constructor-specific boundaries, including `Threshold` equality and `Split` equality. |
| `FlippedCurve`, `ElasticInCurve`, `ElasticOutCurve`, `ElasticInOutCurve` | `anim_curves__curve_classes__edges` | Match, including non-default elastic period 0.6; default-period constants are covered below. |
| `Curve2D`, `Curve2DSample`, `CatmullRomSpline`, `CatmullRomCurve` | `anim_curves__curve2d__spline` | Match on spline transforms, adaptive sample count and representative samples, inverse lookup, explicit handles, curve validation, and Catmull-Rom timing samples. |
| All 43 `Curves` constants | `anim_curves__curves__interior` | Match at 0, 0.21, 0.37, 0.63, 0.79, and 1: `linear`, `decelerate`, `fastLinearToSlowEaseIn`, `fastEaseInToSlowEaseOut`, `ease`, `easeIn`, `easeInToLinear`, `easeInSine`, `easeInQuad`, `easeInCubic`, `easeInQuart`, `easeInQuint`, `easeInExpo`, `easeInCirc`, `easeInBack`, `easeOut`, `linearToEaseOut`, `easeOutSine`, `easeOutQuad`, `easeOutCubic`, `easeOutQuart`, `easeOutQuint`, `easeOutExpo`, `easeOutCirc`, `easeOutBack`, `easeInOut`, `easeInOutSine`, `easeInOutQuad`, `easeInOutCubic`, `easeInOutCubicEmphasized`, `easeInOutQuart`, `easeInOutQuint`, `easeInOutExpo`, `easeInOutCirc`, `easeInOutBack`, `fastOutSlowIn`, `slowMiddle`, `bounceIn`, `bounceOut`, `bounceInOut`, `elasticIn`, `elasticOut`, `elasticInOut`. |
| `Animatable`, `Tween`, `CurveTween`, `ReverseTween` | `anim_curves__tweens__scalar` | Match for linear, curved, reversed, endpoint, and extrapolated scalar values. |
| `IntTween`, `StepTween`, `ConstantTween`, `ColorTween`, `SizeTween`, `RectTween` | `anim_curves__tweens__typed` | Integer rounding, floor, constants, and in-range color/size/rect values match. Size and rect overshoot extents diverge (anim_curves-001). Null endpoints are not representable in the Incular aliases (anim_curves-004). |
| `TweenSequence`, `TweenSequenceItem` | `anim_curves__tween_sequence__weighted` | Weighted interior values and endpoints match. A deliberately discontinuous item boundary returns the prior item in Incular and the next item in Flutter (anim_curves-002). Incular's `TweenSegment` holds the same tween/weight pair. |
| `FlippedTweenSequence` | `anim_curves__tween_sequence__weighted` and source review | Flutter's flipped formula is sampled; Incular computes the formula inline because there is no direct sequence wrapper or `Animatable` implementation (anim_curves-003). |

## Findings

### anim_curves-001 — Size and rectangle tweens clamp extrapolated extents

- Severity: P2
- Category: behavior
- Flutter: `SizeTween` and `RectTween` delegate to extrapolating `Size.lerp` / `Rect.lerp` (`tween.dart:426-468`; `sky_engine/lib/ui/geometry.dart:588-599,891-910`).
- Incular: `SizeTween` and `RectTween` use the `Size` tween / lerp paths (`crates/incular-animation/src/value.rs:295-305,321,325`; `crates/incular-animation/src/tween.rs:61-67`; `crates/incular-core/src/geometry.rs:543-548,656-662`). Both clamp interpolated size extents to zero.
- Evidence: `anim_curves__tweens__typed`, frame 0. At progress 2, `size_extrapolated_width` is -10 in Flutter and 0 in Incular; height is -20 vs 0. `rect_extrapolated_right` and `rect_extrapolated_bottom` are 190 vs 200.
- Impact: curves or parent animations that overshoot can produce negative size extents in Flutter, while Incular truncates them to zero. Rect tween edges then differ as well.
- Cause: the shared Incular `Size` interpolation clamps negative dimensions during extrapolation.
- Fix direction: preserve signed interpolated dimensions in tween math, while keeping non-negative validation at APIs that require layout sizes.
- Confidence: high

### anim_curves-002 — Tween sequences choose the previous item at an exact boundary

- Severity: P2
- Category: behavior
- Flutter: sequence intervals are half-open (`tween_sequence.dart:80-88,156-159`): an internal boundary belongs to the following item.
- Incular: `TweenSequence::evaluate` selects the current segment when `target <= end` (`crates/incular-animation/src/tween.rs:178-202`).
- Evidence: `anim_curves__tween_sequence__weighted`, frame 0 at progress 0.5, with a valid discontinuous boundary: Flutter returns 0.7 from the second item; Incular returns 0.2 from the first. The flipped value is 0.3 vs 0.8 for the same boundary.
- Impact: a sequence whose adjacent items do not share an endpoint produces a different value exactly at the item transition.
- Cause: Incular's inclusive upper-bound comparison differs from Flutter's `[start, end)` interval selection.
- Fix direction: use a strict upper-bound comparison for non-final segments and keep the final segment responsible for progress 1.
- Confidence: high

### anim_curves-003 — Flipped tween sequences have no direct Incular equivalent

- Severity: P2
- Category: missing-api
- Flutter: `FlippedTweenSequence.transform(t)` evaluates `1 - super.transform(1 - t)` (`tween_sequence.dart:106-117`).
- Incular: `TweenSequence` is evaluated directly and does not implement `Animatable`; `ReverseTween` wraps `Animatable` values but does not invert the resulting value (`crates/incular-animation/src/tween.rs:143-221`; `crates/incular-animation/src/animations.rs:694-701`).
- Evidence: source review. The sequence scenario samples Flutter's `FlippedTweenSequence`; its Incular twin evaluates the same formula inline because no direct public wrapper is available.
- Impact: callers cannot use a named flipped sequence as an animatable; they must write the reverse-and-invert expression themselves.
- Cause: there is no flipped sequence type, and `TweenSequence` is not wired into the `Animatable` trait used by composition.
- Fix direction: make `TweenSequence` usable as an `Animatable` and add a wrapper that evaluates `1 - sequence(1 - t)`.
- Confidence: high

### anim_curves-004 — Color, size, and rect tweens do not accept nullable endpoints

- Severity: P2
- Category: missing-api
- Flutter: `ColorTween`, `SizeTween`, and `RectTween` allow null endpoints (`tween.dart:401-470`); null means transparent alpha for colors and zero for size/rect. `Color.lerp` scales only alpha when one endpoint is null (`sky_engine/lib/ui/painting.dart:415-437`).
- Incular: aliases require concrete values (`crates/incular-animation/src/value.rs:319-325`; `TweenValue` stores non-optional `begin`/`end` at `crates/incular-animation/src/tween.rs:86-90`).
- Evidence: source review. The typed scenario covers non-null endpoints; the null cases cannot be expressed by the Incular tween types.
- Impact: callers cannot directly fade a color to/from no color while preserving its RGB channels, or pass absent size/rect endpoints as Flutter does.
- Cause: Incular's typed tween aliases accept concrete `Color`, `Size`, and `Rect` values rather than optional endpoints.
- Fix direction: add nullable tween variants or model these endpoint semantics in the corresponding typed tween APIs.
- Confidence: high

## Revalidation (HEAD a19bf487)

- Revalidated anim_curves-001 through -004 against HEAD. `crates/incular-animation/src/curves.rs`, `tween.rs`, `value.rs`, `animations.rs` and the core geometry are unchanged since a425aa2e, so the cited lines apply. Frame report at HEAD: `anim_curves__curves__interior`, `anim_curves__curve_classes__edges`, `anim_curves__curve2d__spline` and `anim_curves__tweens__scalar` match; `anim_curves__tweens__typed` and `anim_curves__tween_sequence__weighted` diverge as recorded.
- The coverage file `audits/flutter-parity/coverage/anim_curves.jsonl` has concrete statuses for all 73 rows: every Curves constant is `scenario` against its `curves.rs` line; FlippedTweenSequence is `missing`.
## Unexamined cases

- `ParametricCurve<T>`, `Curve`, and `Curve2D` are abstract and were exercised through their concrete implementations.
- Invalid curve parameters, invalid Catmull-Rom control points, empty sequences, and non-positive weights were not run: Flutter protects these primarily with debug assertions, while valid input is the parity target here.
- `CatmullRomCurve` lazy and precomputed constructors were not tested separately; the sampled constructor path and validation were exercised.
- Nullable tween endpoints are source-reviewed as a missing Incular API rather than frame-tested because the Rust aliases cannot express `None`.

## Validation

- `harness/run-frames.sh anim_curves__`: Flutter tests passed; 4/6 scenarios match, with expected divergences in typed tween extrapolation and discontinuous sequence boundaries.
- `cargo fmt -p incular-parity-harness`: passed.
- `cargo clippy -p incular-parity-harness --all-targets -- -D warnings`: passed after concurrent lints in sibling scenarios were resolved.
- `flutter analyze`: passed with no issues after removing redundant imports.

## Harness requests

None.
