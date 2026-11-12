# implicit: Implicitly animated widgets

## Summary

Seventeen matching frame scenarios cover the implicit widget family and its shared timing behavior. The Flutter tests pass; the frame comparison matches 15 of 17 scenarios. One real timing difference remains: `SliverAnimatedOpacity` starts one 16 ms frame later in Incular. `AnimatedDefaultTextStyle` has a separate, intermittent one-pixel text line-box difference while fractional font sizes are laid out. Retargeting, `onEnd`, zero duration, and reduced-motion behavior match. Source review found unsupported directional geometry inputs, opacity semantic-preservation options, physical-model shapes, and default-text paragraph options.

## Coverage

| Flutter item | Scenario / evidence | Result |
| --- | --- | --- |
| `BoxConstraintsTween` | `implicit__container__fields` changes tight width and height | Geometry matches. |
| `DecorationTween` | `implicit__container__fields` changes the background `BoxDecoration` color | Recorded color matches. Border and radius paint are not recorded. |
| `EdgeInsetsTween` | `implicit__padding__edge_insets` changes all four physical insets | Child geometry matches. |
| `EdgeInsetsGeometryTween` | Flutter source review; no matching directional input can be expressed by Incular's implicit widget APIs | Not frame-tested; see implicit-003. |
| `BorderRadiusTween`, `BorderTween` | `implicit__container__fields` and `implicit__physical_model__color_elevation_radius` exercise these paths | Frame geometry/color cannot observe corner radii or border paint; pixel harness was not run. |
| `Matrix4Tween` | `implicit__container__fields` animates a translation and transform alignment | Transform and geometry match. |
| `TextStyleTween` | `implicit__default_text_style__text_style` animates font size and color | Width tracks; line-box height differs by 1 px on five intermediate frames (implicit-002). |
| `ImplicitlyAnimatedWidget`, state and base-state lifecycle | `implicit__retarget__current_value_and_on_end`, `implicit__zero_duration__completion`, and `implicit__disabled_animations__reduced_motion` | Retargeting from the current value, callback count, zero duration, and disabled-animation scaling match. |
| `AnimatedContainer` | `implicit__container__fields` changes size, margin, padding, alignment, transform, and decoration | Recorded geometry, transform, and color match. `foregroundDecoration`, border, and radius are not visible to the frame recorder. |
| `AnimatedPadding` | `implicit__padding__edge_insets` | Match for physical insets. Directional values are unavailable in Incular (implicit-003). |
| `AnimatedAlign` | `implicit__align__alignment_and_factors` | Alignment and factors match. Directional alignment input is unavailable (implicit-003). |
| `AnimatedPositioned` | `implicit__positioned__edges_and_size` | Edges and size match. |
| `AnimatedPositionedDirectional` | `implicit__positioned_directional__direction` switches LTR to RTL during the tween | Match. |
| `AnimatedScale` | `implicit__scale__alignment` | Geometry and transform match. Filter-quality paint behavior was not checked. |
| `AnimatedRotation` | `implicit__rotation__turns_and_alignment` | Geometry and transform match. Filter-quality paint behavior was not checked. |
| `AnimatedSlide` | `implicit__slide__fractional_offset` | Fractional translation matches. |
| `AnimatedOpacity` | `implicit__opacity__opacity`; retarget and reduced-motion scenarios | Opacity and timing match. `alwaysIncludeSemantics` behavior is not available (implicit-004). |
| `SliverAnimatedOpacity` | `implicit__sliver_opacity__opacity` | Opacity is delayed by one 16 ms frame (implicit-001). Semantics option is also unavailable (implicit-004). |
| `AnimatedDefaultTextStyle` | `implicit__default_text_style__text_style` | Small intermittent line-box difference (implicit-002). Flutter's alignment, wrapping, overflow, line-count, width-basis, and height-behavior options are unavailable (implicit-006). |
| `AnimatedPhysicalModel` | `implicit__physical_model__color_elevation_radius` | Recorded frame fields match. Elevation, shadow, radius, and clipping are outside the frame recorder; circle shape is unsupported (implicit-005). |
| `AnimatedFractionallySizedBox` | `implicit__fractional_sized_box__factors` | Factors and alignment match. Directional alignment input is unavailable (implicit-003). |
| `TweenAnimationBuilder` | `implicit__tween_animation_builder__initial_and_retarget` samples the builder value and `onEnd` | Initial `begin`, retarget from current value, ignored later `begin`, and completion match. |
| Target changes while active, `onEnd`, zero duration, disabled animations | `implicit__retarget__current_value_and_on_end`, `implicit__zero_duration__completion`, `implicit__disabled_animations__reduced_motion` | Match. Duration-only/curve-only updates were not separately frame-tested; source paths were reviewed. |

