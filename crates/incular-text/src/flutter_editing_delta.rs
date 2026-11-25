//! Flutter's engine text-delta envelope, kept separate from Incular's
//! byte-addressed editor operation.

use std::fmt;

use serde_json::Value;

use crate::{TextAffinity, TextEditingValue, TextRange, TextSelection};

/// The inferred change sent by Flutter's platform text-input model. Offsets
/// in this wire model count UTF-16 code units, not UTF-8 bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlutterTextEdit {
    Insertion { offset: usize, text: String },
    Deletion { range: TextRange },
    Replacement { range: TextRange, text: String },
    NonTextUpdate,
}

/// A Flutter engine delta with the platform's own pre-edit snapshot.
/// Applying it is last-write-wins; it does not rebase onto a newer local text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlutterTextEditingDelta {
    pub old_text: String,
    pub edit: FlutterTextEdit,
    /// Base and extent in UTF-16 order. `None` is Flutter's invalid selection.
    pub selection: Option<TextSelection>,
    pub selection_affinity: TextAffinity,
    pub selection_is_directional: bool,
    /// The resulting composing range in UTF-16 units.
    pub composing: Option<TextRange>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlutterTextDeltaError {
    InvalidField(&'static str),
    /// A non-sentinel range was reversed, out of bounds, or split a scalar.
    InvalidRange(&'static str),
}

impl fmt::Display for FlutterTextDeltaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidField(name) => write!(f, "invalid Flutter text delta field {name}"),
            Self::InvalidRange(name) => {
                write!(f, "invalid UTF-16 {name} range in Flutter text delta")
            }
        }
    }
}

impl std::error::Error for FlutterTextDeltaError {}

