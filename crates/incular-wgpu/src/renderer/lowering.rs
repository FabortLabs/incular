use super::*;

impl WgpuRenderer {
    pub(super) fn lower_commands(
        &mut self,
        commands: &[PaintCommand],
        scale: f32,
        initial_transform: Transform,
        initial_clip: ClipState,
    ) -> Result<Vec<DrawBatch>, RendererError> {
        let mut batches = Vec::new();
        let mut transforms = vec![initial_transform];
        let mut clips = vec![initial_clip];
        let mut clip_masks: Vec<Option<ClipMask>> = vec![None];
        let mut command_index = 0_usize;
        while command_index < commands.len() {
            let command = &commands[command_index];
            let transform = *transforms.last().expect("transform stack");
            let translation = transform.translation_offset();
            match command {
                PaintCommand::PushSurfacePartition { .. } | PaintCommand::PopSurfacePartition => {}
                PaintCommand::Rect { rect, color } => {
                    let clip = *clips.last().expect("clip stack");
                    if transform.is_translation() {
                        append_rectangle(
                            &mut batches,
                            clip,
                            RectangleInstance {
                                rect: translated_rect(*rect, translation),
                                color: *color,
                            },
                        );
                    } else {
                        // The fast rectangle instance shader is axis-aligned.
                        // Preserve arbitrary retained affine geometry by
                        // routing it through the already-cached Lyon path
                        // pipeline instead of expanding to a bounding box.
                        self.append_path(
                            &mut batches,
                            clip,
                            &Arc::new(rect_path(*rect)),
                            PathMeshKind::Fill(FillRule::NonZero),
                            &Brush::Solid(*color),
                            PathPlacement { transform, scale },
                        );
                    }
                }
                // Common solid rounded primitives retain painter order even on
                // renderers that do not yet select the analytic pipeline.
                PaintCommand::RRect { rrect, brush } => append_rrect(
                    &mut batches,
                    *clips.last().expect("clip stack"),
                    self.ensure_gradient(brush),
                    rrect_instance(
                        *rrect,
                        brush,
                        transform,
                        scale,
                        self.target_width as f32,
                        self.target_height as f32,
                    ),
                ),
                PaintCommand::Border { rrect, border } => {
                    // Inside-aligned border: analytical shader discards the inset.
                    append_rrect(
                        &mut batches,
                        *clips.last().expect("clip stack"),
                        None,
                        border_instance(
                            *rrect,
                            *border,
                            transform,
                            scale,
                            self.target_width as f32,
                            self.target_height as f32,
                        ),
                    );
                }
                PaintCommand::FillPath {
                    path,
                    brush,
                    fill_rule,
                } => self.append_path(
                    &mut batches,
                    *clips.last().expect("clip stack"),
                    path,
                    PathMeshKind::Fill(*fill_rule),
                    brush,
                    PathPlacement { transform, scale },
                ),
                PaintCommand::StrokePath {
                    path,
                    brush,
                    stroke,
                } => self.append_path(
                    &mut batches,
                    *clips.last().expect("clip stack"),
                    path,
                    stroke_mesh_kind(*stroke),
                    brush,
                    PathPlacement { transform, scale },
                ),
                PaintCommand::GlyphRun { run, color } => {
                    if *clips.last().expect("clip stack") == ClipState::Empty {
                        command_index += 1;
                        continue;
                    }
                    for glyph in run.glyphs.iter() {
                        let glyph_surface = GlyphSurface {
                            transform,
                            width: self.target_width as f32,
                            height: self.target_height as f32,
                            scale,
                        };
                        let phase =
                            glyph_phase(glyph_device_origin(run, glyph.offset, glyph_surface));
                        let Some(raster) = self.shared.rasterize_glyph(
                            run,
                            glyph.id,
                            f64::from(scale),
                            phase,
                            &self.frame_pinned_glyph_pages,
                        ) else {
                            self.counters.glyphs_skipped += 1;
                            continue;
                        };
                        if let Some(bitmap) = raster.bitmap.as_deref() {
                            self.upload_glyph(raster.entry, bitmap);
                        } else if raster.entry.width > 0 && raster.entry.height > 0 {
                            // The texture is device-shared, while this window
                            // still needs its own pipeline-compatible bind
                            // group for that atlas page.
                            self.ensure_atlas_page(raster.entry.page, raster.entry.generation);
                        }
                        if raster.entry.width > 0 && raster.entry.height > 0 {
                            append_glyph(
                                &mut batches,
                                *clips.last().expect("clip stack"),
                                raster.entry.page,
                                glyph_instance(
                                    run,
                                    glyph.offset,
                                    raster.entry,
                                    *color,
                                    glyph_surface,
                                ),
                            );
                            // Pin the page for the rest of this frame: shared
                            // eviction must not reuse it while an emitted
                            // batch still references its content, or the
                            // replacement would rebind under those batches at
                            // submit time.
                            self.frame_pinned_glyph_pages.insert(raster.entry.page);
                        }
                    }
                }
                PaintCommand::Image {
                    image,
                    source,
                    destination,
                    sampling,
                } => {
                    if *clips.last().expect("clip stack") == ClipState::Empty {
                        command_index += 1;
                        continue;
                    }
                    self.ensure_gpu_image(image)?;
                    append_image(
                        &mut batches,
                        *clips.last().expect("clip stack"),
                        image.id(),
                        *sampling,
                        image_instance(
                            *destination,
                            *source,
                            image,
                            self.target_width as f32,
                            self.target_height as f32,
                            scale,
                            transform,
                        ),
                    );
                }
                PaintCommand::PushOpacity {
                    layer,
                    alpha,
                    generation,
                    bounds,
                } => {
                    let end = find_opacity_end(commands, command_index)?;
                    let start = command_index + 1;
                    let parent_clip = *clips.last().expect("clip stack");
                    let normalized = normalize_opacity(*alpha);
                    command_index = end.saturating_add(1);
                    if normalized <= 0. {
                        self.counters.opacity_zero_fast_paths += 1;
                        continue;
                    }
                    if normalized >= 1. {
                        self.counters.opacity_one_fast_paths += 1;
                        let child = self.lower_commands(
                            &commands[start..end],
                            scale,
                            transform,
                            parent_clip,
                        )?;
                        batches.extend(child);
                        continue;
                    }
                    if let Some(batch) = self.lower_opacity_group(
                        LayerGroup {
                            commands: &commands[start..end],
                            scale,
                            layer: *layer,
                            generation: *generation,
                            bounds: *bounds,
                            parent_clip,
                            translation,
                        },
                        normalized,
                    )? {
                        batches.push(batch);
                    }
                    continue;
                }
                PaintCommand::PopOpacity => {
                    return Err(RendererError::UnbalancedClipStack);
                }
                PaintCommand::PushBlur {
                    layer,
                    blur,
                    generation,
                    bounds,
                } => {
                    let end = find_effect_end(commands, command_index)?;
                    let start = command_index + 1;
                    let parent_clip = *clips.last().expect("clip stack");
                    command_index = end.saturating_add(1);
                    let blur = GaussianBlur::new(blur.sigma_x, blur.sigma_y);
                    if blur.sigma_x <= f32::EPSILON && blur.sigma_y <= f32::EPSILON {
                        let child = self.lower_commands(
                            &commands[start..end],
                            scale,
                            transform,
                            parent_clip,
                        )?;
                        batches.extend(child);
                    } else if let Some(batch) = self.lower_blur_group(
                        LayerGroup {
                            commands: &commands[start..end],
                            scale,
                            layer: *layer,
                            generation: *generation,
                            bounds: *bounds,
                            parent_clip,
                            translation,
                        },
                        blur,
                    )? {
                        batches.push(batch);
                    }
                    continue;
                }
                PaintCommand::PushDropShadow {
                    layer,
                    shadow,
                    generation,
                    bounds,
                } => {
                    let end = find_effect_end(commands, command_index)?;
                    let start = command_index + 1;
                    let parent_clip = *clips.last().expect("clip stack");
                    command_index = end.saturating_add(1);
                    let shadow = DropShadowEffect::asymmetric(
                        shadow.offset,
                        shadow.sigma_x,
                        shadow.sigma_y,
                        shadow.color,
                    );
                    let child = self.lower_drop_shadow_group(
                        LayerGroup {
                            commands: &commands[start..end],
                            scale,
                            layer: *layer,
                            generation: *generation,
                            bounds: *bounds,
                            parent_clip,
                            translation,
                        },
                        shadow,
                    )?;
                    batches.extend(child);
                    continue;
                }
                PaintCommand::PushColorFilter {
                    layer,
                    filter,
                    generation,
                    bounds,
                } => {
                    let end = find_effect_end(commands, command_index)?;
                    let start = command_index + 1;
                    let parent_clip = *clips.last().expect("clip stack");
                    command_index = end.saturating_add(1);
                    if filter.is_identity() {
                        let child = self.lower_commands(
                            &commands[start..end],
                            scale,
                            transform,
                            parent_clip,
                        )?;
                        batches.extend(child);
                    } else if let Some(inner_commands) =
                        commands.get(start..end).filter(|child| !child.is_empty())
                        && let Some(PaintCommand::PushColorFilter {
                            filter: inner_filter,
                            generation: inner_generation,
                            ..
                        }) = inner_commands.first()
                        && let Ok(inner_end) = find_effect_end(inner_commands, 0)
                        && inner_end + 1 == inner_commands.len()
                    {
                        // Nested color matrices are adjacent single-input
                        // stages.  Remove the inner boundary and compose
                        // `inner` followed by `outer`; no blur or shadow is
                        // crossed, so painter order remains exact.
                        let combined = inner_filter.then(*filter);
                        if let Some(batch) = self.lower_color_filter_group(
                            LayerGroup {
                                commands: &inner_commands[1..inner_end],
                                scale,
                                layer: *layer,
                                // The fused pass's source is the inner
                                // filter's input. Its generation deliberately
                                // excludes the inner matrix itself, so changing
                                // either adjacent matrix rerenders only this
                                // fused stage rather than its source texture.
                                generation: *inner_generation,
                                bounds: *bounds,
                                parent_clip,
                                translation,
                            },
                            combined,
                        )? {
                            batches.push(batch);
                        }
                        self.counters.effect_stage_fusions += 1;
                    } else if let Some(batch) = self.lower_color_filter_group(
                        LayerGroup {
                            commands: &commands[start..end],
                            scale,
                            layer: *layer,
                            generation: *generation,
                            bounds: *bounds,
                            parent_clip,
                            translation,
                        },
                        *filter,
                    )? {
                        batches.push(batch);
                    }
                    continue;
                }
                PaintCommand::PushBlend {
                    layer,
                    mode,
                    generation,
                    bounds,
                } => {
                    let end = find_effect_end(commands, command_index)?;
                    let start = command_index + 1;
                    let parent_clip = *clips.last().expect("clip stack");
                    command_index = end.saturating_add(1);
                    if *mode == BlendMode::SrcOver {
                        // SrcOver is the ordinary painter operation. Keep it
                        // on the direct path instead of isolating a source
                        // texture solely to apply the default blend mode.
                        let child = self.lower_commands(
                            &commands[start..end],
                            scale,
                            transform,
                            parent_clip,
                        )?;
                        batches.extend(child);
                    } else if let Some(batch) = self.lower_blend_group(
                        LayerGroup {
                            commands: &commands[start..end],
                            scale,
                            layer: *layer,
                            generation: *generation,
                            bounds: *bounds,
                            parent_clip,
                            translation,
                        },
                        *mode,
                    )? {
                        batches.push(batch);
                    }
                    continue;
                }
                command @ (PaintCommand::PushShaderMask { .. }
                | PaintCommand::PushBackdropFilter { .. }) => {
                    let end = find_effect_end(commands, command_index)?;
                    let start = command_index + 1;
                    let parent_clip = *clips.last().expect("clip stack");
                    command_index = end.saturating_add(1);
                    if parent_clip == ClipState::Empty {
                        continue;
                    }
                    // Unsupported stages fail the frame here instead of
                    // drawing their children unaffected; supported
                    // passthroughs (disabled and zero-sigma backdrops)
                    // keep the direct path below.
                    if let Some(error) = crate::diagnostics::unsupported_effect(command) {
                        return Err(error);
                    }
                    let child =
                        self.lower_commands(&commands[start..end], scale, transform, parent_clip)?;
                    batches.extend(child);
                    continue;
                }
                PaintCommand::PopEffect => {
                    return Err(RendererError::UnbalancedClipStack);
                }
                PaintCommand::PushTransform {
                    transform: local_transform,
                } => transforms.push(transform.then(*local_transform)),
                PaintCommand::PopTransform => {
                    if transforms.len() > 1 {
                        transforms.pop();
                    }
                }
                PaintCommand::PushClip { rect } => {
                    let next = ClipRect {
                        rect: transform.transform_rect_bbox(*rect),
                    };
                    let combined = match *clips.last().expect("clip stack") {
                        ClipState::Unbounded => ClipState::Rect(next),
                        ClipState::Rect(current) => intersect_rect(current.rect, next.rect)
                            .map(|rect| ClipState::Rect(ClipRect { rect }))
                            .unwrap_or(ClipState::Empty),
                        ClipState::Stencil { rect, depth } => match rect {
                            Some(current) => intersect_rect(current.rect, next.rect)
                                .map(|rect| ClipState::Stencil {
                                    rect: Some(ClipRect { rect }),
                                    depth,
                                })
                                .unwrap_or(ClipState::Empty),
                            None => ClipState::Stencil {
                                rect: Some(next),
                                depth,
                            },
                        },
                        ClipState::Empty => ClipState::Empty,
                    };
                    clips.push(combined);
                    clip_masks.push(None);
                    self.counters.clip_rect_pushes += 1;
                }
                PaintCommand::PushClipRRect { rrect } => {
                    let next = ClipRect {
                        rect: translated_rect(rrect.rect, translation),
                    };
                    let parent = *clips.last().expect("clip stack");
                    let scissor = intersect_clip_with_rect(parent, next);
                    let depth = stencil_depth(parent);
                    if depth == MAX_STENCIL_CLIP_DEPTH {
                        return Err(RendererError::StencilDepthOverflow);
                    }
                    let instance = rrect_instance(
                        *rrect,
                        &Brush::Solid(Color::TRANSPARENT),
                        transform,
                        scale,
                        self.target_width as f32,
                        self.target_height as f32,
                    );
                    if scissor == ClipState::Empty {
                        clips.push(ClipState::Empty);
                        clip_masks.push(None);
                    } else {
                        batches.push(DrawBatch::StencilRRect {
                            clip: parent,
                            instance,
                            increment: true,
                        });
                        clips.push(with_stencil_depth(scissor, depth + 1));
                        clip_masks.push(Some(ClipMask::RRect(instance)));
                        self.counters.stencil_depth_max =
                            self.counters.stencil_depth_max.max(u64::from(depth + 1));
                    }
                    self.counters.clip_rrect_pushes += 1;
                }
                PaintCommand::PushClipOval { rect } => {
                    let parent = *clips.last().expect("clip stack");
                    let depth = stencil_depth(parent);
                    let path = Arc::new(oval_path(*rect));
                    let key = PathMeshKey {
                        path: path.id(),
                        kind: PathMeshKind::Fill(FillRule::NonZero),
                    };
                    if self.ensure_path_mesh(key, &path) {
                        let instance = clip_path_instance(
                            transform,
                            scale,
                            self.target_width as f32,
                            self.target_height as f32,
                        );
                        batches.push(DrawBatch::StencilPath {
                            clip: parent,
                            key,
                            instance,
                            increment: true,
                        });
                        clips.push(with_stencil_depth(parent, depth + 1));
                        clip_masks.push(Some(ClipMask::Path { key, instance }));
                    } else {
                        clips.push(ClipState::Empty);
                        clip_masks.push(None);
                    }
                }
                PaintCommand::PushClipPath { path, fill_rule } => {
                    let parent = *clips.last().expect("clip stack");
                    let depth = stencil_depth(parent);
                    if depth == MAX_STENCIL_CLIP_DEPTH {
                        return Err(RendererError::StencilDepthOverflow);
                    }
                    let Some(bounds) = path.bounds() else {
                        clips.push(ClipState::Empty);
                        clip_masks.push(None);
                        self.counters.clip_path_pushes += 1;
                        command_index += 1;
                        continue;
                    };
                    let scissor = intersect_clip_with_rect(
                        parent,
                        ClipRect {
                            rect: translated_rect(bounds, translation),
                        },
                    );
                    let key = PathMeshKey {
                        path: path.id(),
                        kind: PathMeshKind::Fill(*fill_rule),
                    };
                    if scissor == ClipState::Empty || !self.ensure_path_mesh(key, path) {
                        clips.push(ClipState::Empty);
                        clip_masks.push(None);
                    } else {
                        let instance = clip_path_instance(
                            transform,
                            scale,
                            self.target_width as f32,
                            self.target_height as f32,
                        );
                        batches.push(DrawBatch::StencilPath {
                            clip: parent,
                            key,
                            instance,
                            increment: true,
                        });
                        clips.push(with_stencil_depth(scissor, depth + 1));
                        clip_masks.push(Some(ClipMask::Path { key, instance }));
                        self.counters.stencil_depth_max =
                            self.counters.stencil_depth_max.max(u64::from(depth + 1));
                    }
                    self.counters.clip_path_pushes += 1;
                }
                PaintCommand::PopClip => {
                    if clips.len() > 1 {
                        let child = clips.pop().expect("checked clip stack");
                        let parent = *clips.last().expect("parent clip stack");
                        if let Some(mask) = clip_masks.pop().expect("matching mask stack") {
                            debug_assert_eq!(stencil_depth(child), stencil_depth(parent) + 1);
                            match mask {
                                ClipMask::RRect(instance) => {
                                    batches.push(DrawBatch::StencilRRect {
                                        clip: child,
                                        instance,
                                        increment: false,
                                    })
                                }
                                ClipMask::Path { key, instance } => {
                                    batches.push(DrawBatch::StencilPath {
                                        clip: child,
                                        key,
                                        instance,
                                        increment: false,
                                    })
                                }
                            }
                        }
                    }
                    self.counters.clip_pops += 1;
                }
            }
            command_index += 1;
        }
        if clips.len() != 1 || clip_masks.len() != 1 {
            return Err(RendererError::UnbalancedClipStack);
        }
        Ok(batches)
    }
    pub(super) fn lower_opacity_group(
        &mut self,
        group: LayerGroup<'_>,
        alpha: f32,
    ) -> Result<Option<DrawBatch>, RendererError> {
        let (parent_clip, layer, scale) = (group.parent_clip, group.layer, group.scale);
        let active = Placed::new(self.target_origin, self.target_width, self.target_height);
        let Some((target, hit)) =
            self.render_offscreen_group(group, "incular retained opacity group")?
        else {
            return Ok(None);
        };
        if hit {
            self.counters.offscreen_group_cache_hits += 1;
        } else {
            self.counters.offscreen_group_cache_misses += 1;
            self.counters.offscreen_group_rerenders += 1;
        }
        Ok(Some(DrawBatch::Offscreen {
            clip: parent_clip,
            layer,
            instance: composite_instance(
                Placed::new(target.origin, target.width, target.height),
                active,
                scale,
                alpha,
            ),
        }))
    }

