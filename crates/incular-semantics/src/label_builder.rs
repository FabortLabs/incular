//! Composition of multi-part accessibility labels.

use std::borrow::Cow;

use incular_core::TextDirection;

/// Builds a single accessibility label from ordered, nonempty parts.
///
/// The default separator is a space. With an overall direction, any
/// subsequent part whose explicit direction differs is wrapped in Unicode
/// directional embedding marks. The first part is always left unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticsLabelBuilder {
    separator: Cow<'static, str>,
    text_direction: Option<TextDirection>,
    value: String,
    len: usize,
}

impl Default for SemanticsLabelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticsLabelBuilder {
    /// Starts an empty label using space separators and no overall direction.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            separator: Cow::Borrowed(" "),
            text_direction: None,
            value: String::new(),
            len: 0,
        }
    }

    /// Starts an empty label with a separator and optional overall direction.
    #[must_use]
    pub fn with_options(
        separator: impl Into<Cow<'static, str>>,
        text_direction: Option<TextDirection>,
    ) -> Self {
        Self {
            separator: separator.into(),
            text_direction,
            ..Self::new()
        }
    }

    /// Returns the separator used between nonempty parts.
    #[must_use]
    pub fn separator(&self) -> &str {
        &self.separator
    }

    /// Returns the optional overall text direction.
    #[must_use]
    pub const fn text_direction(&self) -> Option<TextDirection> {
        self.text_direction
    }

    /// Appends a part that uses the overall direction. Empty parts are ignored.
    pub fn add_part(&mut self, label: &str) {
        self.add_part_with_direction(label, None);
    }

    /// Appends a part with an optional direction override.
    pub fn add_part_with_direction(&mut self, label: &str, direction: Option<TextDirection>) {
        if label.is_empty() {
            return;
        }
        if self.len != 0 {
            self.value.push_str(&self.separator);
        }
        if self.len != 0
            && let (Some(overall), Some(part)) = (self.text_direction, direction)
            && overall != part
        {
            self.value.push(match part {
                TextDirection::Ltr => '\u{202a}',
                TextDirection::Rtl => '\u{202b}',
            });
            self.value.push_str(label);
            self.value.push('\u{202c}');
        } else {
            self.value.push_str(label);
        }
        self.len += 1;
    }

    /// Returns the number of nonempty parts appended since the last clear.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether no nonempty parts have been appended.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the current label without clearing the builder.
    #[must_use]
    pub fn build(&self) -> String {
        self.value.clone()
    }

    /// Clears all parts while preserving allocation and options for reuse.
    pub fn clear(&mut self) {
        self.value.clear();
        self.len = 0;
    }
}
