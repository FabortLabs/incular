# transitions: Explicit animation widgets

## Summary

Seven paired frame scenarios cover the explicit transition families, builder callbacks, relative positioning, decoration, matrix transforms, default text styling and sliver fade. Six scenarios match all recorded fields. The default text style scenario has the same fractional-size, one-pixel line-box drift already reported as `implicit-002`; it is cross-referenced below rather than duplicated. Source review found missing options and types for slide, matrix, relative-position and decoration transitions, as well as the sliver fade input API. Opacity semantics, directional alignment and default text layout options are cross-referenced to existing findings.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| `AnimatedWidget` | `AnimatedBuilder` / `ListenableBuilder` composition; no direct subclass base | reading | none | partial: no user-defined animation-listening base widget (transitions-007) |
| `AnimatedBuilder`, `ListenableBuilder` | same-named builders | scenario + reading | `transitions__builders__cached_child` | match: build counts and child geometry |
| `FadeTransition` | `FadeTransition` / `FadeTransition::from_animation` | scenario + reading | `transitions__compositor__fade_slide_scale_rotation` | match for opacity; `alwaysIncludeSemantics` gap is covered by `implicit-004` |
| `SlideTransition` | `SlideTransition::from_animation` | scenario + reading | `transitions__compositor__fade_slide_scale_rotation` | match for LTR, hit-testing enabled; direction and hit-test options unavailable (transitions-001/002) |
| `ScaleTransition`, `RotationTransition` | same-named transitions / `from_animation` helpers | scenario + reading | `transitions__compositor__fade_slide_scale_rotation` | match at center alignment. `filterQuality` gaps are covered with `MatrixTransition` (transitions-003). |
| `SizeTransition` | `SizeTransition` | scenario + reading | `transitions__layout__size_clip` | wrapper geometry matches at factors 1 and 0.5. The frame probe is on the outer clip render object, so child clipping is not independently visible; see pixel proposal. Negative fixed cross-axis factor handling differs (transitions-008). |
| `AlignTransition` | `AlignTransition` | scenario + reading | `transitions__layout__align` | match for physical alignment. Directional geometry gap is cross-referenced to `implicit-003`. |
| `PositionedTransition` | `PositionedTransition` | scenario + reading | `transitions__positioned__relative_rect` | match for non-null relative rectangles |
| `RelativePositionedTransition` | `RelativePositionedTransition` | scenario + reading | `transitions__positioned__relative_rect` | match for non-null rectangles; nullable rect behavior unavailable (transitions-004) |
| `DecoratedBoxTransition` | `DecoratedBoxTransition` | scenario + reading | `transitions__paint__decoration_matrix` | color changes match; arbitrary `Decoration` types are not supported (transitions-005) |
| `MatrixTransition` | `MatrixTransition` | scenario + reading | `transitions__paint__decoration_matrix` | translation and default center alignment match; `filterQuality` has no equivalent (transitions-003) |
| `DefaultTextStyleTransition` | `DefaultTextStyleTransition` | scenario + reading | `transitions__text__default_style` | fractional font line-box differs by 1 px at frames 3–5; same root and scale as `implicit-002`, no duplicate finding filed. Paragraph options are cross-referenced to `implicit-006`. |
| `SliverFadeTransition` | `SliverFadeTransition` | scenario + reading | `transitions__sliver_fade__opacity` | opacity and geometry match; `alwaysIncludeSemantics` is cross-referenced to `implicit-004`, and its input type gap is transitions-006 |
| `RelativeRectTween` | `RelativeRect` + generic `TweenAnimation<RelativeRect>` | reading | `transitions__positioned__relative_rect` uses Flutter's tween; Incular interpolates the same sample through a generic wrapper | no named public tween equivalent (transitions-009) |
| `DualTransitionBuilder` | `DualTransitionBuilder` | existing Rust tests + source reading | `crates/incular-widgets/tests/explicit_transitions.rs::dual_transition_builder_routes_the_animation_to_the_running_side` and `::an_interrupted_dual_transition_keeps_playing_the_transition_it_started` | source formulas and basic interruption behavior reviewed; no new Flutter frame twin, residual callback lifecycle untested |

