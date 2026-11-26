use super::super::*;
use crate::tree::text::{
    editable_content_clip, editable_horizontal_max_scroll, reveal_horizontal_caret,
};
use incular_rendering::{ClipCoverage, ClipIsolation};
use incular_text::TextGeometry;

impl WidgetTree {
    pub(super) fn paint_content_kind(
        &mut self,
        id: RenderObjectId,
        kind: &RenderKind,
        size: Size,
        cache: &mut DisplayList,
    ) {
        match *kind {
            RenderKind::Text {
                ref style,
                overflow,
                ..
            } => {
                if let Some(layout) = self
                    .render_live(id, "retained render must remain live")
                    .text_layout_cloned()
                {
                    if overflow == TextOverflow::Clip {
                        cache.push(PaintCommand::PushClip {
                            rect: Rect::from_origin_size(Offset::ZERO, size),
                        });
                    }
                    // A `Text` under a `SelectionArea` is one of its
                    // selectables (Flutter's `RenderParagraph` registrar)
                    // and paints its share of the selection below its glyphs.
                    let selection = if self.static_selections.is_empty() {
                        None
                    } else {
                        self.element_for_render(id)
                            .and_then(|element| self.static_selection_range(element))
                    };
                    if let Some(selection) = selection {
                        for rect in selection_rects(&layout, selection, 0., 0., 0.) {
                            cache.push(PaintCommand::Rect {
                                rect,
                                color: Color::rgba(72, 120, 220, 150),
                            });
                        }
                    }
                    push_text_backgrounds(cache, &layout, style.effective_background_color());
                    let layered = self
                        .render_live(id, "retained render must remain live")
                        .object
                        .layers
                        .shadow()
                        .is_some();
                    push_text_shadows(cache, &layout, &style.shadows, layered);
                    push_styled_layout_runs(cache, &layout, style.effective_foreground_color());
                    if overflow == TextOverflow::Clip {
                        cache.push(PaintCommand::PopClip);
                    }
                }
            }
            RenderKind::SelectableText { ref style, .. } => {
                let selection = self
                    .element_for_render(id)
                    .and_then(|element| self.static_selection_range(element));
                if let (Some(layout), Some(selection)) = (
                    self.render_live(id, "retained render must remain live")
                        .text_layout_cloned(),
                    selection,
                ) {
                    for rect in selection_rects(&layout, selection, 0., 0., 0.) {
                        cache.push(PaintCommand::Rect {
                            rect,
                            color: Color::rgba(72, 120, 220, 150),
                        });
                    }
                    push_text_backgrounds(cache, &layout, style.effective_background_color());
                    push_text_shadows(cache, &layout, &style.shadows, false);
                    push_layout_runs(cache, &layout, style.effective_foreground_color());
                } else if let Some(layout) = self
                    .render_live(id, "retained render must remain live")
                    .text_layout_cloned()
                {
                    push_text_backgrounds(cache, &layout, style.effective_background_color());
                    push_text_shadows(cache, &layout, &style.shadows, false);
                    push_layout_runs(cache, &layout, style.effective_foreground_color());
                }
            }
            RenderKind::SelectionArea
            | RenderKind::SelectionContainer
            | RenderKind::SelectionListener
            | RenderKind::IndexedSemantics
            | RenderKind::SemanticsDebugger { .. } => {}
            RenderKind::Image { image: None, .. } => {}
            RenderKind::Image {
                image: Some(ref image),
                scale,
                fit,
                repeat,
                alignment,
                sampling,
                center_slice,
                flip_horizontally,
                ..
            } => {
                let bounds = Rect::from_origin_size(Offset::ZERO, size);
                if size.width <= 0.0 || size.height <= 0.0 {
                    return;
                }
                if center_slice.is_some() || flip_horizontally {
                    cache.extend_from(&crate::painting_effects::paint_image(
                        &crate::painting_effects::ImagePaint {
                            image: image.clone(),
                            scale,
                            fit: Some(fit),
                            alignment,
                            center_slice,
                            repeat,
                            flip_horizontally,
                            sampling,
                        },
                        bounds,
                    ));
                    return;
                }
                let intrinsic = image.decoded();
                let source = Rect::from_origin_size(
                    Offset::ZERO,
                    Size::new(intrinsic.width() as f32, intrinsic.height() as f32),
                );
                let (source, destination) =
                    image_fit_rects_scaled(source, bounds, fit, alignment, scale);
                if repeat == ImageRepeat::NoRepeat || destination.size == bounds.size {
                    cache.push(PaintCommand::Image {
                        image: image.clone(),
                        source,
                        destination,
                        sampling,
                    });
                } else {
                    cache.push(PaintCommand::PushClip { rect: bounds });
                    for destination in image_repeat_destinations(destination, bounds, repeat) {
                        cache.push(PaintCommand::Image {
                            image: image.clone(),
                            source,
                            destination,
                            sampling,
                        });
                    }
                    cache.push(PaintCommand::PopClip);
                }
            }
            RenderKind::TextField(ref field) => {
                let TextFieldRender {
                    ref controller,
                    ref style,
                    ref placeholder,
                    placeholder_color,
                    focused_border,
                    focused_border_outset,
                    content_padding,
                    multiline,
                    obscure_text,
                    obscuring_character,
                    cursor_width,
                    cursor_height,
                    cursor_radius,
                    cursor_opacity_animates,
                    show_cursor,
                    cursor_color,
                    selection_color,
                    spell_check_selection_color,
                    ..
                } = **field;
                let (layout, focused, scroll_x, scroll_y, revealed, decorations) = {
                    let node = self.render_live(id, "retained render must remain live");
                    let state = self.text_field_state_live(id);
                    (
                        node.field_layout(),
                        state.focused,
                        state.scroll_x,
                        state.scroll_y,
                        state.revealed,
                        state.decorations.clone(),
                    )
                };
                let alternate_selection_color = spell_check_selection_color.filter(|_| {
                    self.element_for_render(id)
                        .is_some_and(|element| self.text_field_spell_check_menu_visible(element))
                });
                let selection_color = alternate_selection_color.or(selection_color);
                let (value_selection, value_empty) =
                    controller.with_value(|value| (value.selection, value.text.is_empty()));
                let reveal_key = (controller.revisions().0, value_selection, focused, size);
                let reveal = revealed != Some(reveal_key);
                let has_layout = layout.is_some();
                let mask = obscure_text.then_some(obscuring_character);
                let shows_placeholder = value_empty
                    && with_text_field_display(controller, placeholder, mask, |display| {
                        display == placeholder.as_str()
                    });
                let mut active_scroll_x = scroll_x;
                let mut active_scroll_y = scroll_y;
                if let Some(layout) = layout {
                    let (caret, caret_y, caret_height) = text_field_caret_geometry(
                        &layout,
                        value_selection.extent,
                        controller.caret_affinity(),
                        value_empty,
                        field,
                    );
                    let available = (size.width - content_padding.horizontal()).max(0.);
                    if !multiline {
                        if field.text_direction == TextDirection::Ltr {
                            if reveal {
                                active_scroll_x = reveal_horizontal_caret(
                                    active_scroll_x,
                                    caret,
                                    cursor_width,
                                    available,
                                );
                            }
                            active_scroll_x = active_scroll_x.clamp(
                                0.,
                                editable_horizontal_max_scroll(field, &layout, available),
                            );
                        } else {
                            if reveal && caret - active_scroll_x > available {
                                active_scroll_x = caret - available;
                            } else if reveal && caret < active_scroll_x {
                                active_scroll_x = caret;
                            }
                            if matches!(field.text_align, TextAlign::Start | TextAlign::End) {
                                // RTL overflow can have a negative line origin.
                                let (left, right) = layout.lines().fold(
                                    (f32::INFINITY, f32::NEG_INFINITY),
                                    |(left, right), line| {
                                        (
                                            left.min(line.offset()),
                                            right.max(line.offset() + line.width()),
                                        )
                                    },
                                );
                                active_scroll_x = if right - left > available {
                                    active_scroll_x.clamp(left, right - available)
                                } else {
                                    0.
                                };
                            } else {
                                active_scroll_x = active_scroll_x.max(0.);
                            }
                        }
                    }
                    let top = text_field_content_top(
                        multiline,
                        size.height,
                        layout.metrics().line_height,
                        content_padding,
                    );
                    if multiline {
                        let viewport = (size.height - content_padding.vertical()).max(0.);
                        if reveal && caret_y - active_scroll_y < 0. {
                            active_scroll_y = caret_y;
                        } else if reveal && caret_y + caret_height - active_scroll_y > viewport {
                            active_scroll_y = caret_y + caret_height - viewport;
                        }
                        active_scroll_y = active_scroll_y
                            .clamp(0., (layout.metrics().size.height - viewport).max(0.));
                    }
                    let clip = editable_content_clip(
                        field,
                        &layout,
                        size,
                        Offset::new(active_scroll_x, active_scroll_y),
                    );
                    if let Some(rect) = clip {
                        match field.clip_behavior {
                            Clip::None => unreachable!("an unclipped editor has no clip bounds"),
                            Clip::HardEdge => cache.push(PaintCommand::PushClip { rect }),
                            Clip::AntiAlias => cache.push(PaintCommand::PushClipRRect {
                                rrect: RRect::uniform(rect, 0.0),
                                coverage: ClipCoverage::AntiAlias,
                                isolation: None,
                            }),
                            Clip::AntiAliasWithSaveLayer => {
                                // The editable picture has exactly one isolated
                                // clip. Its own layer ID gives the renderer a
                                // stable cache key without isolating its border.
                                let layer = self
                                    .render_live(id, "retained editor")
                                    .object
                                    .layers
                                    .picture
                                    .expect("editor picture layer");
                                let state = self.text_field_state_live_mut(id);
                                state.clip_picture_generation =
                                    state.clip_picture_generation.wrapping_add(1);
                                cache.push(PaintCommand::PushClipRRect {
                                    rrect: RRect::uniform(rect, 0.0),
                                    coverage: ClipCoverage::AntiAliasWithSaveLayer,
                                    isolation: Some(ClipIsolation {
                                        layer,
                                        generation: state.clip_picture_generation,
                                    }),
                                });
                            }
                        }
                    }
                    let selection = value_selection.range();
                    if focused
                        && !selection.is_empty()
                        && let Some(selection_color) = selection_color
                    {
                        let selection = TextRange::new(
                            text_field_display_offset(controller, selection.start, mask),
                            text_field_display_offset(controller, selection.end, mask),
                        );
                        for rect in selection_rects(
                            &layout,
                            selection,
                            active_scroll_x - content_padding.left,
                            active_scroll_y,
                            top,
                        ) {
                            cache.push(PaintCommand::Rect {
                                rect,
                                color: selection_color,
                            });
                        }
                    }
                    // The caret paints below the glyph runs, as Flutter does
                    // unless paintCursorAboveText is set (editable.dart). It
                    // shows only for a collapsed selection: Flutter's
                    // `_showBlinkingCursor` requires `selection.isCollapsed`.
                    let caret_opacity =
                        controller.caret_opacity(Instant::now(), cursor_opacity_animates);
                    if focused && show_cursor && selection.is_empty() && caret_opacity > 0.0 {
                        // Flutter's `_cursorColor`: the colour's own opacity,
                        // capped by the blink opacity.
                        let alpha = f32::from(cursor_color.alpha).min(caret_opacity * 255.0);
                        let cursor_color = Color {
                            alpha: alpha.round() as u8,
                            ..cursor_color
                        };
                        let height = cursor_height.unwrap_or(caret_height).max(0.0);
                        let y = top + caret_y - active_scroll_y
                            + (caret_height - height).max(0.0) * 0.5;
                        let rect = Rect::from_origin_size(
                            Offset::new(caret - active_scroll_x + content_padding.left, y),
                            Size::new(cursor_width.max(0.0), height),
                        );
                        if cursor_radius > 0.0 {
                            cache.push(PaintCommand::RRect {
                                rrect: RRect::uniform(rect, cursor_radius),
                                brush: cursor_color.into(),
                            });
                        } else if rect.size.width > 0.0 && rect.size.height > 0.0 {
                            cache.push(PaintCommand::Rect {
                                rect,
                                color: cursor_color,
                            });
                        }
                    }
                    cache.push(PaintCommand::PushTransform {
                        transform: CoreTransform::translation(Offset::new(
                            content_padding.left - active_scroll_x,
                            top - active_scroll_y,
                        )),
                    });
                    let color = if shows_placeholder {
                        placeholder_color
                    } else {
                        style.effective_foreground_color()
                    };
                    // Only lines inside the clipped viewport, padded for
                    // overhanging glyphs: a long document otherwise emits every
                    // line on each keystroke and caret blink.
                    let pad = layout.metrics().line_height.max(style.size) * 2.;
                    let visible_top = active_scroll_y - top - pad;
                    let color_runs = if shows_placeholder {
                        &[][..]
                    } else {
                        layout.color_runs()
                    };
                    let visible_bottom = visible_top + size.height + pad * 2.;
                    let lines = || {
                        if clip.is_some() {
                            layout.lines_between(visible_top, visible_bottom)
                        } else {
                            layout.lines()
                        }
                    };
                    for line in lines() {
                        if color_runs.is_empty() {
                            push_line_runs(cache, line, color);
                        } else {
                            push_colored_line_runs(cache, line, color, color_runs);
                        }
                    }
                    // Decorations paint over the glyphs, as SkParagraph's
                    // `TextLine::paint` draws them after the text.
                    if !shows_placeholder && !decorations.is_empty() {
                        for line in lines() {
                            push_line_decorations(cache, line, &decorations);
                        }
                    }
                    cache.push(PaintCommand::PopTransform);
                    if clip.is_some() {
                        cache.push(PaintCommand::PopClip);
                    }
                }
                if focused && let Some((border, radius)) = focused_border {
                    let outset = focused_border_outset;
                    let rect = Rect::from_origin_size(
                        Offset::new(-outset.left, -outset.top),
                        Size::new(
                            size.width + outset.left + outset.right,
                            size.height + outset.top + outset.bottom,
                        ),
                    );
                    cache.push(PaintCommand::Border {
                        rrect: RRect::uniform(rect, radius),
                        border,
                    });
                }
                let state = self.text_field_state_live_mut(id);
                state.spell_selection_visible = alternate_selection_color.is_some();
                state.scroll_x = active_scroll_x;
                state.scroll_y = active_scroll_y;
                if has_layout {
                    state.revealed = Some(reveal_key);
                }
            }
            _ => unreachable!("content paint received a non-content render kind"),
        }
    }
}