## Findings

### implicit-001 — Sliver opacity starts one frame late

- Severity: P2
- Category: behavior
- Flutter: `packages/flutter/lib/src/widgets/implicit_animations.dart:387-404` updates the tween and starts the controller during `didUpdateWidget` with `forward(from: 0.0)`.
- Incular: `crates/incular-widgets/src/scrolling/layout.rs:2058-2068` transfers retained sliver state; `crates/incular-widgets/src/scrolling/sliver_fade.rs:131-147` adopts the old controller and schedules `forward_on_next_tick`.
- Evidence: `implicit__sliver_opacity__opacity` has matching frame-0 geometry and opacity. At frame 4 / 64 ms Flutter is at `0.9963184`, while Incular is still `1.0`; Incular frame 5 / 80 ms then matches Flutter frame 4. Both reach `0.25` at frame 26, so the visible difference is a 16 ms start delay rather than a different endpoint.
- Impact: the visible opacity trajectory lags Flutter's by one frame during the fade, then converges to the same target.
- Cause: retained sliver adoption schedules the new controller after the current frame's tick, whereas Flutter starts it during widget update before the next frame tick.
- Fix direction: begin the adopted sliver animation early enough for its first elapsed tick to match Flutter's first post-update frame.
- Confidence: high

### implicit-002 — Animated text has intermittent one-pixel line-box drift

- Severity: P3
- Category: layout
- Flutter: `packages/flutter/lib/src/widgets/implicit_animations.dart:2140-2156` evaluates `TextStyleTween` and rebuilds `DefaultTextStyle` on each animation tick.
- Incular: `crates/incular-widgets/src/implicit_animations.rs:1146-1160` evaluates its `TextStyle` tween; text layout is produced by `crates/incular-text/src/engine.rs:1649-1680`.
- Evidence: `implicit__default_text_style__text_style` matches at frame 0. Its first difference is frame 8 / 128 ms: Flutter's text size is `[65.0670, 19]`, Incular's is `[65.0670, 20]`. The horizontal extents differ by less than `0.00002` px, while top and bottom differ by `0.5` px. The same one-pixel line-box difference appears at five intermediate frames and disappears at other adjacent frames.
- Impact: a text widget's reported layout bounds can shift by one logical pixel during a font-size animation, which can also move nearby content.
- Cause: the recorded tween progress and text width agree; the remaining difference is in line-box metrics for fractional font sizes. This points to text-engine metric rounding rather than implicit-animation timing.
- Fix direction: compare Flutter and Incular line metrics for the same Inter font sizes and align the fractional-size line-box rounding.
- Confidence: medium

### implicit-003 — Implicit layout widgets cannot animate directional geometry

- Severity: P2
- Category: missing-api
- Flutter: `packages/flutter/lib/src/widgets/implicit_animations.dart:120-129,663-688,915,1025` uses `EdgeInsetsGeometry` and `AlignmentGeometry`; `packages/flutter/lib/src/rendering/tweens.dart:63-70` supplies `AlignmentGeometryTween`.
- Incular: `crates/incular-widgets/src/implicit_animations.rs:291-300,342-359,471-487,519-537,619-650` accepts only `Alignment` and physical `EdgeInsets`. `incular-config` has separate directional value types at `src/alignment.rs:239` and `src/insets.rs:243`, but the implicit widget setters do not accept or interpolate those types.
- Evidence: source review; a twin using Flutter's `AlignmentDirectional` or `EdgeInsetsDirectional` cannot be expressed through Incular's `AnimatedContainer`, `AnimatedPadding`, `AnimatedAlign`, or `AnimatedFractionallySizedBox` APIs.
- Impact: callers cannot preserve start/end alignment or insets through these implicit widgets in RTL layouts, and cannot interpolate between physical and directional geometry as Flutter allows.
- Cause: the widget APIs and tween fields are specialized to the physical `Alignment` and `EdgeInsets` structs.
- Fix direction: accept Flutter-equivalent geometry enums and interpolate them with direction-aware geometry lerps.
- Confidence: high

### implicit-004 — Opacity widgets cannot match Flutter's semantic visibility option

