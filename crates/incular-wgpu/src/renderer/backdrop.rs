//! Execution of backdrop-filter stages. Which stage captures and how the
//! matrix samples are decided in `incular-rendering`; this module copies
//! the part of the target a stage reads, filters it and caches the result
//! for the `Filtered` composite that follows the stage.
//!
//! As Impeller's coverage hint does, a blur stage only produces the pixels
//! its parent clip can show and only reads those grown by the kernel's
//! reach, so a small clipped glass button blurs a small texture, not the
//! window.

use super::backdrop_cache::BackdropPlan;
use super::effects::BlurJob;
use super::*;
#[cfg(incular_d3d11)]
use crate::d3d11 as wgpu;

impl WgpuRenderer {
    /// Starts a frame: forgets the previous frame's captures and returns
    /// their textures to the pool.
    pub(super) fn begin_backdrop_frame(&mut self) {
        self.backdrop_captures.clear();
        self.backdrop_capture_regions.clear();
        self.backdrop_scope = 0;
        for (_, (target, _)) in self.backdrop_capture_targets.drain() {
            self.offscreen_target_pool.recycle(target);
        }
    }

    /// Lowers an active backdrop-filter stage (Flutter's `BackdropFilter`
    /// with `BlendMode.srcOver`): a `Backdrop` batch that filters the target
    /// as painted so far, and the `Filtered` composite that draws the result
    /// within `parent_clip`. The stage's children lower after these.
    /// `transform` maps the stage's coordinates to the target's.
    ///
    /// A blur produces only the target pixels `parent_clip` can show (its
    /// conservative rectangle, or the whole target when it has none) and
    /// reads those grown by the kernel's reach; past the target's edge the
    /// backdrop is transparent.
    #[expect(
        clippy::too_many_arguments,
        reason = "lowering takes the full backdrop context; grouping it into a struct is tracked separately"
    )]
    pub(super) fn lower_backdrop(
        &mut self,
        layer: incular_painting::LayerId,
        blur: GaussianBlur,
        matrix: Option<Transform>,
        image_filter: Option<&incular_painting::ImageFilter>,
        key: Option<incular_painting::BackdropKey>,
        transform: Transform,
        scale: f32,
        parent_clip: ClipState,
    ) -> Result<Vec<DrawBatch>, RendererError> {
        let (width, height) = (self.target_width, self.target_height);
        let Some(visible) = clip_device_region(parent_clip, scale, width, height) else {
            return Ok(Vec::new());
        };
        let full = PixelRect::full(width, height);
        let (op, region) = match (image_filter, matrix) {
            (Some(filter), _) if filter.matrix_parts().is_some() => {
                let (matrix, quality) = filter.matrix_parts().expect("matrix filter");
                let Some(sampling) =
                    incular_painting::backdrop_matrix_sampling(transform, matrix, scale)
                else {
                    return Ok(Vec::new());
                };
                (
                    BackdropOp::Matrix(sampling, quality.matrix_sampling()),
                    BackdropRegion {
                        output: full,
                        input: full,
                    },
                )
            }
            // A general filter runs each stage over the whole target, so
            // every stage reads and writes the same frame of pixels.
            (Some(filter), _) => {
                let mut stages = Vec::new();
                for stage in filter.stages() {
                    stages.push(match stage {
                        incular_painting::ImageFilter::Blur(blur) => BackdropStage::Blur {
                            blur,
                            tile_mode: incular_painting::TileMode::Decal,
                            bounds: None,
                        },
                        incular_painting::ImageFilter::SampledBlur(sampled) => {
                            // `transform` is relative to the active target, which may
                            // be a nonzero-origin isolated layer rather than the window.
                            let bounds = match sampled.bounds {
                                Some(rect) => {
                                    let device_from_local = Transform::scale(scale).then(transform);
                                    let Some(local_from_device) = device_from_local.inverse()
                                    else {
                                        return Ok(Vec::new());
                                    };
                                    Some(BackdropBlurBounds {
                                        local_from_device,
                                        rect,
                                    })
                                }
                                None => None,
                            };
                            BackdropStage::Blur {
                                blur: sampled.blur,
                                tile_mode: sampled.tile_mode,
                                bounds,
                            }
                        }
                        matrix @ (incular_painting::ImageFilter::Matrix(_)
                        | incular_painting::ImageFilter::SampledMatrix { .. }) => {
                            let (matrix, quality) = matrix.matrix_parts().expect("matrix filter");
                            // A singular matrix leaves nothing to sample.
                            let Some(sampling) = incular_painting::backdrop_matrix_sampling(
                                transform, matrix, scale,
                            ) else {
                                return Ok(Vec::new());
                            };
                            BackdropStage::Matrix(sampling, quality.matrix_sampling())
                        }
                        incular_painting::ImageFilter::Color(color) => BackdropStage::Color(color),
                        // Refused before lowering (`unsupported_effect`).
                        _ => return Err(RendererError::UnsupportedBackdropFilter),
                    });
                }
                (
                    BackdropOp::Chain(stages.into()),
                    BackdropRegion {
                        output: full,
                        input: full,
                    },
                )
            }
            (None, Some(matrix)) => {
                // A singular matrix leaves nothing to sample: the stage's
                // layer starts empty and the backdrop shows through.
                let Some(sampling) =
                    incular_painting::backdrop_matrix_sampling(transform, matrix, scale)
                else {
                    return Ok(Vec::new());
                };
                // The matrix can sample any of the backdrop.
                (
                    BackdropOp::Matrix(sampling, incular_painting::ImageSampling::LinearMipmap),
                    BackdropRegion {
                        output: full,
                        input: full,
                    },
                )
            }
            (None, None) => {
                let sigma_x = normalize_sigma(blur.sigma_x * scale);
                let sigma_y = normalize_sigma(blur.sigma_y * scale);
                let factor = choose_blur_downsample_factor(sigma_x.max(sigma_y));
                let input = visible.expanded(
                    blur_read_margin(sigma_x, factor),
                    blur_read_margin(sigma_y, factor),
                    factor,
                    width,
                    height,
                );
                (
                    BackdropOp::Blur(blur),
                    BackdropRegion {
                        output: visible,
                        input,
                    },
                )
            }
        };
        let sample = self
            .backdrop_captures
            .sample(layer, key, self.backdrop_scope);
        // A shared capture covers every stage that reads it. All stages of a
        // target lower before any of them executes.
        let capturer = match sample {
            incular_painting::BackdropSample::Capture => layer,
            incular_painting::BackdropSample::Shared(first) => first,
        };
        self.backdrop_capture_regions
            .entry(capturer)
            .and_modify(|covered| *covered = covered.union(region.input))
            .or_insert(region.input);
        let output = region.output;
        let placed = Placed::new(
            self.target_origin + Offset::new(output.x as f32 / scale, output.y as f32 / scale),
            output.width,
            output.height,
        );
        let target = Placed::new(self.target_origin, width, height);
        Ok(vec![
            DrawBatch::Backdrop {
                layer,
                sample,
                op,
                region,
            },
            DrawBatch::Filtered {
                clip: parent_clip,
                layer,
                instance: composite_instance(placed, target, scale, 1.),
            },
        ])
    }

    /// Runs a `Backdrop` batch against `current`, the target as painted so
    /// far: captures the pixels it needs (or takes the keyed capture),
    /// filters them and stores the result as the effect of `layer`. The
    /// capture and a blur's passes go to the GPU in one submission. A stage
    /// whose `plan` reuses its retained result only takes the capture the
    /// stages sharing it need.
    #[expect(
        clippy::too_many_arguments,
        reason = "lowering takes the full backdrop context; grouping it into a struct is tracked separately"
    )]
    pub(super) fn execute_backdrop(
        &mut self,
        layer: incular_painting::LayerId,
        sample: incular_painting::BackdropSample,
        op: BackdropOp,
        region: BackdropRegion,
        plan: BackdropPlan,
        current: &OffscreenTarget,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<(), RendererError> {
        let mut passes = self.begin_effect_passes("incular backdrop passes");
        let source_layer = match sample {
            incular_painting::BackdropSample::Capture if plan.capture => {
                let covered = self
                    .backdrop_capture_regions
                    .get(&layer)
                    .copied()
                    .unwrap_or(region.input);
                let capture = self.take_effect_temp(covered.width, covered.height);
                passes.encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &current._color,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: covered.x,
                            y: covered.y,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &capture._color,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: covered.width,
                        height: covered.height,
                        depth_or_array_layers: 1,
                    },
                );
                self.counters.backdrop_capture_texels += covered.area();
                if let Some((previous, _)) = self
                    .backdrop_capture_targets
                    .insert(layer, (capture, covered))
                {
                    self.offscreen_target_pool.recycle(previous);
                }
                self.counters.offscreen_render_passes += 1;
                layer
            }
            incular_painting::BackdropSample::Capture => layer,
            incular_painting::BackdropSample::Shared(first) => first,
        };
        if plan.reuse {
            // Only the capture was needed: the retained result stands.
            self.submit_effect_passes(passes);
            self.counters.backdrop_results_reused += 1;
            if let Some(entry) = self.effect_cache.get_mut(&layer) {
                entry.last_used_frame = self.counters.frames;
            }
            return Ok(());
        }
        let Some((source, covered)) = self
            .backdrop_capture_targets
            .get(&source_layer)
            .map(|(capture, covered)| (capture.clone(), *covered))
        else {
            self.submit_effect_passes(passes);
            return Ok(());
        };
        // A backdrop result is reused through its signature, never through
        // the generation of a source layer.
        let generation = u64::MAX - self.counters.frames;
        match op {
            BackdropOp::Blur(blur) => {
                let sigma = [
                    normalize_sigma(blur.sigma_x * scale),
                    normalize_sigma(blur.sigma_y * scale),
                ];
                let downsample_factor = choose_blur_downsample_factor(sigma[0].max(sigma[1]));
                // Target pixels are the common frame; the capture holds
                // `covered` of them.
                let job = BlurJob {
                    source: &source.color_view,
                    source_origin: [covered.x as f32, covered.y as f32],
                    source_size: [covered.width as f32, covered.height as f32],
                    input: region.input,
                    output: region.output,
                    sigma,
                    tile_mode: incular_painting::TileMode::Decal,
                    opaque_normalization: false,
                    downsample_factor,
                };
                let blurred = self.encode_blur(&mut passes, &job);
                self.submit_effect_passes(passes);
                let (target, temps) = blurred?;
                for temp in temps {
                    self.recycle_effect_temp(temp);
                }
                self.store_blur_effect(layer, target, scale, generation, sigma, downsample_factor);
            }
            BackdropOp::Matrix(sampling, quality) => {
                self.submit_effect_passes(passes);
                self.render_backdrop_matrix(layer, &source, sampling, quality, scale)?;
            }
            BackdropOp::Chain(stages) => {
                // The capture covers the whole target (the chain registered a
                // full region), so each stage maps one full-target texture to
                // the next.
                self.submit_effect_passes(passes);
                let mut current: Option<OffscreenTarget> = None;
                for stage in stages.iter() {
                    let input = current.as_ref().unwrap_or(&source);
                    let origin = if current.is_some() {
                        [0., 0.]
                    } else {
                        [covered.x as f32, covered.y as f32]
                    };
                    let size = if current.is_some() {
                        [width as f32, height as f32]
                    } else {
                        [covered.width as f32, covered.height as f32]
                    };
                    let next = match *stage {
                        BackdropStage::Blur {
                            blur,
                            tile_mode,
                            bounds,
                        } => {
                            let sigma = [
                                normalize_sigma(blur.sigma_x * scale),
                                normalize_sigma(blur.sigma_y * scale),
                            ];
                            let mut blur_passes =
                                self.begin_effect_passes("incular backdrop chain");
                            // Exact zero sigma returns the unmodified source, even for a
                            // bounded configuration. Otherwise reject out-of-bounds scene
                            // texels before *any* linear mixing or downsampling.
                            let bounded =
                                bounds.filter(|_| sigma.iter().any(|v| *v > f32::EPSILON));
                            let masked = if let Some(bounds) = bounded {
                                let target = self.take_effect_temp(width, height);
                                self.record_effect_pass(
                                    &mut blur_passes,
                                    &input.color_view,
                                    &target.color_view,
                                    &bounded_source_params(
                                        bounds,
                                        origin,
                                        size,
                                        [width as f32, height as f32],
                                    ),
                                    false,
                                )?;
                                Some(target)
                            } else {
                                None
                            };
                            let job = BlurJob {
                                source: masked
                                    .as_ref()
                                    .map_or(&input.color_view, |target| &target.color_view),
                                source_origin: if masked.is_some() { [0.; 2] } else { origin },
                                source_size: if masked.is_some() {
                                    [width as f32, height as f32]
                                } else {
                                    size
                                },
                                input: region.input,
                                output: region.output,
                                sigma,
                                tile_mode: if masked.is_some() {
                                    incular_painting::TileMode::Decal
                                } else {
                                    tile_mode
                                },
                                opaque_normalization: masked.is_some(),
                                downsample_factor: choose_blur_downsample_factor(
                                    sigma[0].max(sigma[1]),
                                ),
                            };
                            let blurred = self.encode_blur(&mut blur_passes, &job);
                            self.submit_effect_passes(blur_passes);
                            if let Some(masked) = masked {
                                self.recycle_effect_temp(masked);
                            }
                            let (target, temps) = blurred?;
                            for temp in temps {
                                self.recycle_effect_temp(temp);
                            }
                            target
                        }
                        BackdropStage::Matrix(sampling, quality) => {
                            let target = self.take_effect_temp(width, height);
                            let shifted =
                                Transform::translation(Offset::new(-origin[0], -origin[1]))
                                    .then(sampling);
                            self.sample_matrix_target(input, &target, shifted, quality)?;
                            target
                        }
                        BackdropStage::Color(filter) => {
                            let target = self.take_effect_temp(width, height);
                            self.run_color_matrix_pass(
                                &input.color_view,
                                &target.color_view,
                                filter,
                            )?;
                            self.counters.color_matrix_passes += 1;
                            target
                        }
                    };
                    if let Some(previous) = current.replace(next) {
                        self.recycle_effect_temp(previous);
                    }
                }
                if let Some(target) = current {
                    self.store_blur_effect(layer, target, scale, generation, [0., 0.], 1);
                }
                self.counters.backdrop_filter_chains += 1;
            }
        }
        self.counters.effect_chain_compilations += 1;
        self.counters.backdrop_results_rendered += 1;
        if let Some(entry) = self.effect_cache.get_mut(&layer) {
            entry.backdrop_signature = Some(plan.signature);
        }
        Ok(())
    }

    /// `ImageFilter.matrix` on a backdrop: resamples the capture through
    /// `sampling` (output device pixel to capture pixel) into the effect
    /// cache entry of `layer`, transparent where the capture does not reach.
    fn render_backdrop_matrix(
        &mut self,
        layer: incular_painting::LayerId,
        source: &OffscreenTarget,
        sampling: Transform,
        quality: incular_painting::ImageSampling,
        scale: f32,
    ) -> Result<(), RendererError> {
        let (width, height) = (source.width, source.height);
        if let Some(previous) = self.effect_cache.remove(&layer) {
            self.remove_effect_cache_bytes(previous.bytes);
            self.offscreen_target_pool.recycle(previous.target);
        }
        let target = self.take_effect_temp(width, height);
        let [a, b, c, d, e, f] = sampling.to_kurbo().as_coeffs().map(|value| value as f32);
        self.sample_matrix_target(source, &target, sampling, quality)?;
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
                stage_bits: [0; 6],
                device_generation: self.device_generation,
                last_used_frame: self.counters.frames,
                bytes,
                downsample_factor: 1,
                revision: next_texture_revision(),
                backdrop_signature: None,
            },
        );
        self.add_effect_cache_bytes(bytes);
        Ok(())
    }
}

fn bounded_source_params(
    bounds: BackdropBlurBounds,
    source_origin: [f32; 2],
    source_size: [f32; 2],
    output_size: [f32; 2],
) -> GpuBlurParams {
    let [a, b, c, d, e, f] = bounds
        .local_from_device
        .coefficients()
        .map(|value| value as f32);
    let rect = bounds.rect;
    let mut weights = [0.; BLUR_WEIGHT_SLOTS];
    weights[..12].copy_from_slice(&[
        a,
        b,
        c,
        d,
        e,
        f,
        0.,
        0.,
        rect.origin.x,
        rect.origin.y,
        rect.origin.x + rect.size.width,
        rect.origin.y + rect.size.height,
    ]);
    GpuBlurParams {
        source_origin,
        source_size,
        output_size,
        direction: [1., 1.],
        radius: 0,
        mode: 5,
        sampling_mode: 0,
        _padding: 0,
        weights,
    }
}
