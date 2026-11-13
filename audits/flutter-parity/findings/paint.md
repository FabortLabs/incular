# paint: Painting widgets

## Summary

The geometry recorder matches opacity changes, zero-opacity layout, solid `DecoratedBox` output and intrinsic `RawImage` sizing in the four paint scenarios. Code review confirms several paint-only gaps that the frame harness cannot see: Flutter's default image fitting is scale-down while Incular scales images up, `PhysicalShape` discards its physical properties, `ClipRSuperellipse` becomes a rounded rectangle, and image filters/color filters support fewer operations. `BoxDecoration` and `ShapeDecoration` have additional missing image behavior, and `CustomPaint` lacks Flutter's painter/child layering API. The frame and existing crate tests cover useful subsets, but the proposed pixel scenes still need the orchestrator's final pixel batch.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| `Opacity` | `tree/descriptors.rs::Opacity` | scenario + reading | `paint__opacity__phase_values`, `paint__opacity__zero_keeps_layout` | alpha/geometry match; semantic flag gap is `implicit-004` |
| `ClipRect` | `layout/basic/clipping.rs::ClipRect` | reading + existing tests | `clip_opacity.rs` | default bounds/behavior represented; custom clipper gap `paint-010` |
| `ClipRRect` | `layout/basic/clipping.rs::ClipRRect` | reading + existing tests | `clip_opacity.rs` | static rounded clip represented; custom clipper gap `paint-010` |
| `ClipOval` | `layout/basic/clipping.rs::ClipOval` | reading + existing tests | `clip_opacity.rs` | static oval clip represented; custom clipper gap `paint-010` |
| `ClipPath` | `layout/basic/clipping.rs::ClipPath` | reading + existing tests | `clip_opacity.rs` | static path clip represented; callback/default-rect gap `paint-010` |
| `ClipRSuperellipse` | `painting_effects/layers.rs::ClipRSuperellipse` | reading | pixel proposal `clip_r_superellipse` | divergent: `paint-006` |
| `DecoratedBox` | `tree/descriptors.rs::DecoratedBox` | scenario + reading + existing tests | `paint__decorated_box__solid_background` | solid background matches; paint-only decorations need pixels |
| `BoxDecoration` | `painting_effects/decoration.rs::BoxDecoration` | reading + existing tests | `box_decoration_painting.rs`; pixel proposals | partial: see image-fit `paint-003` and findings table |
| `DecorationImage` | `painting_effects/decoration.rs::DecorationImage` | reading + existing tests | `image_layout.rs`; pixel proposal `decoration_image_default_fit` | divergent default: `paint-003` |
| `ShapeDecoration` | `painting_effects/shape_decoration.rs::ShapeDecoration` | reading + existing tests | `shape_decoration.rs` | partial: missing image field `paint-011` |
| `PhysicalModel` | `painting_effects/layers.rs::PhysicalModel` | reading + existing tests | `painting_effects.rs`; pixel proposal `physical_model_circle_clip` | divergent: `paint-005` |
| `PhysicalShape` | `painting_effects/layers.rs::PhysicalShape` | reading + existing tests | `painting_effects.rs`; pixel proposal `physical_shape_triangle` | divergent: `paint-004` |
| `ColorFiltered` | `tree/descriptors.rs::ColorFiltered` | reading | none | divergent: matrix translation units `paint-015`; missing filter kinds `paint-008` |
| `ShaderMask` | `compositing.rs::ShaderMask` | reading | none | retained API exists; pixels cannot be compared because Incular's WGPU backend rejects the layer |
| `BackdropFilter` | `compositing.rs::BackdropFilter` | reading | existing pixel `backdrop_rrect` | partial: general filter types missing, `paint-001`; supported blur still needs final pixel batch |
| `BackdropGroup` | `compositing.rs::BackdropGroup` | reading + existing tests | `backdrop_group.rs` | group key sharing matches tests |
| `ImageFiltered` | `painting_effects/images.rs::ImageFiltered` | reading + existing pixel scene | `blur` | partial: only blur is represented, `paint-007` |
| `CustomPaint` / `CustomPainter` | `layout/basic/painting.rs::CustomPaint`; `painting_effects/custom_paint.rs::CustomPainter` | reading + existing tests | existing `path` pixel scene; `tree_layout_painting.rs` | partial: static display-list painting works; missing painter/child layers, `paint-009` |
| `RepaintBoundary` | `layout/basic/painting.rs::RepaintBoundary` | reading | none | wrapper is represented; cache/performance behavior is out of scope |
| `Image` | `tree/descriptors.rs::Image` | scenario + reading + existing tests | `paint__raw_image__intrinsic_scale`, `image_layout.rs`; pixel proposal `image_default_fit` | partial: intrinsic geometry matches; default fit and stream options diverge (`paint-002`, `paint-012`) |
| `RawImage` | `painting_effects/images.rs::RawImage` | scenario + reading + existing tests | `paint__raw_image__intrinsic_scale`, `image_layout.rs`; pixel proposal `image_default_fit` | partial: intrinsic geometry matches; default fit/options differ (`paint-002`, `paint-013`) |
| `ImageIcon` | `painting_effects/images.rs::ImageIcon` | reading | `image_layout.rs` | size/tint lowering present; semantic label and ambient icon-theme behavior not measured here |
| `FadeInImage` | `fade_in_image.rs::FadeInImage` | reading + existing tests | `fade_in_image.rs` | core placeholder/target fade is tested; several paint/loading options absent (`paint-014`) |

