# paint: Painting widgets

## Summary

Revision `a19bf487`. All 189 inventory rows carry a final status; the only `unchecked` row is `positionDependentBox`, which needs a positioned-gradient pixel scene. Four paired `paint__data__*` frame scenarios were added at HEAD. Three diverge (EdgeInsets and Alignment lerp extrapolation, HSV hue wrap) and one matches (BorderRadius lerp). Accepted from SUMMARY: paint-001, 002, 003, 004, 005, 006 and 015. Pending review: paint-007 to 014 and the new candidates paint-016 to 022. Pixel evidence for accepted items was recorded at a425aa2e and needs a HEAD rerun (see Harness requests).

## Coverage

Frame run `harness/run-frames.sh paint__` at HEAD: 9 scenarios, 6 match, 3 diverge.

| Scenario | Result | First divergence |
| --- | --- | --- |
| `paint__opacity__phase_values` | match | - |
| `paint__opacity__zero_keeps_layout` | match | - |
| `paint__decorated_box__solid_background` | match | - |
| `paint__raw_image__intrinsic_scale` | match | - |
| `transitions__paint__decoration_matrix` | match | - |
| `paint__data__border_radius_lerp_extrapolated` | match | - |
| `paint__data__edge_insets_lerp_extrapolated` | DIVERGES | frame 0, value.left Flutter 26 vs Incular 20 (paint-020) |
| `paint__data__alignment_lerp_extrapolated` | DIVERGES | frame 0, value.x Flutter 2 vs Incular 1 (paint-020) |
| `paint__data__hsv_lerp_hue_wrap` | DIVERGES | frame 0, value.hue Flutter 330 vs Incular 150 (paint-021) |

Row-level statuses are in `audits/flutter-parity/coverage/paint.jsonl`. Main groups:

| Flutter item | Incular counterpart | Status | Findings / scenarios |
| --- | --- | --- | --- |
| `Opacity`, `RenderOpacity` | `tree/descriptors.rs:1283` | scenario+reading / partial-scenario | paint__opacity__*; semantics aspect implicit-004 |
| `ClipRect`, `ClipRRect`, `ClipOval`, `ClipPath` | `layout/basic/clipping.rs` | partial-reading | paint-010 |
| `ClipRSuperellipse` | `painting_effects/layers.rs:36` | partial-reading | paint-006 |
| `DecoratedBox`, `BoxDecoration` | `tree/descriptors.rs:445`, `decoration.rs:451` | partial-scenario | paint__decorated_box__solid_background; paint-003, paint-016 |
| `DecorationImage`, `paintImage` | `decoration.rs:411`, `box_painter.rs:305` | partial | paint-003, paint-016 |
| `ShapeDecoration`, shape borders | `shape_decoration.rs`, `shape_borders.rs` | partial-reading | paint-011, paint-017 |
| `PhysicalModel`, `PhysicalShape` | `layers.rs:75,151` | partial-reading | paint-004, paint-005 |
| `ColorFiltered`, `ColorFilterLayer` | `descriptors.rs:1720`, `display_list.rs:153` | partial-reading | paint-008, paint-015 |
| `ShaderMask`, `ShaderMaskLayer` | `compositing.rs:75` | partial-reading | paint-022 |
| `BackdropFilter`, `BackdropGroup` | `compositing.rs:121,235` | partial-reading | paint-001 |
| `ImageFiltered` | `painting_effects/images.rs:196` | partial-reading | paint-007 |
| `CustomPaint`, `CustomPainter` | `layout/basic/painting.rs:74`, `custom_paint.rs` | partial-reading | paint-009 |
| `Image`, `RawImage`, `ImageIcon` | `descriptors.rs:34`, `images.rs:12,140` | partial-scenario / partial-reading | paint__raw_image__intrinsic_scale; paint-002, 012, 013 |
| `FadeInImage` | `fade_in_image.rs:23` | partial-reading | paint-014 |
| Gradients | `incular-rendering/src/gradients.rs` | partial-reading / missing | paint-018 |
| `ImageProvider`, `AssetImage`, `ImageConfiguration` | `incular-image/src/lib.rs` | partial-reading / missing | paint-012, paint-019 |
| `EdgeInsets`, `Alignment`, `BorderRadius`, `HSVColor` | `incular-config`, `incular-core/src/geometry.rs` | scenario+reading / partial-scenario | paint-020, paint-021 |
| `BoxShadow` | `decoration.rs:326` | partial-reading | paint-016 |

