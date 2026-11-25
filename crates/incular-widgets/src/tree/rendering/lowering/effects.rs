use super::super::super::*;

pub(super) fn lower_effects(widget: &Widget, context: &DependencyContext) -> RenderKind {
    match widget.kind() {
        WidgetKind::Translate { controller, .. } => RenderKind::Translate {
            controller: controller.clone(),
        },
        WidgetKind::Transform {
            transform,
            origin,
            alignment,
            fraction,
            transform_hit_tests,
            slot,
            filter_quality,
            ..
        } => RenderKind::Transform {
            transform: *transform,
            origin: *origin,
            alignment: *alignment,
            fraction: *fraction,
            transform_hit_tests: *transform_hit_tests,
            slot: slot.clone(),
            filter_quality: *filter_quality,
        },
        WidgetKind::Affine { controller, .. } => RenderKind::Affine {
            controller: controller.clone(),
        },
        WidgetKind::Scale {
            controller,
            origin,
            filter_quality,
            ..
        } => RenderKind::Scale {
            controller: controller.clone(),
            origin: *origin,
            filter_quality: *filter_quality,
        },
        WidgetKind::Rotation {
            controller,
            origin,
            alignment,
            filter_quality,
            ..
        } => RenderKind::Rotation {
            controller: controller.clone(),
            origin: *origin,
            alignment: *alignment,
            filter_quality: *filter_quality,
        },
        WidgetKind::FittedBox {
            fit,
            alignment,
            clip_behavior,
            ..
        } => RenderKind::FittedBox {
            fit: *fit,
            alignment: match alignment {
                crate::AlignmentGeometry::Physical(value) => *value,
                value => value.resolve(
                    context
                        .depend::<TextDirection>()
                        .unwrap_or(TextDirection::Ltr),
                ),
            },
            clip_behavior: *clip_behavior,
        },
        WidgetKind::Opacity {
            alpha, controller, ..
        } => RenderKind::Opacity {
            alpha: *alpha,
            controller: controller.clone(),
        },
        WidgetKind::Blur {
            sigma_x,
            sigma_y,
            controller,
            ..
        } => RenderKind::Blur {
            sigma_x: *sigma_x,
            sigma_y: *sigma_y,
            controller: controller.clone(),
        },
        WidgetKind::DropShadow {
            offset,
            sigma_x,
            sigma_y,
            color,
            controller,
            ..
        } => RenderKind::DropShadow {
            offset: *offset,
            sigma_x: *sigma_x,
            sigma_y: *sigma_y,
            color: *color,
            controller: controller.clone(),
        },
        WidgetKind::ColorFiltered {
            filter, controller, ..
        } => RenderKind::ColorFiltered {
            filter: filter.clone(),
            controller: controller.clone(),
        },
        WidgetKind::Blend { mode, .. } => RenderKind::Blend { mode: *mode },
        WidgetKind::ShaderMask {
            shader, blend_mode, ..
        } => RenderKind::ShaderMask {
            shader: shader.clone(),
            blend_mode: *blend_mode,
        },
        WidgetKind::BackdropFilter {
            blur,
            blend_mode,
            enabled,
            matrix,
            image_filter,
            backdrop_key,
            ..
        } => RenderKind::BackdropFilter {
            blur: *blur,
            blend_mode: *blend_mode,
            enabled: *enabled,
            matrix: *matrix,
            image_filter: image_filter.clone(),
            backdrop_key: *backdrop_key,
        },
        WidgetKind::AnnotatedRegion {
            annotation, sized, ..
        } => RenderKind::AnnotatedRegion {
            annotation: annotation.clone(),
            sized: *sized,
        },
        WidgetKind::CompositedTransformTarget { link, .. } => {
            RenderKind::Leader { link: link.clone() }
        }
        WidgetKind::CompositedTransformFollower {
            link,
            show_when_unlinked,
            offset,
            target_anchor,
            follower_anchor,
            ..
        } => RenderKind::Follower {
            link: link.clone(),
            show_when_unlinked: *show_when_unlinked,
            offset: *offset,
            target_anchor: *target_anchor,
            follower_anchor: *follower_anchor,
        },
        _ => unreachable!("effect lowering received a non-effect widget"),
    }
}