Scenario verification: 4 paint scenarios run; all 4 matched every recorded field. Existing crate tests read: `clip_opacity.rs`, `painting_effects.rs`, `box_decoration_painting.rs`, `image_layout.rs`, `shape_decoration.rs`, `backdrop_group.rs`, `fade_in_image.rs`, and `tree_layout_painting.rs`.

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
- Evidence: code reading; `paint__raw_image__intrinsic_scale` matches geometry, but the frame recorder cannot see the fitted paint rect. Pixel scene `image_default_fit` supplies a 40×20 image to a 100×100 destination; at 1x, 16.41% of pixels differ by more than 16 channel levels (maximum 255). See `../pixel-run.md`.
- Impact: a smaller source remains at native size in Flutter but is enlarged to fill the destination in Incular.
- Likely cause: Incular stores `Contain` as the default where Flutter defers to `paintImage`'s scale-down default.
- Fix direction: make the default represent Flutter's nullable fit or use scale-down when fit is omitted.
- Confidence: high

### paint-003 — DecorationImage defaults to cover instead of scale-down

- Severity: P1
- Category: default-value
- Flutter: `painting/decoration_image.dart:48-84` leaves `fit` null; its paint path resolves that to `BoxFit.scaleDown` at `:561` when `centerSlice` is null.
- Incular: `crates/incular-widgets/src/painting_effects/decoration.rs:411-443` defaults `DecorationImage.fit` to `ImageFit::Cover`.
- Evidence: code reading; pixel scene `decoration_image_default_fit` uses a 40×20 source in a 100×100 decoration box. At every tested scale 35.94% of pixels differ by more than 16 channel levels (maximum 255); see `../pixel-run.md`.
- Impact: default decoration images can be enlarged and cropped in Incular where Flutter keeps them at or below intrinsic size.
- Likely cause: Incular uses an explicit cover fit instead of preserving Flutter's omitted-fit behavior.
- Fix direction: align the default to scale-down while retaining explicit `Cover` for callers who request it.
- Confidence: high

### paint-004 — PhysicalShape drops fill, elevation, shadow and clip behavior