Out-of-scope rows are debug, web-only, Material-only, engine-only or diagnostic items, each with its reason in the JSONL. Rows belonging to other packages carry `reassign` (services, text).

## Findings

### paint-001 — BackdropFilter cannot accept general image filters

- Severity: P2
- Category: missing-api
- Flutter: `widgets/basic.dart:699` accepts a `ui.ImageFilter`; `bin/cache/pkg/sky_engine/lib/ui/painting.dart:4302` and `:4331` provide dilation and composed filters.
- Incular: `crates/incular-widgets/src/compositing.rs:121` stores a blur and optional matrix; `:135`, `:171`, and `:189` expose those filter forms only.
- Evidence: code reading of the references above.
- Impact: an app cannot express a backdrop dilation or a composed backdrop filter through this widget.
- Likely cause: the widget's filter representation is restricted to two specific effects.
- Fix direction: provide a renderer-neutral representation for the required filter forms and explicitly report unsupported backend execution where applicable.
- Confidence: high

### paint-002 — Image defaults scale small sources up

- Severity: P1
- Category: default-value
- Flutter: `widgets/image.dart:363-388` leaves `fit` null by default; `painting/decoration_image.dart:561` resolves null to `BoxFit.scaleDown` when there is no center slice.
- Incular: `crates/incular-widgets/src/tree/descriptors.rs:46-59` defaults `Image.fit` to `ImageFit::Contain`; `painting_effects/images.rs:12-54` gives `RawImage` the same default.
- Evidence: code reading; `paint__raw_image__intrinsic_scale` matches geometry, but the frame recorder cannot see the fitted paint rect. Pixel scene `image_default_fit` supplies a 40×20 image to a 100×100 destination; at 1x, 16.41% of pixels differ by more than 16 channel levels (maximum 255) at a425aa2e.
- Impact: a smaller source remains at native size in Flutter but is enlarged to fill the destination in Incular.
- Likely cause: Incular stores `Contain` as the default where Flutter defers to `paintImage`'s scale-down default. FadeInImage inherits this default.
- Fix direction: make the default represent Flutter's nullable fit or use scale-down when fit is omitted.
- Confidence: high

### paint-003 — DecorationImage defaults to cover instead of scale-down

- Severity: P1
- Category: default-value
- Flutter: `painting/decoration_image.dart:48-84` leaves `fit` null; its paint path resolves that to `BoxFit.scaleDown` at `:561` when `centerSlice` is null.
- Incular: `crates/incular-widgets/src/painting_effects/decoration.rs:411-443` defaults `DecorationImage.fit` to `ImageFit::Cover`.
- Evidence: pixel scene `decoration_image_default_fit` (40×20 source, 100×100 box) differs in 35.94% of pixels at every tested scale at a425aa2e, maximum 255.
- Impact: default decoration images can be enlarged and cropped in Incular where Flutter keeps them at or below intrinsic size.
- Likely cause: Incular uses an explicit cover fit instead of preserving Flutter's omitted-fit behavior.
- Fix direction: align the default to scale-down while retaining explicit `Cover` for callers who request it.
- Confidence: high

### paint-004 — PhysicalShape drops fill, elevation, shadow and clip behavior

- Severity: P2
- Category: paint
- Flutter: `widgets/basic.dart:1471-1529` passes `clipper`, `clipBehavior`, `elevation`, `color` and `shadowColor` to `RenderPhysicalShape`; `clipBehavior` defaults to `Clip.none`.
- Incular: `crates/incular-widgets/src/painting_effects/layers.rs:151-194` stores color, elevation and shadow color but lowers only to `ClipPath::new(value.clipper, value.child)`, whose clip defaults to anti-aliasing (`layout/basic/clipping.rs:117-125`).
- Evidence: pixel scene `physical_shape_triangle` at a425aa2e: 30.08% of pixels differ by more than 16 channel levels at 1x. The existing `painting_effects.rs` test checks only the ClipPath lowering. Not re-measured at HEAD.
- Impact: a PhysicalShape does not paint its configured surface or elevation shadow, and clips by default where Flutter does not.
- Likely cause: the lowering discards every physical property except the path and child.
- Fix direction: retain a physical-shape render representation carrying clip mode, fill and elevation/shadow.
- Confidence: high