fn push_text_backgrounds(cache: &mut DisplayList, layout: &TextLayout, color: Option<Color>) {
    for background in layout.backgrounds.as_deref().unwrap_or_default() {
        if let Some(color) = background.color_override.or(color)
            && color.alpha > 0
        {
            cache.push(PaintCommand::Rect { rect: background.rect, color });
        }
    }
}

fn push_layout_runs(cache: &mut DisplayList, layout: &TextLayout, color: Color) {
    for line in layout.lines.iter() {
        for run in line.runs.iter() {
            cache.push(PaintCommand::GlyphRun {
                run: run.clone(),
                color,
            });
        }
    }
}

/// Paints `shadows` beneath the text in list order, as Flutter's paragraph
/// painter does: each is the glyphs again, offset and in the shadow color.
/// When `layered`, the render's drop-shadow layer already draws the blurred
/// shadow [`crate::utilities::layered_text_shadow`] picks, so it is skipped here; any further
/// blurred shadow is drawn sharp.
fn push_text_shadows(
    cache: &mut DisplayList,
    layout: &TextLayout,
    shadows: &[incular_text::TextShadow],
    layered: bool,
) {
    let skipped = layered
        .then(|| crate::utilities::layered_text_shadow(shadows))
        .flatten();
    let mut skipped_once = false;
    for shadow in shadows {
        if shadow.color.alpha == 0 {
            continue;
        }
        if !skipped_once && Some(*shadow) == skipped {
            skipped_once = true;
            continue;
        }
        let moved = shadow.offset != Offset::ZERO;
        if moved {
            cache.push(PaintCommand::PushTransform {
                transform: CoreTransform::translation(shadow.offset),
            });
        }
        push_layout_runs(cache, layout, shadow.color);
        if moved {
            cache.push(PaintCommand::PopTransform);
        }
    }
}