The probes in the size scenario wrap the `SizeTransition` render object so the harness can compare its outer geometry. The existing `crates/incular-widgets/tests/animated_builders.rs::size_transition_clips_the_child_to_its_factor` checks Incular's clip size, but does not compare that clip against Flutter pixels.

## Findings

### transitions-001 — SlideTransition cannot apply reading direction

- Severity: P2
- Category: missing-api
- Flutter: `widgets/transitions.dart:193-199, 235-245` accepts optional `textDirection` and negates the horizontal offset in RTL.
- Incular: `crates/incular-widgets/src/explicit_animations.rs:386-391` maps an `Offset` animation directly into `FractionalTranslation`; that helper has no text-direction parameter or RTL resolution.
- Evidence: code reading. The frame twin uses LTR, where both systems apply the same positive x translation. A Flutter `SlideTransition(textDirection: TextDirection.rtl, position: Offset(0.4, 0))` moves left; Incular's `from_animation` path always uses the offset as canvas coordinates and has no equivalent setting.
- Impact: in RTL layouts, a ported reading-direction slide moves toward the right instead of toward the left.
- Likely cause: the Incular transition consumes the sampled `Offset` without resolving its horizontal component against ambient or explicit direction.
- Fix direction: add an optional text direction and mirror the horizontal component for RTL, or resolve an explicit directional offset type.
- Confidence: high

### transitions-002 — SlideTransition cannot disable transformed hit testing

- Severity: P2
- Category: missing-api
- Flutter: `widgets/transitions.dart:193-199, 221-245` exposes `transformHitTests`, defaulting to true, and passes it to `FractionalTranslation`.
- Incular: `crates/incular-widgets/src/explicit_animations.rs:386-391` creates `FractionalTranslation::new`, whose `transform_hit_tests` field defaults to true at `crates/incular-widgets/src/layout/basic/transforms.rs:39-60`; the transition does not expose a setter.
- Evidence: code reading. The paired LTR scenario uses the matching default. Incular callers cannot request Flutter's `transformHitTests: false`, which lets interaction remain at the child's unshifted position while it is visually sliding.
- Impact: callers cannot preserve target hit regions at the child's final position during a slide.
- Likely cause: the convenience constructor always uses `FractionalTranslation`'s default hit-test policy.
- Fix direction: expose the hit-test policy on `SlideTransition::from_animation` and pass it through to `FractionalTranslation`.
- Confidence: high

### transitions-003 — Matrix and scale transitions omit filter quality

- Severity: P2
- Category: missing-api
- Flutter: `widgets/transitions.dart:276-283, 301-325` accepts `filterQuality` and passes it to `Transform` while the animation runs; `ScaleTransition` and `RotationTransition` forward the property at `:351-361` and `:392-400`.
- Incular: `crates/incular-widgets/src/animation_transitions.rs:402-448` stores animation, callback, alignment and child only. The underlying `crates/incular-widgets/src/tree/descriptors.rs:1375-1454` `Transform` has no filter-quality field.
- Evidence: code reading. The paired matrix translation uses no filter quality and matches geometry. An animated bitmap transform that requests filtering cannot be expressed in Incular.
- Impact: callers cannot select the same sampling quality during scale, rotation or custom matrix transitions; transformed raster content can look different.
- Likely cause: the Incular renderer-neutral transform descriptor does not carry an image-filter quality option.
- Fix direction: add optional filter quality to the transition and retained transform descriptor and honor Flutter's rule to apply it only while the animation runs.
- Confidence: high

### transitions-004 — RelativePositionedTransition cannot represent a null rect