### paint-005 — PhysicalModel ignores its shape and clipBehavior

- Severity: P2
- Category: paint
- Flutter: `widgets/basic.dart:1371-1437` supports `BoxShape.rectangle` or `BoxShape.circle` and clips the child when `clipBehavior` requests it.
- Incular: `crates/incular-widgets/src/painting_effects/layers.rs:75-147` stores no shape field; `From<PhysicalModel>` builds a rounded `DecoratedBox`/`DropShadow` and never reads `clip_behavior`. The animated wrapper's missing circle option is implicit-005 (same root cause, not a duplicate).
- Evidence: diagnostic pixel scene `physical_model_circle` (non-equivalent: Incular has no shape option). At a425aa2e, 12.66% of pixels differ at 1x. The clipping claim is source-only.
- Impact: callers cannot request a circular physical model, and a child that should be clipped can paint past the model boundary.
- Likely cause: conversion substitutes decoration and shadow widgets without preserving shape or clipping.
- Fix direction: preserve the model shape and clip behavior through paint lowering.
- Confidence: high

### paint-006 — ClipRSuperellipse is rendered as a rounded rectangle

- Severity: P2
- Category: paint
- Flutter: `widgets/basic.dart:1123-1188` creates `RenderClipRSuperellipse` from `BorderRadiusGeometry`.
- Incular: `crates/incular-widgets/src/painting_effects/layers.rs:36-68` converts it to `ClipRRect` with uniform corner radii, although `Path::rounded_superellipse` exists (`incular-rendering/src/paths.rs:337-340`).
- Evidence: pixel scene `clip_r_superellipse` at a425aa2e: 0.61% of pixels differ at 1x, maximum 122.
- Impact: corners follow circular arcs instead of Flutter's squircle profile, visibly changing larger-radius edges.
- Likely cause: lowering targets the RRect clip path instead of the existing superellipse primitive.
- Fix direction: preserve the rounded-superellipse clip geometry through render lowering.
- Confidence: high

### paint-007 — ImageFiltered supports blur only

- Severity: P2
- Category: missing-api
- Flutter: `widgets/image_filter.dart:39-51` accepts any `ui.ImageFilter` and exposes `enabled`.
- Incular: `crates/incular-widgets/src/painting_effects/images.rs:196-231` stores only a sigma (or `BlurController`) and lowers to `Blur`.
- Evidence: code reading; the existing `blur` pixel scene checks the supported case.
- Impact: callers cannot apply non-blur filters or disable an existing filter while keeping its configuration.
- Likely cause: the widget maps `ImageFiltered` directly to the blur layer.
- Fix direction: add renderer-neutral image-filter variants and the enabled state.
- Confidence: high

### paint-008 — ColorFiltered accepts matrix filters only

- Severity: P2
- Category: missing-api
- Flutter: `widgets/color_filter.dart:36-52` accepts `ui.ColorFilter`; `sky_engine/lib/ui/painting.dart:3998-4087` defines mode, matrix and both gamma filters.
- Incular: `crates/incular-rendering/src/effects.rs:43-58` represents a 4x5 matrix, which `crates/incular-widgets/src/tree/descriptors.rs:1720-1750` uses as the complete filter type.
- Evidence: code reading.
- Impact: callers cannot request blend-mode or gamma-transfer `ColorFiltered` effects.
- Likely cause: the color-filter layer has only one matrix representation.
- Fix direction: extend the renderer-neutral color-filter enum and keep matrix behavior as one variant.
- Confidence: high

### paint-009 — CustomPaint cannot layer foregroundPainter and child