- Severity: P2
- Category: paint
- Flutter: `widgets/basic.dart:1471-1529` passes `clipper`, `clipBehavior`, `elevation`, `color` and `shadowColor` to `RenderPhysicalShape`; `clipBehavior` defaults to `Clip.none`.
- Incular: `crates/incular-widgets/src/painting_effects/layers.rs:151-194` stores color/elevation/shadow color but lowers only to `ClipPath::new(value.clipper, value.child)`. That path clip defaults to anti-aliasing.
- Evidence: code reading; the existing `painting_effects.rs` test checks only that the widget lowers to `ClipPath`. Pixel scene `physical_shape_triangle`, independently compared at revision `a425aa2e`, has 30.08% of pixels differing by more than 16 channel levels at 1x; see `../pixel-run.md`.
- Impact: a `PhysicalShape` does not paint its configured surface or elevation shadow, and Incular clips by default when Flutter's default does not.
- Likely cause: the lowering discards every physical property except the static path and child.
- Fix direction: retain a physical-shape render representation carrying the clip mode, fill and elevation/shadow properties.
- Confidence: high

### paint-005 — PhysicalModel ignores its shape and clipBehavior

- Severity: P2
- Category: paint
- Flutter: `widgets/basic.dart:1371-1437` supports `BoxShape.rectangle` or `BoxShape.circle` and clips the child when `clipBehavior` requests it.
- Incular: `crates/incular-widgets/src/painting_effects/layers.rs:75-147` stores no shape field, and `From<PhysicalModel>` converts to a rounded `DecoratedBox`/`DropShadow` without using `clip_behavior`.
- Evidence: code reading; `painting_effects.rs` checks construction equality only. Pixel scene `physical_model_circle` is an intentionally non-equivalent missing-API diagnostic: Flutter requests a circle and Incular uses its rectangle default because it has no shape option. At 1x, 12.66% of pixels differ by more than 16 channel levels (maximum 243); see `../pixel-run.md`. The separate clipping claim remains source-only.
- Impact: callers cannot request a circular physical model, and a child that should be clipped can paint beyond the model boundary.
- Likely cause: physical-model conversion substitutes decoration and shadow widgets without preserving shape or clipping.
- Fix direction: preserve the model shape and clip behavior through paint lowering.
- Confidence: high

### paint-006 — ClipRSuperellipse is rendered as a rounded rectangle

- Severity: P2
- Category: paint
- Flutter: `widgets/basic.dart:1123-1188` creates `RenderClipRSuperellipse` from `BorderRadiusGeometry`.
- Incular: `crates/incular-widgets/src/painting_effects/layers.rs:36-68` converts it to `ClipRRect` with uniform corner radii.
- Evidence: code reading; pixel scene `clip_r_superellipse` isolates the corners and at 1x has 0.61% of pixels differing by more than 16 channel levels, maximum difference 122; see `../pixel-run.md`.
- Impact: corners follow circular rounded-rectangle arcs instead of Flutter's squircle profile, visibly changing the clipped edge at larger radii.
- Likely cause: `ClipRSuperellipse` is lowered to the RRect clip path instead of the existing superellipse path primitive.
- Fix direction: preserve the rounded-superellipse clip geometry through render lowering.
- Confidence: high

### paint-007 — ImageFiltered supports blur only

- Severity: P2
- Category: missing-api
- Flutter: `widgets/image_filter.dart:39-51` accepts any `ui.ImageFilter` and exposes `enabled`.
- Incular: `crates/incular-widgets/src/painting_effects/images.rs:196-229` stores only a sigma (or `BlurController`) and lowers to `Blur`.
- Evidence: code reading; the existing `blur` pixel scene checks the supported case, but no matching scene can express dilation, erosion, or composed filters.
- Impact: callers cannot apply non-blur filters to a child or disable an existing filter while retaining its configuration.
- Likely cause: the widget maps `ImageFiltered` directly to the blur layer type.
- Fix direction: add renderer-neutral image-filter variants and the enabled state to the widget representation.
- Confidence: high

### paint-008 — ColorFiltered accepts matrix filters only