impl FlutterTextEditingDelta {
    /// Flutter `TextEditingDelta.fromJSON`. An IME may send an entire
    /// composing replacement even for insertion/deletion at its end.
    ///
    /// # Errors
    /// Returns a typed error for a missing/invalid field or a range that
    /// Rust strings cannot represent, including a split surrogate.
    pub fn from_json(value: &Value) -> Result<Self, FlutterTextDeltaError> {
        use FlutterTextDeltaError::{InvalidField, InvalidRange};

        let text = |key| {
            value
                .get(key)
                .and_then(Value::as_str)
                .ok_or(InvalidField(key))
        };
        let int = |key, default| match value.get(key) {
            None | Some(Value::Null) if default => Ok(-1),
            Some(value) => value.as_i64().ok_or(InvalidField(key)),
            _ => Err(InvalidField(key)),
        };
        let range = |base, extent| {
            let (base, extent) = (int(base, true)?, int(extent, true)?);
            Ok::<_, FlutterTextDeltaError>(match (usize::try_from(base), usize::try_from(extent)) {
                (Ok(base), Ok(extent)) => Some(TextRange::new(base, extent)),
                _ => None,
            })
        };

        let old_text = text("oldText")?;
        let replacement = text("deltaText")?;
        let start = int("deltaStart", false)?;
        let end = int("deltaEnd", false)?;
        let edit = if start == -1 && end == -1 {
            FlutterTextEdit::NonTextUpdate
        } else {
            let span = TextRange::new(
                start.try_into().map_err(|_| InvalidRange("replacement"))?,
                end.try_into().map_err(|_| InvalidRange("replacement"))?,
            );
            let bytes = byte_range([old_text, "", ""], span, "replacement")?;
            let replaced = &old_text[bytes];
            let old_len = span.end - span.start;
            let new_len = replacement.encode_utf16().count();
            if replaced == replacement {
                FlutterTextEdit::NonTextUpdate
            } else if replacement.is_empty() {
                FlutterTextEdit::Deletion { range: span }
            } else if old_len == new_len + 1 && replaced.starts_with(replacement) {
                FlutterTextEdit::Deletion {
                    range: TextRange::new(span.end - 1, span.end),
                }
            } else if let Some(inserted) = replacement.strip_prefix(replaced) {
                FlutterTextEdit::Insertion {
                    offset: span.end,
                    text: inserted.to_owned(),
                }
            } else {
                FlutterTextEdit::Replacement {
                    range: span,
                    text: replacement.to_owned(),
                }
            }
        };

        let selection_affinity = match value.get("selectionAffinity") {
            None | Some(Value::Null) => TextAffinity::Downstream,
            Some(Value::String(value)) if value == "TextAffinity.upstream" => {
                TextAffinity::Upstream
            }
            Some(Value::String(_)) => TextAffinity::Downstream,
            _ => return Err(InvalidField("selectionAffinity")),
        };
        let selection_is_directional = match value.get("selectionIsDirectional") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(value)) => *value,
            _ => return Err(InvalidField("selectionIsDirectional")),
        };
        let decoded = Self {
            old_text: old_text.to_owned(),
            edit,
            selection: range("selectionBase", "selectionExtent")?
                .map(|range| TextSelection::new(range.start, range.end)),
            selection_affinity,
            selection_is_directional,
            composing: range("composingBase", "composingExtent")?,
        };
        decoded.result_ranges(decoded.result_parts()?)?;
        Ok(decoded)
    }

    /// The platform text removed by a deletion or replacement.
    ///
    /// # Errors
    /// A manually constructed range may not address UTF-16 boundaries in
    /// the saved text.
    pub fn removed_text(&self) -> Result<Option<&str>, FlutterTextDeltaError> {
        let range = match self.edit {
            FlutterTextEdit::Deletion { range } | FlutterTextEdit::Replacement { range, .. } => {
                range
            }
            _ => return Ok(None),
        };
        let bytes = byte_range([&self.old_text, "", ""], range, "replacement")?;
        Ok(Some(&self.old_text[bytes]))
    }

    /// Applies the platform snapshot and converts it to Incular UTF-8 byte
    /// offsets. An unspecified platform selection becomes an end caret;
    /// the original validity, affinity and direction remain on the envelope.
    ///
    /// # Errors
    /// A manually constructed delta may still refer to an invalid UTF-16
    /// range, in which case nothing is applied.
    pub fn try_apply(
        &self,
        _current: &TextEditingValue,
    ) -> Result<TextEditingValue, FlutterTextDeltaError> {
        let parts = self.result_parts()?;
        let (selection, composing) = self.result_ranges(parts)?;
        let text = parts.concat();
        Ok(TextEditingValue {
            selection: selection.unwrap_or_else(|| TextSelection::collapsed(text.len())),
            text,
            composing,
        })
    }

    /// Validate without building a second full text snapshot on decoding.
    fn result_parts(&self) -> Result<[&str; 3], FlutterTextDeltaError> {
        let old = self.old_text.as_str();
        let (range, replacement) = match &self.edit {
            FlutterTextEdit::Insertion { offset, text } => {
                (TextRange::collapsed(*offset), text.as_str())
            }
            FlutterTextEdit::Deletion { range } => (*range, ""),
            FlutterTextEdit::Replacement { range, text } => (*range, text.as_str()),
            FlutterTextEdit::NonTextUpdate => return Ok([old, "", ""]),
        };
        let bytes = byte_range([old, "", ""], range, "replacement")?;
        Ok([&old[..bytes.start], replacement, &old[bytes.end..]])
    }

    fn result_ranges(
        &self,
        parts: [&str; 3],
    ) -> Result<(Option<TextSelection>, Option<TextRange>), FlutterTextDeltaError> {
        let selection = self
            .selection
            .map(|selection| {
                let base = byte_at(parts, selection.base, "selection")?;
                let extent = byte_at(parts, selection.extent, "selection")?;
                Ok::<_, FlutterTextDeltaError>(TextSelection::new(base, extent))
            })
            .transpose()?;
        let composing = self
            .composing
            .map(|range| {
                byte_range(parts, range, "composing")
                    .map(|bytes| TextRange::new(bytes.start, bytes.end))
            })
            .transpose()?;
        Ok((selection, composing))
    }
}

fn byte_range(
    parts: [&str; 3],
    span: TextRange,
    label: &'static str,
) -> Result<std::ops::Range<usize>, FlutterTextDeltaError> {
    if span.start > span.end {
        return Err(FlutterTextDeltaError::InvalidRange(label));
    }
    Ok(byte_at(parts, span.start, label)?..byte_at(parts, span.end, label)?)
}

fn byte_at(
    parts: [&str; 3],
    offset: usize,
    label: &'static str,
) -> Result<usize, FlutterTextDeltaError> {
    let mut units = 0;
    let mut bytes = 0;
    for ch in parts.into_iter().flat_map(str::chars) {
        if units == offset {
            return Ok(bytes);
        }
        if units > offset {
            break;
        }
        units += ch.len_utf16();
        bytes += ch.len_utf8();
    }
    if units == offset {
        Ok(bytes)
    } else {
        Err(FlutterTextDeltaError::InvalidRange(label))
    }
}