/// Paints a paragraph whose spans carry their own colors. Each glyph run is
/// split where the color changes, so every span keeps its color. A paragraph
/// without per-span colors paints entirely in `color`.
fn push_styled_layout_runs(cache: &mut DisplayList, layout: &TextLayout, color: Color) {
    if layout.color_runs.is_empty() {
        push_layout_runs(cache, layout, color);
        return;
    }
    let color_at = |byte: u32| {
        let byte = byte as usize;
        let index = layout
            .color_runs
            .partition_point(|run| run.range.start <= byte);
        index
            .checked_sub(1)
            .map(|index| &layout.color_runs[index])
            .filter(|run| byte < run.range.end)
            .map_or(color, |run| run.color)
    };
    for line in layout.lines.iter() {
        for run in line.runs.iter() {
            push_colored_glyph_segments(cache, run, color, color_at);
        }
    }
}

/// Pushes `run` as contiguous glyph segments, one per color change.
fn push_colored_glyph_segments(
    cache: &mut DisplayList,
    run: &incular_rendering::GlyphRun,
    color: Color,
    color_at: impl Fn(u32) -> Color,
) {
    let mut start = 0;
    let mut segment_color = color;
    for (index, glyph) in run.glyphs.iter().enumerate() {
        let glyph_color = color_at(glyph.cluster);
        if index > start && glyph_color != segment_color {
            push_glyph_segment(cache, run, &run.glyphs[start..index], segment_color);
            start = index;
        }
        segment_color = glyph_color;
    }
    if start < run.glyphs.len() {
        push_glyph_segment(cache, run, &run.glyphs[start..], segment_color);
    }
}