    pub(super) fn lower_source_group(
        &mut self,
        group: LayerGroup<'_>,
    ) -> Result<Option<CachedSource>, RendererError> {
        let base_source = !commands_have_effects(group.commands);
        let Some((source, hit)) =
            self.render_offscreen_group(group, "incular retained effect source")?
        else {
            return Ok(None);
        };
        match (base_source, hit) {
            (true, true) => self.counters.effect_source_cache_hits += 1,
            (false, true) => self.counters.effect_stage_cache_hits += 1,
            (true, false) => {
                self.counters.effect_source_cache_misses += 1;
                self.counters.effect_source_rerenders += 1;
            }
            (false, false) => {
                self.counters.effect_stage_cache_misses += 1;
                self.counters.effect_stage_rerenders += 1;
            }
        }
        Ok(Some(source))
    }

    /// Renders a layer group into its retained offscreen target, reusing the
    /// cached target while its generation, size, scale, format and device
    /// still match. Returns the target placement and whether it was a cache
    /// hit, or `None` when the group covers no pixels.
    fn render_offscreen_group(
        &mut self,
        group: LayerGroup<'_>,
        label: &'static str,
    ) -> Result<Option<(CachedSource, bool)>, RendererError> {
        let LayerGroup {
            commands,
            scale,
            layer,
            generation,
            bounds,
            parent_clip,
            translation,
        } = group;
        let active_origin = self.target_origin;
        if parent_clip == ClipState::Empty
            || !bounds.origin.x.is_finite()
            || !bounds.origin.y.is_finite()
            || !bounds.size.width.is_finite()
            || !bounds.size.height.is_finite()
            || bounds.size.width <= 0.
            || bounds.size.height <= 0.
        {
            return Ok(None);
        }
        let left = ((bounds.origin.x - active_origin.x) * scale).floor();
        let top = ((bounds.origin.y - active_origin.y) * scale).floor();
        let right = ((bounds.origin.x + bounds.size.width - active_origin.x) * scale).ceil();
        let bottom = ((bounds.origin.y + bounds.size.height - active_origin.y) * scale).ceil();
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
        let width = width_f as u32;
        let height = height_f as u32;
        if width == 0 || height == 0 {
            return Ok(None);
        }
        let target = CachedSource {
            origin: Offset::new(
                active_origin.x + left / scale,
                active_origin.y + top / scale,
            ),
            width,
            height,
        };
        let frame = self.counters.frames;
        if let Some(entry) = self.offscreen_cache.get_mut(&layer)
            && entry.generation == generation
            && entry.width == width
            && entry.height == height
            && entry.scale_factor_bits == scale.to_bits()
            && entry.target.format == self.window_gpu.config.format
            && entry.device_generation == self.device_generation
        {
            entry.last_used_frame = frame;
            self.counters.offscreen_texture_reuses += 1;
            return Ok(Some((target, true)));
        }
        if let Some(previous) = self.offscreen_cache.remove(&layer) {
            self.counters.offscreen_cached_bytes = self
                .counters
                .offscreen_cached_bytes
                .saturating_sub(previous.bytes);
            self.offscreen_target_pool.recycle(previous.target);
        }
        let saved_target = (self.target_width, self.target_height, self.target_origin);
        self.target_width = width;
        self.target_height = height;
        self.target_origin = target.origin;
        self.offscreen_nesting_depth += 1;
        self.counters.max_offscreen_nesting_depth = self
            .counters
            .max_offscreen_nesting_depth
            .max(self.offscreen_nesting_depth);
        let rendered = self.render_offscreen_children(
            commands,
            scale,
            translation - (target.origin - active_origin),
            layer,
            generation,
            label,
        );
        self.target_width = saved_target.0;
        self.target_height = saved_target.1;
        self.target_origin = saved_target.2;
        self.offscreen_nesting_depth = self.offscreen_nesting_depth.saturating_sub(1);
        Ok(rendered?.then_some((target, false)))
    }