- Severity: P2
- Category: missing-api
- Flutter: `widgets/basic.dart:831-884` accepts `painter`, `foregroundPainter` and `child`; it paints the foreground after the child.
- Incular: `crates/incular-widgets/src/layout/basic/painting.rs:74-95` stores a fixed size and one `DisplayList`; `from_painter` evaluates `paint` immediately. `CustomPainter::should_repaint` (`painting_effects/custom_paint.rs:8`) has no call site in the widget implementation.
- Evidence: code reading; the existing `path` pixel scene exercises only static drawing.
- Impact: apps cannot express custom-paint-before-child and after-child ordering in one `CustomPaint`, or the `shouldRepaint` lifecycle.
- Likely cause: custom paint is a static display list rather than a painter delegate with child slots.
- Fix direction: retain painter delegates and foreground/background ordering with child updates.
- Confidence: high

### paint-010 — Clip widgets do not accept CustomClipper delegates

- Severity: P2
- Category: missing-api
- Flutter: `widgets/basic.dart:950-1353` exposes nullable `CustomClipper` callbacks on `ClipRect`, `ClipRRect`, `ClipOval` and `ClipPath`; the delegate derives geometry from the laid-out size and controls reclipping through `shouldReclip`.
- Incular: `crates/incular-widgets/src/layout/basic/clipping.rs:18-144` accepts static behavior, radii and path values only.
- Evidence: code reading; `clip_opacity.rs` verifies static clip paths only.
- Impact: callers cannot provide a size-dependent clip delegate with Flutter's reclip lifecycle.
- Likely cause: clip geometry is stored in the descriptor rather than resolved by a delegate at layout or paint time.
- Fix direction: add a retained clipper abstraction that receives current size and decides whether a new clip is needed.
- Confidence: high

### paint-011 — ShapeDecoration cannot paint a DecorationImage

- Severity: P2
- Category: missing-api
- Flutter: `painting/shape_decoration.dart:65-140` accepts an `image` field (line 140) and paints it inside the shape outline over its color or gradient.
- Incular: `crates/incular-widgets/src/painting_effects/shape_decoration.rs:17-52` stores color, gradient, shadows and shape but no image field; `paint` fills only color or gradient and strokes the border.
- Evidence: code reading; existing `shape_decoration.rs` tests cover paths and borders, not images.
- Impact: an app cannot decorate an arbitrary `ShapeBorder` with an image clipped to that shape.
- Likely cause: the shape decoration painter has no image configuration or image stage.
- Fix direction: add a decoration-image field and paint it clipped to the shape's outer path.
- Confidence: high

### paint-012 — Image lacks Flutter's image-stream lifecycle options

- Severity: P2
- Category: missing-api
- Flutter: `widgets/image.dart:343-388` accepts `ImageProvider`, `frameBuilder`, `loadingBuilder`, `errorBuilder` and `gaplessPlayback`.
- Incular: `crates/incular-widgets/src/tree/descriptors.rs:34-112` takes an already-decoded `ImageHandle`. `incular-image` has an `ImageProvider` trait (`lib.rs:117-126`), but no widget consumes it (rg of `crates/incular-widgets/src` finds only the re-export at `lib.rs:233`). No stream, chunk events, multi-frame sequence or precache API exists.
- Evidence: code reading; `image_layout.rs` tests decoded-handle layout and fit only.
- Impact: callers must implement loading, error and gapless replacement outside `Image`, and cannot receive Flutter's frame-builder lifecycle or animated multi-frame images.
- Likely cause: the widget consumes a final image handle instead of a provider or stream state.
- Fix direction: connect the existing provider trait to a provider-backed image state with builder and gapless-replacement options.
- Confidence: high

### paint-013 — RawImage exposes fewer paint options than Flutter

- Severity: P2
- Category: missing-api
- Flutter: `widgets/basic.dart:6828-6993` exposes alignment, center slice, match-text-direction, invert colors, filter quality, anti-aliasing, animated opacity and color blend mode.
- Incular: `crates/incular-widgets/src/painting_effects/images.rs:12-99` exposes dimensions, scale, tint color, fit and repeat; `From<RawImage>` (around `:103-135`) forwards those and implements color as a constant tint.
- Evidence: code reading; `paint__raw_image__intrinsic_scale` (match) covers intrinsic geometry only.
- Impact: raw-image callers cannot configure alignment, nine-slice, mirroring, invert, quality, opacity or arbitrary tint blend.
- Likely cause: `RawImage` is a smaller convenience wrapper around the basic `Image` descriptor.
- Fix direction: carry the missing fields through to the image paint command and color-filter layer.
- Confidence: high

