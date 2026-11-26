use super::prelude::*;
use super::*;

pub(crate) fn find_opacity_end(
    commands: &[PaintCommand],
    start: usize,
) -> Result<usize, RendererError> {
    let mut depth = 0_usize;
    for (index, command) in commands.iter().enumerate().skip(start) {
        match command {
            PaintCommand::PushOpacity { .. } => depth += 1,
            PaintCommand::PopOpacity => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    Err(RendererError::UnbalancedClipStack)
}

pub(crate) fn find_clip_end(
    commands: &[PaintCommand],
    start: usize,
) -> Result<usize, RendererError> {
    let mut depth: usize = 0;
    for (index, command) in commands.iter().enumerate().skip(start) {
        match command {
            PaintCommand::PushClip { .. }
            | PaintCommand::PushClipRRect { .. }
            | PaintCommand::PushClipOval { .. }
            | PaintCommand::PushClipPath { .. } => depth += 1,
            PaintCommand::PopClip => {
                depth = depth
                    .checked_sub(1)
                    .ok_or(RendererError::UnbalancedClipStack)?;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    Err(RendererError::UnbalancedClipStack)
}

/// Whether an opacity layer over `commands` can fade its content directly
/// instead of isolating it in a texture: the content is exactly one image,
/// under any transforms and clips and inside any further opacity layers. One
/// image quad covers each pixel once, so scaling its alpha by the layer
/// opacity yields the pixels the isolated group would composite, without a
/// layer-sized texture and its render pass. (Impeller and Skia collapse the
/// same case.) Anything that can overlap itself keeps its isolated group.
pub(crate) fn opacity_folds_into_content(commands: &[PaintCommand]) -> bool {
    let mut images = 0_usize;
    for command in commands {
        match command {
            PaintCommand::Image { .. } => images += 1,
            PaintCommand::PushTransform { .. }
            | PaintCommand::PopTransform
            | PaintCommand::PushClip { .. }
            | PaintCommand::PushClipRRect { .. }
            | PaintCommand::PushClipOval { .. }
            | PaintCommand::PushClipPath { .. }
            | PaintCommand::PopClip
            | PaintCommand::PushOpacity { .. }
            | PaintCommand::PopOpacity => {}
            _ => return false,
        }
    }
    images == 1
}

pub(crate) fn find_effect_end(
    commands: &[PaintCommand],
    start: usize,
) -> Result<usize, RendererError> {
    let mut depth = 0_usize;
    for (index, command) in commands.iter().enumerate().skip(start) {
        match command {
            PaintCommand::PushBlur { .. }
            | PaintCommand::PushDropShadow { .. }
            | PaintCommand::PushColorFilter { .. }
            | PaintCommand::PushBlend { .. }
            | PaintCommand::PushShaderMask { .. }
            | PaintCommand::PushBackdropFilter { .. }
            | PaintCommand::PushMatrixFilter { .. } => depth += 1,
            PaintCommand::PopEffect => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    Err(RendererError::UnbalancedClipStack)
}
pub(crate) fn commands_have_effects(commands: &[PaintCommand]) -> bool {
    commands.iter().any(|command| {
        matches!(
            command,
            PaintCommand::PushOpacity { .. }
                | PaintCommand::PushBlur { .. }
                | PaintCommand::PushDropShadow { .. }
                | PaintCommand::PushColorFilter { .. }
                | PaintCommand::PushBlend { .. }
                | PaintCommand::PushShaderMask { .. }
                | PaintCommand::PushBackdropFilter { .. }
                | PaintCommand::PushMatrixFilter { .. }
        )
    })
}

/// Whether painting `commands` reads what is already painted: a blend
/// mode that samples the destination, or an active backdrop filter.
pub(crate) fn commands_have_destination_blend(commands: &[PaintCommand]) -> bool {
    commands.iter().any(|command| match command {
        PaintCommand::PushBlend { mode, .. } => mode.requires_destination_read(),
        PaintCommand::PushBackdropFilter {
            blur,
            enabled,
            matrix,
            image_filter,
            ..
        } => incular_painting::backdrop_stage_is_active(
            *blur,
            *enabled,
            *matrix,
            image_filter.as_deref(),
        ),
        _ => false,
    })
}

/// Whether a lowered batch must run on the destination-reading path.
pub(crate) fn batch_reads_destination(batch: &DrawBatch) -> bool {
    match batch {
        DrawBatch::Blend { mode, .. } => mode.requires_destination_read(),
        DrawBatch::Backdrop { .. } => true,
        _ => false,
    }
}

pub(crate) fn add_composition_bounds(bounds: &mut Option<Rect>, candidate: Rect) {
    if !candidate.origin.x.is_finite()
        || !candidate.origin.y.is_finite()
        || !candidate.size.width.is_finite()
        || !candidate.size.height.is_finite()
        || candidate.size.width <= 0.
        || candidate.size.height <= 0.
    {
        return;
    }
    *bounds = Some(match bounds.take() {
        Some(current) => {
            let left = current.origin.x.min(candidate.origin.x);
            let top = current.origin.y.min(candidate.origin.y);
            let right = (current.origin.x + current.size.width)
                .max(candidate.origin.x + candidate.size.width);
            let bottom = (current.origin.y + current.size.height)
                .max(candidate.origin.y + candidate.size.height);
            Rect::from_origin_size(
                Offset::new(left, top),
                Size::new(right - left, bottom - top),
            )
        }
        None => candidate,
    });
}

fn outset(rect: Rect, by: Offset) -> Rect {
    Rect::from_origin_size(
        rect.origin - by,
        Size {
            width: rect.size.width + by.x * 2.,
            height: rect.size.height + by.y * 2.,
        },
    )
}

fn uniform_outset(rect: Rect, by: f32) -> Rect {
    outset(rect, Offset::new(by, by))
}

/// Outward displacement performed by the path vertex shader. Ordinary
/// meshes move along their transformed contour miter; hairline meshes
/// instead carry a miter already normalized in device space.
fn path_fringe_reach(transform: Transform, scale: f32) -> Offset {
    let ([a, b, c, d], _) = physical_affine(transform, scale);
    let normalized_area = (a * d - b * c).abs().max(1e-12).sqrt();
    let reach = 0.5 * crate::geometry::MAX_FRINGE_MITER / (normalized_area * scale);
    Offset::new(a.hypot(c) * reach, b.hypot(d) * reach)
}

fn path_composition_bounds(
    path: &Path,
    stroke: Option<Stroke>,
    transform: Transform,
    scale: f32,
) -> Option<Rect> {
    let mut local = path.tight_bounds()?;
    if let Some(stroke) = stroke {
        if !stroke.width.is_finite() || stroke.width < 0. {
            return None;
        }
        let half_width_reach =
            0.5 * stroke_reach_factor(stroke.cap, stroke.join, stroke.miter_limit);
        if stroke.width == 0. {
            transform.inverse()?;
            let device_reach = half_width_reach + 0.5 * crate::geometry::MAX_FRINGE_MITER;
            return Some(uniform_outset(
                transform.transform_rect_bbox(local),
                device_reach / scale,
            ));
        }
        local = uniform_outset(local, half_width_reach * stroke.width);
    }
    Some(outset(
        transform.transform_rect_bbox(local),
        path_fringe_reach(transform, scale),
    ))
}

/// Computes a conservative logical scene scope for destination promotion. It
/// deliberately includes every visible command (and expanded blur/shadow
/// bounds), so presenting a tight target cannot omit content outside a small
/// blend group. The ordinary direct path still uses the complete surface.
pub(crate) fn display_list_composition_bounds(
    commands: &[PaintCommand],
    scale: f32,
) -> Option<Rect> {
    let mut transforms = vec![Transform::IDENTITY];
    let mut bounds = None;
    for command in commands {
        let transform = *transforms.last().expect("root transform");
        match command {
            PaintCommand::Rect { rect, .. } => {
                let world = transform.transform_rect_bbox(*rect);
                let candidate = if !transform.is_translation() {
                    outset(world, path_fringe_reach(transform, scale))
                } else if !device_pixel_aligned(world, scale) {
                    uniform_outset(world, 1. / scale)
                } else {
                    world
                };
                add_composition_bounds(&mut bounds, candidate);
            }
            PaintCommand::HardEdgeRect { rect, .. }
            | PaintCommand::Image {
                destination: rect, ..
            } => add_composition_bounds(&mut bounds, transform.transform_rect_bbox(*rect)),
            PaintCommand::RRect { rrect, .. } | PaintCommand::Border { rrect, .. } => {
                // The analytic quad extends one pixel along each transformed
                // basis vector, at most two pixels along a world axis.
                let world = transform.transform_rect_bbox(rrect.rect);
                add_composition_bounds(&mut bounds, uniform_outset(world, 2. / scale));
            }
            PaintCommand::FillPath { path, .. } => {
                if let Some(candidate) = path_composition_bounds(path, None, transform, scale) {
                    add_composition_bounds(&mut bounds, candidate);
                }
            }
            PaintCommand::StrokePath { path, stroke, .. } => {
                if let Some(candidate) =
                    path_composition_bounds(path, Some(*stroke), transform, scale)
                {
                    add_composition_bounds(&mut bounds, candidate);
                }
            }
            PaintCommand::GlyphRun { run, .. } => {
                if let Some((left, top, right, bottom)) = run
                    .glyphs
                    .iter()
                    .map(|glyph| {
                        let x = run.origin.x + glyph.offset.x;
                        let y = run.origin.y - glyph.offset.y;
                        let end = x + glyph.advance;
                        (x.min(end), y, x.max(end), y)
                    })
                    .reduce(|(left, top, right, bottom), (l, t, r, b)| {
                        (left.min(l), top.min(t), right.max(r), bottom.max(b))
                    })
                {
                    let pens = Rect::from_origin_size(
                        Offset::new(left, top),
                        Size::new(right - left, bottom - top),
                    );
                    let pad = run.font_size.abs() * 2. + 1. / scale;
                    let candidate = transform.transform_rect_bbox(uniform_outset(pens, pad));
                    add_composition_bounds(&mut bounds, candidate);
                }
            }
            // Effect bounds emitted by LayerTree are already in the current
            // world coordinate space. Expand only the effects that can paint
            // outside their source rectangle.
            PaintCommand::PushOpacity { bounds: rect, .. }
            | PaintCommand::PushColorFilter { bounds: rect, .. }
            | PaintCommand::PushBlend { bounds: rect, .. } => {
                add_composition_bounds(&mut bounds, *rect);
            }
            PaintCommand::PushBlur {
                bounds: rect, blur, ..
            } => {
                add_composition_bounds(&mut bounds, blur_bounds(*rect, blur.sigma_x, blur.sigma_y))
            }
            PaintCommand::PushDropShadow {
                bounds: rect,
                shadow,
                ..
            } => add_composition_bounds(
                &mut bounds,
                drop_shadow_bounds(*rect, shadow.offset, shadow.sigma_x, shadow.sigma_y),
            ),
            PaintCommand::PushShaderMask { bounds: rect, .. } => {
                add_composition_bounds(&mut bounds, *rect);
            }
            PaintCommand::PushMatrixFilter {
                filtered_bounds, ..
            } => add_composition_bounds(&mut bounds, *filtered_bounds),
            PaintCommand::PushBackdropFilter {
                bounds: rect,
                blur,
                enabled,
                image_filter,
                ..
            } => {
                let candidate = if *enabled {
                    let (sigma_x, sigma_y) = image_filter
                        .as_deref()
                        .map_or((blur.sigma_x, blur.sigma_y), backdrop_filter_blur_reach);
                    blur_bounds(*rect, sigma_x, sigma_y)
                } else {
                    *rect
                };
                add_composition_bounds(&mut bounds, candidate);
            }
            PaintCommand::PushTransform { transform: local } => {
                transforms.push(transform.then(*local));
            }
            PaintCommand::PopTransform => {
                if transforms.len() > 1 {
                    transforms.pop();
                }
            }
            PaintCommand::PushClip { .. }
            | PaintCommand::PushSurfacePartition { .. }
            | PaintCommand::PopSurfacePartition
            | PaintCommand::PushClipRRect { .. }
            | PaintCommand::PushClipOval { .. }
            | PaintCommand::PushClipPath { .. }
            | PaintCommand::PopClip
            | PaintCommand::PopOpacity
            | PaintCommand::PopEffect => {}
        }
    }
    bounds
}

pub(crate) fn destination_composition_scope(
    list: &DisplayList,
    scale: f32,
    surface_width: u32,
    surface_height: u32,
) -> (Offset, u32, u32) {
    let full = Rect::from_origin_size(
        Offset::ZERO,
        Size::new(surface_width as f32 / scale, surface_height as f32 / scale),
    );
    let scope = display_list_composition_bounds(list.commands(), scale)
        .and_then(|bounds| intersect_rect(bounds, full))
        .unwrap_or(full);
    let left = (scope.origin.x * scale).floor().max(0.) as u32;
    let top = (scope.origin.y * scale).floor().max(0.) as u32;
    let right = ((scope.origin.x + scope.size.width) * scale)
        .ceil()
        .min(surface_width as f32) as u32;
    let bottom = ((scope.origin.y + scope.size.height) * scale)
        .ceil()
        .min(surface_height as f32) as u32;
    if right <= left || bottom <= top {
        return (Offset::ZERO, surface_width.max(1), surface_height.max(1));
    }
    (
        Offset::new(left as f32 / scale, top as f32 / scale),
        right - left,
        bottom - top,
    )
}

pub(crate) fn translated_rect(rect: Rect, offset: Offset) -> Rect {
    Rect::from_origin_size(rect.origin + offset, rect.size)
}
pub(crate) fn intersect_rect(a: Rect, b: Rect) -> Option<Rect> {
    let left = a.origin.x.max(b.origin.x);
    let top = a.origin.y.max(b.origin.y);
    let right = (a.origin.x + a.size.width).min(b.origin.x + b.size.width);
    let bottom = (a.origin.y + a.size.height).min(b.origin.y + b.size.height);
    (right > left && bottom > top).then(|| {
        Rect::from_origin_size(
            Offset::new(left, top),
            Size::new(right - left, bottom - top),
        )
    })
}
/// The scissor rectangle `clip` leaves of a `width` x `height` target, as
/// `[x, y, width, height]` in device pixels; `None` when it shows no pixel.
pub(crate) fn scissor_rect(
    clip: ClipState,
    width: u32,
    height: u32,
    scale: f32,
) -> Option<[u32; 4]> {
    let clip = match clip {
        ClipState::Rect(clip) => clip,
        ClipState::Stencil {
            rect: Some(clip), ..
        } => clip,
        ClipState::Empty => return None,
        ClipState::Unbounded | ClipState::Stencil { rect: None, .. } => {
            return Some([0, 0, width, height]);
        }
    };
    let x0 = (clip.rect.origin.x * scale).floor().max(0.) as u32;
    let y0 = (clip.rect.origin.y * scale).floor().max(0.) as u32;
    let x1 = ((clip.rect.origin.x + clip.rect.size.width) * scale)
        .ceil()
        .max(0.) as u32;
    let y1 = ((clip.rect.origin.y + clip.rect.size.height) * scale)
        .ceil()
        .max(0.) as u32;
    let x0 = x0.min(width);
    let y0 = y0.min(height);
    let scissor_width = x1.min(width).saturating_sub(x0);
    let scissor_height = y1.min(height).saturating_sub(y0);
    if scissor_width == 0 || scissor_height == 0 {
        return None;
    }
    Some([x0, y0, scissor_width, scissor_height])
}
pub(crate) fn normalized_scale(scale: f64) -> f32 {
    if scale.is_finite() && scale > 0. {
        scale as f32
    } else {
        1.
    }
}
pub(crate) fn physical_debug_size(bounds: Rect, scale: f32) -> (u32, u32) {
    (
        (bounds.size.width.max(0.) * scale).ceil() as u32,
        (bounds.size.height.max(0.) * scale).ceil() as u32,
    )
}
pub(crate) fn us_since(started: incular_core::time::Instant) -> u32 {
    started.elapsed().as_micros().try_into().unwrap_or(u32::MAX)
}
pub(crate) fn clamp_i16(value: i32) -> i16 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

/// The blur reach of a general backdrop filter: the sigmas of its blur
/// stages summed per axis, a conservative bound for a chain.
pub(crate) fn backdrop_filter_blur_reach(filter: &incular_painting::ImageFilter) -> (f32, f32) {
    filter
        .stages()
        .iter()
        .fold((0., 0.), |(x, y), stage| match stage {
            incular_painting::ImageFilter::Blur(blur) => (x + blur.sigma_x, y + blur.sigma_y),
            _ => (x, y),
        })
}

#[cfg(test)]
pub(crate) fn destination_hairline_fixture(path: Arc<Path>, transform: Transform) -> DisplayList {
    let mut line = DisplayList::new();
    let line_bounds = path.tight_bounds().expect("line bounds");
    line.push(PaintCommand::StrokePath {
        path,
        brush: Brush::Solid(Color::WHITE),
        stroke: Stroke {
            width: 0.,
            ..Stroke::default()
        },
    });

    let blend_rect = Rect::from_origin_size(Offset::new(4., 4.), Size::new(4., 4.));
    let mut square = DisplayList::new();
    square.push(PaintCommand::Rect {
        rect: blend_rect,
        color: Color::WHITE,
    });
    let mut tree = incular_rendering::LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let transformed = tree.create_transform(transform);
    let line_picture = tree.create_picture(line, line_bounds);
    let blend = tree.create_blend(BlendMode::Multiply);
    let blend_picture = tree.create_picture(square, blend_rect);
    tree.set_children(transformed, vec![line_picture]);
    tree.set_children(blend, vec![blend_picture]);
    tree.set_children(root, vec![transformed, blend]);
    tree.set_root(root);
    tree.flatten()
}

#[cfg(test)]
mod destination_scope_tests {
    use super::*;

    #[test]
    fn transparent_destination_scope_keeps_a_scaled_hairline_outside_a_blend_group() {
        let mut path = Path::builder();
        path.move_to(Offset::new(8., 8.))
            .line_to(Offset::new(24., 8.));
        let list = destination_hairline_fixture(Arc::new(path.build()), Transform::scale(2.));

        assert!(commands_have_destination_blend(list.commands()));
        for scale in [1., 2.] {
            let side = (64. * scale) as u32;
            let (origin, width, height) = destination_composition_scope(&list, scale, side, side);
            assert_eq!(origin, Offset::new(4., 4.));
            assert_eq!(
                (width, height),
                ((44. * scale + 4.) as u32, (12. * scale + 4.) as u32)
            );
        }
    }

    #[test]
    fn destination_scope_uses_nested_affines_in_draw_order_and_restores_after_pop() {
        let positioned = Transform::translation(Offset::new(30., 20.));
        let rotated_and_scaled = Transform::from_coefficients([0., 2., -3., 0., 0., 0.]);
        let rect = |x, y, w, h| PaintCommand::HardEdgeRect {
            rect: Rect::from_origin_size(Offset::new(x, y), Size::new(w, h)),
            color: Color::WHITE,
        };
        let commands = [
            PaintCommand::PushTransform {
                transform: positioned,
            },
            PaintCommand::PushTransform {
                transform: rotated_and_scaled,
            },
            // (1..5, 2..7) maps to (9..24, 22..30).
            rect(1., 2., 4., 5.),
            PaintCommand::PopTransform,
            PaintCommand::PopTransform,
            rect(35., 2., 2., 2.),
        ];
        assert_eq!(
            display_list_composition_bounds(&commands, 1.),
            Some(Rect::from_origin_size(
                Offset::new(9., 2.),
                Size::new(28., 28.)
            ))
        );
    }
}
