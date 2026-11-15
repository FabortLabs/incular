# paint fixes

Group: paint. Records: `paint.jsonl` in this folder. Owned files: `crates/incular-widgets/src/painting_effects/**`, `compositing.rs`, `fade_in_image.rs`, `crates/incular-rendering/src/effects.rs`, `crates/incular-wgpu/src/**`, `crates/incular-image/src/lib.rs`, and their tests. Shared-file edits outside ownership: `incular-material/src/inputs/text_selection_toolbar.rs` and `incular-widgets/src/magnifier.rs` (add `image: None` to ShapeDecoration literals only), and `incular-widgets/src/lib.rs` and `painting_effects.rs` (re-exports of the new shape borders). Not run here: `harness/run-pixels.sh`, per the brief.

Pixel scenes for the orchestrator to rerun: `image_default_fit`, `decoration_image_default_fit`, `color_filter_matrix_bias` (twin must switch to `ColorFilter::flutter_matrix`), `physical_shape_triangle`, `physical_model_circle` (twin must add `.shape(BoxShape::Circle)`), `clip_r_superellipse`.

## paint-022 (P1): ShaderMask fails the frame
- Outcome: fixed as a fallback. Not a masked render.
- Cause: no WGPU shader-mask stage; lowering returned `UnsupportedShaderMask` for the whole frame.
- Change: the child paints unmasked, and `unmasked_shader_mask_stages` counts each bypass. `unsupported_effect` no longer reports masks.
- Tests: `shader_mask_paints_unmasked_without_a_frame_error` (incular-wgpu `unsupported_effects.rs`).
- Limitations: the mask is not applied. Policy decision for the orchestrator. The live desktop test `unsupported_effect_execution.rs` still expects the failure and will fail when run with live GPU tests.
- WGPU pass (wgpu-effects): outcome now **fixed** with a real masked render; the unmasked fallback and `unmasked_shader_mask_stages` are gone (`shader_mask_passes` counts mask fills). Lowering follows Flutter's `ShaderMaskLayer`: the child renders into the layer's retained group target, the mask rect (`mask_size` at `mask_transform`) filled with the brush renders into the layer's effect texture, and a `DrawBatch::Blend` reading that texture (`BlendSource::Effect`) blends it over the child with the mask blend mode, scissored to the mask rect. The group then composites with srcOver. Fixed-function pipelines serve srcIn, dstIn, srcATop and the other Porter-Duff modes; modulate and the artistic modes use the destination-read ping-pong path. `blend_shader.wgsl` gained the missing modulate branch (it previously fell through to the artistic formula, which also affected `Blend` groups using modulate). Every brush shader executes, so `UnsupportedShaderMask` stays unproduced. Tests: `shader_mask_blend.rs` (GPU: production blend shader against `blend_premultiplied` for modulate, srcIn, dstIn, srcATop), `unsupported_effects.rs::shader_masks_execute_for_every_brush_shader`, and the live `incular-desktop` test `unsupported_effect_execution.rs`, whose mask phase now presents and checks a left-bright, right-dark fade (run with `INCULAR_DESKTOP_LIVE_TESTS=1`: passed). Limitations: the blend scissor is the mask rect's device bbox, so rotated or skewed mask transforms blend the bbox corners; `mask_transform` is a resolve-time snapshot, so a mask moved by a scroll or transform layer without a repaint stays behind (suggested compositor follow-up: use the live flatten world transform). Proposed pixel scene `shader_mask_linear_fade`: a 120x80 white box under `ShaderMask` with a left-to-right white-to-transparent `LinearGradient` over the callback bounds (default modulate), plus a second row with `BlendMode::SrcIn` and a red-to-blue gradient over a black-and-white checker child.


## paint-002 (P1): Image default fit
- Outcome: fixed. Confirmed against `decoration_image.dart:561` (null fit to scaleDown).
- Change: `Image::new`, `RawImage::new` and the RawImage builder default to `ImageFit::ScaleDown`. FadeInImage inherits it.
- Tests: `image_layout.rs::omitted_fit_scales_small_sources_down_and_centers_them`.
- Pixel: `image_default_fit` to rerun.

## paint-003 (P1): DecorationImage default fit
- Outcome: fixed.
- Change: `fit` is `Option<ImageFit>`, resolved by `resolved_fit()` to scaleDown without a center slice and to fill with one.
- Tests: `painting_effects.rs::painting_data_builders_match_existing_defaults_and_conversions`.
- API: field type changed; builder call unchanged.
- Pixel: `decoration_image_default_fit` to rerun.

## paint-015 (P1): ColorFilter bias units
- Outcome: fixed. Confirmed against `painting.dart` (translation column 0..255).
- Change: `ColorFilter::flutter_matrix` divides indices 4, 9, 14, 19 by 255. `ColorFilter::matrix` keeps its normalized meaning.
- Tests: `rendering.rs::flutter_matrix_translations_are_unnormalized_0_to_255`.
- Pixel: `color_filter_matrix_bias`; the twin needs `flutter_matrix`.

## paint-004 (P2): PhysicalShape
- Outcome: fixed. The elevation shadow is an approximation shared with PhysicalModel.
- Change: a Stack of the path fill (with DropShadow when elevated) and the child. The child is clipped only when `clip_behavior` is set (default none).
- Tests: `physical_models.rs` (three PhysicalShape tests); `painting_effects.rs` lowering type.
- Pixel: `physical_shape_triangle` to rerun; a shadow residual is expected.