### paint-014 — FadeInImage omits several paint and load-error options

- Severity: P2
- Category: missing-api
- Flutter: `widgets/fade_in_image.dart:61-100` includes image and placeholder error builders, separate image colors and blend modes, filter qualities, repeat and match-text-direction options.
- Incular: `crates/incular-widgets/src/fade_in_image.rs:22-130` supports predecoded handles, fade timing and curves, size, fit, placeholder fit, alignment and semantics. The default fit inherits paint-002.
- Evidence: code reading; existing `fade_in_image.rs` tests verify synchronous display and placeholder/target opacity timing.
- Impact: callers cannot reproduce Flutter's error callbacks, separate placeholder/image filters, or repeated/mirrored paint options.
- Likely cause: the widget owns only its fade state and relies on the limited `Image` descriptor.
- Fix direction: expose and forward the missing paint and provider-error options while retaining the tested fade timeline.
- Confidence: high

### paint-015 — ColorFilter matrix translations use different units

- Severity: P1
- Category: paint
- Flutter: `bin/cache/pkg/sky_engine/lib/ui/painting.dart:4029-4071` specifies a 4x5 matrix whose translation column is in the 0..255 range (its invert example uses 255 translations).
- Incular: `crates/incular-rendering/src/effects.rs:43-58` documents 4x5 input in normalized 0..1 straight RGBA, and `incular-wgpu/src/shaders/color_matrix_shader.wgsl:19-31` adds the translation column directly to normalized channels, then clamps.
- Evidence: pixel scene `color_filter_matrix_bias` at a425aa2e: 25% of pixels differ by more than 16 channel levels at every scale, maximum 255. The shader path is unchanged since a425aa2e.
- Impact: Flutter's documented invert matrix produces black from white, while the same literal in Incular clamps `255 - 1` to white.
- Likely cause: the matrix layout matches Flutter but the translation normalization does not.
- Fix direction: convert Flutter-style bias values to the normalized representation at the widget boundary, or expose and document a parity-compatible constructor.
- Confidence: high

### paint-016 — Accepted paint parameters are stored but never painted

- Severity: P2
- Category: behavior
- Flutter: `painting/box_decoration.dart` (backgroundBlendMode); `painting/decoration_image.dart` (centerSlice, matchTextDirection); `painting/box_shadow.dart` (blurStyle).
- Incular: `decoration.rs:470` (background_blend_mode), `decoration.rs:411-420` (center_slice, match_text_direction), `decoration.rs:336` (BoxShadow.blur_style). Documented gap at `box_painter.rs:281-282`.
- Evidence: source. No read of these fields exists outside construction, validation and lerp; `box_painter.rs` never reads `BoxShadow.blur_style`.
- Impact: blend modes, nine-slice slices, text-direction mirroring and blur styles are silently ignored.
- Likely cause: the painter implements the common paths and stores the rest.
- Fix direction: paint the parameters, or reject them at construction (validation already exists for blend without fill).
- Confidence: high. Limitation: no scenario.

### paint-017 — Shape border family lacks several Flutter shapes

- Severity: P2
- Category: missing-api
- Flutter: `painting/circle_border.dart:37`, `oval_border.dart:29`, `beveled_rectangle_border.dart:23`, `continuous_rectangle_border.dart:38`, `linear_border.dart:33,140`.
- Incular: `painting_effects/shape_borders.rs:77,134,189,261` (Stadium, RoundedRectangle, RoundedSuperellipse, Star only); `decoration.rs:305` (BoxShape::Circle is not a ShapeBorder).
- Evidence: rg over `crates/*/src` finds none of the five names.
- Impact: these shapes cannot be used with ShapeDecoration or ClipPath::shape.
- Likely cause: ShapeBorder covers only framework-needed shapes.
- Fix direction: add ShapeBorder implementations and a BoxShape::Circle conversion.
- Confidence: high. Limitation: inventory only.

