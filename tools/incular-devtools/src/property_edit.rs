//! Pure value logic behind the typed property editors: parsing, formatting,
//! clamping and stepping, the editor kind chosen for a property, original-value
//! tracking for per-property revert, and the edit-rate throttle.

use incular_devtools_protocol::{DebugProperty, DebugValue, DevWidgetId};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Largest edit rate sent to the target while a stepper or drag is spammed.
pub const EDIT_INTERVAL: Duration = Duration::from_millis(33);

/// Twelve preset swatches offered by the colour popover.
pub const PALETTE: [&str; 12] = [
    "#FFFFFF", "#000000", "#9E9E9E", "#F44336", "#FF9800", "#FFC107", "#4CAF50", "#009688",
    "#2196F3", "#3F51B5", "#9C27B0", "#E91E63",
];

/// Enum properties with at most this many options use a segmented control.
pub const SEGMENTED_MAX: usize = 4;

/// Renders a number without trailing zeros (at most three decimals).
pub fn format_float(value: f64) -> String {
    let rounded = (value * 1000.).round() / 1000.;
    let text = format!("{rounded:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() || text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}

/// Parses a finite number; `∞` and empty input are not numbers.
pub fn parse_float(input: &str) -> Option<f64> {
    input
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

/// Clamps to the inclusive bounds the target advertises.
pub fn clamp_to(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    let value = min.map_or(value, |min| value.max(min));
    max.map_or(value, |max| value.min(max))
}

/// One keyboard or stepper increment: `direction` is +1 or -1, `fast` is the
/// Shift multiplier. The result snaps to the step grid (no float drift such as
/// 0.15000000000000002) and is clamped to the bounds.
pub fn step_value(
    value: f64,
    direction: f64,
    step: f64,
    fast: bool,
    min: Option<f64>,
    max: Option<f64>,
) -> f64 {
    let step = if step.is_finite() && step > 0. {
        step
    } else {
        1.
    };
    let amount = if fast { step * 10. } else { step };
    let next = value + direction * amount;
    let snapped = (next / step).round() * step;
    // Round away binary noise, then keep within bounds.
    let snapped = (snapped * 1e6).round() / 1e6;
    clamp_to(snapped, min, max)
}

/// Parses `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA` (the `#` is optional).
pub fn parse_color(input: &str) -> Option<(u8, u8, u8, u8)> {
    let input = input.trim();
    let hex = input.strip_prefix('#').unwrap_or(input);
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |index: usize| {
        u8::from_str_radix(&hex[index..=index], 16)
            .ok()
            .map(|v| v * 17)
    };
    let byte = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).ok();
    match hex.len() {
        3 => Some((nibble(0)?, nibble(1)?, nibble(2)?, 255)),
        4 => Some((nibble(0)?, nibble(1)?, nibble(2)?, nibble(3)?)),
        6 => Some((byte(0)?, byte(2)?, byte(4)?, 255)),
        8 => Some((byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
        _ => None,
    }
}

/// `#RRGGBB` when opaque, `#RRGGBBAA` otherwise.
pub fn format_color(red: u8, green: u8, blue: u8, alpha: u8) -> String {
    if alpha == 255 {
        format!("#{red:02X}{green:02X}{blue:02X}")
    } else {
        format!("#{red:02X}{green:02X}{blue:02X}{alpha:02X}")
    }
}

/// How an insets edit lands: with `linked`, one field sets all four sides.
pub fn apply_inset_edit(
    current: [f32; 4],
    side: usize,
    value: f32,
    linked: bool,
    min: Option<f64>,
    max: Option<f64>,
) -> [f32; 4] {
    let value = clamp_to(f64::from(value), min, max) as f32;
    if linked {
        [value; 4]
    } else {
        let mut next = current;
        next[side.min(3)] = value;
        next
    }
}

/// Whether all four sides are equal, the default state of "link all".
pub fn insets_uniform(insets: [f32; 4]) -> bool {
    insets.iter().all(|side| (side - insets[0]).abs() < 1e-4)
}

/// The nine alignment options a 3×3 grid can place.
pub fn is_alignment_grid(options: &[String]) -> bool {
    options.len() == 9
        && [
            "TopLeft",
            "TopCenter",
            "TopRight",
            "CenterLeft",
            "Center",
            "CenterRight",
        ]
        .iter()
        .chain(["BottomLeft", "BottomCenter", "BottomRight"].iter())
        .all(|name| options.iter().any(|option| option == name))
}

/// Position of an alignment name in the 3×3 grid as `(column, row)`.
pub fn alignment_cell(name: &str) -> Option<(usize, usize)> {
    let row = if name.starts_with("Top") {
        0
    } else if name.starts_with("Bottom") {
        2
    } else if name.starts_with("Center") {
        1
    } else {
        return None;
    };
    let column = if name.ends_with("Left") {
        0
    } else if name.ends_with("Right") {
        2
    } else if name.ends_with("Center") || name == "Center" {
        1
    } else {
        return None;
    };
    Some((column, row))
}

/// Alignment option name for a grid cell.
pub fn alignment_name(column: usize, row: usize) -> &'static str {
    match (row.min(2), column.min(2)) {
        (0, 0) => "TopLeft",
        (0, 1) => "TopCenter",
        (0, 2) => "TopRight",
        (1, 0) => "CenterLeft",
        (1, 1) => "Center",
        (1, 2) => "CenterRight",
        (2, 0) => "BottomLeft",
        (2, 1) => "BottomCenter",
        _ => "BottomRight",
    }
}

/// The editor presented for one editable property.
#[derive(Clone, Debug, PartialEq)]
pub enum EditorKind {
    Float {
        min: Option<f64>,
        max: Option<f64>,
        step: f64,
    },
    /// A number that may be unbounded (`Optional(None)` shows as ∞).
    OptionalFloat {
        min: Option<f64>,
        max: Option<f64>,
        step: f64,
    },
    Color,
    OptionalColor,
    Segmented(Vec<String>),
    Menu(Vec<String>),
    AlignmentGrid(Vec<String>),
    Insets {
        min: Option<f64>,
        max: Option<f64>,
        step: f64,
    },
    Bool,
    Text,
    /// Any other editable value, edited as raw text through
    /// [`crate::inspector::parse_debug_value`].
    Raw,
}

/// Chooses the editor for a property from its value shape and metadata.
pub fn editor_kind(property: &DebugProperty) -> EditorKind {
    let step = property.step.filter(|step| *step > 0.).unwrap_or(1.);
    let (min, max) = (property.min, property.max);
    match &property.value {
        DebugValue::Float(_) => EditorKind::Float { min, max, step },
        DebugValue::Int(_) | DebugValue::Uint(_) => EditorKind::Raw,
        DebugValue::Color(..) => EditorKind::Color,
        DebugValue::Bool(_) => EditorKind::Bool,
        DebugValue::Str(_) => EditorKind::Text,
        DebugValue::Insets(_) => EditorKind::Insets { min, max, step },
        DebugValue::Enum(_) if !property.options.is_empty() => {
            if is_alignment_grid(&property.options) {
                EditorKind::AlignmentGrid(property.options.clone())
            } else if property.options.len() <= SEGMENTED_MAX {
                EditorKind::Segmented(property.options.clone())
            } else {
                EditorKind::Menu(property.options.clone())
            }
        }
        DebugValue::Optional(inner) => match inner.as_deref() {
            Some(DebugValue::Color(..)) => EditorKind::OptionalColor,
            Some(DebugValue::Float(_)) | None if property_is_numeric(property) => {
                EditorKind::OptionalFloat { min, max, step }
            }
            None if property.name.to_lowercase().contains("color")
                || property.name == "background" =>
            {
                EditorKind::OptionalColor
            }
            _ => EditorKind::Raw,
        },
        _ => EditorKind::Raw,
    }
}

fn property_is_numeric(property: &DebugProperty) -> bool {
    matches!(
        property.value,
        DebugValue::Optional(Some(ref value)) if matches!(**value, DebugValue::Float(_))
    ) || property.min.is_some()
        || property.max.is_some()
        || property.name.to_lowercase().contains("width")
        || property.name.to_lowercase().contains("height")
}

/// The text shown in a numeric field for a value.
pub fn float_text(value: &DebugValue) -> Option<String> {
    match value {
        DebugValue::Float(value) => Some(format_float(*value)),
        DebugValue::Optional(Some(inner)) => float_text(inner),
        DebugValue::Optional(None) => Some("∞".into()),
        _ => None,
    }
}

/// The numeric value inside a (possibly optional) float property.
pub fn float_of(value: &DebugValue) -> Option<f64> {
    match value {
        DebugValue::Float(value) => Some(*value),
        DebugValue::Optional(Some(inner)) => float_of(inner),
        _ => None,
    }
}

/// The colour inside a (possibly optional) colour property.
pub fn color_of(value: &DebugValue) -> Option<(u8, u8, u8, u8)> {
    match value {
        DebugValue::Color(r, g, b, a) => Some((*r, *g, *b, *a)),
        DebugValue::Optional(Some(inner)) => color_of(inner),
        _ => None,
    }
}

/// Whether two values are the same up to numeric noise.
pub fn values_equal(a: &DebugValue, b: &DebugValue) -> bool {
    match (a, b) {
        (DebugValue::Float(a), DebugValue::Float(b)) => (a - b).abs() < 1e-6,
        (DebugValue::Insets(a), DebugValue::Insets(b)) => {
            a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-4)
        }
        (DebugValue::Optional(a), DebugValue::Optional(b)) => match (a, b) {
            (Some(a), Some(b)) => values_equal(a, b),
            (None, None) => true,
            _ => false,
        },
        _ => a == b,
    }
}

/// Stable key of a property edit session (widget plus property name).
pub fn property_key(id: DevWidgetId, name: &str) -> String {
    format!("{id}|{name}")
}

/// First-seen value of every property edited since the last "Reset all".
///
/// The target has no per-property revert request, so reverting a property
/// means sending its original value back.
#[derive(Debug, Default)]
pub struct OriginalValues {
    values: HashMap<String, DebugValue>,
}

impl OriginalValues {
    /// Remembers `current` as the original unless the property was already edited.
    pub fn record(&mut self, key: &str, current: &DebugValue) {
        self.values
            .entry(key.to_owned())
            .or_insert_with(|| current.clone());
    }

    pub fn original(&self, key: &str) -> Option<&DebugValue> {
        self.values.get(key)
    }

    /// The value to send to revert, if the property was edited from here.
    pub fn revert_value(&self, key: &str) -> Option<DebugValue> {
        self.values.get(key).cloned()
    }

    /// Whether the property differs from its original. A property the target
    /// reports as overridden but this session never edited counts as modified.
    pub fn is_modified(&self, key: &str, overridden: bool, current: &DebugValue) -> bool {
        match self.values.get(key) {
            Some(original) => !values_equal(original, current),
            None => overridden,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }
}

/// What to do with an edit offered to the [`EditThrottle`].
#[derive(Clone, Debug, PartialEq)]
pub enum Offer {
    /// Send this value now.
    SendNow(DebugValue),
    /// Rate-limited. When `flush_after` is set the caller must call
    /// [`EditThrottle::flush`] after that delay; the latest value wins.
    Deferred { flush_after: Option<Duration> },
}

#[derive(Debug, Default)]
struct Slot {
    last_sent: Option<Instant>,
    pending: Option<DebugValue>,
    flush_scheduled: bool,
}

/// Leading-and-trailing rate limiter per property.
#[derive(Debug)]
pub struct EditThrottle {
    interval: Duration,
    slots: HashMap<String, Slot>,
}

impl Default for EditThrottle {
    fn default() -> Self {
        Self::new(EDIT_INTERVAL)
    }
}

impl EditThrottle {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            slots: HashMap::new(),
        }
    }

    pub fn offer(&mut self, key: &str, value: DebugValue, now: Instant) -> Offer {
        let slot = self.slots.entry(key.to_owned()).or_default();
        let ready = slot
            .last_sent
            .is_none_or(|sent| now.duration_since(sent) >= self.interval);
        if ready && slot.pending.is_none() {
            slot.last_sent = Some(now);
            return Offer::SendNow(value);
        }
        slot.pending = Some(value);
        if slot.flush_scheduled {
            return Offer::Deferred { flush_after: None };
        }
        slot.flush_scheduled = true;
        let remaining = slot.last_sent.map_or(Duration::ZERO, |sent| {
            self.interval.saturating_sub(now.duration_since(sent))
        });
        Offer::Deferred {
            flush_after: Some(remaining),
        }
    }

    /// Takes the newest deferred value, counting it as sent at `now`.
    pub fn flush(&mut self, key: &str, now: Instant) -> Option<DebugValue> {
        let slot = self.slots.get_mut(key)?;
        slot.flush_scheduled = false;
        let value = slot.pending.take()?;
        slot.last_sent = Some(now);
        Some(value)
    }

    /// Drops a deferred value (a revert supersedes it).
    pub fn cancel(&mut self, key: &str, now: Instant) {
        if let Some(slot) = self.slots.get_mut(key) {
            slot.pending = None;
            slot.last_sent = Some(now);
        }
    }
}
