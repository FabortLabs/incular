use super::super::super::*;

/// Resolves a widget's optional explicit text direction against the nearest
/// ambient `Directionality`.
///
/// Flex and Wrap validate required directionality during build in debug.
/// Table also supports an LTR fallback for context-free Incular layouts.
fn resolve_layout_direction(
    explicit: Option<TextDirection>,
    context: &DependencyContext,
) -> TextDirection {
    explicit
        .or_else(|| context.depend::<TextDirection>())
        .unwrap_or(TextDirection::Ltr)
}

fn resolve_box_alignment(
    alignment: crate::AlignmentGeometry,
    context: &DependencyContext,
) -> Alignment {
    match alignment {
        crate::AlignmentGeometry::Physical(value) => value,
        value => value.resolve(resolve_layout_direction(None, context)),
    }
}

fn resolve_stack_alignment(
    alignment: Option<crate::AlignmentGeometry>,
    direction: Option<TextDirection>,
    context: &DependencyContext,
) -> Alignment {
    match alignment.unwrap_or_else(|| incular_config::AlignmentDirectional::TOP_START.into()) {
        crate::AlignmentGeometry::Physical(alignment) => alignment,
        alignment => alignment.resolve(resolve_layout_direction(direction, context)),
    }
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
            fit,
            ..
        } => RenderKind::Overflow {
            min_width: *min_width,
            max_width: *max_width,
            min_height: *min_height,
            max_height: *max_height,
            alignment: resolve_box_alignment(*alignment, context),
            fit: *fit,
        },
        WidgetKind::SizedOverflow {
            size, alignment, ..
        } => RenderKind::SizedOverflow {
            size: *size,
            alignment: resolve_box_alignment(*alignment, context),
        },
        WidgetKind::Unconstrained {
            constrained_axis,
            alignment,
            text_direction,
            transform,
            clip_behavior,
            ..
        } => RenderKind::Unconstrained {
            constrained_axis: *constrained_axis,
            alignment: match alignment {
                crate::AlignmentGeometry::Physical(value) => *value,
                value => value.resolve(resolve_layout_direction(*text_direction, context)),
            },
            transform: transform.clone(),
            clip_behavior: *clip_behavior,
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
            alignment: resolve_box_alignment(*alignment, context),
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
            clip_behavior,
            ..
        } => RenderKind::Flex {
            clip_behavior: *clip_behavior,
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
        WidgetKind::ListBody { axis, reverse, .. } => {
            let direction = match axis {
                Axis::Horizontal => {
                    let rtl = resolve_layout_direction(None, context) == TextDirection::Rtl;
                    if rtl ^ *reverse {
                        AxisDirection::Left
                    } else {
                        AxisDirection::Right
                    }
                }
                Axis::Vertical => {
                    if *reverse {
                        AxisDirection::Up
                    } else {
                        AxisDirection::Down
                    }
                }
            };
            RenderKind::ListBody { direction }
        }
        WidgetKind::OverflowBar {
            spacing,
            overflow_spacing,
            overflow_alignment,
            alignment,
            overflow_direction,
            text_direction,
            ..
        } => RenderKind::OverflowBar {
            spacing: *spacing,
            overflow_spacing: *overflow_spacing,
            overflow_alignment: *overflow_alignment,
            alignment: *alignment,
            overflow_direction: *overflow_direction,
            text_direction: resolve_layout_direction(*text_direction, context),
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
            clip_behavior,
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
            clip_behavior: *clip_behavior,
        },
        WidgetKind::Table {
            columns,
            column_spacing,
            row_spacing,
            column_widths,
            border,
            default_column_width,
            default_vertical_alignment,
            text_direction,
            ..
        } => RenderKind::Table {
            columns: *columns,
            column_spacing: *column_spacing,
            row_spacing: *row_spacing,
            column_widths: column_widths.clone(),
            border: border.clone(),
            default_column_width: *default_column_width,
            default_vertical_alignment: *default_vertical_alignment,
            text_direction: resolve_layout_direction(*text_direction, context),
        },
        WidgetKind::Stack {
            alignment,
            text_direction,
            fit,
            clip_behavior,
            ..
        } => RenderKind::Stack {
            stack: incular_layout::Stack {
                alignment: resolve_stack_alignment(*alignment, *text_direction, context),
                fit: *fit,
                clip_behavior: *clip_behavior,
            },
        },
        WidgetKind::ClipRect {
            clip_behavior,
            clipper,
            ..
        } => RenderKind::ClipRect {
            clip_behavior: *clip_behavior,
            clipper: clipper.as_ref().map(crate::layout::ClipDelegate::tracked),
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
                clipper: clipper.as_ref().map(crate::layout::ClipDelegate::tracked),
            }
        }
        WidgetKind::ClipOval {
            clip_behavior,
            clipper,
            ..
        } => RenderKind::ClipOval {
            clip_behavior: *clip_behavior,
            clipper: clipper.as_ref().map(crate::layout::ClipDelegate::tracked),
        },
        WidgetKind::ClipPath {
            path,
            clip_behavior,
            clipper,
            ..
        } => RenderKind::ClipPath {
            path: path.clone(),
            clip_behavior: *clip_behavior,
            clipper: clipper.as_ref().map(crate::layout::ClipDelegate::tracked),
        },
        WidgetKind::CustomLayout { delegate, .. } => RenderKind::CustomLayout {
            delegate: delegate.tracked(),
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
            text_direction,
            index,
            fit,
            clip_behavior,
            ..
        } => RenderKind::IndexedStack {
            alignment: resolve_stack_alignment(*alignment, *text_direction, context),
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
