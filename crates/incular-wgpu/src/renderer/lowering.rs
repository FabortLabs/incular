use super::*;

enum AnalyticClip {
    Rounded([f32; 4]),
    Oval,
}

impl AnalyticClip {
    fn shader_parts(self) -> ([f32; 4], u32) {
        match self {
            Self::Rounded(radii) if radii == [0.; 4] => (radii, 2),
            Self::Rounded(radii) => (radii, 0),
            Self::Oval => ([0.; 4], 1),
        }
    }
}

impl WgpuRenderer {
    fn push_hard_round_rect_mask(
        &mut self,
        parent: ClipState,
        rrect: RRect,
        transform: Transform,
        scale: f32,
        batches: &mut Vec<DrawBatch>,
    ) -> Result<(ClipState, Option<ClipMask>), RendererError> {
        if rrect.rect.size.width <= 0. || rrect.rect.size.height <= 0. {
            return Ok((ClipState::Empty, None));
        }
        // The stencil uses the full affine, so its scissor only bounds it.
        let scissor = intersect_clip_with_rect(
            parent,
            ClipRect::hard(transform.transform_rect_bbox(rrect.rect)),
        );
        if scissor == ClipState::Empty {
            return Ok((ClipState::Empty, None));
        }
        let depth = stencil_depth(parent);
        if depth == MAX_STENCIL_CLIP_DEPTH {
            return Err(RendererError::StencilDepthOverflow);
        }
        let instance = rrect_instance(
            rrect,
            &Brush::Solid(Color::TRANSPARENT),
            transform,
            scale,
            self.target_width as f32,
            self.target_height as f32,
        );
        batches.push(DrawBatch::StencilRRect {
            clip: parent,
            instance,
            increment: true,
        });
        self.counters.stencil_depth_max = self.counters.stencil_depth_max.max(u64::from(depth + 1));
        Ok((
            with_stencil_depth(scissor, depth + 1),
            Some(ClipMask::RRect(instance)),
        ))
    }

    #[cfg(not(incular_d3d11))]
    fn push_analytic_clip(
        &mut self,
        parent: ClipState,
        rect: Rect,
        scale: f32,
        shape: AnalyticClip,
    ) -> ClipState {
        if !(rect.size.width > 0.
            && rect.size.height > 0.
            && rect.size.width.is_finite()
            && rect.size.height.is_finite()
            && rect.origin.x.is_finite()
            && rect.origin.y.is_finite())
        {
            return ClipState::Empty;
        }
        let guard = 1. / scale;
        let bounds = Rect::from_origin_size(
            rect.origin - Offset::new(guard, guard),
            Size::new(rect.size.width + 2. * guard, rect.size.height + 2. * guard),
        );
        if intersect_clip_with_rect(parent, ClipRect::hard(bounds)) == ClipState::Empty {
            return ClipState::Empty;
        }
        let (radii, shape) = shape.shader_parts();
        self.soft_clips.push(GpuSoftClip {
            rect: [
                rect.origin.x * scale,
                rect.origin.y * scale,
                rect.size.width * scale,
                rect.size.height * scale,
            ],
            radii,
            link: [parent.soft_index(), shape, 0, 0],
        });
        intersect_clip_with_rect(
            parent,
            ClipRect {
                rect: bounds,
                soft: self.soft_clips.len() as u32,
            },
        )
    }

    pub(super) fn lower_commands(
        &mut self,
        commands: &[PaintCommand],
        scale: f32,
        initial_transform: Transform,
        initial_clip: ClipState,
    ) -> Result<Vec<DrawBatch>, RendererError> {
        let mut batches = Vec::new();
        self.lower_commands_into(
            commands,
            scale,
            initial_transform,
            initial_clip,
            &mut batches,
        )?;
        Ok(batches)
    }

