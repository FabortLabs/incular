# blockers-render-input fixes

Group: blockers-render-input. Records: `blockers-render-input.jsonl`. These supersede the earlier `paint-2` (paint-001, gestures-008), `animated-widgets-2` (transitions-003) and `gestures-api` (gestures-008) entries. The group is added to `$passOrder` in `inventory_tool/consolidate-fixes.ps1` after `scroll-3`. `harness/run-pixels.sh` was not run, per the brief.

## paint-001 (P2): BackdropFilter general ImageFilter (fixed)
- Cause: the backdrop stage carried only a blur and a matrix end to end. WGPU had only blur and matrix backdrop ops.
- Change:
  - `incular-rendering`:
    - `image_filter: Option<ImageFilter>` on `LayerKind::BackdropFilter` and `PaintCommand::PushBackdropFilter`;
    - `update_backdrop_image_filter`;
    - `backdrop_stage_is_active` and `backdrop_image_filter_executable`;
    - cache-key hashing.
  - `incular-wgpu`:
    - `BackdropOp::Chain` runs blur, matrix and colour stages over a full-target capture. It reuses the blur encoder, the affine resample and the colour-matrix pass.
    - `unsupported_effect` reports dilate and erode.
  - `incular-widgets`:
    - the field threads through `WidgetKind`/`RenderKind`, lowering, layers and the compositor-only update;
    - `BackdropFilter::image_filter` accepts colour and composed filters and refuses only morphology.
- Tests:
  - `backdrop_plan.rs::general_backdrop_filters_flatten_and_refuse_morphology`;
  - `unsupported_effects.rs::composed_backdrop_filters_execute_and_morphology_reports_unsupported`;
  - `paint_options.rs::backdrop_filter_accepts_only_executable_filters` (rewritten).
- Limitations:
  - there is no backdrop morphology pass;
  - chains run over the whole target;
  - a singular matrix in a chain drops the stage;
  - not checked end to end on GPU.
- Proposed pixel scene `backdrop_filter_compose`: a checker under `BackdropFilter(ImageFilter.compose(outer: ColorFilter.matrix(grayscale), inner: blur 4))`.

## transitions-003 (P2): filterQuality on transforms (fixed)
- Cause: the transform layer had no sampling. Transformed children were never rasterized and resampled, as Flutter's `ImageFilterLayer(ImageFilter.matrix(..., filterQuality))` does.
- Change:
  - `LayerKind::Transform.sampling` and `update_transform_sampling`. Flatten emits the new `PaintCommand::PushMatrixFilter`, with the child untransformed.
  - WGPU `renderer/matrix_filter.rs`:
    - renders the child into the retained source;
    - makes one affine resample into the effect cache, bilinear (shader mode 2) or nearest (new mode 4);
    - composites the result as `Filtered`.
  - `filter_quality` on the `Transform`, `Scale` and `Rotation` kinds.
  - Builders:
    - `Transform`;
    - `MatrixTransition`, applied only while `is_animating`, as in Flutter;
    - `ScaleTransition` and `RotationTransition`, applied only while the controller is active.
- Tests:
  - `backdrop_plan.rs::a_sampled_transform_flattens_to_a_matrix_filter_over_the_untransformed_child`;
  - `effect_sampling.rs::affine_resampling_mode_4_samples_the_nearest_texel` (GPU);
  - three tests in `explicit_transitions.rs`.
- Harness: `transitions__` 9/9 match. The recorder does not record sampling.
- Limitations:
  - Medium and High resample bilinearly;
  - alignment pivots stay outside the filter;
  - an `ImageFiltered` matrix still lowers to a vector transform. Flutter defaults that filter to low quality.
- Proposed pixel scene `transform_filter_quality_none`: a 4x4 checker scaled 8x with `filterQuality: FilterQuality.none`, beside one with `low`.

## gestures-008 (P2): force press (fixed)
- Cause: the runtime dropped the pointer sample before gesture dispatch. `GestureDetector` built no force-press recognizer.
- Change:
  - Core, platform and desktop already normalize pressure (winit `Touch.force`). They were not changed.
  - `frame.rs` passes the sample to the new `dispatch_pointer_gesture_with_hit_sample`.
  - A `ForcePress` arena member (`ArenaGestureRecognizer<ForcePressGestureRecognizer>`) joins primary presses that report pressure. It shares the arena with taps, drags and scale, and takes sole-press acceptance.
  - `GestureCallbacks` and `GestureDetector` gain `on_force_press_start`, `peak`, `update` and `end`.
  - `force_press.rs` replays the starting event's peak and update after a deferred acceptance. `onStart` therefore comes first, as in Flutter's closed arena.
- Tests: `incular-widgets/tests/force_press.rs` (4 tests) and `flutter_recognizers.rs::force_press_accepted_later_still_starts_before_updating`.
- Harness: `gestures__` 11/11 match.
- Limitations:
  - `pressure_min` and `pressure_max` are 0 and 1 by the normalization contract. They are not separate fields.
  - Normalized touch pressure takes part where Flutter would reject `pressureMax <= 1`.

## Out-of-ownership and shared edits
- `crates/incular-runtime/src/frame.rs`: one call now uses the sample-carrying dispatch.
- `PushMatrixFilter` added to exhaustive matches in `crates/incular-runtime/tests/runtime.rs` and `crates/incular-widgets/tests/common/mod.rs`.
- `crates/incular-wgpu/tests/unsupported_effects.rs`: `image_filter: None` added to a literal.
- `crates/incular-widgets/src/lib.rs`: re-exports `ForcePressDetails`.
- Shared tree files, changed only in the relevant arms:
  - `tree.rs`, `tree/interaction.rs`, `tree/specs.rs`;
  - `tree/widget/constructors.rs` and `tree/widget/structure.rs`;
  - `render_object/layers.rs`, `custom_layout.rs`, `painting_effects/images.rs`.

## Validation
- `incular-rendering`: `backdrop_plan` 9 passed, `rendering` 70, `color_filter_mode` 5.
- `incular-wgpu`:
  - `unsupported_effects`: 6 passed.
  - `effect_sampling`: 6 of 7 pass, including the new test.
  - Pre-existing failure, not from this group: `identity_effects_preserve_orientation_and_pixel_centers`. It binds an 80-byte colour-matrix uniform, but the shader needs 112 bytes since paint-2 grew `GpuColorMatrixParams`.
- `incular-gestures`: all pass.
- `incular-widgets`: these test files all pass: `force_press`, `explicit_transitions`, `paint_options`, `shader_backdrop`, `magnifier`, `backdrop_group`, `gesture_buttons`, `tap_cancel_arena`, `gesture_local_positions`, `tree_layout_painting`, `implicit_animations`, `image_layout`.
- `cargo check -p incular-desktop -p incular-runtime -p incular-parity-harness -p incular --tests`: passes.
- `cargo check -p incular-web --target wasm32-unknown-unknown`: passes.
- `cargo clippy -D warnings`:
  - `incular-rendering`, `incular-wgpu` and `incular-gestures` (all targets): clean.
  - `incular-widgets` and `incular-runtime` (lib and touched tests): clean once a pre-existing `large_enum_variant` is allowed. It is in another worker's in-progress `LayerAttachment::Clip.applied` in `render_object/layers.rs`.
- Harness: `gestures__` 11/11 match, `transitions__` 9/9 match.