- Severity: P2
- Category: missing-api
- Flutter: `widgets/color_filter.dart:36-52` accepts `ui.ColorFilter`; `bin/cache/pkg/sky_engine/lib/ui/painting.dart:3998-4087` defines mode, matrix, and both gamma filters.
- Incular: `crates/incular-rendering/src/effects.rs:43-58` represents a 4x5 matrix, which `crates/incular-widgets/src/tree/descriptors.rs:1720-1750` uses as the complete `ColorFiltered` filter type.
- Evidence: code reading.
- Impact: callers cannot request Flutter's blend-mode or gamma-transfer `ColorFiltered` effects.
- Likely cause: the retained color-filter layer has only one matrix representation.
- Fix direction: extend the renderer-neutral color-filter enum and keep matrix behavior as one variant.
- Confidence: high

### paint-009 — CustomPaint cannot layer foregroundPainter and child

- Severity: P2
- Category: missing-api
- Flutter: `widgets/basic.dart:831-884` accepts `painter`, `foregroundPainter`, and `child`; it paints the foreground after the child.
- Incular: `crates/incular-widgets/src/layout/basic/painting.rs:74-95` stores only a fixed size and one `DisplayList`; `from_painter` evaluates `paint` immediately. Its `CustomPainter::should_repaint` method has no call site in the widget implementation.
- Evidence: code reading; the existing `path` pixel scene exercises only static custom drawing.
- Impact: apps cannot express Flutter's custom-paint-before-child and custom-paint-after-child ordering in one `CustomPaint`, or the `shouldRepaint` delegate lifecycle.
- Likely cause: Incular represents custom paint as a single static display list instead of a painter delegate with child slots.
- Fix direction: retain painter delegates and foreground/background ordering with child updates.
- Confidence: high

### paint-010 — Clip widgets do not accept CustomClipper delegates

- Severity: P2
- Category: missing-api
- Flutter: `widgets/basic.dart:950-1353` exposes nullable `CustomClipper` callbacks on `ClipRect`, `ClipRRect`, `ClipOval`, and `ClipPath`; the delegate derives geometry from the laid-out size and controls reclipping through `shouldReclip`.
- Incular: `crates/incular-widgets/src/layout/basic/clipping.rs:18-147` accepts static behavior/radii/path values only; `ClipPath` requires a `Path` and there is no clipper callback parameter.
- Evidence: code reading; `clip_opacity.rs` verifies static clip paths only.
- Impact: callers cannot provide a size-dependent clip delegate with Flutter's reclip lifecycle through these widgets.
- Likely cause: clip geometry is stored in the descriptor rather than resolved by a delegate at layout/paint time.
- Fix direction: add a retained clipper abstraction that receives current size and decides whether changed delegate state needs a new clip.
- Confidence: high

### paint-011 — ShapeDecoration cannot paint a DecorationImage

- Severity: P2
- Category: missing-api
- Flutter: `painting/shape_decoration.dart:65-118` accepts an `image` and paints it inside the shape outline over its color or gradient.
- Incular: `crates/incular-widgets/src/painting_effects/shape_decoration.rs:17-52` stores color, gradient, shadows, and shape but no image field; `paint` fills only color/gradient and strokes the border.
- Evidence: code reading; existing `shape_decoration.rs` tests cover paths and borders but not images.
- Impact: an app cannot decorate an arbitrary `ShapeBorder` with an image clipped to that shape.
- Likely cause: the shape decoration painter has no image configuration or image paint stage.
- Fix direction: add a decoration-image field and paint it clipped to the shape's outer path.
- Confidence: high

### paint-012 — Image lacks Flutter's image-stream lifecycle options

