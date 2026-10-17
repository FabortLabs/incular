//! Typed, temporary DevTools property edits for built-in widget kinds.
//!
//! An edit mutates a clone of the retained descriptor and is then reconciled
//! through the same update path a parent rebuild uses, so each property gets
//! exactly the invalidation its render kind reports (paint, layout or
//! composite). Applied edits are remembered per element and re-applied when
//! the application's own rebuild replaces the descriptor, until reverted.

use crate::tree::WidgetKind;
use incular_config::{
    Alignment, Constraints, CrossAxisAlignment, EdgeInsets, MainAxisAlignment, MainAxisSize,
};
use incular_core::Color;
use incular_devtools_protocol::DebugValue;
use incular_rendering::{Border, Brush, CornerRadii};

/// Result of one property edit request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevEdit {
    /// Unknown property, wrong value type, out-of-range value, stale target.
    Rejected,
    /// Accepted, but the retained value already matched.
    Unchanged,
    /// The retained element was updated and invalidated.
    Applied,
}

pub(crate) const MAIN_AXIS_ALIGNMENTS: &[&str] = &[
    "Start",
    "End",
    "Center",
    "SpaceBetween",
    "SpaceAround",
    "SpaceEvenly",
];
pub(crate) const CROSS_AXIS_ALIGNMENTS: &[&str] =
    &["Start", "End", "Center", "Stretch", "Baseline"];
pub(crate) const MAIN_AXIS_SIZES: &[&str] = &["Min", "Max"];
pub(crate) const ALIGNMENTS: &[&str] = &[
    "TopLeft",
    "TopCenter",
    "TopRight",
    "CenterLeft",
    "Center",
    "CenterRight",
    "BottomLeft",
    "BottomCenter",
    "BottomRight",
];

pub(crate) fn alignment_name(alignment: Alignment) -> Option<&'static str> {
    ALIGNMENTS
        .iter()
        .copied()
        .find(|name| parse_alignment(name) == Some(alignment))
}

fn parse_alignment(name: &str) -> Option<Alignment> {
    Some(match name {
        "TopLeft" => Alignment::TOP_LEFT,
        "TopCenter" => Alignment::TOP_CENTER,
        "TopRight" => Alignment::TOP_RIGHT,
        "CenterLeft" => Alignment::CENTER_LEFT,
        "Center" => Alignment::CENTER,
        "CenterRight" => Alignment::CENTER_RIGHT,
        "BottomLeft" => Alignment::BOTTOM_LEFT,
        "BottomCenter" => Alignment::BOTTOM_CENTER,
        "BottomRight" => Alignment::BOTTOM_RIGHT,
        _ => return None,
    })
}

fn parse_main_axis(name: &str) -> Option<MainAxisAlignment> {
    Some(match name {
        "Start" => MainAxisAlignment::Start,
        "End" => MainAxisAlignment::End,
        "Center" => MainAxisAlignment::Center,
        "SpaceBetween" => MainAxisAlignment::SpaceBetween,
        "SpaceAround" => MainAxisAlignment::SpaceAround,
        "SpaceEvenly" => MainAxisAlignment::SpaceEvenly,
        _ => return None,
    })
}

fn parse_cross_axis(name: &str) -> Option<CrossAxisAlignment> {
    Some(match name {
        "Start" => CrossAxisAlignment::Start,
        "End" => CrossAxisAlignment::End,
        "Center" => CrossAxisAlignment::Center,
        "Stretch" => CrossAxisAlignment::Stretch,
        "Baseline" => CrossAxisAlignment::Baseline,
        _ => return None,
    })
}

fn parse_main_size(name: &str) -> Option<MainAxisSize> {
    Some(match name {
        "Min" => MainAxisSize::Min,
        "Max" => MainAxisSize::Max,
        _ => return None,
    })
}

fn finite(value: &DebugValue) -> Option<f32> {
    let number = match value {
        DebugValue::Float(value) => *value,
        DebugValue::Int(value) => *value as f64,
        DebugValue::Uint(value) => *value as f64,
        _ => return None,
    };
    (number.is_finite() && number.abs() < 1.0e7).then_some(number as f32)
}

fn non_negative(value: &DebugValue) -> Option<f32> {
    finite(value).filter(|number| *number >= 0.)
}

fn color(value: &DebugValue) -> Option<Color> {
    match value {
        DebugValue::Color(red, green, blue, alpha) => {
            Some(Color::rgba(*red, *green, *blue, *alpha))
        }
        _ => None,
    }
}

fn enum_name(value: &DebugValue) -> Option<&str> {
    match value {
        DebugValue::Enum(name) | DebugValue::Str(name) => Some(name),
        _ => None,
    }
}

fn set<T: PartialEq>(slot: &mut T, next: T) -> Option<bool> {
    if *slot == next {
        Some(false)
    } else {
        *slot = next;
        Some(true)
    }
}

/// An unbounded maximum is edited as `Optional(None)` (or any non-finite).
fn maximum(value: &DebugValue) -> Option<f32> {
    match value {
        DebugValue::Optional(None) => Some(f32::INFINITY),
        other => non_negative(other),
    }
}

