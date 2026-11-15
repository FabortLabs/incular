# paint-2 fixes

Group: paint-2. Records: `paint-2.jsonl`. These records supersede the earlier `paint.md` and `gestures-api.md` entries for the same ids. Not run here: `harness/run-pixels.sh`, per the brief.

Out-of-ownership edits (all small):
- `crates/incular-widgets/src/lib.rs`: re-exports `BlurStyle`, `ContinuousRectangleBorder`, `LinearBorder` and `LinearBorderEdge`.
- `crates/incular-widgets/src/painting_effects.rs`: the same re-exports, plus a crate-internal `ImagePaint`/`painted_image` re-export.
- `crates/incular-rendering/src/compositor.rs`: adds the colour-filter stage bits to the cache fingerprint. Nothing else changed.

Pixel scenes for the orchestrator (proposed):
- `decoration_center_slice`: a 30x30 bordered image as `DecorationImage(centerSlice: Rect.fromLTWH(10,10,10,10))` in a 100x60 box.
- `decoration_match_text_direction_rtl`: a top-left-aligned asymmetric image with `matchTextDirection` under RTL.
- `box_shadow_blur_styles`: four 40x40 boxes, one per blur style, with a `0x80000000` shadow of blur 8.
- `background_blend_multiply`: a red `backgroundBlendMode: multiply` box over a gradient.
- `color_filtered_gamma`: a gradient under `ColorFilter.linearToSrgbGamma()` and one under `srgbToLinearGamma()`.
- `color_filtered_mode_xor`: a `ColorFilter.mode(color, xor)` scene.
- `image_filtered_compose`: `ImageFilter.compose(outer: ColorFilter.matrix(grayscale), inner: blur 3)`.
- `continuous_rectangle_border` and `linear_border_bottom`.

## paint-016 (P2): accepted but unpainted parameters (fixed)
- Cause: the decoration painter never read these four fields.
- `background_blend_mode`: the fill is wrapped in a Blend layer, as Flutter's `_getBackgroundPaint` does. A blend mode now forces the full painter.
- `blur_style`, approach: a non-normal style composes the opaque shadow shape the way `SkBlurMask` combines masks. The combined mask then takes the colour's alpha once, through an Opacity group.
- `blur_style`, per style:
  - solid: the shape over the blur, giving `s+b-sb`;
  - outer: the shape with `DstOut`, giving `b(1-s)`;
  - inner: the shape with `DstIn`, giving `bs`.
- `center_slice` and `match_text_direction`: `paint_image` ports Flutter's `paintImage`:
  - the slice border arithmetic and `applyBoxFit`;
  - `drawImageNine`, whose corners shrink proportionally when they do not fit;
  - the repeat tiles;
  - the flip about the rect centre, with the alignment mirrored.
- Direction: comes from the ambient `Directionality`.
- Tests: four tests in `incular-widgets/tests/paint_options.rs`.
- Limitations:
  - Each non-normal shadow costs one extra group.
  - Sliced or flipped decoration images paint through a CustomPaint display list.

## paint-001 (P2): BackdropFilter general filters (blocked, API added)
- Added: a renderer-neutral `ImageFilter` (blur, matrix, dilate, erode, color, compose) in `incular-rendering` effects.rs, with `UnsupportedImageFilter` and `ImageFilterTarget`.
- Added: `BackdropFilter::image_filter(filter, child) -> Result`. It accepts one blur or one matrix. Anything else returns a typed error rather than drawing as a different filter.
- Blocker: the backdrop filter value would need to be threaded through `WidgetKind`/`RenderKind` (`tree/specs.rs`, `render_object/layers.rs`, `lowering/effects.rs`, `interaction.rs`). It would also need the `LayerKind`/`PaintCommand` fields, which are matched in about 40 places, and new WGPU `BackdropOp`s:
  - colour: a colour-matrix pass over a full-target capture;
  - morphology: a separable min/max pass.
- Test: `backdrop_filter_accepts_only_executable_filters`.

## paint-007 (P2): ImageFiltered (fixed except morphology)
- Change: `ImageFiltered::new(ImageFilter, child) -> Result` lowers the flattened stages, innermost first:
  - blur: the Blur layer;
  - colour: ColorFiltered, including the new gamma and blend stages;
  - matrix: a retained Transform about the top-left that does not transform hit tests, as Flutter's `ImageFilterLayer` does.
- Dilate and erode return `UnsupportedImageFilter { target: Subtree }`. They need a morphology layer kind and a WGPU pass.
- Test: `image_filtered_executes_composed_filters_and_refuses_morphology`.