    /// Appends the batches of `commands` to `batches`, so nested groups lower
    /// straight into their parent's list instead of a temporary per group.
    fn lower_commands_into(
        &mut self,
        commands: &[PaintCommand],
        scale: f32,
        initial_transform: Transform,
        initial_clip: ClipState,
        batches: &mut Vec<DrawBatch>,
    ) -> Result<(), RendererError> {
        let mut transforms = vec![initial_transform];
        let mut clips = vec![initial_clip];
        let mut clip_masks: Vec<Option<ClipMask>> = vec![None];
        #[cfg(not(incular_d3d11))]
        let mut has_destination_effect = None;
        let mut command_index = 0_usize;
        while command_index < commands.len() {
            let command = &commands[command_index];
            let transform = *transforms.last().expect("transform stack");
            // Only the push and pop arms change the clip stack, and they read
            // it before they change it.
            let clip = *clips.last().expect("clip stack");
            let translation = transform.translation_offset();
            match command {
                PaintCommand::PushSurfacePartition { .. } | PaintCommand::PopSurfacePartition => {}
                PaintCommand::Rect { rect, color } => {
                    if transform.is_translation()
                        && !device_pixel_aligned(translated_rect(*rect, translation), scale)
                    {
                        // Fractional device edges are antialiased analytically,
                        // as Skia does, instead of snapping to pixel centres.
                        append_rrect(
                            batches,
                            clip,
                            None,
                            rrect_instance(
                                RRect {
                                    rect: *rect,
                                    radii: incular_rendering::CornerRadii::ZERO,
                                },
                                &Brush::Solid(*color),
                                transform,
                                scale,
                                self.target_width as f32,
                                self.target_height as f32,
                            ),
                        );
                    } else if transform.is_translation() {
                        append_rectangle(
                            batches,
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
                        // The path is the rectangle at the origin, placed by
                        // the transform, so its mesh is reused every frame
                        // and by every rectangle of that size.
                        let path = self.shape_path(false, rect.size);
                        self.append_path(
                            batches,
                            clip,
                            &path,
                            PathMeshKind::Fill(FillRule::NonZero),
                            &Brush::Solid(*color),
                            PathPlacement {
                                transform: transform.then(Transform::translation(rect.origin)),
                                scale,
                            },
                        );
                    }
                }
                // Common solid rounded primitives retain painter order even on
                // renderers that do not yet select the analytic pipeline.
                PaintCommand::RRect { rrect, brush } => append_rrect(
                    batches,
                    clip,
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
                        batches,
                        clip,
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
                    batches,
                    clip,
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
                    batches,
                    clip,
                    path,
                    stroke_mesh_kind(*stroke),
                    brush,
                    PathPlacement { transform, scale },
                ),
                PaintCommand::GlyphRun { run, color } => {
                    // Encoding discards a batch with an empty device scissor.
                    // Skip it before rasterizing glyphs or sizing instance buffers.
                    if scissor_rect(clip, self.target_width, self.target_height, scale).is_none() {
                        command_index += 1;
                        continue;
                    }
                    // Long scrolled documents paint every line; without this
                    // each off-screen glyph still took an atlas lookup and an
                    // instance the scissor then discarded.
                    if glyph_run_outside_clip(run, transform, clip) {
                        self.counters.glyph_runs_culled += 1;
                        command_index += 1;
                        continue;
                    }
                    let glyph_surface = GlyphSurface {
                        transform,
                        width: self.target_width as f32,
                        height: self.target_height as f32,
                        scale,
                    };
                    let paint = GlyphRunPaint::new(*color, glyph_surface);
                    // Bindings to build once the run is resolved: the
                    // resolver holds the shared lock that building one takes.
                    let mut stale_pages: Vec<(u16, u64)> = Vec::new();
                    let mut checked_page = None;
                    let mut resolver = self.shared.resolve_glyph_run(run, f64::from(scale));
                    for glyph in run.glyphs.iter() {
                        let origin = glyph_device_origin(run, glyph.offset, glyph_surface);
                        let Some((entry, fresh)) = resolver.resolve(
                            glyph.id,
                            glyph_phase(origin),
                            &mut self.frame_pinned_glyph_pages,
                        ) else {
                            self.counters.glyphs_skipped += 1;
                            continue;
                        };
                        // Spaces have a placement but no mask and no quad.
                        if entry.width == 0 || entry.height == 0 {
                            continue;
                        }
                        if fresh {
                            // The shared atlas staged the fresh mask with its
                            // cache admission; `encode_batches` flushes it.
                            self.counters.texture_upload_bytes +=
                                u64::from(entry.width + ATLAS_PADDING * 2)
                                    * u64::from(entry.height + ATLAS_PADDING * 2);
                        }
                        // The texture is device-shared, while this window
                        // still needs its own pipeline-compatible bind group
                        // for each atlas page it draws from.
                        let page = (entry.page, entry.generation);
                        if checked_page != Some(page) {
                            checked_page = Some(page);
                            if self.atlas_pages.generation(entry.page) != Some(entry.generation)
                                && !stale_pages.contains(&page)
                            {
                                stale_pages.push(page);
                            }
                        }
                        append_glyph(
                            batches,
                            clip,
                            entry.page,
                            glyph_instance(run, glyph.offset, entry, &paint, glyph_surface, origin),
                        );
                    }
                    drop(resolver);
                    // The run pinned these pages as it resolved them, so their
                    // generations are still the ones recorded.
                    for (page, generation) in stale_pages {
                        self.ensure_atlas_page(page, generation);
                    }
                }
                PaintCommand::Image {
                    image,
                    source,
                    destination,
                    sampling,
                } => {
                    if clip == ClipState::Empty {
                        command_index += 1;
                        continue;
                    }
                    self.ensure_gpu_image(image)?;
                    append_image(
                        batches,
                        clip,
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
                        )
                        .with_opacity(self.content_opacity),
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
                    let parent_clip = clip;
                    let normalized = normalize_opacity(*alpha);
                    command_index = end.saturating_add(1);
                    if normalized <= 0. {
                        self.counters.opacity_zero_fast_paths += 1;
                        continue;
                    }
                    if normalized >= 1. {
                        self.counters.opacity_one_fast_paths += 1;
                        self.lower_commands_into(
                            &commands[start..end],
                            scale,
                            transform,
                            parent_clip,
                            batches,
                        )?;
                        continue;
                    }
                    if opacity_folds_into_content(&commands[start..end]) {
                        // A lone image fades in its own draw: no layer texture.
                        self.counters.opacity_content_folds += 1;
                        let inherited = self.content_opacity;
                        self.content_opacity = inherited * normalized;
                        let result = self.lower_commands_into(
                            &commands[start..end],
                            scale,
                            transform,
                            parent_clip,
                            batches,
                        );
                        self.content_opacity = inherited;
                        result?;
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
                    let parent_clip = clip;
                    command_index = end.saturating_add(1);
                    let blur = GaussianBlur::new(blur.sigma_x, blur.sigma_y);
                    if blur.sigma_x <= f32::EPSILON && blur.sigma_y <= f32::EPSILON {
                        self.lower_commands_into(
                            &commands[start..end],
                            scale,
                            transform,
                            parent_clip,
                            batches,
                        )?;
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
                    let parent_clip = clip;
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
                    let parent_clip = clip;
                    command_index = end.saturating_add(1);
                    if filter.is_identity() {
                        self.lower_commands_into(
                            &commands[start..end],
                            scale,
                            transform,
                            parent_clip,
                            batches,
                        )?;
                    } else if let Some(inner_commands) =
                        commands.get(start..end).filter(|child| !child.is_empty())
                        && let Some(PaintCommand::PushColorFilter {
                            filter: inner_filter,
                            generation: inner_generation,
                            ..
                        }) = inner_commands.first()
                        && let Ok(inner_end) = find_effect_end(inner_commands, 0)
                        && inner_end + 1 == inner_commands.len()
                        && let Some(combined) = inner_filter.try_then(*filter)
                    {
                        // Nested color matrices are adjacent single-input
                        // stages.  Remove the inner boundary and compose
                        // `inner` followed by `outer`; no blur or shadow is
                        // crossed, so painter order remains exact. A stage
                        // on the outer filter cannot fuse after the inner
                        // matrix, so that pair keeps both passes.
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
                    let parent_clip = clip;
                    command_index = end.saturating_add(1);
                    if *mode == BlendMode::SrcOver {
                        // SrcOver is the ordinary painter operation. Keep it
                        // on the direct path instead of isolating a source
                        // texture solely to apply the default blend mode.
                        self.lower_commands_into(
                            &commands[start..end],
                            scale,
                            transform,
                            parent_clip,
                            batches,
                        )?;
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
                command @ PaintCommand::PushBackdropFilter {
                    layer,
                    blur,
                    enabled,
                    matrix,
                    image_filter,
                    backdrop_key,
                    ..
                } => {
                    let end = find_effect_end(commands, command_index)?;
                    let start = command_index + 1;
                    let parent_clip = clip;
                    command_index = end.saturating_add(1);
                    if parent_clip == ClipState::Empty {
                        continue;
                    }
                    // Blend modes other than srcOver blend the stage's whole
                    // layer and still fail the frame.
                    if let Some(error) = crate::diagnostics::unsupported_effect(command) {
                        return Err(error);
                    }
                    if incular_painting::backdrop_stage_is_active(
                        *blur,
                        *enabled,
                        *matrix,
                        image_filter.as_deref(),
                    ) {
                        let stage = self.lower_backdrop(
                            *layer,
                            *blur,
                            *matrix,
                            image_filter.as_deref(),
                            *backdrop_key,
                            transform,
                            scale,
                            parent_clip,
                        )?;
                        batches.extend(stage);
                    }
                    self.lower_commands_into(
                        &commands[start..end],
                        scale,
                        transform,
                        parent_clip,
                        batches,
                    )?;
                    continue;
                }
                command @ PaintCommand::PushShaderMask {
                    layer,
                    shader,
                    blend_mode,
                    mask_size,
                    mask_transform,
                    generation,
                    bounds,
                } => {
                    let end = find_effect_end(commands, command_index)?;
                    let start = command_index + 1;
                    let parent_clip = clip;
                    command_index = end.saturating_add(1);
                    if parent_clip == ClipState::Empty {
                        continue;
                    }
                    if let Some(error) = crate::diagnostics::unsupported_effect(command) {
                        return Err(error);
                    }
                    if let Some(batch) = self.lower_shader_mask_group(
                        LayerGroup {
                            commands: &commands[start..end],
                            scale,
                            layer: *layer,
                            generation: *generation,
                            bounds: *bounds,
                            parent_clip,
                            translation,
                        },
                        &ShaderMaskStage {
                            shader,
                            blend_mode: *blend_mode,
                            mask_size: *mask_size,
                            mask_transform: **mask_transform,
                        },
                    )? {
                        batches.push(batch);
                    }
                    continue;
                }
                PaintCommand::PushMatrixFilter {
                    layer,
                    matrix,
                    stage,
                    sampling,
                    generation,
                    bounds,
                    filtered_bounds,
                } => {
                    let end = find_effect_end(commands, command_index)?;
                    let start = command_index + 1;
                    let parent_clip = clip;
                    command_index = end.saturating_add(1);
                    if parent_clip == ClipState::Empty {
                        continue;
                    }
                    if let Some(batch) = self.lower_matrix_filter_group(
                        LayerGroup {
                            commands: &commands[start..end],
                            scale,
                            layer: *layer,
                            generation: *generation,
                            bounds: *bounds,
                            parent_clip,
                            translation,
                        },
                        MatrixFilterStage {
                            matrix: *matrix,
                            stage: **stage,
                            sampling: *sampling,
                            filtered_bounds: *filtered_bounds,
                        },
                    )? {
                        batches.push(batch);
                    }
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
                    let [_, b, c, _, _, _] = transform.to_kurbo().as_coeffs();
                    let (combined, mask) = if b == 0. && c == 0. {
                        let next = ClipRect::hard(transform.transform_rect_bbox(*rect));
                        (intersect_clip_with_rect(clip, next), None)
                    } else {
                        // Rotation and shear turn the rectangle into a
                        // parallelogram, not its axis-aligned bounding box.
                        self.push_hard_round_rect_mask(
                            clip,
                            RRect::new(*rect, incular_rendering::CornerRadii::ZERO),
                            transform,
                            scale,
                            batches,
                        )?
                    };
                    clips.push(combined);
                    clip_masks.push(mask);
                    self.counters.clip_rect_pushes += 1;
                }
                PaintCommand::PushClipRRect {
                    rrect,
                    coverage,
                    isolation,
                } => {
                    let [a, b, c, d, _, _] = transform.to_kurbo().as_coeffs();
                    #[cfg(not(incular_d3d11))]
                    let uniform_scale = a as f32;
                    #[cfg(not(incular_d3d11))]
                    if *coverage == incular_painting::ClipCoverage::AntiAlias
                        && b == 0. && c == 0. && a == d
                        && uniform_scale.is_finite() && uniform_scale > 0.
                        // Destination-reading effects use a replacement
                        // pass rather than a source color/alpha blend.
                        // Keep hard containment until they can mix their
                        // filtered destination with fractional coverage.
                        && !*has_destination_effect.get_or_insert_with(|| commands.iter().any(
                            |command| matches!(command, PaintCommand::PushBlend { .. } | PaintCommand::PushBackdropFilter { .. })
                        ))
                    {
                        let parent = *clips.last().expect("clip stack");
                        let state = if rrect.rect.size.width <= 0. || rrect.rect.size.height <= 0. {
                            ClipState::Empty
                        } else {
                            let radii = rrect.radii.normalized(rrect.rect.size);
                            let physical_scale = uniform_scale * scale;
                            let radii = [
                                radii.top_left,
                                radii.top_right,
                                radii.bottom_right,
                                radii.bottom_left,
                            ]
                            .map(|radius| radius * physical_scale);
                            self.push_analytic_clip(
                                parent,
                                transform.transform_rect_bbox(rrect.rect),
                                scale,
                                AnalyticClip::Rounded(radii),
                            )
                        };
                        clips.push(state);
                        clip_masks.push(None);
                        self.counters.clip_rrect_pushes += 1;
                        command_index += 1;
                        continue;
                    }
                    if *coverage == incular_painting::ClipCoverage::AntiAliasWithSaveLayer
                        && let Some(isolation) = isolation
                        && (a, b, c, d) == (1., 0., 0., 1.)
                    {
                        let end = crate::compositor::find_clip_end(commands, command_index)?;
                        let start = command_index + 1;
                        command_index = end + 1;
                        let bounds = Rect::from_origin_size(
                            self.target_origin + transform.transform_point(rrect.rect.origin),
                            rrect.rect.size,
                        );
                        let radii = rrect_instance(
                            *rrect,
                            &Brush::Solid(Color::TRANSPARENT),
                            transform,
                            scale,
                            self.target_width as f32,
                            self.target_height as f32,
                        )
                        .radii;
                        batches.extend(self.lower_isolated_clip(
                            LayerGroup {
                                commands: &commands[start..end],
                                scale,
                                layer: isolation.layer,
                                generation: isolation.generation,
                                bounds,
                                parent_clip: clip,
                                translation,
                            },
                            AnalyticClip::Rounded(radii),
                        )?);
                        continue;
                    }
                    let (combined, mask) =
                        self.push_hard_round_rect_mask(clip, *rrect, transform, scale, batches)?;
                    clips.push(combined);
                    clip_masks.push(mask);
                    self.counters.clip_rrect_pushes += 1;
                }
                PaintCommand::PushClipOval {
                    rect,
                    coverage,
                    isolation,
                } => {
                    let parent = *clips.last().expect("clip stack");
                    let [a, b, c, d, _, _] = transform.to_kurbo().as_coeffs();
                    if *coverage == incular_painting::ClipCoverage::AntiAliasWithSaveLayer
                        && let Some(isolation) = isolation
                        && (a, b, c, d) == (1., 0., 0., 1.)
                    {
                        let end = crate::compositor::find_clip_end(commands, command_index)?;
                        let start = command_index + 1;
                        command_index = end + 1;
                        let bounds = Rect::from_origin_size(
                            self.target_origin + transform.transform_point(rect.origin),
                            rect.size,
                        );
                        batches.extend(self.lower_isolated_clip(
                            LayerGroup {
                                commands: &commands[start..end],
                                scale,
                                layer: isolation.layer,
                                generation: isolation.generation,
                                bounds,
                                parent_clip: parent,
                                translation,
                            },
                            AnalyticClip::Oval,
                        )?);
                        continue;
                    }
                    #[cfg(not(incular_d3d11))]
                    {
                        if *coverage == incular_painting::ClipCoverage::AntiAlias
                            && b == 0.
                            && c == 0.
                            && !*has_destination_effect.get_or_insert_with(|| {
                                commands.iter().any(|command| {
                                    matches!(
                                        command,
                                        PaintCommand::PushBlend { .. }
                                            | PaintCommand::PushBackdropFilter { .. }
                                    )
                                })
                            })
                        {
                            let state = if rect.size.width <= 0. || rect.size.height <= 0. {
                                ClipState::Empty
                            } else {
                                self.push_analytic_clip(
                                    parent,
                                    transform.transform_rect_bbox(*rect),
                                    scale,
                                    AnalyticClip::Oval,
                                )
                            };
                            clips.push(state);
                            clip_masks.push(None);
                            command_index += 1;
                            continue;
                        }
                    }
                    let depth = stencil_depth(parent);
                    if depth == MAX_STENCIL_CLIP_DEPTH {
                        return Err(RendererError::StencilDepthOverflow);
                    }
                    // The oval at the origin, placed by the transform: a
                    // scrolling or repeated oval keeps one retained mesh.
                    let path = self.shape_path(true, rect.size);
                    let transform = transform.then(Transform::translation(rect.origin));
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
                PaintCommand::PushClipPath {
                    path, fill_rule, ..
                } => {
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
                        ClipRect::hard(transform.transform_rect_bbox(bounds)),
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
        Ok(())
    }
    fn lower_isolated_clip(
        &mut self,
        group: LayerGroup<'_>,
        shape: AnalyticClip,
    ) -> Result<Option<DrawBatch>, RendererError> {
        let scale = group.scale;
        let rect = group.bounds;
        let visible = Rect::from_origin_size(
            self.target_origin,
            Size::new(
                self.target_width as f32 / scale,
                self.target_height as f32 / scale,
            ),
        );
        let Some(bounds) = rect.intersection(visible) else {
            return Ok(None);
        };
        let Some(DrawBatch::Offscreen {
            clip,
            layer,
            mut instance,
        }) = self.lower_opacity_group(LayerGroup { bounds, ..group }, 1.)?
        else {
            return Ok(None);
        };
        // Mask the composed source once in parent framebuffer pixels. Keep the
        // shape in its original coordinates even if the target was cropped.
        let local = rect.origin - self.target_origin;
        let (radii, shape) = shape.shader_parts();
        instance.clip_rect = [
            local.x * scale,
            local.y * scale,
            rect.size.width * scale,
            rect.size.height * scale,
        ];
        instance.clip_radii = radii;
        instance.options[1] = shape as f32;
        Ok(Some(DrawBatch::Offscreen {
            clip,
            layer,
            instance,
        }))
    }

    pub(super) fn lower_opacity_group(
        &mut self,
        group: LayerGroup<'_>,
        alpha: f32,
    ) -> Result<Option<DrawBatch>, RendererError> {
        let (parent_clip, layer, scale) = (group.parent_clip, group.layer, group.scale);
        let active = Placed::new(self.target_origin, self.target_width, self.target_height);
        let Some((target, hit)) =
            self.render_offscreen_group(group, "incular retained opacity group", None)?
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

    /// Lowers a shader mask the way Flutter's `ShaderMaskLayer` paints it:
    /// the child renders into an isolated group, the mask rectangle is then
    /// drawn into that group with the shader as source and the child as
    /// destination under the mask blend mode, and the result composites
    /// into the parent with srcOver.
    pub(super) fn lower_shader_mask_group(
        &mut self,
        group: LayerGroup<'_>,
        mask: &ShaderMaskStage<'_>,
    ) -> Result<Option<DrawBatch>, RendererError> {
        let (parent_clip, layer, scale) = (group.parent_clip, group.layer, group.scale);
        let active = Placed::new(self.target_origin, self.target_width, self.target_height);
        let Some((target, hit)) =
            self.render_offscreen_group(group, "incular retained shader mask group", Some(mask))?
        else {
            return Ok(None);
        };
        if hit {
            self.counters.effect_stage_cache_hits += 1;
        } else {
            self.counters.effect_stage_cache_misses += 1;
            self.counters.effect_stage_rerenders += 1;
        }
        self.counters.effect_chain_compilations += 1;
        Ok(Some(DrawBatch::Offscreen {
            clip: parent_clip,
            layer,
            instance: composite_instance(
                Placed::new(target.origin, target.width, target.height),
                active,
                scale,
                1.,
            ),
        }))
    }

    /// Renders the mask rectangle filled with the shader into the effect
    /// texture of `layer`, sized like the group target being lowered, and
    /// returns the batch that blends it over the group's child. The blend
    /// is scissored to the mask rectangle, so child pixels outside it stay
    /// unchanged, as with Flutter's `drawRect` of the mask. `None` when the
    /// mask rectangle is empty, which leaves the child unmasked like an
    /// empty Flutter mask rect.
    fn lower_shader_mask_fill(
        &mut self,
        mask: &ShaderMaskStage<'_>,
        scale: f32,
        translation: Offset,
        layer: incular_painting::LayerId,
    ) -> Result<Option<DrawBatch>, RendererError> {
        let size = mask.mask_size;
        if !(size.width.is_finite() && size.height.is_finite())
            || size.width <= 0.
            || size.height <= 0.
        {
            return Ok(None);
        }
        let rect = Rect::from_origin_size(Offset::ZERO, size);
        let initial = Transform::translation(translation);
        let fill = self.lower_commands(
            &[
                PaintCommand::PushTransform {
                    transform: mask.mask_transform,
                },
                PaintCommand::RRect {
                    rrect: RRect {
                        rect,
                        radii: incular_rendering::CornerRadii::ZERO,
                    },
                    brush: mask.shader.clone(),
                },
                PaintCommand::PopTransform,
            ],
            scale,
            initial,
            ClipState::Unbounded,
        )?;
        let mask_rect = initial.then(mask.mask_transform).transform_rect_bbox(rect);
        let (width, height) = (self.target_width, self.target_height);
        if let Some(previous) = self.effect_cache.remove(&layer) {
            self.remove_effect_cache_bytes(previous.bytes);
            self.offscreen_target_pool.recycle(previous.target);
        }
        let target = if let Some(target) =
            self.offscreen_target_pool
                .take(width, height, self.config.format)
        {
            self.counters.effect_texture_pool_hits += 1;
            target
        } else {
            self.counters.effect_texture_pool_misses += 1;
            self.counters.effect_texture_creations += 1;
            self.create_offscreen_target(width, height, "incular shader mask fill")
        };
        self.render_batches_into(
            &fill,
            scale,
            width,
            height,
            &target,
            "incular shader mask fill",
        )?;
        let bind_group =
            self.create_composite_bind_group(&target, "incular shader mask bind group");
        let bytes = target.bytes();
        self.effect_cache.insert(
            layer,
            EffectCacheEntry {
                target,
                bind_group,
                width,
                height,
                scale_factor_bits: scale.to_bits(),
                source_generation: 0,
                sigma_x_bits: 0,
                sigma_y_bits: 0,
                matrix_bits: [0; 20],
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
        self.counters.shader_mask_passes += 1;
        let placed = Placed::new(self.target_origin, width, height);
        Ok(Some(DrawBatch::Blend {
            clip: ClipState::Rect(ClipRect::hard(mask_rect)),
            layer,
            source: BlendSource::Effect,
            mode: mask.blend_mode,
            instance: blend_instance(placed, placed, scale, mask.blend_mode),
        }))
    }

    pub(super) fn lower_source_group(
        &mut self,
        group: LayerGroup<'_>,
    ) -> Result<Option<CachedSource>, RendererError> {
        let base_source = !commands_have_effects(group.commands);
        let Some((source, hit)) =
            self.render_offscreen_group(group, "incular retained effect source", None)?
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
        mask: Option<&ShaderMaskStage<'_>>,
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
        // An isolated group is its own backdrop: its stages never share a
        // capture of another target.
        self.next_backdrop_scope += 1;
        let saved_scope = std::mem::replace(&mut self.backdrop_scope, self.next_backdrop_scope);
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
            mask,
        );
        self.target_width = saved_target.0;
        self.target_height = saved_target.1;
        self.target_origin = saved_target.2;
        self.backdrop_scope = saved_scope;
        self.offscreen_nesting_depth = self.offscreen_nesting_depth.saturating_sub(1);
        Ok(rendered?.then_some((target, false)))
    }

    /// Lowers a group's commands for the current (offscreen) target and, if
    /// they draw anything, renders them into a newly cached target for
    /// `layer`. Returns whether anything was rendered.
    #[expect(
        clippy::too_many_arguments,
        reason = "the offscreen group parameters are forwarded unchanged to lower_commands and the cache entry; bundling them would add a per-call struct"
    )]
    fn render_offscreen_children(
        &mut self,
        commands: &[PaintCommand],
        scale: f32,
        translation: Offset,
        layer: incular_painting::LayerId,
        generation: u64,
        label: &'static str,
        mask: Option<&ShaderMaskStage<'_>>,
    ) -> Result<bool, RendererError> {
        let mut batches = self.lower_commands(
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
        if let Some(mask) = mask
            && let Some(batch) = self.lower_shader_mask_fill(mask, scale, translation, layer)?
        {
            // The mask blends over the child already painted in this
            // group, as Flutter's `ShaderMaskLayer` draws its mask rect
            // into the child's save layer.
            batches.push(batch);
        }
        let (width, height) = (self.target_width, self.target_height);
        let target = if let Some(target) =
            self.offscreen_target_pool
                .take(width, height, self.config.format)
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
                revision: next_texture_revision(),
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
