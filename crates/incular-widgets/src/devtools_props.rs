//! Structured inspection properties for built-in widget kinds.
//!
//! This is Incular's equivalent of Flutter's diagnostics properties: a closed,
//! typed contract rather than `Debug` dumps. Only these curated values cross
//! the DevTools wire; sensitive content is excluded at the source.

use crate::devtools_edit::{
    ALIGNMENTS, CROSS_AXIS_ALIGNMENTS, MAIN_AXIS_ALIGNMENTS, MAIN_AXIS_SIZES, alignment_name,
};
use crate::tree::WidgetKind;
use crate::util::truncate;
use incular_devtools_protocol::{DebugProperty, DebugValue, PropertyChange};
fn prop(name: &str, value: DebugValue) -> DebugProperty {
    DebugProperty {
        name: name.to_owned(),
        value,
        overridden: false,
        editable: false,
        options: Vec::new(),
        min: None,
        max: None,
        step: None,
    }
}

fn editable(name: &str, value: DebugValue) -> DebugProperty {
    let mut property = prop(name, value);
    property.editable = true;
    property
}

fn number(name: &str, value: f32, min: f64, max: f64, step: f64) -> DebugProperty {
    let mut property = editable(name, DebugValue::Float(f64::from(value)));
    property.min = Some(min);
    property.max = Some(max);
    property.step = Some(step);
    property
}

fn choice(name: &str, current: String, options: &[&str]) -> DebugProperty {
    let mut property = editable(name, DebugValue::Enum(current));
    property.options = options.iter().map(|option| (*option).to_owned()).collect();
    property
}

fn color_value(color: incular_core::Color) -> DebugValue {
    DebugValue::Color(color.red, color.green, color.blue, color.alpha)
}

fn size_value(size: incular_core::Size) -> DebugValue {
    DebugValue::Size([size.width, size.height])
}

fn offset_value(offset: incular_core::Offset) -> DebugValue {
    DebugValue::Offset([offset.x, offset.y])
}