- Severity: P2
- Category: missing-api
- Flutter: `widgets/transitions.dart:820-850` accepts `Animation<Rect?>` and treats a null sample as `Rect.zero`.
- Incular: `crates/incular-widgets/src/animation_transitions.rs:294-327` requires `TweenAnimation<Rect>` and converts the non-null rectangle to a relative rect.
- Evidence: code reading. `transitions__positioned__relative_rect` exercises non-null rectangles and matches; a null animation endpoint cannot be represented by the Incular transition API.
- Impact: a caller cannot animate a positioned child through Flutter's null-to-zero-rect behavior.
- Likely cause: the Incular property tween requires a concrete `Rect` for both endpoints.
- Fix direction: accept an optional rect animation or provide a nullable rect tween that maps `None` to `Rect::ZERO` at layout.
- Confidence: high

### transitions-005 — DecoratedBoxTransition accepts only BoxDecoration

- Severity: P2
- Category: missing-api
- Flutter: `widgets/transitions.dart:885-908` accepts `Animation<Decoration>` and paints its sampled decoration.
- Incular: `crates/incular-widgets/src/animation_transitions.rs:333-365` requires `TweenAnimation<BoxDecoration>`.
- Evidence: code reading and `transitions__paint__decoration_matrix`, where box-color interpolation matches. Flutter callers can animate other `Decoration` implementations through this transition, but Incular's parameter type only admits `BoxDecoration`.
- Impact: callers cannot port transitions over `ShapeDecoration` or another `Decoration` implementation.
- Likely cause: the retained transition is specialized to the box-decoration data type.
- Fix direction: accept a renderer-neutral decoration interpolation interface that covers Flutter's supported decoration types, or document and name the API as box-only.
- Confidence: high

### transitions-006 — SliverFadeTransition requires an OpacityController

- Severity: P2
- Category: missing-api
- Flutter: `widgets/transitions.dart:658-697` accepts any `Animation<double>` and exposes its sampled opacity.
- Incular: `crates/incular-widgets/src/scrolling/sliver_fade.rs:16-33` accepts only `OpacityController`.
- Evidence: code reading and `transitions__sliver_fade__opacity`, which matches after setting the controller directly. A curved, reversed or composed animation cannot be passed into this widget.
- Impact: callers cannot reuse the same animation graph across box and sliver fade transitions or provide a derived opacity animation.
- Likely cause: the sliver wrapper is coupled to the retained opacity controller instead of accepting a scalar animation source.
- Fix direction: add a constructor that accepts `DoubleAnimation` and updates opacity from that source, preserving the existing controller constructor if useful.
- Confidence: high

### transitions-007 — AnimatedWidget has no direct public base equivalent

- Severity: P3
- Category: missing-api
- Flutter: `widgets/transitions.dart:80-157` provides an `AnimatedWidget` base that subscribes to a `Listenable`, invokes an overridable `build`, and detaches its listener when replaced or disposed.
- Incular: `crates/incular-widgets/src/explicit_animations.rs:95-117, 121-160` provides an internal `animated_widget_for` helper and public closure-based `AnimatedBuilder`, but no public widget base to subclass.
- Evidence: source reading. `transitions__builders__cached_child` verifies the closure builder behavior, not custom `AnimatedWidget` subclass lifecycle.
- Impact: callers that port a reusable custom stateful animated widget must restructure it around a closure builder.
- Likely cause: Incular exposes retained closure builders and transition composition rather than a user-subclassable widget-state base.
- Fix direction: consider a public builder or retained-state trait with explicit listenable lifecycle if direct subclass parity is a supported goal.
- Confidence: high

### transitions-008 — SizeTransition clamps negative fixed cross-axis factors

