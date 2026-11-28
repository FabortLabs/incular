//! Execution of matrix image-filter stages over a subtree: Flutter's
//! `ImageFilterLayer` with `ImageFilter.matrix`, which `Transform` uses when
//! it has a `filterQuality`. The child renders untransformed into the
//! layer's retained source, and one affine resample with the requested
//! sampling draws it through the matrix into the layer's effect texture.

#[cfg(not(incular_d3d11))]
use super::effects::EffectPasses;
use super::*;

impl WgpuRenderer {
    /// Lowers a matrix filter group to the `Filtered` composite of its
    /// resampled source, clipped by `parent_clip`. A singular matrix or
    /// stage has no raster output, even if the frontend recorded it.
    /// Reject it before allocating an offscreen source.
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
        self.render_matrix_filter(
            MatrixResample {
                layer,
                generation,
                scale,
                sampling,
                quality: filter.sampling,
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
            quality,
            source,
        } = resample;
        let mode = matrix_mode(quality);
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
        let source_target = self
            .offscreen_cache
            .get(&layer)
            .expect("source cache for matrix filter")
            .target
            .clone();
        self.sample_matrix_target(&source_target, &target, sampling, quality)?;
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
                revision: next_texture_revision(),
                backdrop_signature: None,
            },
        );
        self.add_effect_cache_bytes(bytes);
        self.counters.effect_stage_cache_misses += 1;
        self.counters.effect_stage_rerenders += 1;
        self.counters.matrix_filter_passes += 1;
        Ok(())
    }

    /// Resamples either an isolated subtree or a captured backdrop with
    /// the same device-pixel affine map, sampling policy, and edge rules.
    pub(super) fn sample_matrix_target(
        &mut self,
        source: &OffscreenTarget,
        target: &OffscreenTarget,
        sampling: Transform,
        quality: incular_painting::ImageSampling,
    ) -> Result<(), RendererError> {
        let mode = matrix_mode(quality);
        let coefficients = sampling.to_kurbo().as_coeffs().map(|value| value as f32);
        let mut weights = [0.; BLUR_WEIGHT_SLOTS];
        weights[..6].copy_from_slice(&coefficients);
        let mut passes = self.begin_effect_passes("incular matrix sampling");
        #[cfg(not(incular_d3d11))]
        let mip_view = if mode == 6 {
            let lod = matrix_mip_level(coefficients, source.width, source.height);
            weights[6] = lod;
            if lod > 0. {
                Some(self.matrix_mip_source(source, lod.ceil() as u32, &mut passes)?)
            } else {
                None
            }
        } else {
            None
        };
        #[cfg(incular_d3d11)]
        let mip_view: Option<wgpu::TextureView> = None;
        let params = GpuBlurParams {
            source_origin: [0.; 2],
            source_size: [source.width as f32, source.height as f32],
            output_size: [target.width as f32, target.height as f32],
            direction: [1.; 2],
            radius: 0,
            mode,
            sampling_mode: 0,
            _padding: 0,
            weights,
        };
        self.record_effect_pass(
            &mut passes,
            mip_view.as_ref().unwrap_or(&source.color_view),
            &target.color_view,
            &params,
            false,
        )?;
        self.submit_effect_passes(passes);
        Ok(())
    }

    #[cfg(not(incular_d3d11))]
    fn matrix_mip_source(
        &mut self,
        source: &OffscreenTarget,
        levels_above_base: u32,
        passes: &mut EffectPasses,
    ) -> Result<wgpu::TextureView, RendererError> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("incular matrix filter mip source"),
            size: wgpu::Extent3d {
                width: source.width,
                height: source.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels_above_base + 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: source.format,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        passes.encoder.copy_texture_to_texture(
            source._color.as_image_copy(),
            texture.as_image_copy(),
            wgpu::Extent3d {
                width: source.width,
                height: source.height,
                depth_or_array_layers: 1,
            },
        );
        let level_view = |level| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let mut previous = level_view(0);
        for level in 1..=levels_above_base {
            let next = level_view(level);
            let width = |shift: u32| (source.width >> shift).max(1);
            let height = |shift: u32| (source.height >> shift).max(1);
            let params = GpuBlurParams {
                source_origin: [0.; 2],
                source_size: [width(level - 1) as f32, height(level - 1) as f32],
                output_size: [width(level) as f32, height(level) as f32],
                direction: [1.; 2],
                radius: 0,
                mode: 7,
                sampling_mode: 0,
                _padding: 0,
                weights: [0.; BLUR_WEIGHT_SLOTS],
            };
            self.record_effect_pass(passes, &previous, &next, &params, false)?;
            previous = next;
        }
        Ok(texture.create_view(&wgpu::TextureViewDescriptor::default()))
    }
}

fn matrix_mode(quality: incular_painting::ImageSampling) -> u32 {
    use incular_painting::ImageSampling;
    match quality {
        ImageSampling::Linear => 2,
        ImageSampling::Nearest => 4,
        ImageSampling::MatrixCubic => 8,
        ImageSampling::LinearMipmap if cfg!(incular_d3d11) => 2,
        ImageSampling::LinearMipmap => 6,
    }
}

#[cfg(not(incular_d3d11))]
fn matrix_mip_level([a, b, c, d, ..]: [f32; 6], width: u32, height: u32) -> f32 {
    // Largest singular value of the output-to-source linear map estimates
    // the major axis of one output pixel's footprint in source pixels.
    let ab = a.hypot(b);
    let cd = c.hypot(d);
    let dot = a * c + b * d;
    let major = (0.5 * (ab * ab + cd * cd + (ab * ab - cd * cd).hypot(2. * dot))).sqrt();
    if !major.is_finite() {
        return 0.;
    }
    major.max(1.).log2().min(width.max(height).ilog2() as f32)
}

/// One matrix-filter resample: the layer whose retained source it reads,
/// the source's generation and placement, and the output-texel to
/// source-texel map with its sampling policy.
#[derive(Clone, Copy)]
struct MatrixResample {
    layer: incular_painting::LayerId,
    generation: u64,
    scale: f32,
    sampling: Transform,
    quality: incular_painting::ImageSampling,
    source: CachedSource,
}