/// Extracts the curated diagnostic properties for one widget kind. The
/// returned list is bounded; large collections are summarized.
pub fn inspect_properties(kind: &WidgetKind) -> Vec<DebugProperty> {
    let mut out = Vec::new();
    match kind {
        WidgetKind::Text {
            text,
            style,
            align,
            soft_wrap,
            max_lines,
            overflow,
            text_direction,
        } => {
            if let Some(direction) = text_direction {
                out.push(prop(
                    "textDirection",
                    DebugValue::Enum(format!("{direction:?}")),
                ));
            }
            let mut text_property = prop("text", DebugValue::Str(truncate(text, 120)));
            text_property.editable = text.chars().count() <= 120;
            out.push(text_property);
            out.push(number("fontSize", style.size, 1., 200., 1.));
            out.push(editable("color", color_value(style.color)));
            if let Some(height) = style.line_height {
                let val = match height {
                    incular_text::LineHeight::Normal => 0.0,
                    incular_text::LineHeight::Multiplier(m) => m as f64,
                    incular_text::LineHeight::Absolute(a) => a as f64,
                };
                out.push(prop("lineHeight", DebugValue::Float(val)));
            }
            out.push(prop(
                "fontWeight",
                DebugValue::Enum(format!("{:?}", style.weight)),
            ));
            out.push(prop("alignment", DebugValue::Enum(format!("{align:?}"))));
            out.push(prop("softWrap", DebugValue::Bool(*soft_wrap)));
            out.push(match max_lines {
                Some(lines) => prop("maxLines", DebugValue::Uint(*lines as u64)),
                None => prop("maxLines", DebugValue::Optional(None)),
            });
            out.push(prop("overflow", DebugValue::Enum(format!("{overflow:?}"))));
        }
        WidgetKind::Button(spec) => {
            out.push(prop("enabled", DebugValue::Bool(spec.callback.is_some())));
            if let Some(child) = spec.child.as_ref()
                && let Some(label) = child.text_if_any()
            {
                out.push(prop("label", DebugValue::Str(truncate(&label, 48))));
            }
        }
        WidgetKind::SelectionArea { controller, child } => {
            out.push(prop(
                "selectionRevision",
                DebugValue::Uint(controller.revision()),
            ));
            out.push(prop(
                "registeredChildCount",
                DebugValue::Uint(controller.registered_child_count() as u64),
            ));
            summarize_child(&mut out, child);
        }
        WidgetKind::SelectionContainer { delegate, child } => {
            out.push(prop("enabled", DebugValue::Bool(delegate.is_enabled())));
            out.push(prop(
                "selectionRevision",
                DebugValue::Uint(delegate.controller().revision()),
            ));
            summarize_child(&mut out, child);
        }
        WidgetKind::SelectionListener {
            notifier, child, ..
        } => {
            out.push(prop("registered", DebugValue::Bool(notifier.registered())));
            out.push(prop(
                "selectionRevision",
                DebugValue::Uint(notifier.revision()),
            ));
            summarize_child(&mut out, child);
        }
        WidgetKind::IndexedSemantics { index, child } => {
            out.push(prop("index", DebugValue::Uint(*index as u64)));
            summarize_child(&mut out, child);
        }
        WidgetKind::SemanticsDebugger {
            label_style,
            max_nodes,
            child,
        } => {
            out.push(prop("maxNodes", DebugValue::Uint(*max_nodes as u64)));
            out.push(prop(
                "fontSize",
                DebugValue::Float(f64::from(label_style.size)),
            ));
            summarize_child(&mut out, child);
        }
        WidgetKind::Flex {
            axis,
            children,
            main_axis_alignment,
            main_axis_size,
            cross_axis_alignment,
            spacing,
            ..
        } => {
            out.push(prop("direction", DebugValue::Enum(format!("{axis:?}"))));
            out.push(choice(
                "mainAxisAlignment",
                format!("{main_axis_alignment:?}"),
                MAIN_AXIS_ALIGNMENTS,
            ));
            out.push(choice(
                "crossAxisAlignment",
                format!("{cross_axis_alignment:?}"),
                CROSS_AXIS_ALIGNMENTS,
            ));
            out.push(choice(
                "mainAxisSize",
                format!("{main_axis_size:?}"),
                MAIN_AXIS_SIZES,
            ));
            out.push(number("spacing", *spacing, 0., 200., 1.));
            out.push(prop("childCount", DebugValue::Uint(children.len() as u64)));
        }
        WidgetKind::Wrap {
            axis,
            spacing,
            run_spacing,
            children,
            ..
        } => {
            out.push(prop("direction", DebugValue::Enum(format!("{axis:?}"))));
            out.push(number("spacing", *spacing, 0., 200., 1.));
            out.push(number("runSpacing", *run_spacing, 0., 200., 1.));
            out.push(prop("childCount", DebugValue::Uint(children.len() as u64)));
        }
        WidgetKind::Stack {
            alignment,
            children,
            ..
        } => {
            out.push(prop(
                "alignment",
                DebugValue::Enum(format!("{alignment:?}")),
            ));
            out.push(prop("childCount", DebugValue::Uint(children.len() as u64)));
        }
        WidgetKind::Padding { padding, child } => {
            let mut property = editable(
                "padding",
                DebugValue::Insets([padding.left, padding.top, padding.right, padding.bottom]),
            );
            property.min = Some(0.);
            property.max = Some(200.);
            property.step = Some(1.);
            out.push(property);
            summarize_child(&mut out, child);
        }
        WidgetKind::Align {
            alignment, child, ..
        } => {
            match alignment_name(*alignment) {
                Some(name) => out.push(choice("alignment", name.to_owned(), ALIGNMENTS)),
                None => out.push(prop(
                    "alignment",
                    DebugValue::Enum(format!("{alignment:?}")),
                )),
            }
            summarize_child(&mut out, child);
        }
        WidgetKind::Decorated {
            background,
            border,
            radius,
            child,
            ..
        } => {
            match background {
                Some(brush) => {
                    let mut property = prop("background", brush_summary(brush));
                    property.editable = matches!(brush, incular_rendering::Brush::Solid(_));
                    out.push(property);
                }
                None => out.push(editable("background", DebugValue::Optional(None))),
            }
            out.push(number(
                "borderWidth",
                border.map_or(0., |border| border.width),
                0.,
                32.,
                0.5,
            ));
            if let Some(border) = border {
                out.push(editable("borderColor", color_value(border.color)));
            }
            out.push(number("radius", radius.top_left, 0., 200., 1.));
            summarize_child(&mut out, child);
        }
        WidgetKind::Opacity {
            alpha,
            controller,
            child,
        } => {
            let mut opacity = number("opacity", *alpha, 0., 1., 0.05);
            opacity.editable = controller.is_none();
            out.push(opacity);
            summarize_child(&mut out, child);
        }
        WidgetKind::Blur {
            sigma_x,
            sigma_y,
            child,
            ..
        } => {
            out.push(prop("sigmaX", DebugValue::Float(f64::from(*sigma_x))));
            out.push(prop("sigmaY", DebugValue::Float(f64::from(*sigma_y))));
            summarize_child(&mut out, child);
        }
        WidgetKind::Translate {
            controller, child, ..
        } => {
            let offset = controller.offset();
            out.push(prop("offset", offset_value(offset)));
            summarize_child(&mut out, child);
        }
        WidgetKind::Scale {
            controller, child, ..
        } => {
            out.push(prop(
                "scale",
                DebugValue::Float(f64::from(controller.scale())),
            ));
            summarize_child(&mut out, child);
        }
        WidgetKind::Rotation {
            controller, child, ..
        } => {
            out.push(prop(
                "angle",
                DebugValue::Float(f64::from(controller.angle_degrees())),
            ));
            summarize_child(&mut out, child);
        }
        WidgetKind::Image {
            image,
            width,
            height,
            sampling,
            ..
        } => {
            out.push(prop("imageId", DebugValue::Uint(image.id().0)));
            if let Some(width) = width {
                out.push(prop("width", DebugValue::Float(f64::from(*width))));
            }
            if let Some(height) = height {
                out.push(prop("height", DebugValue::Float(f64::from(*height))));
            }
            out.push(prop("sampling", DebugValue::Enum(format!("{sampling:?}"))));
        }
        WidgetKind::Scroll { controller, .. } => {
            out.push(prop(
                "scrollOffset",
                DebugValue::Float(f64::from(controller.offset())),
            ));
            out.push(prop(
                "maxOffset",
                DebugValue::Float(f64::from(controller.max_offset())),
            ));
            out.push(prop(
                "viewportExtent",
                DebugValue::Float(f64::from(controller.viewport_extent())),
            ));
            out.push(prop(
                "contentExtent",
                DebugValue::Float(f64::from(controller.content_extent())),
            ));
        }
        WidgetKind::SliverViewport { config } => {
            out.push(prop(
                "itemCount",
                DebugValue::Uint(config.delegate.child_count().unwrap_or(0) as u64),
            ));
            out.push(prop(
                "sliverCount",
                DebugValue::Uint(config.delegate.sliver_count() as u64),
            ));
            out.push(prop(
                "cacheExtent",
                DebugValue::Float(f64::from(config.cache_extent)),
            ));
        }
        WidgetKind::Visibility {
            visible,
            hidden,
            child,
        } => {
            out.push(editable("visible", DebugValue::Bool(*visible)));
            out.push(prop(
                "maintainSize",
                DebugValue::Bool(hidden.layout == crate::tree::HiddenLayout::PreserveSpace),
            ));
            out.push(prop(
                "maintainAnimation",
                DebugValue::Bool(hidden.animation),
            ));
            out.push(prop(
                "maintainSemantics",
                DebugValue::Bool(hidden.semantics),
            ));
            summarize_child(&mut out, child);
        }
        WidgetKind::Gesture {
            callbacks, child, ..
        } => {
            out.push(prop("hasTap", DebugValue::Bool(callbacks.on_tap.is_some())));
            out.push(prop(
                "hasDrag",
                DebugValue::Bool(
                    callbacks.on_horizontal_drag_update.is_some()
                        || callbacks.on_vertical_drag_update.is_some(),
                ),
            ));
            summarize_child(&mut out, child);
        }
        WidgetKind::RawInput { kind, child } => {
            out.push(prop(
                "inputType",
                DebugValue::Str(kind.type_().name().to_owned()),
            ));
            if let Some(child) = child {
                summarize_child(&mut out, child);
            }
        }
        WidgetKind::Box { size, color } => {
            out.push(prop("size", size_value(*size)));
            out.push(editable("color", color_value(*color)));
        }
        WidgetKind::Constrained { constraints, child } => {
            out.push(prop(
                "constraints",
                DebugValue::Constraints {
                    min_width: constraints.min_width(),
                    max_width: constraints.max_width(),
                    min_height: constraints.min_height(),
                    max_height: constraints.max_height(),
                },
            ));
            constrained_axis(
                &mut out,
                ("width", "minWidth", "maxWidth"),
                constraints.min_width(),
                constraints.max_width(),
            );
            constrained_axis(
                &mut out,
                ("height", "minHeight", "maxHeight"),
                constraints.min_height(),
                constraints.max_height(),
            );
            summarize_child(&mut out, child);
        }
        _ => {}
    }
    out
}

