use super::super::super::*;

/// Resolves a widget's optional explicit text direction against the nearest
/// ambient `Directionality`.
///
/// Flutter lays a `Flex`/`Wrap` out as left-to-right when no direction exists
/// *and* assertions are disabled (a release build); with assertions enabled the
/// missing direction is reported earlier, during build validation
/// (`TreeError::MissingDirectionality`). This fallback is therefore only
/// reachable in release builds.
fn resolve_layout_direction(
    explicit: Option<TextDirection>,
    context: &DependencyContext,
) -> TextDirection {
    explicit
        .or_else(|| context.depend::<TextDirection>())
        .unwrap_or(TextDirection::Ltr)
}

fn resolve_stack_alignment(explicit: Option<Alignment>, context: &DependencyContext) -> Alignment {
    explicit.unwrap_or_else(|| {
        incular_config::AlignmentDirectional::TOP_START
            .resolve(resolve_layout_direction(None, context))
    })
}

pub(super) fn lower_layout(widget: &Widget, context: &DependencyContext) -> RenderKind {
    match widget.kind() {
        WidgetKind::Padding { padding, .. } => RenderKind::Padding { padding: *padding },
        WidgetKind::Constrained { constraints, .. } => RenderKind::Constrained {
            constraints: *constraints,
        },
        WidgetKind::Limited {
            max_width,
            max_height,
            ..
        } => RenderKind::Limited {
            max_width: *max_width,
            max_height: *max_height,
        },
        WidgetKind::Overflow {
            min_width,
            max_width,
            min_height,
            max_height,
            alignment,
            ..
        } => RenderKind::Overflow {
            min_width: *min_width,
            max_width: *max_width,
            min_height: *min_height,
            max_height: *max_height,
            alignment: *alignment,
        },
        WidgetKind::Unconstrained {
            constrained_axis,
            step_width,
            step_height,
            alignment,
            ..
        } => RenderKind::Unconstrained {
            constrained_axis: *constrained_axis,
            step_width: *step_width,
            step_height: *step_height,
            alignment: *alignment,
        },
        WidgetKind::ProxyLayout { mode, .. } => RenderKind::ProxyLayout { mode: *mode },
        WidgetKind::Fractional {
            width_factor,
            height_factor,
            alignment,
            ..
        } => RenderKind::Fractional {
            width_factor: *width_factor,
            height_factor: *height_factor,
            alignment: *alignment,
        },
        WidgetKind::Baseline { baseline, .. } => RenderKind::Baseline {
            baseline: *baseline,
        },
        WidgetKind::RepaintBoundary { .. } => RenderKind::RepaintBoundary,
        WidgetKind::AnimationTicker {
            controller,
            auto_start,
            repeat,
            revision,
            retarget,
            frame,
            ..
        } => RenderKind::AnimationTicker {
            controller: controller.clone(),
            auto_start: *auto_start,
            repeat: *repeat,
            revision: revision.clone(),
            retarget: retarget.clone(),
            frame: frame.clone(),
        },
        WidgetKind::Gesture { .. } => RenderKind::Gesture,
        WidgetKind::RawInput { .. } => RenderKind::Gesture,
        WidgetKind::Draggable { .. } | WidgetKind::DragTarget { .. } => RenderKind::Gesture,
        WidgetKind::IgnorePointer { .. } | WidgetKind::AbsorbPointer { .. } => RenderKind::Gesture,
        WidgetKind::Align {
            alignment,
            width_factor,
            height_factor,
            ..
        } => RenderKind::Align {
            alignment: *alignment,
            width_factor: *width_factor,
            height_factor: *height_factor,
        },
        WidgetKind::Flex {
            axis,
            main_axis_alignment,
            main_axis_size,
            cross_axis_alignment,
            text_direction,
            vertical_direction,
            spacing,
            ..
        } => RenderKind::Flex {
            flex: incular_layout::Flex {
                direction: *axis,
                main_axis_alignment: *main_axis_alignment,
                main_axis_size: *main_axis_size,
                cross_axis_alignment: *cross_axis_alignment,
                text_direction: resolve_layout_direction(*text_direction, context),
                vertical_direction: *vertical_direction,
                spacing: *spacing,
            },
        },
        WidgetKind::Flexible { flex, fit, .. } => RenderKind::Flexible {
            flex: *flex,
            fit: *fit,
        },
        WidgetKind::Wrap {
            axis,
            alignment,
            spacing,
            run_alignment,
            run_spacing,
            cross_axis_alignment,
            text_direction,
            vertical_direction,
            ..
        } => RenderKind::Wrap {
            wrap: incular_layout::Wrap {
                direction: *axis,
                alignment: *alignment,
                spacing: *spacing,
                run_alignment: *run_alignment,
                run_spacing: *run_spacing,
                cross_axis_alignment: *cross_axis_alignment,
                text_direction: resolve_layout_direction(*text_direction, context),
                vertical_direction: *vertical_direction,
            },
        },
        WidgetKind::Table {
            columns,
            column_spacing,
            row_spacing,
            column_widths,
            default_column_width,
            default_vertical_alignment,
            ..
        } => RenderKind::Table {
            columns: *columns,
            column_spacing: *column_spacing,
            row_spacing: *row_spacing,
            column_widths: column_widths.clone(),
            default_column_width: *default_column_width,
            default_vertical_alignment: *default_vertical_alignment,
        },
        WidgetKind::Stack {
            alignment,
            fit,
            clip_behavior,
            ..
        } => RenderKind::Stack {
            stack: incular_layout::Stack {
                alignment: resolve_stack_alignment(*alignment, context),
                fit: *fit,
                clip_behavior: *clip_behavior,
            },
        },
        WidgetKind::SafeArea {
            minimum,
            left,
            top,
            right,
            bottom,
            maintain_bottom_view_padding,
            ..
        } => RenderKind::SafeArea {
            minimum: *minimum,
            left: *left,
            top: *top,
            right: *right,
            bottom: *bottom,
            maintain_bottom_view_padding: *maintain_bottom_view_padding,
        },
        WidgetKind::ClipRect {
            clip_behavior,
            clipper,
            ..
        } => RenderKind::ClipRect {
            clip_behavior: *clip_behavior,
            clipper: clipper.clone(),
        },
        WidgetKind::ClipRRect {
            radius,
            directional_radius,
            clip_behavior,
            clipper,
            ..
        } => {
            let text_direction = directional_radius.and_then(|_| context.depend::<TextDirection>());
            let radius = directional_radius.map_or(*radius, |value| {
                value
                    .resolve(text_direction.unwrap_or(TextDirection::Ltr))
                    .to_corner_radii()
            });
            RenderKind::ClipRRect {
                radius,
                text_direction,
                clip_behavior: *clip_behavior,
                clipper: clipper.clone(),
            }
        }
        WidgetKind::ClipOval {
            clip_behavior,
            clipper,
            ..
        } => RenderKind::ClipOval {
            clip_behavior: *clip_behavior,
            clipper: clipper.clone(),
        },
        WidgetKind::ClipPath {
            path,
            clip_behavior,
            clipper,
            ..
        } => RenderKind::ClipPath {
            path: path.clone(),
            clip_behavior: *clip_behavior,
            clipper: clipper.clone(),
        },
        WidgetKind::CustomLayout { delegate, .. } => RenderKind::CustomLayout {
            delegate: delegate.clone(),
        },
        WidgetKind::LayoutId { id, .. } => RenderKind::LayoutId { id: id.clone() },
        WidgetKind::Positioned {
            left,
            top,
            right,
            bottom,
            width,
            height,
            ..
        } => RenderKind::Positioned {
            left: *left,
            top: *top,
            right: *right,
            bottom: *bottom,
            width: *width,
            height: *height,
        },
        WidgetKind::IndexedStack {
            alignment,
            index,
            fit,
            clip_behavior,
            ..
        } => RenderKind::IndexedStack {
            alignment: resolve_stack_alignment(*alignment, context),
            index: *index,
            fit: *fit,
            clip_behavior: *clip_behavior,
        },
        WidgetKind::LayoutBuilder { .. } => RenderKind::LayoutBuilder,
        WidgetKind::Visibility {
            visible, hidden, ..
        } => RenderKind::Visibility {
            visible: *visible,
            maintain_size: hidden.layout == HiddenLayout::PreserveSpace,
            interactive: hidden.interactivity,
        },
        WidgetKind::AspectRatio { ratio, .. } => RenderKind::AspectRatio { ratio: *ratio },
        WidgetKind::SizeObserver { observer, .. } => RenderKind::SizeObserver {
            observer: observer.clone(),
        },
        _ => unreachable!("layout lowering received a non-layout widget"),
    }
}