/// Applies `name = value` to a descriptor kind. `None` rejects the edit;
/// `Some(changed)` reports whether the descriptor differs afterwards.
pub(crate) fn apply_edit(kind: &mut WidgetKind, name: &str, value: &DebugValue) -> Option<bool> {
    match (kind, name) {
        (WidgetKind::Text { text, .. }, "text") => {
            let DebugValue::Str(next) = value else {
                return None;
            };
            if next.chars().count() > 4096 {
                return None;
            }
            set(text, next.clone())
        }
        (WidgetKind::Text { style, .. }, "fontSize") => {
            let size = finite(value)?.clamp(1., 512.);
            let changed = set(&mut style.size, size)?;
            style.has_explicit_size = true;
            Some(changed)
        }
        (WidgetKind::Text { style, .. }, "color") => {
            let next = color(value)?;
            let changed = set(&mut style.color, next)?;
            style.has_explicit_color = true;
            Some(changed)
        }
        (WidgetKind::Box { color: slot, .. }, "color") => set(slot, color(value)?),
        (WidgetKind::Decorated { background, .. }, "background") => match value {
            DebugValue::Optional(None) => set(background, None),
            other => {
                if background
                    .as_ref()
                    .is_some_and(|brush| !matches!(brush, Brush::Solid(_)))
                {
                    return None;
                }
                set(background, Some(Brush::Solid(color(other)?)))
            }
        },
        (WidgetKind::Decorated { border, .. }, "borderWidth") => {
            let width = non_negative(value)?.min(256.);
            match border {
                Some(border) if border.width == width => Some(false),
                Some(border) => {
                    border.width = width;
                    Some(true)
                }
                None if width == 0. => Some(false),
                None => {
                    *border = Some(Border::new(width, Color::rgba(136, 136, 136, 255)));
                    Some(true)
                }
            }
        }
        (WidgetKind::Decorated { border, .. }, "borderColor") => {
            let next = color(value)?;
            let border = border.as_mut()?;
            set(&mut border.color, next)
        }
        (WidgetKind::Decorated { radius, .. }, "radius") => set(
            radius,
            CornerRadii::uniform(non_negative(value)?.min(4096.)),
        ),
        (WidgetKind::Padding { padding, .. }, "padding") => {
            let DebugValue::Insets([left, top, right, bottom]) = value else {
                return None;
            };
            if ![left, top, right, bottom]
                .into_iter()
                .all(|edge| edge.is_finite() && *edge >= 0. && *edge < 1.0e7)
            {
                return None;
            }
            set(
                padding,
                EdgeInsets {
                    left: *left,
                    top: *top,
                    right: *right,
                    bottom: *bottom,
                },
            )
        }
        (WidgetKind::Constrained { constraints, .. }, name)
            if matches!(
                name,
                "width" | "height" | "minWidth" | "maxWidth" | "minHeight" | "maxHeight"
            ) =>
        {
            let (mut min_w, mut max_w, mut min_h, mut max_h) = (
                constraints.min_width(),
                constraints.max_width(),
                constraints.min_height(),
                constraints.max_height(),
            );
            match name {
                "width" => {
                    let width = non_negative(value)?;
                    (min_w, max_w) = (width, width);
                }
                "height" => {
                    let height = non_negative(value)?;
                    (min_h, max_h) = (height, height);
                }
                "minWidth" => min_w = non_negative(value)?,
                "maxWidth" => max_w = maximum(value)?,
                "minHeight" => min_h = non_negative(value)?,
                _ => max_h = maximum(value)?,
            }
            let next = Constraints::try_new(min_w, max_w, min_h, max_h).ok()?;
            set(constraints, next)
        }
        (
            WidgetKind::Flex {
                main_axis_alignment,
                ..
            },
            "mainAxisAlignment",
        ) => set(main_axis_alignment, parse_main_axis(enum_name(value)?)?),
        (
            WidgetKind::Flex {
                cross_axis_alignment,
                ..
            },
            "crossAxisAlignment",
        ) => set(cross_axis_alignment, parse_cross_axis(enum_name(value)?)?),
        (WidgetKind::Flex { main_axis_size, .. }, "mainAxisSize") => {
            set(main_axis_size, parse_main_size(enum_name(value)?)?)
        }
        (WidgetKind::Flex { spacing, .. }, "spacing") => {
            set(spacing, non_negative(value)?.min(4096.))
        }
        (WidgetKind::Wrap { spacing, .. }, "spacing") => {
            set(spacing, non_negative(value)?.min(4096.))
        }
        (WidgetKind::Wrap { run_spacing, .. }, "runSpacing") => {
            set(run_spacing, non_negative(value)?.min(4096.))
        }
        (WidgetKind::Align { alignment, .. }, "alignment") => {
            set(alignment, parse_alignment(enum_name(value)?)?)
        }
        (
            WidgetKind::Opacity {
                alpha,
                controller: None,
                ..
            },
            "opacity",
        ) => set(alpha, finite(value)?.clamp(0., 1.)),
        (WidgetKind::Visibility { visible, .. }, "visible") => {
            let DebugValue::Bool(next) = value else {
                return None;
            };
            set(visible, *next)
        }
        _ => None,
    }
}