/// Produces a bounded, structured property diff for a compatible retained
/// widget update. Only curated inspectable properties participate: arbitrary
/// application objects are never recursively serialized for DevTools.
pub fn diff_properties(old: &WidgetKind, new: &WidgetKind) -> Vec<PropertyChange> {
    let old = inspect_properties(old)
        .into_iter()
        .map(|property| (property.name, property.value))
        .collect::<std::collections::BTreeMap<_, _>>();
    let new = inspect_properties(new)
        .into_iter()
        .map(|property| (property.name, property.value))
        .collect::<std::collections::BTreeMap<_, _>>();
    old.keys()
        .chain(new.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter_map(|name| {
            let before = old.get(name).cloned();
            let after = new.get(name).cloned();
            (before != after).then(|| PropertyChange {
                name: name.clone(),
                old: before,
                new: after,
            })
        })
        .take(32)
        .collect()
}

/// A tight axis (SizedBox) edits as one extent; otherwise as min and max.
fn constrained_axis(
    out: &mut Vec<DebugProperty>,
    (extent, min_name, max_name): (&str, &str, &str),
    min: f32,
    max: f32,
) {
    if min == max {
        out.push(number(extent, min, 0., 2000., 1.));
        return;
    }
    out.push(number(min_name, min, 0., 2000., 1.));
    if max.is_finite() {
        out.push(number(max_name, max, 0., 2000., 1.));
    } else {
        let mut unbounded = editable(max_name, DebugValue::Optional(None));
        unbounded.min = Some(0.);
        unbounded.max = Some(2000.);
        unbounded.step = Some(1.);
        out.push(unbounded);
    }
}

fn summarize_child(out: &mut Vec<DebugProperty>, child: &crate::tree::Widget) {
    if let WidgetKind::Text { text, .. } = child.kind() {
        out.push(prop("child", DebugValue::Str(truncate(text, 24))));
    } else {
        out.push(prop(
            "child",
            DebugValue::Str(kind_display_name(child.kind())),
        ));
    }
}

fn brush_summary(brush: &incular_rendering::Brush) -> DebugValue {
    match brush {
        incular_rendering::Brush::Solid(color) => color_value(*color),
        other => DebugValue::Str(format!("{other:?}")),
    }
}

/// Display name for any widget kind (single source used by tree + props).
pub fn kind_display_name(kind: &WidgetKind) -> String {
    match kind {
        WidgetKind::Flex { axis, .. } => match axis {
            incular_config::Axis::Vertical => "Column".to_owned(),
            incular_config::Axis::Horizontal => "Row".to_owned(),
        },
        _ => kind.structure().widget_type.name().to_owned(),
    }
}