    /// Lowers a group's commands for the current (offscreen) target and, if
    /// they draw anything, renders them into a newly cached target for
    /// `layer`. Returns whether anything was rendered.
    fn render_offscreen_children(
        &mut self,
        commands: &[PaintCommand],
        scale: f32,
        translation: Offset,
        layer: incular_painting::LayerId,
        generation: u64,
        label: &'static str,
    ) -> Result<bool, RendererError> {
        let batches = self.lower_commands(
            commands,
            scale,
            Transform::translation(translation),
            ClipState::Unbounded,
        )?;
        // Clip-only command streams (or an effect around no child) have no
        // pixels to isolate: allocate no target and submit no pass for them.
        if !batches_have_content(&batches) {
            return Ok(false);
        }
        let (width, height) = (self.target_width, self.target_height);
        let target = if let Some(target) =
            self.offscreen_target_pool
                .take(width, height, self.config.format, true)
        {
            self.counters.offscreen_texture_reuses += 1;
            target
        } else {
            self.create_offscreen_target(width, height, label)
        };
        let bind_group = self.create_composite_bind_group(&target, label);
        let bytes = target.bytes();
        self.offscreen_cache.insert(
            layer,
            OffscreenCacheEntry {
                target,
                bind_group,
                width,
                height,
                scale_factor_bits: scale.to_bits(),
                generation,
                device_generation: self.device_generation,
                last_used_frame: self.counters.frames,
                bytes,
            },
        );
        self.counters.offscreen_cached_bytes =
            self.counters.offscreen_cached_bytes.saturating_add(bytes);
        self.counters.offscreen_peak_cached_bytes = self
            .counters
            .offscreen_peak_cached_bytes
            .max(self.counters.offscreen_cached_bytes);
        self.render_cached_batches(&batches, scale, width, height, layer, label)?;
        Ok(true)
    }
}
