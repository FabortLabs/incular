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
            // A general filter runs each stage over the whole target, so
            // every stage reads and writes the same frame of pixels.
            (Some(filter), _) => {
                let mut stages = Vec::new();
                for stage in filter.stages() {
                    stages.push(match stage {
                        incular_painting::ImageFilter::Blur(blur) => BackdropStage::Blur(blur),
                        incular_painting::ImageFilter::Matrix(matrix) => {
                            // A singular matrix leaves nothing to sample.
                            let Some(sampling) = incular_painting::backdrop_matrix_sampling(
                                transform, matrix, scale,
                            ) else {
                                return Ok(Vec::new());
                            };
                            BackdropStage::Matrix(sampling)
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
                    BackdropOp::Matrix(sampling),
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
        let Some((source_view, covered)) = self
            .backdrop_capture_targets
            .get(&source_layer)
            .map(|(capture, covered)| (capture.color_view.clone(), *covered))
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
                    source: &source_view,
                    source_origin: [covered.x as f32, covered.y as f32],
                    source_size: [covered.width as f32, covered.height as f32],
                    input: region.input,
                    output: region.output,
                    sigma,
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
            BackdropOp::Matrix(sampling) => {
                self.submit_effect_passes(passes);
                self.render_backdrop_matrix(layer, &source_view, sampling, width, height, scale)?;
            }
            BackdropOp::Chain(stages) => {
                // The capture covers the whole target (the chain registered a
                // full region), so each stage maps one full-target texture to
                // the next.
                self.submit_effect_passes(passes);
                let mut current: Option<OffscreenTarget> = None;
                for stage in stages.iter() {
                    let input = current
                        .as_ref()
                        .map_or_else(|| source_view.clone(), |target| target.color_view.clone());
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
                        BackdropStage::Blur(blur) => {
                            let sigma = [
                                normalize_sigma(blur.sigma_x * scale),
                                normalize_sigma(blur.sigma_y * scale),
                            ];
                            let mut blur_passes =
                                self.begin_effect_passes("incular backdrop chain");
                            let job = BlurJob {
                                source: &input,
                                source_origin: origin,
                                source_size: size,
                                input: region.input,
                                output: region.output,
                                sigma,
                                downsample_factor: choose_blur_downsample_factor(
                                    sigma[0].max(sigma[1]),
                                ),
                            };
                            let blurred = self.encode_blur(&mut blur_passes, &job);
                            self.submit_effect_passes(blur_passes);
                            let (target, temps) = blurred?;
                            for temp in temps {
                                self.recycle_effect_temp(temp);
                            }
                            target
                        }
                        BackdropStage::Matrix(sampling) => {
                            let target = self.take_effect_temp(width, height);
                            let shifted =
                                Transform::translation(Offset::new(-origin[0], -origin[1]))
                                    .then(sampling);
                            let params =
                                matrix_params(shifted, size, [width as f32, height as f32]);
                            self.submit_effect_pass(&input, &target.color_view, &params, false)?;
                            target
                        }
                        BackdropStage::Color(filter) => {
                            let target = self.take_effect_temp(width, height);
                            self.run_color_matrix_pass(&input, &target.color_view, filter)?;
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
        let size = [width as f32, height as f32];
        let params = matrix_params(sampling, size, size);
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

/// Resample parameters for an affine map from output pixel positions to
/// input texel positions over an input of `source_size` texels.
fn matrix_params(
    sampling: Transform,
    source_size: [f32; 2],
    output_size: [f32; 2],
) -> GpuBlurParams {
    let coefficients = sampling.to_kurbo().as_coeffs().map(|value| value as f32);
    let mut weights = [0.; BLUR_WEIGHT_SLOTS];
    weights[..6].copy_from_slice(&coefficients);
    GpuBlurParams {
        source_origin: [0., 0.],
        source_size,
        output_size,
        direction: [1., 1.],
        radius: 0,
        mode: 2,
        _padding: [0; 2],
        weights,
    }
}