- Severity: P2
- Category: behavior
- Flutter: `widgets/transitions.dart:452-460` asserts that a non-null `fixedCrossAxisSizeFactor` is non-negative.
- Incular: `crates/incular-widgets/src/animation_transitions.rs:223-228` silently clamps the supplied factor with `.max(0.0)`.
- Evidence: code reading. The valid-factor twin `transitions__layout__size_clip` matches the wrapper geometry; behavior differs only for a negative invalid factor.
- Impact: debug builds surface a bad configuration in Flutter but silently produce a zero cross-axis factor in Incular.
- Likely cause: the Incular builder normalizes invalid input instead of enforcing Flutter's assertion contract.
- Fix direction: align the documented validity check and failure behavior, or clearly document the intentional normalization.
- Confidence: high

### transitions-009 — RelativeRectTween has no named public equivalent

- Severity: P3
- Category: missing-api
- Flutter: `widgets/transitions.dart:720-728` exposes `RelativeRectTween` as a named `Tween<RelativeRect>`.
- Incular: `crates/incular-widgets/src/animation_transitions.rs:27-124, 580-627` provides a `RelativeRect` value and generic `TweenAnimation<T>` but no named animatable tween type.
- Evidence: source reading and the positioned twin, which uses the generic `TweenAnimation` directly.
- Impact: callers cannot port code that constructs or composes Flutter's named `RelativeRectTween` without substituting a generic Incular tween.
- Likely cause: Incular centralizes typed interpolation in generic tween wrappers rather than mirroring every Flutter named subclass.
- Fix direction: expose a named `RelativeRectTween` if Flutter API-level source compatibility is a goal, or document the generic replacement.
- Confidence: high

## Pixel scene proposals

The frame scenarios already compare geometry and the first solid fill. The text scenario's one-pixel line-box difference is the same fractional Inter metric issue recorded in `implicit-002`; it should be checked with the existing text pixel scenes rather than duplicated here. A SizeTransition clip scene can use the same static scene in both harnesses:

```dart
canvas(Colors.black, [
  Stack(children: [
    Positioned(
      left: 20,
      top: 20,
      child: SizeTransition(
        sizeFactor: const AlwaysStoppedAnimation<double>(0.5),
        axis: Axis.vertical,
        child: const SizedBox(width: 120, height: 100, child: ColoredBox(color: Colors.white)),
      ),
    ),
  ]),
]);
```

```rust
canvas(
    Color::BLACK,
    vec![Widget::from(
        Positioned::new(SizeTransition::new(
            0.5,
            SizedBox::new()
                .width(120.)
                .height(100.)
                .child(ColoredBox::new(Color::WHITE, SizedBox::shrink())),
        ))
        .left(20.)
        .top(20.),
    )],
)
```

This checks clip extent and paint outside the half-height transition. The frame twin only measures the outer transition geometry.

## Harness requests

None.

## Open questions

- `FadeTransition.alwaysIncludeSemantics` and `SliverFadeTransition.alwaysIncludeSemantics` are unavailable in Incular; cross-reference `implicit-004` instead of duplicating that finding.
- `AlignTransition` does not accept directional `AlignmentGeometry`; cross-reference `implicit-003`.
- `DefaultTextStyleTransition` lacks Flutter's paragraph options; cross-reference `implicit-006`.
- The text scenario reports the same intermittent 1 px fractional font line-box drift as `implicit-002`; its cause is not specific to transitions.
- `DualTransitionBuilder`'s existing Rust tests cover effective direction, interruption, and routed values. Listener replacement/disposal ordering and direct Flutter twin verification remain unchecked.

## Status at clean stop

- Done: coverage rows rewritten (`coverage/transitions.jsonl`); findings 001-009 revalidated at HEAD (references updated; transitions-003 confirmed since the Transform descriptor has no filter quality); scenario `transitions__size__reverse_mid_flight` matches.
- Not done: findings JSONL not normalized; RepeatingAnimationBuilder, ListenableBuilder and DualTransitionBuilder lifecycle not frame-tested; navigation-owned rows reassigned, not audited.

## Resume update

- JSONL normalized; transitions-010 added (StatusTransitionWidget has no Incular equivalent, P3). All other transitions rows have a status; navigation-owned rows reassigned. Frame run: 8/9 match (text default-style diverges as implicit-002).