- Severity: P2
- Category: semantics
- Flutter: `packages/flutter/lib/src/widgets/implicit_animations.dart:1852-1906,1965-2019` exposes `alwaysIncludeSemantics`, defaulting to false; `packages/flutter/lib/src/rendering/proxy_box.dart:957,1084` omits child semantics at zero opacity unless that flag is true.
- Incular: `crates/incular-widgets/src/tree/descriptors.rs:1279-1295` states that opacity never affects semantic exposure, and `crates/incular-widgets/src/implicit_animations.rs:1079-1119` plus `crates/incular-widgets/src/scrolling/sliver_fade.rs:51-90` expose no equivalent option.
- Evidence: source review. Incular keeps fully transparent opacity children semantically exposed; Flutter hides them by default and permits callers to retain them explicitly.
- Impact: at opacity zero, accessibility services may announce or navigate to content Flutter removes from the semantic tree; Incular callers cannot select Flutter's `alwaysIncludeSemantics` behavior.
- Cause: Incular opacity is paint-only for hit testing and semantic exposure and has no policy field for the Flutter option.
- Fix direction: add the option to box and sliver implicit opacity widgets and apply it when projecting semantics, defaulting to Flutter's false behavior.
- Confidence: high

### implicit-005 — AnimatedPhysicalModel has no circular shape option

- Severity: P2
- Category: missing-api
- Flutter: `packages/flutter/lib/src/widgets/implicit_animations.dart:2179-2211,2294` accepts a `BoxShape` and passes it to `PhysicalModel`; the shape is immediate while radius and elevation animate.
- Incular: `crates/incular-widgets/src/implicit_animations.rs:1169-1174,1258-1275` documents and builds its physical model as a rounded rectangle with no shape field.
- Evidence: source review. An `AnimatedPhysicalModel(shape: BoxShape.circle, ...)` twin is not representable in Incular.
- Impact: callers cannot create Flutter's circular physical model or match its clip and shadow silhouette.
- Cause: Incular's physical-model widget stores corner radii but does not model Flutter's rectangle/circle shape choice.
- Fix direction: add the shape option to the widget and renderer-neutral physical-model descriptor, with matching paint and clip behavior.
- Confidence: high

### implicit-006 — AnimatedDefaultTextStyle omits Flutter's paragraph options

- Severity: P3
- Category: missing-api
- Flutter: `packages/flutter/lib/src/widgets/implicit_animations.dart:2044-2101,2152-2162` accepts `textAlign`, `softWrap`, `overflow`, `maxLines`, `textWidthBasis`, and `textHeightBehavior`, applying them immediately alongside the animated style.
- Incular: `crates/incular-widgets/src/implicit_animations.rs:1123-1127,1133-1143,1157-1160` stores only the style, child, and timing.
- Evidence: source review. No frame twin can specify those Flutter options with Incular's `AnimatedDefaultTextStyle`.
- Impact: callers cannot configure inherited paragraph wrapping, truncation, alignment, or line-height behavior through the animated style widget.
- Cause: Incular's default text style descriptor only carries the `TextStyle` value.
- Fix direction: add the same paragraph options to `AnimatedDefaultTextStyle` and pass them through to its default-style scope.
- Confidence: high

## Harness requests

- The physical-model fill recorder support was extended during this audit; `implicit__physical_model__color_elevation_radius` now matches its color field. Frame traces still cannot observe elevation, shadow color, border radius, border paint, or clipping. A future opt-in recorder field for these properties or a pixel scenario would cover those properties; pixel execution was outside this audit request.
- The frame recorder does not expose semantic-tree nodes, so `alwaysIncludeSemantics` was source-reviewed only.

## Unexamined portions

- `AnimatedContainer.foregroundDecoration`, decoration border interpolation, border-radius interpolation, and `AnimatedPhysicalModel` elevation, shadow, radius, and clip output are not measurable in the current frame schema; no pixel harness was run.
- `AnimatedScale` and `AnimatedRotation` `filterQuality` changes affect transformed paint quality, which frame geometry does not record; no pixel harness was run.
- Directional geometry variants were not framed because the corresponding Incular implicit API cannot represent the inputs; the missing API is documented in implicit-003.
- Duration-only and curve-only widget updates were reviewed in source but not given a standalone frame twin; the frame suite does cover active-target retargeting and curve application.
- Flutter constructor assertions for invalid opacity, negative factors, constraints, and invalid `AnimatedContainer` decoration combinations were not exercised; scenarios use valid values.
