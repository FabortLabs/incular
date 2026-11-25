//! Live editing formatters and selection/composition remapping.

use incular_config::TargetPlatform;
use incular_text::{TextEditingValue, TextRange, TextSelection, byte_offset_at_utf16_boundary};
use std::{ops::Range, rc::Rc};

/// A contract for formatting and validating live text edits.
pub trait TextInputFormatter {
    /// Formats an edit update from `old_value` to `new_value`.
    fn format_edit_update(
        &self,
        old_value: &TextEditingValue,
        new_value: &TextEditingValue,
    ) -> TextEditingValue;
}

/// Formats user edits through a callback, receiving both unmodified values.
pub struct CallbackTextInputFormatter<F>(F);

impl<F> CallbackTextInputFormatter<F> {
    #[must_use]
    pub const fn new(callback: F) -> Self {
        Self(callback)
    }
}

impl<F> TextInputFormatter for CallbackTextInputFormatter<F>
where
    F: Fn(&TextEditingValue, &TextEditingValue) -> TextEditingValue,
{
    fn format_edit_update(
        &self,
        old_value: &TextEditingValue,
        new_value: &TextEditingValue,
    ) -> TextEditingValue {
        (self.0)(old_value, new_value)
    }
}

#[derive(Clone)]
enum FilterPattern {
    Scalar(Rc<dyn Fn(char) -> bool>),
    Literal(Rc<str>),
    Regex(Rc<regex::Regex>),
}

/// Replaces disallowed matches, or each gap between allowed matches.
/// Offsets remain UTF-8 bytes while equal-length replacement behavior is
/// defined in UTF-16 code units. Regex constructors use Rust regex syntax.
#[derive(Clone)]
pub struct FilteringTextInputFormatter {
    allow: bool,
    filter: FilterPattern,
    replacement: Option<Rc<str>>,
}

impl FilteringTextInputFormatter {
    fn new(allow: bool, filter: FilterPattern) -> Self {
        Self {
            allow,
            filter,
            replacement: None,
        }
    }

    #[must_use]
    pub fn allow(filter: impl Fn(char) -> bool + 'static) -> Self {
        Self::new(true, FilterPattern::Scalar(Rc::new(filter)))
    }

    #[must_use]
    pub fn deny(filter: impl Fn(char) -> bool + 'static) -> Self {
        Self::new(false, FilterPattern::Scalar(Rc::new(filter)))
    }

    /// Keeps each non-overlapping literal match and replaces everything between.
    #[must_use]
    pub fn allow_literal(pattern: impl Into<Rc<str>>) -> Self {
        Self::new(true, FilterPattern::Literal(pattern.into()))
    }

    /// Replaces each non-overlapping literal match.
    #[must_use]
    pub fn deny_literal(pattern: impl Into<Rc<str>>) -> Self {
        Self::new(false, FilterPattern::Literal(pattern.into()))
    }

    pub fn allow_regex(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self::new(
            true,
            FilterPattern::Regex(Rc::new(regex::Regex::new(pattern)?)),
        ))
    }

    pub fn deny_regex(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self::new(
            false,
            FilterPattern::Regex(Rc::new(regex::Regex::new(pattern)?)),
        ))
    }

    /// Sets the literal replacement text. The default is the empty string.
    #[must_use]
    pub fn replacement_string(mut self, replacement: impl Into<Rc<str>>) -> Self {
        let replacement = replacement.into();
        self.replacement = (!replacement.is_empty()).then_some(replacement);
        self
    }

    #[must_use]
    pub fn digits_only() -> Self {
        Self::allow(|c| c.is_ascii_digit())
    }

    #[must_use]
    pub fn single_line_formatter() -> Self {
        Self::deny(|c| c == '\n')
    }
}

impl TextInputFormatter for FilteringTextInputFormatter {
    fn format_edit_update(
        &self,
        _old_value: &TextEditingValue,
        new_value: &TextEditingValue,
    ) -> TextEditingValue {
        let source = &new_value.text;
        match &self.filter {
            FilterPattern::Scalar(predicate) if self.replacement.is_none() => {
                self.filter_scalars(new_value, predicate.as_ref())
            }
            FilterPattern::Scalar(predicate) => {
                let matches = source.char_indices().filter_map(|(offset, ch)| {
                    predicate(ch).then(|| offset..offset + ch.len_utf8())
                });
                self.filter_matches(new_value, matches)
            }
            FilterPattern::Literal(literal) => self.filter_matches(
                new_value,
                source
                    .match_indices(literal.as_ref())
                    .map(|(offset, value)| offset..offset + value.len()),
            ),
            FilterPattern::Regex(regex) => self.filter_matches(
                new_value,
                regex.find_iter(source).map(|found| found.range()),
            ),
        }
    }
}