fn push_glyph_segment(
    cache: &mut DisplayList,
    run: &incular_rendering::GlyphRun,
    glyphs: &[incular_rendering::GlyphPosition],
    color: Color,
) {
    cache.push(PaintCommand::GlyphRun {
        run: Arc::new(incular_rendering::GlyphRun {
            font: run.font.clone(),
            font_size: run.font_size,
            origin: run.origin,
            glyphs: Arc::from(glyphs),
        }),
        color,
    });
}

/// [`push_line_runs`] with each glyph in the color of the styled run its
/// cluster belongs to.
fn push_colored_line_runs(
    cache: &mut DisplayList,
    line: incular_text::TextLineRef<'_>,
    color: Color,
    color_runs: &[incular_text::TextColorRun],
) {
    let color_at = |byte: u32| {
        let byte = byte as usize;
        let index = color_runs.partition_point(|run| run.range.start <= byte);
        index
            .checked_sub(1)
            .map(|index| &color_runs[index])
            .filter(|run| byte < run.range.end)
            .map_or(color, |run| run.color)
    };
    let origin = line.paint_origin();
    for run in line.runs() {
        let mut placed = (**run).clone();
        placed.origin.x += origin.x;
        placed.origin.y += origin.y;
        push_colored_glyph_segments(cache, &placed, color, color_at);
    }
}

