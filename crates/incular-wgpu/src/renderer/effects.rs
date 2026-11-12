use super::*;

impl WgpuRenderer {
    pub(super) fn create_offscreen_target(
        &mut self,
        width: u32,
        height: u32,
        label: &'static str,
    ) -> OffscreenTarget {
        let target = create_scene_target(&self.device, self.config.format, width, height, label);
        self.counters.offscreen_color_texture_creations += 1;
        self.counters.offscreen_stencil_texture_creations += 1;
        target
    }

    pub(super) fn create_composite_bind_group(
        &self,
        target: &OffscreenTarget,
        label: &'static str,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&target.color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.composite_sampler),
                },
            ],
        })
    }

    pub(super) fn render_cached_batches(
        &mut self,
        batches: &[DrawBatch],
        scale: f32,
        width: u32,
        height: u32,
        layer: incular_painting::LayerId,
        label: &'static str,
    ) -> Result<(), RendererError> {
        if batches.iter().any(batch_reads_destination) {
            // Nested destination reads use the same ping-pong graph as the
            // presentation path, then copy the completed result into the
            // retained source target.  The copy is outside the blend pass,
            // so no texture is sampled while it is being rendered.
            let Some(output) = self
                .offscreen_cache
                .get(&layer)
                .map(|entry| entry.target.clone())
            else {
                return Ok(());
            };
            self.ensure_destination_targets(width, height);
            let mut targets = self
                .destination_targets
                .take()
                .expect("destination targets after ensure");
            let rendered = self.render_destination_batches(
                batches,
                scale,
                width,
                height,
                &mut targets,
                label,
                wgpu::Color::TRANSPARENT,
            );
            let (current_first, _, _, passes) = match rendered {
                Ok(result) => result,
                Err(error) => {
                    self.destination_targets = Some(targets);
                    return Err(error);
                }
            };
            let final_target = if current_first {
                &targets.first
            } else {
                &targets.second
            };
            self.copy_color_texture(
                final_target,
                &output,
                width,
                height,
                "incular nested destination composition copy",
            );
            self.destination_targets = Some(targets);
            self.counters.offscreen_render_passes += u64::from(passes);
            return Ok(());
        }
        let rectangles = count_rectangles(batches);
        let glyphs = count_glyphs(batches);
        let images = count_images(batches);
        let rounded = count_rounded(batches);
        let paths = count_paths(batches);
        let composites = count_composites(batches);
        self.ensure_rectangle_capacity(rectangles);
        self.ensure_glyph_capacity(glyphs);
        self.ensure_image_capacity(images);
        self.ensure_composite_capacity(composites);
        if rounded > self.rounded_rect_instance_capacity {
            self.rounded_rect_instance_capacity = rounded.next_power_of_two();
            self.rounded_rect_instances =
                create_rrect_buffer(&self.device, self.rounded_rect_instance_capacity);
        }
        if paths > self.path_instance_capacity {
            self.path_instance_capacity = paths.next_power_of_two();
            self.path_instances =
                create_path_instance_buffer(&self.device, self.path_instance_capacity);
        }
        self.upload_instance_data(batches, scale, width, height);
        self.prepare_image_bind_groups(batches);
        let Some(entry) = self.offscreen_cache.get(&layer) else {
            return Ok(());
        };
        let color_view = entry.target.color_view.clone();
        let stencil_view = entry.target.stencil_view.clone();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        self.encode_batches(
            &mut encoder,
            &color_view,
            Some(&stencil_view),
            batches,
            width,
            height,
            scale,
            wgpu::Color::TRANSPARENT,
            None,
            false,
        )?;
        self.queue.submit(Some(encoder.finish()));
        self.counters.offscreen_render_passes += 1;
        Ok(())
    }

    pub(super) fn lower_blur_group(
        &mut self,
        group: LayerGroup<'_>,
        blur: GaussianBlur,
    ) -> Result<Option<DrawBatch>, RendererError> {
        let LayerGroup {
            scale,
            layer,
            generation,
            bounds,
            parent_clip,
            ..
        } = group;
        let Some(source) = self.lower_source_group(group)? else {
            return Ok(None);
        };
        let Some(effect) = self.render_blur_from_source(
            scale,
            layer,
            generation,
            bounds,
            GaussianBlur::new(blur.sigma_x, blur.sigma_y),
            source,
            false,
        )?
        else {
            return Ok(None);
        };
        self.counters.effect_chain_compilations += 1;
        let active_origin = self.target_origin;
        Ok(Some(DrawBatch::Filtered {
            clip: parent_clip,
            layer,
            instance: composite_instance(
                Placed::new(effect.origin, effect.width, effect.height),
                Placed::new(active_origin, self.target_width, self.target_height),
                scale,
                1.,
            ),
        }))
    }

    pub(super) fn lower_color_filter_group(
        &mut self,
        group: LayerGroup<'_>,
        filter: ColorFilter,
    ) -> Result<Option<DrawBatch>, RendererError> {
        let LayerGroup {
            scale,
            layer,
            generation,
            parent_clip,
            ..
        } = group;
        let Some(source) = self.lower_source_group(group)? else {
            return Ok(None);
        };
        let Some(effect) =
            self.render_color_filter_from_source(scale, layer, generation, filter, source)?
        else {
            return Ok(None);
        };
        self.counters.effect_chain_compilations += 1;
        Ok(Some(DrawBatch::Filtered {
            clip: parent_clip,
            layer,
            instance: composite_instance(
                Placed::new(effect.origin, effect.width, effect.height),
                Placed::new(self.target_origin, self.target_width, self.target_height),
                scale,
                1.,
            ),
        }))
    }

    pub(super) fn lower_blend_group(
        &mut self,
        group: LayerGroup<'_>,
        mode: BlendMode,
    ) -> Result<Option<DrawBatch>, RendererError> {
        let LayerGroup {
            scale,
            layer,
            parent_clip,
            ..
        } = group;
        if parent_clip == ClipState::Empty {
            return Ok(None);
        }
        let Some(source) = self.lower_source_group(group)? else {
            return Ok(None);
        };
        self.counters.effect_chain_compilations += 1;
        Ok(Some(DrawBatch::Blend {
            clip: parent_clip,
            layer,
            mode,
            instance: blend_instance(
                Placed::new(source.origin, source.width, source.height),
                Placed::new(self.target_origin, self.target_width, self.target_height),
                scale,
                mode,
            ),
        }))
    }

    pub(super) fn render_color_filter_from_source(
        &mut self,
        scale: f32,
        layer: incular_painting::LayerId,
        generation: u64,
        filter: ColorFilter,
        source: CachedSource,
    ) -> Result<Option<CachedEffect>, RendererError> {
        let frame = self.counters.frames;
        let matrix_bits = filter.to_matrix().map(f32::to_bits);
        if let Some(entry) = self.effect_cache.get_mut(&layer)
            && entry.source_generation == generation
            && entry.width == source.width
            && entry.height == source.height
            && entry.scale_factor_bits == scale.to_bits()
            && entry.matrix_bits == matrix_bits
            && entry.sigma_x_bits == 0
            && entry.sigma_y_bits == 0
            && entry.target.format == self.window_gpu.config.format
            && entry.device_generation == self.device_generation
        {
            entry.last_used_frame = frame;
            self.counters.effect_stage_cache_hits += 1;
            return Ok(Some(CachedEffect {
                origin: source.origin,
                width: source.width,
                height: source.height,
            }));
        }
        if let Some(previous) = self.effect_cache.remove(&layer) {
            self.remove_effect_cache_bytes(previous.bytes);
            self.offscreen_target_pool.recycle(previous.target);
        }
        let target = if let Some(target) =
            self.offscreen_target_pool
                .take(source.width, source.height, self.config.format, true)
        {
            self.counters.effect_texture_pool_hits += 1;
            target
        } else {
            self.counters.effect_texture_pool_misses += 1;
            self.counters.effect_texture_creations += 1;
            self.create_offscreen_target(source.width, source.height, "incular color matrix result")
        };
        let source_view = self
            .offscreen_cache
            .get(&layer)
            .expect("source cache for color filter")
            .target
            .color_view
            .clone();
        self.run_color_matrix_pass(&source_view, &target.color_view, filter)?;
        let bind_group =
            self.create_composite_bind_group(&target, "incular color matrix bind group");
        let bytes = target.bytes();
        self.effect_cache.insert(
            layer,
            EffectCacheEntry {
                target,
                bind_group,
                width: source.width,
                height: source.height,
                scale_factor_bits: scale.to_bits(),
                source_generation: generation,
                sigma_x_bits: 0,
                sigma_y_bits: 0,
                matrix_bits,
                device_generation: self.device_generation,
                last_used_frame: frame,
                bytes,
                downsample_factor: 1,
            },
        );
        self.add_effect_cache_bytes(bytes);
        self.counters.effect_stage_cache_misses += 1;
        self.counters.effect_stage_rerenders += 1;
        self.counters.color_matrix_passes += 1;
        Ok(Some(CachedEffect {
            origin: source.origin,
            width: source.width,
            height: source.height,
        }))
    }

    pub(super) fn lower_drop_shadow_group(
        &mut self,
        group: LayerGroup<'_>,
        shadow: DropShadowEffect,
    ) -> Result<Vec<DrawBatch>, RendererError> {
        let LayerGroup {
            scale,
            layer,
            generation,
            bounds,
            parent_clip,
            ..
        } = group;
        let Some(source) = self.lower_source_group(group)? else {
            return Ok(Vec::new());
        };
        self.counters.effect_chain_compilations += 1;
        let active_origin = self.target_origin;
        let blur = GaussianBlur::new(shadow.sigma_x, shadow.sigma_y);
        let shadow_batch = if blur.sigma_x <= f32::EPSILON && blur.sigma_y <= f32::EPSILON {
            DrawBatch::Shadow {
                clip: parent_clip,
                layer,
                instance: composite_instance_with_color(
                    Placed::new(source.origin + shadow.offset, source.width, source.height),
                    Placed::new(active_origin, self.target_width, self.target_height),
                    scale,
                    1.,
                    shadow.color,
                    true,
                ),
            }
        } else {
            let Some(effect) =
                self.render_blur_from_source(scale, layer, generation, bounds, blur, source, true)?
            else {
                return Ok(Vec::new());
            };
            DrawBatch::Shadow {
                clip: parent_clip,
                layer,
                instance: composite_instance_with_color(
                    Placed::new(effect.origin + shadow.offset, effect.width, effect.height),
                    Placed::new(active_origin, self.target_width, self.target_height),
                    scale,
                    1.,
                    shadow.color,
                    true,
                ),
            }
        };
        Ok(vec![
            shadow_batch,
            DrawBatch::Offscreen {
                clip: parent_clip,
                layer,
                instance: composite_instance(
                    Placed::new(source.origin, source.width, source.height),
                    Placed::new(active_origin, self.target_width, self.target_height),
                    scale,
                    1.,
                ),
            },
        ])
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_blur_from_source(
        &mut self,
        scale: f32,
        layer: incular_painting::LayerId,
        generation: u64,
        source_bounds: Rect,
        blur: GaussianBlur,
        source: CachedSource,
        for_shadow: bool,
    ) -> Result<Option<CachedEffect>, RendererError> {
        self.render_blur(
            scale,
            layer,
            generation,
            source_bounds,
            blur,
            source,
            for_shadow,
        )
    }

    /// Where the blur of `source_bounds` lands in the active target: its
    /// logical origin and physical size, or `None` when it covers no pixel.
    pub(super) fn blur_effect_placement(
        &self,
        source_bounds: Rect,
        blur: GaussianBlur,
        scale: f32,
    ) -> Result<Option<CachedEffect>, RendererError> {
        let active_origin = self.target_origin;
        let effect_bounds = blur_bounds(source_bounds, blur.sigma_x, blur.sigma_y);
        let left = ((effect_bounds.origin.x - active_origin.x) * scale).floor();
        let top = ((effect_bounds.origin.y - active_origin.y) * scale).floor();
        let right =
            ((effect_bounds.origin.x + effect_bounds.size.width - active_origin.x) * scale).ceil();
        let bottom =
            ((effect_bounds.origin.y + effect_bounds.size.height - active_origin.y) * scale).ceil();
        let width_f = right - left;
        let height_f = bottom - top;
        let limit = self.device.limits().max_texture_dimension_2d;
        if width_f <= 0. || height_f <= 0. {
            return Ok(None);
        }
        if width_f > limit as f32 || height_f > limit as f32 {
            return Err(RendererError::OffscreenTargetTooLarge {
                width: width_f.min(u32::MAX as f32) as u32,
                height: height_f.min(u32::MAX as f32) as u32,
                limit,
            });
        }
        Ok(Some(CachedEffect {
            origin: Offset::new(
                active_origin.x + left / scale,
                active_origin.y + top / scale,
            ),
            width: width_f as u32,
            height: height_f as u32,
        }))
    }

    /// Blurs the offscreen cache entry of `layer` (`source`, the group's
    /// content) into the effect cache entry of `layer`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_blur(
        &mut self,
        scale: f32,
        layer: incular_painting::LayerId,
        generation: u64,
        source_bounds: Rect,
        blur: GaussianBlur,
        source: CachedSource,
        for_shadow: bool,
    ) -> Result<Option<CachedEffect>, RendererError> {
        let Some(CachedEffect {
            origin: effect_origin,
            width,
            height,
        }) = self.blur_effect_placement(source_bounds, blur, scale)?
        else {
            return Ok(None);
        };
        let sigma_x_physical = normalize_sigma(blur.sigma_x * scale);
        let sigma_y_physical = normalize_sigma(blur.sigma_y * scale);
        let downsample_factor =
            choose_blur_downsample_factor(sigma_x_physical.max(sigma_y_physical));
        let frame = self.counters.frames;
        if let Some(entry) = self.effect_cache.get_mut(&layer)
            && entry.source_generation == generation
            && entry.width == width
            && entry.height == height
            && entry.scale_factor_bits == scale.to_bits()
            && entry.sigma_x_bits == sigma_x_physical.to_bits()
            && entry.sigma_y_bits == sigma_y_physical.to_bits()
            && entry.downsample_factor == downsample_factor
            && entry.target.format == self.window_gpu.config.format
            && entry.device_generation == self.device_generation
        {
            entry.last_used_frame = frame;
            self.counters.blur_cache_hits += 1;
            self.counters.effect_stage_cache_hits += 1;
            if for_shadow {
                self.counters.drop_shadow_blur_reuses += 1;
            }
            return Ok(Some(CachedEffect {
                origin: effect_origin,
                width,
                height,
            }));
        }
        let source_view = self
            .offscreen_cache
            .get(&layer)
            .expect("source cache for effect")
            .target
            .color_view
            .clone();
        // The effect's own pixels are the common frame: the blur reads and
        // writes all of them, and the source lies inside, offset by the
        // margin.
        let effect = PixelRect::full(width, height);
        let job = BlurJob {
            source: &source_view,
            source_origin: [
                (source.origin.x - effect_origin.x) * scale,
                (source.origin.y - effect_origin.y) * scale,
            ],
            source_size: [source.width as f32, source.height as f32],
            input: effect,
            output: effect,
            sigma: [sigma_x_physical, sigma_y_physical],
            downsample_factor,
        };
        let mut passes = self.begin_effect_passes("incular blur passes");
        let blurred = self.encode_blur(&mut passes, &job);
        self.submit_effect_passes(passes);
        let (final_target, temps) = blurred?;
        for temp in temps {
            self.recycle_effect_temp(temp);
        }
        self.counters.effect_stage_cache_misses += 1;
        self.counters.effect_stage_rerenders += 1;
        self.store_blur_effect(
            layer,
            final_target,
            scale,
            generation,
            [sigma_x_physical, sigma_y_physical],
            downsample_factor,
        );
        Ok(Some(CachedEffect {
            origin: effect_origin,
            width,
            height,
        }))
    }

    /// Replaces the effect cache entry of `layer` with `target`, a blur's
    /// result.
    pub(super) fn store_blur_effect(
        &mut self,
        layer: incular_painting::LayerId,
        target: OffscreenTarget,
        scale: f32,
        generation: u64,
        sigma: [f32; 2],
        downsample_factor: u32,
    ) {
        if let Some(previous) = self.effect_cache.remove(&layer) {
            self.remove_effect_cache_bytes(previous.bytes);
            self.offscreen_target_pool.recycle(previous.target);
        }
        self.counters.blur_cache_misses += 1;
        let bind_group =
            self.create_composite_bind_group(&target, "incular retained blur bind group");
        let bytes = target.bytes();
        let (width, height) = (target.width, target.height);
        self.effect_cache.insert(
            layer,
            EffectCacheEntry {
                target,
                bind_group,
                width,
                height,
                scale_factor_bits: scale.to_bits(),
                source_generation: generation,
                sigma_x_bits: sigma[0].to_bits(),
                sigma_y_bits: sigma[1].to_bits(),
                matrix_bits: [0; 20],
                device_generation: self.device_generation,
                last_used_frame: self.counters.frames,
                bytes,
                downsample_factor,
            },
        );
        self.add_effect_cache_bytes(bytes);
    }

    /// Records the passes of `job` into `passes`: at full resolution a
    /// horizontal pass over the output columns and every input row, then a
    /// vertical pass into the output; downsampled, a box downsample of the
    /// input, both passes at low resolution and a bilinear upsample into the
    /// output. Returns the output target and the temporaries to recycle
    /// once the passes are submitted.
    pub(super) fn encode_blur(
        &mut self,
        passes: &mut EffectPasses,
        job: &BlurJob<'_>,
    ) -> Result<(OffscreenTarget, Vec<OffscreenTarget>), RendererError> {
        let BlurJob {
            source,
            source_origin,
            source_size,
            input,
            output,
            sigma,
            downsample_factor: factor,
        } = *job;
        let final_target = self.take_effect_temp(output.width, output.height);
        self.counters.blur_output_texels += output.area();
        if factor <= 1 {
            let kernel_x = self.blur_kernel(sigma[0], 1);
            let kernel_y = self.blur_kernel(sigma[1], 1);
            let horizontal = self.take_effect_temp(output.width, input.height);
            self.record_effect_pass(
                passes,
                source,
                &horizontal.color_view,
                &blur_params(
                    [
                        source_origin[0] - output.x as f32,
                        source_origin[1] - input.y as f32,
                    ],
                    source_size,
                    [output.width as f32, input.height as f32],
                    [1., 0.],
                    &kernel_x,
                ),
                true,
            )?;
            self.counters.blur_horizontal_passes += 1;
            self.record_effect_pass(
                passes,
                &horizontal.color_view,
                &final_target.color_view,
                &blur_params(
                    [0., input.y as f32 - output.y as f32],
                    [output.width as f32, input.height as f32],
                    [output.width as f32, output.height as f32],
                    [0., 1.],
                    &kernel_y,
                ),
                true,
            )?;
            self.counters.blur_vertical_passes += 1;
            return Ok((final_target, vec![horizontal]));
        }
        let scale = factor as f32;
        let low_width = input.width.div_ceil(factor).max(1);
        let low_height = input.height.div_ceil(factor).max(1);
        let low_size = [low_width as f32, low_height as f32];
        let downsample = self.take_effect_temp(low_width, low_height);
        let horizontal = self.take_effect_temp(low_width, low_height);
        let vertical = self.take_effect_temp(low_width, low_height);
        self.record_effect_pass(
            passes,
            source,
            &downsample.color_view,
            &resample_params(
                [
                    source_origin[0] - input.x as f32,
                    source_origin[1] - input.y as f32,
                ],
                source_size,
                low_size,
                [scale, scale],
                3,
            ),
            false,
        )?;
        self.counters.blur_downsample_passes += 1;
        let kernel_x = self.blur_kernel(sigma[0] / scale, factor);
        let kernel_y = self.blur_kernel(sigma[1] / scale, factor);
        self.record_effect_pass(
            passes,
            &downsample.color_view,
            &horizontal.color_view,
            &blur_params([0., 0.], low_size, low_size, [1., 0.], &kernel_x),
            true,
        )?;
        self.counters.blur_horizontal_passes += 1;
        self.record_effect_pass(
            passes,
            &horizontal.color_view,
            &vertical.color_view,
            &blur_params([0., 0.], low_size, low_size, [0., 1.], &kernel_y),
            true,
        )?;
        self.counters.blur_vertical_passes += 1;
        self.record_effect_pass(
            passes,
            &vertical.color_view,
            &final_target.color_view,
            &resample_params(
                [
                    (input.x as f32 - output.x as f32) / scale,
                    (input.y as f32 - output.y as f32) / scale,
                ],
                low_size,
                [output.width as f32, output.height as f32],
                [1. / scale, 1. / scale],
                1,
            ),
            false,
        )?;
        self.counters.blur_upsample_passes += 1;
        Ok((final_target, vec![downsample, horizontal, vertical]))
    }

    pub(super) fn take_effect_temp(&mut self, width: u32, height: u32) -> OffscreenTarget {
        if let Some(target) =
            self.offscreen_target_pool
                .take(width, height, self.config.format, true)
        {
            self.counters.effect_texture_pool_hits += 1;
            target
        } else {
            self.counters.effect_texture_pool_misses += 1;
            self.counters.effect_texture_creations += 1;
            self.create_offscreen_target(width, height, "incular transient blur target")
        }
    }
    pub(super) fn recycle_effect_temp(&mut self, target: OffscreenTarget) {
        self.offscreen_target_pool.recycle(target);
    }
    pub(super) fn run_color_matrix_pass(
        &mut self,
        source: &wgpu::TextureView,
        destination: &wgpu::TextureView,
        filter: ColorFilter,
    ) -> Result<(), RendererError> {
        let params = GpuColorMatrixParams {
            matrix: filter.to_matrix(),
        };
        self.queue
            .write_buffer(&self.color_matrix_params, 0, bytemuck::bytes_of(&params));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("incular color matrix pass bind group"),
            layout: &self.color_matrix_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.color_matrix_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.color_matrix_params.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("incular color matrix pass encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("incular color matrix pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: destination,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(self.color_matrix_pipeline.get()?);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, self.mesh.slice(..));
            pass.draw(0..6, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }
    pub(super) fn add_effect_cache_bytes(&mut self, bytes: usize) {
        self.counters.offscreen_cached_bytes =
            self.counters.offscreen_cached_bytes.saturating_add(bytes);
        self.counters.effect_cached_bytes = self.counters.effect_cached_bytes.saturating_add(bytes);
        self.counters.effect_chain_cached_bytes = self.counters.effect_cached_bytes;
        self.counters.effect_peak_bytes = self
            .counters
            .effect_peak_bytes
            .max(self.counters.effect_cached_bytes);
        self.counters.offscreen_peak_cached_bytes = self
            .counters
            .offscreen_peak_cached_bytes
            .max(self.counters.offscreen_cached_bytes);
    }
    pub(super) fn remove_effect_cache_bytes(&mut self, bytes: usize) {
        self.counters.offscreen_cached_bytes =
            self.counters.offscreen_cached_bytes.saturating_sub(bytes);
        self.counters.effect_cached_bytes = self.counters.effect_cached_bytes.saturating_sub(bytes);
        self.counters.effect_chain_cached_bytes = self.counters.effect_cached_bytes;
    }
    /// The kernel of a blur of `sigma` pixels: the centre weight, then the
    /// taps on one side merged into bilinear pairs (Impeller's
    /// `LerpHackKernelSamples`): taps k and k + 1 become one linear fetch at
    /// `(k w_k + (k+1) w_{k+1}) / (w_k + w_{k+1})` weighing their sum.
    pub(super) fn blur_kernel(&mut self, sigma: f32, downsample_factor: u32) -> BlurKernel {
        let sigma = quantize_blur_sigma(sigma);
        let key = BlurKernelKey {
            sigma_bits: sigma.to_bits(),
            downsample_factor,
        };
        if let Some(kernel) = self.blur_kernel_cache.get(&key) {
            self.counters.blur_kernel_cache_hits += 1;
            return kernel.clone();
        }
        let kernel = paired_blur_kernel(sigma);
        self.blur_kernel_cache.insert(key, kernel.clone());
        self.counters.blur_kernel_cache_misses += 1;
        self.counters.blur_kernel_uploads += 1;
        kernel
    }

    /// Starts recording effect passes into one encoder.
    pub(super) fn begin_effect_passes(&self, label: &'static str) -> EffectPasses {
        EffectPasses {
            encoder: self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) }),
            slot: 0,
        }
    }

    /// Submits the recorded passes.
    pub(super) fn submit_effect_passes(&mut self, passes: EffectPasses) {
        self.queue.submit(Some(passes.encoder.finish()));
        self.counters.effect_pass_submissions += 1;
    }

    /// Records one full-target effect pass with `params`, in the next slot
    /// of the parameter ring: the blur pipeline, or the resample pipeline.
    /// A full ring submits what it holds and starts over.
    pub(super) fn record_effect_pass(
        &mut self,
        passes: &mut EffectPasses,
        source: &wgpu::TextureView,
        destination: &wgpu::TextureView,
        params: &GpuBlurParams,
        blur: bool,
    ) -> Result<(), RendererError> {
        if passes.slot == EFFECT_PARAM_SLOTS {
            // Parameter writes land before the next submission, so the
            // recorded passes go before their slots are rewritten.
            let full = std::mem::replace(passes, self.begin_effect_passes("incular effect passes"));
            self.submit_effect_passes(full);
        }
        let offset = passes.slot * self.effect_param_stride;
        passes.slot += 1;
        self.queue
            .write_buffer(&self.blur_params, offset, bytemuck::bytes_of(params));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("incular transient effect bind group"),
            layout: &self.blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.blur_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.blur_params,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<GpuBlurParams>() as u64),
                    }),
                },
            ],
        });
        let pipeline = if blur {
            self.blur_pipeline.get()?
        } else {
            self.resample_pipeline.get()?
        };
        {
            let mut pass = passes
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("incular gaussian pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: destination,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[offset as u32]);
            pass.set_vertex_buffer(0, self.mesh.slice(..));
            pass.draw(0..6, 0..1);
        }
        self.counters.offscreen_render_passes += 1;
        Ok(())
    }

    /// Runs one effect pass with `params` on its own submission.
    pub(super) fn submit_effect_pass(
        &mut self,
        source: &wgpu::TextureView,
        destination: &wgpu::TextureView,
        params: &GpuBlurParams,
        blur: bool,
    ) -> Result<(), RendererError> {
        let mut passes = self.begin_effect_passes("incular transient effect pass");
        let recorded = self.record_effect_pass(&mut passes, source, destination, params, blur);
        self.submit_effect_passes(passes);
        recorded
    }
}