### paint-018 — Gradients have no transform, rotation or focal parameters

- Severity: P2
- Category: missing-api
- Flutter: `painting/gradient.dart:76` (GradientTransform), `:105` (GradientRotation), `:644` (RadialGradient focal and transform).
- Incular: `incular-rendering/src/gradients.rs:189,197,211` (constructors); `decoration.rs:515-518` (BoxDecoration::gradient).
- Evidence: rg for GradientTransform and GradientRotation finds nothing in `crates/*/src`.
- Impact: rotated or transformed-origin gradients cannot be expressed.
- Likely cause: gradient values hold stops and endpoints only.
- Fix direction: add a renderer-neutral gradient transform applied at paint time.
- Confidence: high. Limitation: no pixel scene; positionDependentBox remains unchecked.

### paint-019 — AssetImage ignores device-pixel-ratio and locale variants

- Severity: P2
- Category: missing-api
- Flutter: `painting/image_resolution.dart:236` (AssetImage variant resolution); `painting/image_provider.dart:1810` (ExactAssetImage).
- Incular: `incular-image/src/lib.rs:40-90` (ImageConfiguration), `:117-126` (load_with default), `:130-147` (AssetImage).
- Evidence: AssetImage implements only `load()`, which reads its path (`lib.rs:144-147`). Its `load_with` is not overridden, so the configuration is ignored.
- Impact: 2x and 3x asset variants are not selected on high-density displays; locale-specific assets are never chosen.
- Likely cause: AssetImage is a single-path provider.
- Fix direction: implement `load_with` with Flutter's variant fallback order.
- Confidence: high. Limitation: fallback order not compared.

### paint-020 — EdgeInsets, Alignment and BorderSide lerps clamp the interpolation factor

- Severity: P2
- Category: behavior
- Flutter: `painting/edge_insets.dart:698-712` (EdgeInsets.lerp uses unclamped `ui.lerpDouble`); `painting/alignment.dart:463-472`; `sky_engine/lib/ui/lerp.dart:13-24`.
- Incular: `incular-config/src/insets.rs:227-236` (EdgeInsets Lerp clamps t); `alignment.rs:184-189` (Alignment), `:229` (FractionalOffset), `:287` (AlignmentDirectional); `painting_effects/decoration.rs:102-110` (BorderSide). The tween path `incular-animation/src/value.rs:296-310` passes curve output through these Lerp implementations.
- Evidence: scenario `paint__data__edge_insets_lerp_extrapolated` at t=1.5: first divergence frame 0, value.left Flutter 26 vs Incular 20 (top, right and bottom also differ). `paint__data__alignment_lerp_extrapolated`: Flutter x=y=2 vs Incular 1. The BorderSide clamp is source-only. `paint__data__border_radius_lerp_extrapolated` matches, because Radius lerp is unclamped.
- Impact: with an overshooting curve, a LerpTween of insets or alignment stops at the endpoint instead of overshooting. The animation consequence is source-derived and not frame-verified for AnimatedPadding or AnimatedAlign.
- Likely cause: Lerp clamps t, and the tween path passes curve output through it.
- Fix direction: remove the clamp from these Lerp implementations, keeping endpoint exactness.
- Confidence: high. Limitation: overshoot animation not frame-verified. anim_curves-001 reports the same class of clamp for size and rect tweens; this covers different types, so it is not a duplicate.

### paint-021 — HsvColor lerp wraps hue 360 to 0 before interpolating

- Severity: P3
- Category: behavior
- Flutter: `painting/colors.dart:178-195` (lerpDouble on hue, then `% 360`; fromAHSV accepts hue up to 360).
- Incular: `incular-core/src/geometry.rs:254-285` (HsvColor::new normalizes hue with rem_euclid, `:303-309`); Lerp at `:719-731`.
- Evidence: scenario `paint__data__hsv_lerp_hue_wrap`, hue 300 to 360 at t=0.5: first divergence frame 0, Flutter 330, Incular 150. The saturation sample matches (0.75 on both sides).
- Impact: colors interpolated across the 360 boundary in HSV get the wrong hue.
- Likely cause: hue is normalized in the constructor before Lerp sees it.
- Fix direction: keep hue 360 in the Flutter range, or interpolate before normalization.
- Confidence: high. Limitation: only the 360 endpoint is covered.

