use super::*;

/// Staging capacity a stream keeps however little the last upload used.
const STAGING_RETAIN_BYTES: usize = 256 * 1024;

impl WgpuRenderer {
    pub(super) fn ensure_rectangle_capacity(&mut self, required: usize) -> bool {
        if required <= self.instance_capacity {
            return false;
        }
        self.instance_capacity = required.next_power_of_two();
        self.instances = create_instance_buffer(&self.device, self.instance_capacity);
        self.counters.buffer_reallocations += 1;
        true
    }
    pub(super) fn ensure_glyph_capacity(&mut self, required: usize) -> bool {
        if required <= self.glyph_instance_capacity {
            return false;
        }
        self.glyph_instance_capacity = required.next_power_of_two();
        self.glyph_instances = create_glyph_buffer(&self.device, self.glyph_instance_capacity);
        self.counters.glyph_instance_buffer_reallocations += 1;
        true
    }
    pub(super) fn ensure_image_capacity(&mut self, required: usize) -> bool {
        if required <= self.image_instance_capacity {
            return false;
        }
        self.image_instance_capacity = required.next_power_of_two();
        self.image_instances = create_image_buffer(&self.device, self.image_instance_capacity);
        true
    }
    pub(super) fn ensure_composite_capacity(&mut self, required: usize) -> bool {
        if required <= self.composite_instance_capacity {
            return false;
        }
        self.composite_instance_capacity = required.next_power_of_two();
        self.composite_instances =
            create_composite_buffer(&self.device, self.composite_instance_capacity);
        true
    }
    /// One counted queue write for per-frame instance data.
    pub(super) fn write_counted(&mut self, buffer: wgpu::Buffer, offset: u64, data: &[u8]) {
        self.counters.buffer_uploads += 1;
        self.counters.buffer_upload_bytes += data.len() as u64;
        self.queue.write_buffer(&buffer, offset, data);
    }
    pub(super) fn upload_instance_data(
        &mut self,
        batches: BatchSpan<'_>,
        scale: f32,
        width: u32,
        height: u32,
    ) {
        // Draw order stays in `batches`. Pack each vertex stream in that same
        // order, then make one queue write per stream instead of one allocation
        // per draw batch. The staging vectors are retained across uploads, so
        // a steady frame packs without allocating.
        let mut streams = std::mem::take(&mut self.instance_staging);
        for stream in &mut streams {
            stream.clear();
        }
        for batch in batches.iter() {
            match batch {
                DrawBatch::Rectangles { clip, instances } if *clip != ClipState::Empty => {
                    for instance in instances {
                        streams[0].extend_from_slice(bytemuck::bytes_of(&logical_instance(
                            *instance,
                            width as f32,
                            height as f32,
                            scale,
                        )));
                    }
                }
                DrawBatch::Glyphs {
                    clip, instances, ..
                } if *clip != ClipState::Empty => {
                    streams[1].extend_from_slice(bytemuck::cast_slice(instances));
                }
                DrawBatch::Images {
                    clip, instances, ..
                } if *clip != ClipState::Empty => {
                    streams[2].extend_from_slice(bytemuck::cast_slice(instances));
                }
                DrawBatch::RoundedRects {
                    clip, instances, ..
                } if *clip != ClipState::Empty => {
                    streams[3].extend_from_slice(bytemuck::cast_slice(instances));
                }
                DrawBatch::StencilRRect { clip, instance, .. } if *clip != ClipState::Empty => {
                    streams[3].extend_from_slice(bytemuck::bytes_of(instance));
                }
                DrawBatch::Path { clip, instance, .. }
                | DrawBatch::StencilPath { clip, instance, .. }
                    if *clip != ClipState::Empty =>
                {
                    streams[4].extend_from_slice(bytemuck::bytes_of(instance));
                }
                DrawBatch::Offscreen { clip, instance, .. }
                | DrawBatch::Filtered { clip, instance, .. }
                | DrawBatch::Shadow { clip, instance, .. }
                | DrawBatch::Blend { clip, instance, .. }
                    if *clip != ClipState::Empty =>
                {
                    streams[5].extend_from_slice(bytemuck::bytes_of(instance));
                }
                _ => {}
            }
        }
        for (index, data) in streams.iter_mut().enumerate() {
            if !data.is_empty() {
                let buffer = match index {
                    0 => &self.instances,
                    1 => &self.glyph_instances,
                    2 => &self.image_instances,
                    3 => &self.rounded_rect_instances,
                    4 => &self.path_instances,
                    _ => &self.composite_instances,
                }
                .clone();
                self.write_counted(buffer, 0, data);
            }
            self.instance_staging_peak[index] = self.instance_staging_peak[index].max(data.len());
        }
        self.instance_staging = streams;
    }
    /// Releases staging a frame no longer needs. Offscreen groups and the
    /// main pass share the staging vectors, so the largest upload of the
    /// frame decides, not the last one: one unusually large frame must not
    /// pin its staging forever, and a small group upload must not shrink
    /// what the main pass packs again next frame.
    pub(super) fn trim_instance_staging(&mut self) {
        for (data, peak) in self
            .instance_staging
            .iter_mut()
            .zip(&mut self.instance_staging_peak)
        {
            if data.capacity() > STAGING_RETAIN_BYTES && data.capacity() / 4 > *peak {
                data.clear();
                data.shrink_to((*peak).max(STAGING_RETAIN_BYTES));
            }
            *peak = 0;
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn encode_batches(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        stencil_view: Option<&wgpu::TextureView>,
        batches: BatchSpan<'_>,
        width: u32,
        height: u32,
        scale: f32,
        clear: wgpu::Color,
        destination_view: Option<&wgpu::TextureView>,
        load_existing: bool,
    ) -> Result<(u32, u32), RendererError> {
        // All windows share this upload batch. Submit it before any draw (or
        // pipeline error), so cached masks cannot be observed before their copy.
        self.shared.flush_glyph_uploads();
        let stenciled = stencil_view.is_some();
        // Fail before opening a profiler query/render pass. Successful lookups
        // below are then cache hits, and a failed contract leaves no open query.
        for batch in batches.iter() {
            let pipeline = match batch {
                DrawBatch::Rectangles { .. } => &self.rectangle_pipeline,
                DrawBatch::Glyphs { .. } => &self.text_pipeline,
                DrawBatch::Images { .. } => &self.image_pipeline,
                DrawBatch::RoundedRects { .. } => &self.rounded_rect_pipeline,
                DrawBatch::Path { .. } => &self.path_pipeline,
                DrawBatch::StencilRRect {
                    increment: true, ..
                } => &self.stencil_rrect_increment_pipeline,
                DrawBatch::StencilRRect { .. } => &self.stencil_rrect_decrement_pipeline,
                DrawBatch::StencilPath {
                    increment: true, ..
                } => &self.stencil_path_increment_pipeline,
                DrawBatch::StencilPath { .. } => &self.stencil_path_decrement_pipeline,
                DrawBatch::Offscreen { .. }
                | DrawBatch::Filtered { .. }
                | DrawBatch::Shadow { .. } => &self.composite_pipeline,
                DrawBatch::Blend { mode, .. } if mode.requires_destination_read() => {
                    &self.blend_pipeline
                }
                DrawBatch::Blend { mode, .. } => {
                    let Some(pipeline) = self.fixed_blend_pipelines.get(mode.code() as usize)
                    else {
                        continue;
                    };
                    pipeline
                }
                DrawBatch::Backdrop { .. } => continue,
            };
            pipeline.get_for(stenciled)?;
        }
        let mut draw_calls = 0_u32;
        let mut text_draw_calls = 0_u32;
        let mut rectangle_offset = 0_u64;
        let mut glyph_offset = 0_u64;
        let mut image_offset = 0_u64;
        let mut rounded_offset = 0_u64;
        let mut path_offset = 0_u64;
        let mut composite_offset = 0_u64;
        // wgpu-profiler samples only the top-level compositor pass. Offscreen
        // effect passes remain attributable through their renderer counters
        // instead of consuming profiler scopes for every intermediate target.
        #[cfg(feature = "gpu-profiling")]
        let profiler_query = if self.profiler_next_pass {
            self.profiler_next_pass = false;
            Some(
                self.gpu_profiler
                    .begin_pass_query("incular retained compositor pass", encoder),
            )
        } else {
            None
        };
        #[cfg(not(feature = "gpu-profiling"))]
        let timestamp_writes = None;
        #[cfg(feature = "gpu-profiling")]
        let timestamp_writes = profiler_query
            .as_ref()
            .and_then(wgpu_profiler::GpuProfilerQuery::render_pass_timestamp_writes);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("incular retained compositor pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: if load_existing {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(clear)
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: stencil_view.map(|view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }
            }),
            timestamp_writes,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        // Consecutive batches mostly share their scissor, stencil reference
        // and the unit quad, so each is set only when it changes: a command
        // the pass does not record is one wgpu does not validate or replay.
        let mut bound_scissor = None;
        let mut bound_reference = None;
        let mut quad_bound = false;
        for batch in batches.iter() {
            let clip = match batch {
                DrawBatch::Rectangles { clip, .. }
                | DrawBatch::Glyphs { clip, .. }
                | DrawBatch::Images { clip, .. }
                | DrawBatch::RoundedRects { clip, .. }
                | DrawBatch::Path { clip, .. }
                | DrawBatch::StencilRRect { clip, .. }
                | DrawBatch::StencilPath { clip, .. }
                | DrawBatch::Offscreen { clip, .. }
                | DrawBatch::Filtered { clip, .. }
                | DrawBatch::Shadow { clip, .. }
                | DrawBatch::Blend { clip, .. } => *clip,
                // Executed by the destination path before this pass.
                DrawBatch::Backdrop { .. } => ClipState::Empty,
            };
            if clip == ClipState::Empty {
                continue;
            }
            let Some(scissor) = scissor_rect(clip, width, height, scale) else {
                match batch {
                    DrawBatch::Rectangles { instances, .. } => {
                        rectangle_offset +=
                            (instances.len() * std::mem::size_of::<GpuInstance>()) as u64;
                    }
                    DrawBatch::Glyphs { instances, .. } => {
                        glyph_offset +=
                            (instances.len() * std::mem::size_of::<GpuGlyphInstance>()) as u64;
                    }
                    DrawBatch::Images { instances, .. } => {
                        image_offset +=
                            (instances.len() * std::mem::size_of::<GpuImageInstance>()) as u64;
                    }
                    DrawBatch::RoundedRects { instances, .. } => {
                        rounded_offset +=
                            (instances.len() * std::mem::size_of::<GpuRRectInstance>()) as u64;
                    }
                    DrawBatch::Path { .. } | DrawBatch::StencilPath { .. } => {
                        path_offset += std::mem::size_of::<GpuPathInstance>() as u64;
                    }
                    DrawBatch::StencilRRect { .. } => {
                        rounded_offset += std::mem::size_of::<GpuRRectInstance>() as u64;
                    }
                    DrawBatch::Offscreen { .. } => {
                        composite_offset += std::mem::size_of::<GpuCompositeInstance>() as u64;
                    }
                    DrawBatch::Filtered { .. } | DrawBatch::Shadow { .. } => {
                        composite_offset += std::mem::size_of::<GpuCompositeInstance>() as u64;
                    }
                    DrawBatch::Blend { .. } => {
                        composite_offset += std::mem::size_of::<GpuCompositeInstance>() as u64;
                    }
                    DrawBatch::Backdrop { .. } => {}
                }
                self.counters.clip_culled_draws += 1;
                continue;
            };
            if bound_scissor != Some(scissor) {
                bound_scissor = Some(scissor);
                pass.set_scissor_rect(scissor[0], scissor[1], scissor[2], scissor[3]);
            }
            if stenciled {
                let reference = u32::from(stencil_depth(clip));
                if bound_reference != Some(reference) {
                    bound_reference = Some(reference);
                    pass.set_stencil_reference(reference);
                }
            }
            match batch {
                DrawBatch::Rectangles { instances, .. } if !instances.is_empty() => {
                    let start = rectangle_offset;
                    rectangle_offset +=
                        (instances.len() * std::mem::size_of::<GpuInstance>()) as u64;
                    pass.set_pipeline(self.rectangle_pipeline.get_for(stenciled)?);
                    if !quad_bound {
                        quad_bound = true;
                        pass.set_vertex_buffer(0, self.mesh.slice(..));
                    }
                    pass.set_vertex_buffer(1, self.instances.slice(start..rectangle_offset));
                    pass.draw(0..6, 0..instances.len() as u32);
                    draw_calls += 1;
                }
                DrawBatch::Glyphs {
                    page, instances, ..
                } if !instances.is_empty() => {
                    let start = glyph_offset;
                    glyph_offset +=
                        (instances.len() * std::mem::size_of::<GpuGlyphInstance>()) as u64;
                    let Some(atlas_page) = self.atlas_pages.get(*page) else {
                        continue;
                    };
                    pass.set_pipeline(self.text_pipeline.get_for(stenciled)?);
                    pass.set_bind_group(0, &atlas_page.bind_group, &[]);
                    if !quad_bound {
                        quad_bound = true;
                        pass.set_vertex_buffer(0, self.mesh.slice(..));
                    }
                    pass.set_vertex_buffer(1, self.glyph_instances.slice(start..glyph_offset));
                    pass.draw(0..6, 0..instances.len() as u32);
                    draw_calls += 1;
                    text_draw_calls += 1;
                }
                DrawBatch::Images {
                    image,
                    sampling,
                    instances,
                    ..
                } if !instances.is_empty() => {
                    let start = image_offset;
                    image_offset +=
                        (instances.len() * std::mem::size_of::<GpuImageInstance>()) as u64;
                    let Some(bind_group) = self.image_bind_group(*image, *sampling) else {
                        continue;
                    };
                    pass.set_pipeline(self.image_pipeline.get_for(stenciled)?);
                    pass.set_bind_group(0, bind_group, &[]);
                    if !quad_bound {
                        quad_bound = true;
                        pass.set_vertex_buffer(0, self.mesh.slice(..));
                    }
                    pass.set_vertex_buffer(1, self.image_instances.slice(start..image_offset));
                    pass.draw(0..6, 0..instances.len() as u32);
                    draw_calls += 1;
                    self.counters.image_draw_calls += 1;
                }
                DrawBatch::RoundedRects {
                    gradient,
                    instances,
                    ..
                } if !instances.is_empty() => {
                    let start = rounded_offset;
                    rounded_offset +=
                        (instances.len() * std::mem::size_of::<GpuRRectInstance>()) as u64;
                    pass.set_pipeline(self.rounded_rect_pipeline.get_for(stenciled)?);
                    pass.set_bind_group(0, self.gradient_bind_group(*gradient), &[]);
                    if !quad_bound {
                        quad_bound = true;
                        pass.set_vertex_buffer(0, self.mesh.slice(..));
                    }
                    pass.set_vertex_buffer(
                        1,
                        self.rounded_rect_instances.slice(start..rounded_offset),
                    );
                    pass.draw(0..6, 0..instances.len() as u32);
                    draw_calls += 1;
                }
                DrawBatch::Path { key, gradient, .. } => {
                    let start = path_offset;
                    path_offset += std::mem::size_of::<GpuPathInstance>() as u64;
                    if let Some(mesh) = self.gpu_path_cache.get_mut(key) {
                        mesh.last_used_frame = self.counters.frames;
                    } else {
                        continue;
                    }
                    let gradient_bind_group = self.gradient_bind_group(*gradient);
                    let Some(mesh) = self.gpu_path_cache.get(key) else {
                        continue;
                    };
                    pass.set_pipeline(self.path_pipeline.get_for(stenciled)?);
                    pass.set_bind_group(0, gradient_bind_group, &[]);
                    quad_bound = false;
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_vertex_buffer(1, self.path_instances.slice(start..path_offset));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                    draw_calls += 1;
                    self.counters.path_draw_calls += 1;
                    self.counters.path_triangles += u64::from(mesh.index_count / 3);
                }
                DrawBatch::StencilRRect { increment, .. } => {
                    let start = rounded_offset;
                    rounded_offset += std::mem::size_of::<GpuRRectInstance>() as u64;
                    pass.set_pipeline(if *increment {
                        self.stencil_rrect_increment_pipeline.get_for(stenciled)?
                    } else {
                        self.stencil_rrect_decrement_pipeline.get_for(stenciled)?
                    });
                    pass.set_bind_group(0, self.gradient_bind_group(None), &[]);
                    if !quad_bound {
                        quad_bound = true;
                        pass.set_vertex_buffer(0, self.mesh.slice(..));
                    }
                    pass.set_vertex_buffer(
                        1,
                        self.rounded_rect_instances.slice(start..rounded_offset),
                    );
                    pass.draw(0..6, 0..1);
                    draw_calls += 1;
                    self.counters.stencil_mask_draws += 1;
                }
                DrawBatch::StencilPath { key, increment, .. } => {
                    let start = path_offset;
                    path_offset += std::mem::size_of::<GpuPathInstance>() as u64;
                    let Some(mesh) = self.gpu_path_cache.get(key) else {
                        continue;
                    };
                    pass.set_pipeline(if *increment {
                        self.stencil_path_increment_pipeline.get_for(stenciled)?
                    } else {
                        self.stencil_path_decrement_pipeline.get_for(stenciled)?
                    });
                    pass.set_bind_group(0, self.gradient_bind_group(None), &[]);
                    // The stencil mask is the path's own mesh: its indices
                    // address `mesh.vertices`, never the shared unit quad.
                    quad_bound = false;
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_vertex_buffer(1, self.path_instances.slice(start..path_offset));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                    draw_calls += 1;
                    self.counters.stencil_mask_draws += 1;
                }
                DrawBatch::Offscreen {
                    layer, instance, ..
                } => {
                    let start = composite_offset;
                    composite_offset += std::mem::size_of::<GpuCompositeInstance>() as u64;
                    if let Some(entry) = self.offscreen_cache.get_mut(layer) {
                        entry.last_used_frame = self.counters.frames;
                    } else {
                        continue;
                    }
                    let Some(entry) = self.offscreen_cache.get(layer) else {
                        continue;
                    };
                    pass.set_pipeline(self.composite_pipeline.get_for(stenciled)?);
                    pass.set_bind_group(0, &entry.bind_group, &[]);
                    if !quad_bound {
                        quad_bound = true;
                        pass.set_vertex_buffer(0, self.mesh.slice(..));
                    }
                    pass.set_vertex_buffer(
                        1,
                        self.composite_instances.slice(
                            start..start + std::mem::size_of::<GpuCompositeInstance>() as u64,
                        ),
                    );
                    pass.draw(0..6, 0..1);
                    let _ = instance;
                    draw_calls += 1;
                    self.counters.offscreen_composite_draws += 1;
                }
                DrawBatch::Filtered {
                    layer, instance, ..
                } => {
                    let start = composite_offset;
                    composite_offset += std::mem::size_of::<GpuCompositeInstance>() as u64;
                    if let Some(entry) = self.effect_cache.get_mut(layer) {
                        entry.last_used_frame = self.counters.frames;
                    } else {
                        continue;
                    }
                    let Some(entry) = self.effect_cache.get(layer) else {
                        continue;
                    };
                    pass.set_pipeline(self.composite_pipeline.get_for(stenciled)?);
                    pass.set_bind_group(0, &entry.bind_group, &[]);
                    if !quad_bound {
                        quad_bound = true;
                        pass.set_vertex_buffer(0, self.mesh.slice(..));
                    }
                    pass.set_vertex_buffer(
                        1,
                        self.composite_instances.slice(
                            start..start + std::mem::size_of::<GpuCompositeInstance>() as u64,
                        ),
                    );
                    pass.draw(0..6, 0..1);
                    let _ = instance;
                    draw_calls += 1;
                    self.counters.offscreen_composite_draws += 1;
                }
                DrawBatch::Shadow {
                    layer, instance, ..
                } => {
                    let start = composite_offset;
                    composite_offset += std::mem::size_of::<GpuCompositeInstance>() as u64;
                    if let Some(entry) = self.effect_cache.get_mut(layer) {
                        entry.last_used_frame = self.counters.frames;
                    } else if let Some(entry) = self.offscreen_cache.get_mut(layer) {
                        entry.last_used_frame = self.counters.frames;
                    } else {
                        continue;
                    }
                    pass.set_pipeline(self.composite_pipeline.get_for(stenciled)?);
                    if let Some(entry) = self.effect_cache.get(layer) {
                        pass.set_bind_group(0, &entry.bind_group, &[]);
                    } else if let Some(entry) = self.offscreen_cache.get(layer) {
                        pass.set_bind_group(0, &entry.bind_group, &[]);
                    } else {
                        continue;
                    }
                    if !quad_bound {
                        quad_bound = true;
                        pass.set_vertex_buffer(0, self.mesh.slice(..));
                    }
                    pass.set_vertex_buffer(
                        1,
                        self.composite_instances.slice(
                            start..start + std::mem::size_of::<GpuCompositeInstance>() as u64,
                        ),
                    );
                    pass.draw(0..6, 0..1);
                    let _ = instance;
                    draw_calls += 1;
                    self.counters.drop_shadow_composites += 1;
                }
                DrawBatch::Blend {
                    layer,
                    instance,
                    mode,
                    ..
                } => {
                    let start = composite_offset;
                    composite_offset += std::mem::size_of::<GpuCompositeInstance>() as u64;
                    let Some(source) = self.offscreen_cache.get_mut(layer) else {
                        continue;
                    };
                    source.last_used_frame = self.counters.frames;
                    if !mode.requires_destination_read() {
                        let Some(pipeline) = self.fixed_blend_pipelines.get(mode.code() as usize)
                        else {
                            continue;
                        };
                        pass.set_pipeline(pipeline.get_for(stenciled)?);
                        pass.set_bind_group(0, &source.bind_group, &[]);
                        if !quad_bound {
                            quad_bound = true;
                            pass.set_vertex_buffer(0, self.mesh.slice(..));
                        }
                        pass.set_vertex_buffer(
                            1,
                            self.composite_instances.slice(
                                start..start + std::mem::size_of::<GpuCompositeInstance>() as u64,
                            ),
                        );
                        pass.draw(0..6, 0..1);
                        self.counters.blend_fixed_function_draws += 1;
                        draw_calls += 1;
                        let _ = instance;
                        continue;
                    }
                    let Some(destination_view) = destination_view else {
                        continue;
                    };
                    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("incular destination blend bind group"),
                        layout: &self.blend_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(
                                    &source.target.color_view,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(destination_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::Sampler(&self.blend_sampler),
                            },
                        ],
                    });
                    pass.set_pipeline(self.blend_pipeline.get_for(stenciled)?);
                    pass.set_bind_group(0, &bind_group, &[]);
                    if !quad_bound {
                        quad_bound = true;
                        pass.set_vertex_buffer(0, self.mesh.slice(..));
                    }
                    pass.set_vertex_buffer(
                        1,
                        self.composite_instances.slice(
                            start..start + std::mem::size_of::<GpuCompositeInstance>() as u64,
                        ),
                    );
                    pass.draw(0..6, 0..1);
                    self.counters.blend_destination_read_draws += 1;
                    if !mode.requires_destination_read() {
                        self.counters.blend_fixed_function_draws += 1;
                    }
                    draw_calls += 1;
                    let _ = instance;
                }
                _ => {}
            }
        }
        drop(pass);
        #[cfg(feature = "gpu-profiling")]
        if let Some(query) = profiler_query {
            self.gpu_profiler.end_query(encoder, query);
            self.gpu_profiler.resolve_queries(encoder);
        }
        Ok((draw_calls, text_draw_calls))
    }
}
