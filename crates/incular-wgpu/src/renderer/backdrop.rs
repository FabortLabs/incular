//! Execution of backdrop-filter stages. Which stage captures and how the
//! matrix samples are decided in `incular-rendering`; this module copies
//! the target, filters the copy and caches the result for the `Filtered`
//! composite that follows the stage.

use super::*;

impl WgpuRenderer {
    /// Starts a frame: forgets the previous frame's captures and returns
    /// their textures to the pool.
    pub(super) fn begin_backdrop_frame(&mut self) {
        self.backdrop_captures.clear();
        self.backdrop_scope = 0;
        for (_, target) in self.backdrop_capture_targets.drain() {
            self.offscreen_target_pool.recycle(target);
        }
    }

    /// Lowers an active backdrop-filter stage (Flutter's `BackdropFilter`
    /// with `BlendMode.srcOver`): a `Backdrop` batch that filters the target
    /// as painted so far, and the `Filtered` composite that draws the result
    /// over the whole target within `parent_clip`. The stage's children
    /// lower after these. `transform` maps the stage's coordinates to the
    /// target's.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_backdrop(
        &mut self,
        layer: incular_painting::LayerId,
        blur: GaussianBlur,
        matrix: Option<Transform>,
        key: Option<incular_painting::BackdropKey>,
        transform: Transform,
        scale: f32,
        parent_clip: ClipState,
    ) -> Result<Vec<DrawBatch>, RendererError> {
        let op = match matrix {
            Some(matrix) => {
                // A singular matrix leaves nothing to sample: the stage's
                // layer starts empty and the backdrop shows through.
                let Some(sampling) =
                    incular_painting::backdrop_matrix_sampling(transform, matrix, scale)
                else {
                    return Ok(Vec::new());
                };
                BackdropOp::Matrix(sampling)
            }
            None => BackdropOp::Blur(blur),
        };
        let target = Placed::new(self.target_origin, self.target_width, self.target_height);
        let output = match op {
            BackdropOp::Blur(blur) => {
                let Some(effect) =
                    self.blur_effect_placement(self.target_bounds(scale), blur, scale)?
                else {
                    return Ok(Vec::new());
                };
                Placed::new(effect.origin, effect.width, effect.height)
            }
            BackdropOp::Matrix(_) => target,
        };
        let sample = self
            .backdrop_captures
            .sample(layer, key, self.backdrop_scope);
        Ok(vec![
            DrawBatch::Backdrop { layer, sample, op },
            DrawBatch::Filtered {
                clip: parent_clip,
                layer,
                instance: composite_instance(output, target, scale, 1.),
            },
        ])
    }

    /// The active target in logical coordinates.
    fn target_bounds(&self, scale: f32) -> Rect {
        Rect::from_origin_size(
            self.target_origin,
            Size::new(
                self.target_width as f32 / scale,
                self.target_height as f32 / scale,
            ),
        )
    }

    /// Runs a `Backdrop` batch against `current`, the target as painted so
    /// far: captures it (or takes the keyed capture), filters the capture
    /// and stores the result as the effect of `layer`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_backdrop(
        &mut self,
        layer: incular_painting::LayerId,
        sample: incular_painting::BackdropSample,
        op: BackdropOp,
        current: &OffscreenTarget,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<(), RendererError> {
        let source_layer = match sample {
            incular_painting::BackdropSample::Capture => {
                let capture = self.take_effect_temp(width, height);
                self.copy_color_texture(
                    current,
                    &capture,
                    width,
                    height,
                    "incular backdrop capture",
                );
                if let Some(previous) = self.backdrop_capture_targets.insert(layer, capture) {
                    self.offscreen_target_pool.recycle(previous);
                }
                self.counters.offscreen_render_passes += 1;
                layer
            }
            incular_painting::BackdropSample::Shared(first) => first,
        };
        let Some(source_view) = self
            .backdrop_capture_targets
            .get(&source_layer)
            .map(|capture| capture.color_view.clone())
        else {
            return Ok(());
        };
        // The active target is the one this stage lowered against, so the
        // effect lands where its `Filtered` composite expects it.
        let source = CachedSource {
            origin: self.target_origin,
            width,
            height,
        };
        // A backdrop changes whenever anything behind it does, so the
        // effect never reuses a previous frame's result.
        let generation = u64::MAX - self.counters.frames;
        match op {
            BackdropOp::Blur(blur) => {
                let bounds = self.target_bounds(scale);
                self.render_blur(
                    scale,
                    layer,
                    generation,
                    bounds,
                    blur,
                    source,
                    Some(source_view),
                    false,
                )?;
            }
            BackdropOp::Matrix(sampling) => {
                self.render_backdrop_matrix(layer, &source_view, sampling, width, height, scale)?;
            }
        }
        self.counters.effect_chain_compilations += 1;
        Ok(())
    }

    /// `ImageFilter.matrix` on a backdrop: resamples the capture through
    /// `sampling` (output device pixel to capture pixel) into the effect
    /// cache entry of `layer`, transparent where the capture does not reach.
    fn render_backdrop_matrix(
        &mut self,
        layer: incular_painting::LayerId,
        source_view: &wgpu::TextureView,
        sampling: Transform,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<(), RendererError> {
        if let Some(previous) = self.effect_cache.remove(&layer) {
            self.remove_effect_cache_bytes(previous.bytes);
            self.offscreen_target_pool.recycle(previous.target);
        }
        let target = self.take_effect_temp(width, height);
        let [a, b, c, d, e, f] = sampling.to_kurbo().as_coeffs().map(|value| value as f32);
        let mut weights = [0.; BLUR_WEIGHT_SLOTS];
        weights[..6].copy_from_slice(&[a, b, c, d, e, f]);
        let size = [width as f32, height as f32];
        let params = GpuBlurParams {
            source_origin: [0., 0.],
            source_size: size,
            output_size: size,
            direction: [1., 1.],
            radius: 0,
            mode: 2,
            _padding: [0; 2],
            weights,
        };
        self.submit_effect_pass(source_view, &target.color_view, &params, false)?;
        let bind_group =
            self.create_composite_bind_group(&target, "incular backdrop matrix bind group");
        let bytes = target.bytes();
        let mut matrix_bits = [0; 20];
        for (bits, value) in matrix_bits.iter_mut().zip([a, b, c, d, e, f]) {
            *bits = value.to_bits();
        }
        self.effect_cache.insert(
            layer,
            EffectCacheEntry {
                target,
                bind_group,
                width,
                height,
                scale_factor_bits: scale.to_bits(),
                source_generation: u64::MAX - self.counters.frames,
                sigma_x_bits: 0,
                sigma_y_bits: 0,
                matrix_bits,
                device_generation: self.device_generation,
                last_used_frame: self.counters.frames,
                bytes,
                downsample_factor: 1,
            },
        );
        self.add_effect_cache_bytes(bytes);
        Ok(())
    }
}