/// Paints the decorations of `runs` on `line`, as SkParagraph's
/// `Decorations::paint` does. Font decoration metrics are not exposed by
/// the text engine, so positions use SkParagraph's fallbacks: a thickness of
/// `fontSize / 14` times `decorationThickness`, the underline that far below
/// the baseline plus a tenth of the font size, the line-through at 0.3 of
/// the font size above it and the overline at its ascent.
fn push_line_decorations(
    cache: &mut DisplayList,
    line: incular_text::TextLineRef<'_>,
    runs: &[incular_text::StyledTextRun],
) {
    use incular_text::TextDecoration;
    for run in runs {
        let style = &run.style;
        let Some(decoration) = style.decoration.filter(|decoration| !decoration.is_empty()) else {
            continue;
        };
        let spans = line.selection_spans(run.range.start, run.range.end);
        if spans.is_empty() {
            continue;
        }
        let font_size = style.size.max(0.);
        let thickness =
            (font_size / 14.).max(f32::EPSILON) * style.decoration_thickness.unwrap_or(1.);
        let color = style.decoration_color.unwrap_or(style.effective_foreground_color());
        let decoration_style = style.decoration_style.unwrap_or_default();
        let baseline = line.baseline();
        let mut positions = [0.0_f32; 3];
        let mut count = 0;
        if decoration.contains(TextDecoration::UNDERLINE) {
            positions[count] = baseline + font_size * 0.1;
            count += 1;
        }
        if decoration.contains(TextDecoration::OVERLINE) {
            positions[count] = baseline - font_size * 0.9;
            count += 1;
        }
        if decoration.contains(TextDecoration::LINE_THROUGH) {
            positions[count] = baseline - font_size * 0.3;
            count += 1;
        }
        for &(left, right) in &spans {
            let (left, right) = (left.min(right), left.max(right));
            for &y in &positions[..count] {
                push_decoration_line(cache, left, right, y, thickness, color, decoration_style);
            }
        }
    }
}