impl FilteringTextInputFormatter {
    fn filter_scalars(
        &self,
        new_value: &TextEditingValue,
        predicate: &dyn Fn(char) -> bool,
    ) -> TextEditingValue {
        let source = &new_value.text;
        let selection = new_value.selection.clamp_to(source);
        let composing = new_value
            .composing
            .filter(|range| range.is_valid())
            .map(|range| range.clamp_to(source));
        let mut filtered = String::new();
        let mut base = selection.base;
        let mut extent = selection.extent;
        let mut mapped_composing = composing;

        // Compare with the original endpoints, not the already-shifted ones.
        // After clamping, an endpoint either precedes a removed scalar or
        // follows it; it cannot land inside that scalar's UTF-8 bytes.
        for (offset, c) in source.char_indices() {
            if predicate(c) == self.allow {
                filtered.push(c);
                continue;
            }
            let width = c.len_utf8();
            if offset < selection.base {
                base -= width;
            }
            if offset < selection.extent {
                extent -= width;
            }
            if let Some((original, mapped)) = composing.zip(mapped_composing.as_mut()) {
                if offset < original.start {
                    mapped.start -= width;
                }
                if offset < original.end {
                    mapped.end -= width;
                }
            }
        }

        TextEditingValue {
            text: filtered,
            selection: TextSelection::new(base, extent),
            // Flutter reports a collapsed composing region as TextRange.empty.
            composing: mapped_composing.filter(|range| !range.is_empty()),
        }
    }

    fn filter_matches(
        &self,
        value: &TextEditingValue,
        matches: impl IntoIterator<Item = Range<usize>>,
    ) -> TextEditingValue {
        let source = &value.text;
        let selection = value.selection.clamp_to(source);
        let composing = value
            .composing
            .filter(|range| range.is_valid())
            .map(|range| range.clamp_to(source));
        let original = [
            Some(selection.base),
            Some(selection.extent),
            composing.map(|range| range.start),
            composing.map(|range| range.end),
        ];
        let mut mapped = [None; 4];
        let mut output = String::with_capacity(source.len());
        let replacement = self.replacement.as_deref().unwrap_or_default();
        let replacement_units = replacement.encode_utf16().count();
        let mut process = |range: Range<usize>, banned: bool| {
            if range.is_empty() {
                return;
            }
            let old = &source[range.clone()];
            let start = output.len();
            let next = if banned { replacement } else { old };
            output.push_str(next);
            let mut same_units = None;
            for (original, mapped) in original.into_iter().zip(&mut mapped) {
                let Some(offset) = original else { continue };
                if mapped.is_some() || offset < range.start || offset > range.end {
                    continue;
                }
                *mapped = Some(if !banned {
                    start + offset - range.start
                } else if offset == range.start {
                    start
                } else if offset == range.end {
                    output.len()
                } else if *same_units
                    .get_or_insert_with(|| old.encode_utf16().count() == replacement_units)
                {
                    let relative = old[..offset - range.start].encode_utf16().count();
                    start + byte_offset_at_utf16_boundary(replacement, relative)
                } else {
                    output.len()
                });
            }
        };
        let mut end = 0;
        for range in matches {
            process(end..range.start, self.allow);
            end = range.end;
            process(range, !self.allow);
        }
        process(end..source.len(), self.allow);
        let [base, extent, begin, end] = mapped;
        TextEditingValue {
            text: output,
            selection: TextSelection::new(base.unwrap_or(0), extent.unwrap_or(0)),
            composing: begin
                .zip(end)
                .map(|(begin, end)| TextRange::new(begin, end))
                .filter(|range| !range.is_empty()),
        }
    }
}

/// Enforcement policy for maximum text length.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MaxLengthEnforcement {
    /// Preserve text regardless of length; normalize editing ranges only.
    None,
    /// Enforce the limit on every update, including active composition.
    #[default]
    Enforced,
    /// Allow a valid composing range to exceed the limit until it ends.
    TruncateAfterCompositionEnds,
}