### paint-022 — ShaderMask makes the WGPU frame fail instead of painting

- Severity: P1
- Category: missing-feature (frame-level failure; documented, not an accidental crash)
- Flutter: `widgets/basic.dart:425-467` (ShaderMask).
- Incular: `compositing.rs:75-100`; `incular-wgpu/src/diagnostics.rs:388`; `incular-wgpu/src/renderer/lowering.rs:485-495`.
- Evidence: source. Lowering returns `Err(RendererError::UnsupportedShaderMask)` for the whole frame when a PushShaderMask has a non-empty parent clip. Documented on ShaderMask (`compositing.rs:60-70`); tested by `incular-wgpu/tests/unsupported_effects.rs:21-63` and `incular-desktop/tests/unsupported_effect_execution.rs:84`.
- Reproduction: build a scene whose visible subtree contains `ShaderMask::new(callback, child)` and present it through incular-wgpu. Presentation returns `UnsupportedShaderMask` for that frame, and the child is not drawn unmasked.
- Impact: any WGPU app using ShaderMask loses the frame. This is intended fail-closed behavior, but it is a missing common feature. Whether the app's frame loop recovers was not checked.
- Likely cause: no WGPU shader-mask stage exists.
- Fix direction: implement the stage, or degrade to an unmasked child with a diagnostic; the orchestrator should choose the parity policy.
- Confidence: high. Limitation: pixels cannot be compared until the stage exists.

## Pixel scene proposals

### `physical_shape_triangle`

Checks missing `PhysicalShape` surface paint/elevation and the default clip behavior.

```dart
// Add a _TriangleClipper returning (0,0) -> (width,0) -> (width/2,height).
at(20, 20, 120, 120, PhysicalShape(
  clipper: _TriangleClipper(), elevation: 8, color: Colors.white,
  child: Align(alignment: Alignment.topLeft,
    child: SizedBox(width: 40, height: 40,
      child: const ColoredBox(color: Colors.red))),
))
```

```rust
let mut path = Path::builder();
path.move_to(Offset::ZERO)
    .line_to(Offset::new(120., 0.))
    .line_to(Offset::new(60., 120.))
    .close();
at(20., 20., 120., 120., PhysicalShape::new(
    Arc::new(path.build()), Color::WHITE,
    Align::new(Alignment::TOP_LEFT, SizedBox::new().width(40.).height(40.)
        .child(colored(Color::RED))),
).elevation(8.))
```

### `physical_model_circle_clip`

Checks circle shape with a centered red child. Flutter:

```dart
at(20, 20, 120, 120, PhysicalModel(
  shape: BoxShape.circle, color: Colors.blue,
  child: Align(alignment: Alignment.center,
    child: SizedBox(width: 60, height: 60,
      child: const ColoredBox(color: Colors.red))),
))
```

Rust (Incular has no shape setter):

```rust
at(20., 20., 120., 120., PhysicalModel::new(Color::BLUE,
    Align::new(Alignment::CENTER, SizedBox::new().width(60.).height(60.)
        .child(colored(Color::RED))),
))
```

### `clip_r_superellipse`

Checks the rounded-superellipse outline against Incular's rounded-rectangle lowering.

```dart
at(20, 40, 120, 80, ClipRSuperellipse(
  borderRadius: BorderRadius.circular(32),
  child: const SizedBox.expand(child: ColoredBox(color: Colors.white)),
))
```

```rust
at(20., 40., 120., 80., ClipRSuperellipse::new(
    32., SizedBox::new().width(120.).height(80.).child(colored(Color::WHITE)),
))
```

### `image_default_fit`

Initialize one white 40x20 `ui.Image` fixture via `PictureRecorder` before `runApp`; render it into a 100x100 destination with fit omitted.

```dart
at(30, 30, 100, 100, RawImage(image: fitFixture, width: 100, height: 100))
```

```rust
at(30., 30., 100., 100., Image::new(ImageHandle::from_rgba8(
    40, 20, vec![255; 40 * 20 * 4],
).unwrap()).width(100.).height(100.))
```