## paint-005 (P2): PhysicalModel shape and clip
- Outcome: fixed.
- Change: `shape(BoxShape)`; circle surface; the child is clipped with ClipOval or ClipRRect when `clip_behavior` is set.
- Tests: `physical_models.rs` (two PhysicalModel tests).
- Pixel: `physical_model_circle` twin to add `.shape(BoxShape::Circle)`. The animated wrapper's circle is implicit-005.

## paint-006 (P2): ClipRSuperellipse
- Outcome: fixed.
- Change: a LayoutBuilder builds `Path::rounded_superellipse` for the constraint size and clips with ClipPath (the same sizing rule as `ClipPath::shape`).
- Limitation: exact under tight constraints only.
- Pixel: `clip_r_superellipse` to rerun.

## paint-011 (P2): ShapeDecoration image
- Outcome: fixed.
- Change: `image` field and builder. The image is clipped to the outer path between the fill and the border.
- Tests: `shape_decoration.rs::shape_decoration_paints_its_image_between_fill_and_border`.

## paint-016 (P2): accepted but unpainted parameters
- Outcome: blocked. Each parameter needs a renderer stage: background blend, nine-slice, ambient text direction, and blur style variants.
- Change: documentation only. Construction-time rejection was not added, because `is_valid()` gates painting.

## paint-017 (P2): shape border family
- Outcome: deferred. CircleBorder, OvalBorder and BeveledRectangleBorder are fixed. ContinuousRectangleBorder and LinearBorder remain.
- Tests: `shape_borders.rs` (two new tests).

## paint-018 (P2): gradient transform
- Outcome: blocked. The Brush value has no transform, and the WGPU gradient path samples by `GradientId`.
- WGPU pass (wgpu-effects): outcome **fixed**. Focal radial gradients were already supported end to end (`RadialGradient::focal`/`focal_radius`, WGPU two-point conical kind). `Brush::rotated(radians, bounds)` adds Flutter's `GradientRotation`: endpoints, centres and the focal point rotate about the bounds centre and a sweep start angle advances, which is exact for every brush, so the renderer needs no change. Tests: `incular-rendering/tests/gradient_rotation.rs` (4). Limitation: general `GradientTransform` matrices (scale, skew) are still not representable.


## paint-019 (P2): AssetImage variants
- Outcome: fixed for density variants. The locale part is invalid: Flutter's `image_resolution.dart` does not select by locale either (TODO).
- Change: `AssetImage::variant_path(dpr)` and `load_with` use the `_findBestVariant` rules.
- Tests: `incular-image/tests/asset_variants.rs` (five tests, including Flutter's documented examples).

## paint-013, paint-014 (P2): RawImage and FadeInImage options
- Outcome: deferred, partial fixes.
- RawImage: alignment, filter quality, opacity and invert colors added. Nine-slice, match text direction and color blend mode remain.
- FadeInImage: repeat and both filter qualities added. Error builders wait on paint-012.
- Tests: `image_layout.rs` (two RawImage tests).

## paint-007 (P2): ImageFiltered
- Outcome: deferred. The `enabled` flag is added and tested; non-blur filters remain (paint-001 blocker).
- WGPU pass (wgpu-effects): not reached. Matrix, compose, dilate and erode filters over a subtree still need the paint-001 image-filter value and a WGPU stage over a layer source.


## paint-001, paint-008 (P2): BackdropFilter and ColorFiltered filter forms
- Outcome: deferred. Renderer-neutral image-filter and gamma stages are needed. Not attempted.
- WGPU pass (wgpu-effects): paint-008 partially addressed. `ColorFilter::mode(color, mode)` returns an exact matrix for src, srcIn, dstIn, srcOut, dstOut, srcATop and modulate, executed by the existing color-matrix stage, and `None` for modes no 4x5 straight-alpha matrix can express (srcOver, dstOver, dstATop, xor, plus, artistic). Tests: `incular-rendering/tests/color_filter_mode.rs` (2). Gamma filters and the remaining modes need a new shader stage; still deferred. paint-001 not reached.


## paint-012 (P2): Image provider lifecycle
- Outcome: blocked. Needs a provider-backed state machine with frame and error builders; `ImageProvider` exists but no widget consumes it. The API shape is a decision for the orchestrator.

## Validation
- `cargo test -p incular-widgets` focused targets: `image_layout`, `painting_effects`, `box_decoration_painting`, `fade_in_image`, `clip_opacity`, `shader_backdrop`, `shape_borders`, `shape_decoration`, `box_shadow_paint`, `physical_models`. All passing.
- `cargo test -p incular-image --test asset_variants`: 5 passed.
- `cargo test -p incular-rendering --test rendering`: 70 passed.
- `cargo test -p incular-wgpu --test unsupported_effects --test effect_sampling`: all passing.
- `cargo check -p incular-material -p incular -p incular-desktop -p incular-wgpu --lib`, and `-p incular-wgpu -p incular-parity-harness --tests`: passing.
- Clippy for incular-image, incular-rendering and incular-wgpu (all targets): no warnings in owned code. incular-widgets clippy was blocked by another worker's in-progress edit to incular-text.