/// One decoration line from `left` to `right` centred on `y`.
fn push_decoration_line(
    cache: &mut DisplayList,
    left: f32,
    right: f32,
    y: f32,
    thickness: f32,
    color: Color,
    style: incular_text::TextDecorationStyle,
) {
    use incular_text::TextDecorationStyle;
    let width = right - left;
    if width <= 0. {
        return;
    }
    let bar = |cache: &mut DisplayList, x: f32, length: f32, y: f32| {
        cache.push(PaintCommand::Rect {
            rect: Rect::from_origin_size(
                Offset::new(x, y - thickness * 0.5),
                Size::new(length.min(right - x).max(0.), thickness),
            ),
            color,
        });
    };
    match style {
        TextDecorationStyle::Solid => bar(cache, left, width, y),
        TextDecorationStyle::Double => {
            // SkParagraph draws the second line two thicknesses below the
            // first.
            bar(cache, left, width, y);
            bar(cache, left, width, y + thickness * 2.);
        }
        TextDecorationStyle::Dotted | TextDecorationStyle::Dashed => {
            // SkParagraph's dash intervals: dots of one thickness, dashes of
            // four, each followed by a gap of the same length.
            let length = if style == TextDecorationStyle::Dotted {
                thickness
            } else {
                thickness * 4.
            };
            let mut x = left;
            while x < right {
                bar(cache, x, length, y);
                x += length * 2.;
            }
        }
        TextDecorationStyle::Wavy => {
            // SkParagraph `Decorations::calculateWaves`: quadratic waves of a
            // quarter wavelength equal to the thickness.
            let quarter = thickness.max(0.5);
            let mut builder = Path::builder();
            builder.move_to(Offset::new(left, y));
            let mut x = left;
            let mut up = true;
            while x + quarter * 2. <= right {
                let control = Offset::new(x + quarter, if up { y - quarter } else { y + quarter });
                builder.quadratic_to(control, Offset::new(x + quarter * 2., y));
                x += quarter * 2.;
                up = !up;
            }
            cache.push(PaintCommand::StrokePath {
                path: Arc::new(builder.build()),
                brush: color.into(),
                stroke: incular_rendering::Stroke {
                    width: thickness,
                    ..incular_rendering::Stroke::default()
                },
            });
        }
    }
}
