use super::super::super::*;
use super::super::{
    environment_text_scaler, physical_text_align, resolve_text_style, shared_text_style,
    text_field_style,
};
use crate::environment::DefaultTextParagraph;
pub(super) fn lower_visual(widget: &Widget, context: &DependencyContext) -> RenderKind {
    match widget.kind() {
        WidgetKind::Box { size, color } => RenderKind::Box {
            desired: *size,
            color: *color,
        },
        WidgetKind::Shape {
            path,
            fill,
            stroke,
            size,
        } => RenderKind::Shape {
            path: path.clone(),
            fill: fill.clone(),
            stroke: stroke.clone(),
            desired: size
                .unwrap_or_else(|| path.tight_bounds().map_or(Size::ZERO, |bounds| bounds.size)),
        },
        WidgetKind::CustomPaint {
            size,
            display_list,
            painter,
            ..
        } => RenderKind::CustomPaint {
            desired: *size,
            display_list: display_list.clone(),
            painter: painter.clone(),
        },
        WidgetKind::Colored { color, is_anti_alias, .. } => RenderKind::Colored {
            color: *color,
            is_anti_alias: *is_anti_alias,
        },
        WidgetKind::Decorated {
            size,
            background,
            border,
            radius,
            ..
        } => RenderKind::Decorated {
            desired: *size,
            background: background.clone(),
            border: *border,
            radius: *radius,
        },
        WidgetKind::Banner {
            message,
            text_direction,
            location,
            layout_direction,
            color,
            text_style,
            shadow,
            ..
        } => {
            let ambient_direction = context
                .depend::<TextDirection>()
                .unwrap_or(TextDirection::Ltr);
            RenderKind::Banner {
                message: message.clone(),
                text_direction: text_direction.unwrap_or(ambient_direction),
                location: *location,
                layout_direction: layout_direction.unwrap_or(ambient_direction),
                color: *color,
                text_style: text_style.clone(),
                shadow: *shadow,
            }
        }
        WidgetKind::Button(spec) => RenderKind::Button {
            desired: spec.size,
            color: spec.color,
            hover_color: spec.hover_color,
            pressed_color: spec.pressed_color,
            focused_color: spec.focused_color,
            disabled_color: spec.disabled_color,
            enabled: spec.enabled,
            focusable_when_disabled: spec.focusable_when_disabled,
        },
        WidgetKind::Text {
            text,
            style,
            align,
            soft_wrap,
            max_lines,
            overflow,
            text_direction,
            text_height_behavior,
            text_scaler,
            spans,
            inline,
            strut,
        } => {
            // Unstated paragraph options inherit the nearest DefaultTextStyle.
            // Flutter: `textScaler ?? MediaQuery.textScalerOf(context)`.
            let scaler = text_scaler.unwrap_or_else(|| environment_text_scaler(context));
            let defaults = context.depend::<DefaultTextParagraph>().unwrap_or_default();
            let direction = text_direction
                .or_else(|| context.depend::<TextDirection>())
                .unwrap_or(TextDirection::Ltr);
            RenderKind::Text {
                text: text.clone(),
                style: shared_text_style(style, context, Some(scaler)),
                // Start/End are direction-relative; the paragraph's resolved
                // text direction (explicit, else the ambient `Directionality`,
                // which build validation requires) makes them physical.
                align: physical_text_align(
                    align.or(defaults.align).unwrap_or(TextAlign::Start),
                    direction,
                ),
                soft_wrap: soft_wrap.or(defaults.soft_wrap).unwrap_or(true),
                max_lines: max_lines.unwrap_or(defaults.max_lines),
                overflow: overflow.or(defaults.overflow).unwrap_or(TextOverflow::Clip),
                height_behavior: text_height_behavior
                    .or_else(|| context.depend::<TextHeightBehavior>())
                    .unwrap_or_default(),
                spans: spans.as_ref().map(|runs| {
                    runs.iter()
                        .map(|run| StyledTextRun {
                            text: run.text.clone(),
                            style: resolve_text_style(&run.style, context).scaled(scaler),
                            range: run.range.clone(),
                        })
                        .collect::<Arc<[_]>>()
                }),
                inline: inline.as_ref().map(|inline| Arc::new(inline.0.clone())),
                // TextPainter scales the strut font size with the text scaler;
                // an unset size is SkParagraph's 14.
                strut: strut.as_ref().map(|strut| {
                    let mut strut = (**strut).clone();
                    strut.font_size = Some(scaler.scale(strut.font_size.unwrap_or(14.)));
                    Box::new(strut)
                }),
            }
        }
        WidgetKind::SelectableText { text, style, align } => RenderKind::SelectableText {
            text: text.clone(),
            style: shared_text_style(style, context, Some(environment_text_scaler(context))),
            align: *align,
        },
        WidgetKind::SelectionArea { .. } => RenderKind::SelectionArea,
        WidgetKind::SelectionContainer { .. } => RenderKind::SelectionContainer,
        WidgetKind::SelectionListener { .. } => RenderKind::SelectionListener,
        WidgetKind::IndexedSemantics { .. } => RenderKind::IndexedSemantics,
        WidgetKind::SemanticsDebugger {
            label_style,
            max_nodes,
            ..
        } => RenderKind::SemanticsDebugger {
            label_style: label_style.clone(),
            max_nodes: *max_nodes,
        },
        WidgetKind::Image {
            image,
            width,
            height,
            scale,
            fit,
            repeat,
            alignment,
            sampling,
            blend_mode,
            center_slice,
            match_text_direction,
            source_effects,
        } => RenderKind::Image {
            image: image.clone(),
            width: *width,
            height: *height,
            scale: *scale,
            fit: *fit,
            repeat: *repeat,
            alignment: *alignment,
            sampling: *sampling,
            blend_mode: *blend_mode,
            center_slice: *center_slice,
            flip_horizontally: *match_text_direction
                && context.depend::<TextDirection>() == Some(TextDirection::Rtl),
            source_effects: *source_effects,
        },
        WidgetKind::TextField(spec) => {
            let direction = spec
                .text_direction
                .or_else(|| context.depend::<TextDirection>());
            debug_assert!(
                direction.is_some(),
                "validated EditableText requires a direction"
            );
            let direction = direction.unwrap_or(TextDirection::Ltr);
            let resolved_locale = spec.locale.clone().or_else(|| {
                crate::Localizations::maybe_locale_of(&BuildContext::new(context))
            });
            let resolve_field_style = |style: &incular_text::TextStyle| {
                let mut style = text_field_style(style, context);
                if style.locale.is_none() {
                    style.locale = resolved_locale.as_ref().map(ToString::to_string);
                }
                style
            };
            RenderKind::TextField(Box::new(TextFieldRender {
                controller: spec.controller.clone(),
                desired: spec.size,
                style: resolve_field_style(&spec.style),
                placeholder: spec.placeholder.clone(),
                placeholder_color: spec.placeholder_color,
                focused_border: spec.focused_border,
                focused_border_outset: spec.focused_border_outset,
                content_padding: spec.content_padding,
                multiline: spec.multiline,
                min_lines: spec.min_lines,
                max_lines: spec.max_lines,
                expands: spec.expands,
                text_align: spec.text_align,
                text_direction: direction,
                enabled: spec.enabled,
                read_only: spec.read_only,
                obscure_text: spec.obscure_text,
                obscuring_character: spec.obscuring_character,
                cursor_width: spec.cursor_width,
                cursor_height: spec.cursor_height,
                cursor_radius: spec.cursor_radius,
                cursor_opacity_animates: spec.cursor_opacity_animates,
                show_cursor: spec.show_cursor,
                cursor_color: spec.cursor_color,
                selection_color: spec.selection_color,
                spell_check_selection_color: spec
                    .spell_check
                    .as_ref()
                    .filter(|_| !spec.spell_check_suppressed)
                    .and_then(crate::SpellCheckConfiguration::suggestions_selection_color),
                misspelled_text_style: spec
                    .spell_check
                    .as_ref()
                    .filter(|_| !spec.spell_check_suppressed)
                    .and_then(crate::SpellCheckConfiguration::active_style)
                    .map(resolve_field_style),
                paragraph_layout: spec.paragraph_layout,
            }))
        }
        _ => unreachable!("visual lowering received a non-visual widget"),
    }
}