- Severity: P2
- Category: missing-api
- Flutter: `widgets/image.dart:343-388` accepts `ImageProvider`, `frameBuilder`, `loadingBuilder`, `errorBuilder`, and `gaplessPlayback`.
- Incular: `crates/incular-widgets/src/tree/descriptors.rs:34-112` takes an already-decoded `ImageHandle` with sizing/fit/repeat/alignment/sampling only.
- Evidence: code reading; `image_layout.rs` tests the decoded-handle layout and fit behavior, not provider updates or loading/error states.
- Impact: callers must implement loading/error and gapless image replacement outside Incular's `Image` widget, and cannot receive Flutter's frame-builder lifecycle.
- Likely cause: the widget consumes a final image handle instead of an image stream/provider state.
- Fix direction: add a provider/stream-backed image state layer with builder and gapless-replacement options.
- Confidence: high

### paint-013 — RawImage exposes fewer paint options than Flutter

- Severity: P2
- Category: missing-api
- Flutter: `widgets/basic.dart:6828-6993` exposes alignment, center slice, match-text-direction, invert colors, filter quality, anti-aliasing, animated opacity, and color blend mode.
- Incular: `crates/incular-widgets/src/painting_effects/images.rs:12-99` exposes dimensions, scale, tint color, fit, and repeat; lowering at `:101-128` does not forward alignment or sampling and implements color as a constant tint.
- Evidence: code reading; `paint__raw_image__intrinsic_scale` matches the intrinsic geometry only.
- Impact: raw-image callers cannot configure Flutter's crop/alignment, nine-slice, mirroring, invert, quality, opacity, or arbitrary tint blend behavior.
- Likely cause: `RawImage` is a smaller convenience wrapper around Incular's basic `Image` descriptor.
- Fix direction: carry the missing RawImage fields through to the image paint command and color-filter layer.
- Confidence: high

### paint-014 — FadeInImage omits several paint and load-error options

- Severity: P2
- Category: missing-api
- Flutter: `widgets/fade_in_image.dart:61-100` and its fields include image/placeholder error builders, separate image colors and blend modes, filter qualities, repeat, and match-text-direction options.
- Incular: `crates/incular-widgets/src/fade_in_image.rs:22-130` supports predecoded handles, fade timing/curves, size, fit, alignment, and semantics only.
- Evidence: code reading; `fade_in_image.rs` tests verify synchronous display and placeholder/target opacity timing.
- Impact: callers cannot reproduce Flutter's error callbacks, separate placeholder/image filters, or repeated/mirrored image paint options.
- Likely cause: the widget owns only its fade state and relies on the limited `Image` descriptor for rendering.
- Fix direction: expose and forward the missing image paint and provider-error options while retaining the tested fade timeline.
- Confidence: high

### paint-015 — ColorFilter matrix translations use different units

- Severity: P1
- Category: paint
- Flutter: `bin/cache/pkg/sky_engine/lib/ui/painting.dart:4029-4071` specifies a 4x5 matrix whose translation column is in the 0..255 channel range.
- Incular: `crates/incular-rendering/src/effects.rs:43-58` applies its 4x5 matrix to normalized 0..1 channels and clamps outputs to that range.
- Evidence: code reading; pixel scene `color_filter_matrix_bias` applies Flutter's documented invert matrix to an opaque white child. At every tested scale, 25% of pixels differ by more than 16 channel levels, maximum 255; see `../pixel-run.md`.
- Impact: Flutter's standard invert matrix produces black from white, while the same matrix literal in Incular clamps the `255 - 1` result to white.
- Likely cause: the public matrix layout matches but the channel and bias normalization contracts differ.
- Fix direction: convert Flutter-style matrix bias values to Incular's normalized representation at the widget boundary, or expose and document a parity-compatible matrix constructor.
- Confidence: high

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

None. The frame recorder was sufficient for the tested geometry, opacity, and solid-fill cases. Clipping and non-solid paint effects need the pixel batch.

## Open questions

- The pixel scenes above have not yet been added to the shared pixel harness or run; the orchestrator owns that batch.
- `ShaderMask` retained data exists, but Incular's WGPU renderer returns `UnsupportedShaderMask`; WGPU is excluded from this audit, so shader-mask pixels are not comparable here.
- `RepaintBoundary` raster-cache behavior is performance-only and was not compared.