/// The paired kernel of a blur of `sigma` pixels, as the blur shader reads
/// it (see [`WgpuRenderer::blur_kernel`]).
pub(crate) fn paired_blur_kernel(sigma: f32) -> BlurKernel {
    let full = gaussian_kernel_weights(sigma);
    let radius = ((full.len().saturating_sub(1)) / 2).min(MAX_BLUR_RADIUS);
    let center = full.len() / 2;
    let mut weights = [0.; BLUR_WEIGHT_SLOTS];
    weights[0] = full[center];
    let mut pairs = 0;
    for first in (1..=radius).step_by(2) {
        let near = full[center + first];
        let far = if first < radius {
            full[center + first + 1]
        } else {
            0.
        };
        let total = near + far;
        let offset = if total > 0. {
            first as f32 + far / total
        } else {
            first as f32
        };
        pairs += 1;
        weights[pairs * 4..pairs * 4 + 4].copy_from_slice(&[first as f32, offset, near, far]);
    }
    BlurKernel {
        pairs: pairs as u32,
        weights,
    }
}

/// Effect passes being recorded into one encoder, and the next free slot of
/// the parameter ring.
pub(super) struct EffectPasses {
    pub(super) encoder: wgpu::CommandEncoder,
    slot: u64,
}

/// One Gaussian blur, placed in a common frame of device pixels: where its
/// source texture lies, which pixels it reads and which it produces.
/// Outside the source the image is transparent.
#[derive(Clone, Copy)]
pub(super) struct BlurJob<'a> {
    pub(super) source: &'a wgpu::TextureView,
    /// Frame position of the source's first texel.
    pub(super) source_origin: [f32; 2],
    pub(super) source_size: [f32; 2],
    /// The pixels the blur reads: `output` grown by the kernel's reach.
    pub(super) input: PixelRect,
    /// The pixels the blur produces, the size of its result.
    pub(super) output: PixelRect,
    /// Physical sigma per axis.
    pub(super) sigma: [f32; 2],
    pub(super) downsample_factor: u32,
}

fn blur_params(
    source_origin: [f32; 2],
    source_size: [f32; 2],
    output_size: [f32; 2],
    direction: [f32; 2],
    kernel: &BlurKernel,
) -> GpuBlurParams {
    GpuBlurParams {
        source_origin,
        source_size,
        output_size,
        direction,
        radius: kernel.pairs,
        mode: 0,
        _padding: [0; 2],
        weights: kernel.weights,
    }
}

/// Resample parameters: mode 1 samples bilinearly at `direction` source
/// pixels per output pixel, mode 3 box-averages `direction` x `direction`
/// source blocks.
fn resample_params(
    source_origin: [f32; 2],
    source_size: [f32; 2],
    output_size: [f32; 2],
    direction: [f32; 2],
    mode: u32,
) -> GpuBlurParams {
    GpuBlurParams {
        source_origin,
        source_size,
        output_size,
        direction,
        radius: 0,
        mode,
        _padding: [0; 2],
        weights: [0.; BLUR_WEIGHT_SLOTS],
    }
}