### `decoration_image_default_fit`

Use the same image fixture encoded to PNG before `runApp` and a 100x100 `BoxDecoration` image with no `fit` argument.

```dart
at(30, 30, 100, 100, Container(decoration: BoxDecoration(
  image: DecorationImage(image: MemoryImage(fitFixturePng)),
)))
```

```rust
at(30., 30., 100., 100., DecoratedBox::new(SizedBox::new().width(100.).height(100.))
    .decoration(BoxDecoration::new().image(DecorationImage::new(fit_fixture.clone()))))
```

### `color_filter_matrix_bias`

Apply the documented Flutter invert matrix to an opaque white square on black. The Flutter output is black; with the same literal, Incular's normalized matrix path clamps to white.

```dart
const invert = <double>[
  -1, 0, 0, 0, 255,  0, -1, 0, 0, 255,
   0, 0, -1, 0, 255,  0,  0, 0, 1,   0,
];
ColorFiltered(colorFilter: const ui.ColorFilter.matrix(invert),
  child: const SizedBox(width: 80, height: 80,
    child: ColoredBox(color: Colors.white)))
```

```rust
ColorFiltered::new(ColorFilter::matrix([
    -1., 0., 0., 0., 255., 0., -1., 0., 0., 255.,
    0., 0., -1., 0., 255., 0., 0., 0., 1., 0.,
]), SizedBox::new().width(80.).height(80.).child(colored(Color::WHITE)))
```


## Harness requests

Not run here (GPU contention); the orchestrator runs pixel batches. All pixel scenes are at HEAD unless noted.

1. Rerun at HEAD with the existing definitions in `harness/flutter/lib/main.dart` and `harness/incular/tests/pixels.rs`, at 1, 1.25, 1.5 and 2x: `physical_shape_triangle`, `clip_r_superellipse`, `image_default_fit`, `decoration_image_default_fit`, `color_filter_matrix_bias`. This confirms or revises paint-004, 006, 002, 003 and 015 after the WGPU changes.
2. New scene `positioned_gradient` (positionDependentBox). Flutter: `Align(alignment: Alignment.center, child: Container(width: 100, height: 100, decoration: BoxDecoration(gradient: LinearGradient(colors: [Color(0xFFFF0000), Color(0xFF0000FF)], begin: Alignment.topLeft, end: Alignment.bottomRight))))` in an 800x600 surface. Incular: `Center::new(DecoratedBox::new(SizedBox::new().width(100.).height(100.)).decoration(BoxDecoration::new().gradient(LinearGradient::new(...same colors and endpoints...))))`. Compare the 1x output. Uses the existing pixel runner.
3. New scene `decoration_center_slice` (paint-016). Flutter: 40x20 fixture `DecorationImage(image: fixture, centerSlice: Rect.fromLTRB(10, 5, 30, 15))` in a 100x60 `Container`. Incular: `DecoratedBox` with `DecorationImage::new(fixture)` plus `center_slice = Some(Rect::new(10,5,20,10))` (the Rust rect is origin+size). Expected: Flutter nine-slices; Incular paints without slicing. This is a non-equivalent diagnostic: label it as such.

Frame-only follow-up (no pixels): a curve-driven `AnimatedPadding` twin with `Curves.elasticOut` against Incular's `Curve::ElasticOut` probing the padding value per frame, to check paint-020's animation consequence.

## Open questions

- paint-022: is the fail-closed ShaderMask frame error the intended policy, or should the WGPU stage be implemented? This decides whether the P1 stays.
- The pixel magnitudes above are from a425aa2e; the HEAD rerun decides them.
- RepaintBoundary raster caching remains out of scope (performance-only).

## Validation

- `harness/run-frames.sh paint__` at HEAD (`FLUTTER` set): 9 scenarios, 6 match, 3 diverge as in the Coverage table.
- `flutter analyze test/frames/paint_test.dart`: no issues.
- `cargo check -q -p incular-parity-harness --tests`: passed.
- `coverage/paint.jsonl` (189 rows) and `findings/paint.jsonl` (22 lines) parse as JSON; no row keeps the clean-stop text.