## paint-008 (P2): ColorFilter forms (fixed)
- Change: `ColorFilter` carries a `ColorFilterStage`, which runs before the matrix. The stage is `LinearToSrgbGamma`, `SrgbToLinearGamma` or `Blend { color, mode }`.
- New constructors: `ColorFilter::linear_to_srgb_gamma()` and `srgb_to_linear_gamma()`. `ColorFilter::blend(color, mode)` is the exact matrix when one exists and the stage otherwise.
- Composition: `try_then` composes filters. `then` panics only for a staged filter that follows a non-identity matrix.
- WGPU:
  - `color_matrix_shader.wgsl` executes the stage, using the IEC 61966-2-1 transfer functions and the full `blend_premultiplied` formula set.
  - The parameters gain the stage and the colour.
  - The effect cache is keyed on the stage bits.
  - Nested filters fuse only when `try_then` allows it.
- Tests:
  - `incular-rendering/tests/color_filter_mode.rs` (three new tests);
  - the GPU test `incular-wgpu/tests/color_filter_stage.rs`, which compares the production shader with `ColorFilter::apply` and passed on the local adapter.

## paint-017 (P2): shape borders (fixed)
- `ContinuousRectangleBorder` follows Flutter `_getPath`:
  - cubics with both controls at the corner;
  - radii clamped to the shortest side;
  - Flutter's x/y radius swap kept;
  - the inner path deflated by `side.width`.
- `LinearBorder` and `LinearBorderEdge`: dimensions cover the present edges, and the edges paint per Flutter, with hairline strokes for zero widths.
- Limitation: `ShapeBorder::paint` has no direction, so LinearBorder paints left to right.
- Tests: two tests in `paint_options.rs`.

## paint-013 (P2): RawImage and Image options (fixed)
- `Image`:
  - `fit` is now optional. A null fit means scale-down, or fill when there is a slice, as in Flutter.
  - New builders `center_slice` and `match_text_direction`. Layout is kept by a hidden, maintain-size `Visibility`.
- `RawImage`: new `color_blend_mode` (through `ColorFilter::blend`), `center_slice` and `match_text_direction`.
- `isAntiAlias` not added: no image anti-alias flag exists in the paint command.
- Tests: two tests in `paint_options.rs`; `image_layout.rs` still passes (18 tests).

## paint-014 (P2): FadeInImage options (blocked)
- Error builders need load failures. The widget takes pre-decoded handles, so it waits on paint-012.

## paint-012 (P2): Image provider lifecycle (blocked, design gap recorded)
- Missing pieces:
  - an asynchronous image-stream/load-state value (pending, progress, ready, error) in `incular-image`, driven by wasm-safe runtime tasks;
  - a retained provider-keyed widget state with `gapless_playback`;
  - the `frame_builder`, `loading_builder` and `error_builder` hooks, plus `exclude_from_semantics`.
- No existing loading state exists to hook into.

## gestures-008 (P2): force press (blocked)
- Blocker: tree dispatch receives `incular_gestures::PointerEvent`, which has no pressure. Pressure exists in `InputEvent::PointerWithMetadata` but is dropped before dispatch.
- Remaining work:
  - plumb the sample through `incular-runtime/src/frame.rs`;
  - add the `on_force_press_*` callbacks and builders;
  - add a ForcePress arena member in `tree/interaction.rs`, like the scale member.

## gestures-013 (P3): DeviceGestureSettings (deferred)
- Needs a platform gesture-settings environment value and slop parameters threaded through every recognizer.

## gestures-014 (P3): DiagonalDragBehavior (fixed)
- Change: new `WeightedEvent` and `WeightedContinuous` variants. `TwoDimensionalScrollable` holds the lock state and gains `start_drag`, `drag_update`, `end_drag` and `locked_axis`. These follow Flutter `_evaluateLockedAxis` with kTouchSlop 18.
- `Weighted` is kept and documented as Incular-specific.
- Limitation: no in-tree gesture path drives this model yet.
- Tests: two new tests in `advanced_scrolling.rs`.

## Validation
- `cargo test -p incular-rendering --test color_filter_mode`: 5 passed.
- `cargo test -p incular-wgpu --test color_filter_stage`: 1 passed, on GPU.
- `cargo test -p incular-widgets`, targeted test files:
  - `paint_options`: 10 passed;
  - `image_layout`, `fade_in_image`, `box_decoration_painting`, `box_shadow_paint`, `painting_effects`, `shape_borders`, `shape_decoration`, `shader_backdrop`, `physical_models` and `clip_opacity`: all pass;
  - `advanced_scrolling` and `advanced_scrolling_retained_integration`: all pass.
- `scroll_scrollbar_2d`: 2 failures, `raw_track_click_stays_programmatic` and `retained_replacement_mid_drag_ends_old_bracket`. These are raw-scrollbar activity brackets in code this group did not touch (`raw_scrollbar.rs` is unmodified). Attributed to other in-progress work.
- `cargo clippy -D warnings`: clean for `incular-rendering` and `incular-wgpu`, and clean for `incular-widgets`, all with `--all-targets`.
- `cargo check -p incular-desktop --tests` and `cargo check -p incular-parity-harness --tests`: both pass.