/// Limits text to extended grapheme clusters (user-perceived characters).
/// Offsets remain UTF-8 byte offsets. The incoming selection is clamped to the
/// output. Invalid composition ranges are removed; truncation also discards a
/// composition that collapses. When an over-limit
/// edit is rejected, the previous value is returned unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LengthLimitingTextInputFormatter {
    pub max_length: usize,
    pub max_length_enforcement: MaxLengthEnforcement,
}

impl LengthLimitingTextInputFormatter {
    #[must_use]
    pub const fn new(max_length: usize) -> Self {
        Self::with_enforcement(
            max_length,
            Self::default_enforcement(TargetPlatform::host(), cfg!(target_arch = "wasm32")),
        )
    }

    /// The host's default length policy. Web uses deferred enforcement
    /// regardless of the underlying keyboard platform.
    #[must_use]
    pub const fn default_enforcement(
        platform: TargetPlatform,
        is_web: bool,
    ) -> MaxLengthEnforcement {
        if !is_web && matches!(platform, TargetPlatform::Android | TargetPlatform::Windows) {
            MaxLengthEnforcement::Enforced
        } else {
            MaxLengthEnforcement::TruncateAfterCompositionEnds
        }
    }

    /// Truncates to the first `max_length` extended grapheme clusters.
    /// Range endpoints are sorted and clamped to the retained text.
    #[must_use]
    pub fn truncate(value: TextEditingValue, max_length: usize) -> TextEditingValue {
        let segmenter = icu_segmenter::GraphemeClusterSegmenter::new();
        let end = segmenter
            .segment_str(&value.text)
            .nth(max_length)
            .unwrap_or(value.text.len());
        Self::truncate_at(value, end)
    }

    fn truncate_at(mut value: TextEditingValue, end: usize) -> TextEditingValue {
        value.text.truncate(end);
        let selection = value.selection.clamp_to(&value.text);
        value.selection = TextSelection::new(
            selection.base.min(selection.extent),
            selection.base.max(selection.extent),
        );
        value.composing = value
            .composing
            .filter(|range| range.is_valid())
            .map(|range| range.clamp_to(&value.text))
            .filter(|range| !range.is_empty());
        value
    }

    #[must_use]
    pub const fn with_enforcement(max_length: usize, enforcement: MaxLengthEnforcement) -> Self {
        Self {
            max_length,
            max_length_enforcement: enforcement,
        }
    }
}

impl TextInputFormatter for LengthLimitingTextInputFormatter {
    fn format_edit_update(
        &self,
        old_value: &TextEditingValue,
        new_value: &TextEditingValue,
    ) -> TextEditingValue {
        let mut value = new_value.clone();
        value.composing = value
            .composing
            .filter(|range| range.is_valid())
            .map(|range| range.clamp_to(&value.text));
        // Each grapheme cluster is at least one byte, so text no longer than
        // `max_length` bytes never needs segmenting.
        if self.max_length_enforcement != MaxLengthEnforcement::None
            && value.text.len() > self.max_length
        {
            let segmenter = icu_segmenter::GraphemeClusterSegmenter::new();
            let end = segmenter
                .segment_str(&value.text)
                .nth(self.max_length)
                .unwrap_or(value.text.len());
            if end < value.text.len() {
                let (reject_at_limit, truncate) = match self.max_length_enforcement {
                    MaxLengthEnforcement::None => (false, false),
                    MaxLengthEnforcement::Enforced => (old_value.selection.is_collapsed(), true),
                    MaxLengthEnforcement::TruncateAfterCompositionEnds => {
                        let was_composing =
                            old_value.composing.is_some_and(|range| range.is_valid());
                        (!was_composing, value.composing.is_none())
                    }
                };
                if reject_at_limit
                    && self.max_length != 0
                    && old_value.text.len() >= self.max_length
                    && segmenter.segment_str(&old_value.text).nth(self.max_length)
                        == Some(old_value.text.len())
                {
                    return old_value.clone();
                }
                if truncate {
                    return Self::truncate_at(value, end);
                }
            }
        }
        value.selection = value.selection.clamp_to(&value.text);
        value
    }
}
