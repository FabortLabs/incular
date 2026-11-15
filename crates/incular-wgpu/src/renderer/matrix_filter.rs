//! Execution of matrix image-filter stages over a subtree: Flutter's
//! `ImageFilterLayer` with `ImageFilter.matrix`, which `Transform` uses when
//! it has a `filterQuality`. The child renders untransformed into the
//! layer's retained source, and one affine resample with the requested
//! sampling draws it through the matrix into the layer's effect texture.

use super::*;

impl WgpuRenderer {
    /// Lowers a matrix filter group to the `Filtered` composite of its
    /// resampled source, clipped by `parent_clip`. A singular matrix or
    /// stage draws nothing, as Flutter's `RenderTransform` paints nothing
    /// for a singular transform.
    pub(super) fn lower_matrix_filter_group(
        &mut self,
        group: LayerGroup<'_>,
        filter: MatrixFilterStage,
    ) -> Result<Option<DrawBatch>, RendererError> {
        let LayerGroup {
            scale,
            layer,
            generation,
            parent_clip,
            ..
        } = group;
        let (Some(stage_inverse), Some(matrix_inverse)) =
            (filter.stage.inverse(), filter.matrix.inverse())
        else {
            return Ok(None);
        };
        // The result covers the transformed bounds within the target.
        let target_rect = Rect::from_origin_size(
            self.target_origin,
            Size::new(
                self.target_width as f32 / scale,
                self.target_height as f32 / scale,
            ),
        );
        let Some(visible) = intersect_rect(filter.filtered_bounds, target_rect) else {
            return Ok(None);
        };
        let left = ((visible.origin.x - self.target_origin.x) * scale)
            .floor()
            .max(0.);
        let top = ((visible.origin.y - self.target_origin.y) * scale)
            .floor()
            .max(0.);
        let right = ((visible.origin.x + visible.size.width - self.target_origin.x) * scale)
            .ceil()
            .min(self.target_width as f32);
        let bottom = ((visible.origin.y + visible.size.height - self.target_origin.y) * scale)
            .ceil()
            .min(self.target_height as f32);
        if right <= left || bottom <= top {
            return Ok(None);
        }
        let (width, height) = ((right - left) as u32, (bottom - top) as u32);
        if width == 0 || height == 0 {
            return Ok(None);
        }
        let Some(source) = self.lower_source_group(group)? else {
            return Ok(None);
        };
        let output_origin = self.target_origin + Offset::new(left / scale, top / scale);
        // An output texel `e` shows the source at
        // S · (W · M⁻¹ · W⁻¹ · (e / S + output_origin) - source_origin).
        let sampling = Transform::scale(scale)
            .then(Transform::translation(Offset::new(
                -source.origin.x,
                -source.origin.y,
            )))
            .then(filter.stage)
            .then(matrix_inverse)
            .then(stage_inverse)
            .then(Transform::translation(output_origin))
            .then(Transform::scale(1. / scale));
        let mode = match filter.sampling {
            incular_painting::ImageSampling::Linear => 2,
            incular_painting::ImageSampling::Nearest => 4,
        };
        self.render_matrix_filter(
            MatrixResample {
                layer,
                generation,
                scale,
                sampling,
                mode,
                source,
            },
            width,
            height,
        )?;
        self.counters.effect_chain_compilations += 1;
        Ok(Some(DrawBatch::Filtered {
            clip: parent_clip,
            layer,
            instance: composite_instance(
                Placed::new(output_origin, width, height),
                Placed::new(self.target_origin, self.target_width, self.target_height),
                scale,
                1.,
            ),
        }))
    }

    /// Resamples the retained source of `layer` through `sampling` into
    /// its effect texture, reusing the previous result while the source,
    /// the map and the size are unchanged.
    fn render_matrix_filter(
        &mut self,
        resample: MatrixResample,
        width: u32,
        height: u32,
    ) -> Result<(), RendererError> {
        let MatrixResample {
            layer,
            generation,
            scale,
            sampling,
            mode,
            source,
        } = resample;
        let coefficients = sampling.to_kurbo().as_coeffs().map(|value| value as f32);
        let mut matrix_bits = [0; 20];
        for (bits, value) in matrix_bits.iter_mut().zip(coefficients) {
            *bits = value.to_bits();
        }
        let stage_bits = [mode, source.width, source.height, 0, 0, 0];
        if let Some(entry) = self.effect_cache.get_mut(&layer)
            && entry.source_generation == generation
            && entry.width == width
            && entry.height == height
            && entry.scale_factor_bits == scale.to_bits()
            && entry.matrix_bits == matrix_bits
            && entry.stage_bits == stage_bits
            && entry.target.format == self.window_gpu.config.format
            && entry.device_generation == self.device_generation
        {
            entry.last_used_frame = self.counters.frames;
            self.counters.effect_stage_cache_hits += 1;
            return Ok(());
        }
        if let Some(previous) = self.effect_cache.remove(&layer) {
            self.remove_effect_cache_bytes(previous.bytes);
            self.offscreen_target_pool.recycle(previous.target);
        }
        let target = self.take_effect_temp(width, height);
        let source_view = self
            .offscreen_cache
            .get(&layer)
            .expect("source cache for matrix filter")
            .target
            .color_view
            .clone();
        let mut weights = [0.; BLUR_WEIGHT_SLOTS];
        weights[..6].copy_from_slice(&coefficients);
        let params = GpuBlurParams {
            source_origin: [0., 0.],
            source_size: [source.width as f32, source.height as f32],
            output_size: [width as f32, height as f32],
            direction: [1., 1.],
            radius: 0,
            mode,
            _padding: [0; 2],
            weights,
        };
        self.submit_effect_pass(&source_view, &target.color_view, &params, false)?;
        let bind_group =
            self.create_composite_bind_group(&target, "incular matrix filter bind group");
        let bytes = target.bytes();
        self.effect_cache.insert(
            layer,
            EffectCacheEntry {
                target,
                bind_group,
                width,
                height,
                scale_factor_bits: scale.to_bits(),
                source_generation: generation,
                sigma_x_bits: 0,
                sigma_y_bits: 0,
                matrix_bits,
                stage_bits,
                device_generation: self.device_generation,
                last_used_frame: self.counters.frames,
                bytes,
                downsample_factor: 1,
            },
        );
        self.add_effect_cache_bytes(bytes);
        self.counters.effect_stage_cache_misses += 1;
        self.counters.effect_stage_rerenders += 1;
        self.counters.matrix_filter_passes += 1;
        Ok(())
    }
}

/// One matrix-filter resample: the layer whose retained source it reads,
/// the source's generation and placement, and the output-texel to
/// source-texel map with its shader mode (2 bilinear, 4 nearest).
#[derive(Clone, Copy)]
struct MatrixResample {
    layer: incular_painting::LayerId,
    generation: u64,
    scale: f32,
    sampling: Transform,
    mode: u32,
    source: CachedSource,
}
