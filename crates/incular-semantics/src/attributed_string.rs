//! Platform-neutral text with accessibility pronunciation attributes.

use std::{
    error::Error,
    fmt,
    ops::{Add, Range},
};

/// An accessibility pronunciation annotation. Each range uses UTF-16 code
/// units so it can be mapped to platform text ranges without shifting spans
/// after supplementary characters.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StringAttribute {
    /// Requests individual character pronunciation.
    SpellOut { range: Range<usize> },
    /// Requests pronunciation in a BCP 47 language tag.
    Locale { range: Range<usize>, locale: String },
}

impl StringAttribute {
    #[must_use]
    pub fn spell_out(range: Range<usize>) -> Self {
        Self::SpellOut { range }
    }

    #[must_use]
    pub fn locale(range: Range<usize>, language_tag: impl Into<String>) -> Self {
        Self::Locale {
            range,
            locale: language_tag.into(),
        }
    }

    #[must_use]
    pub fn range(&self) -> &Range<usize> {
        match self {
            Self::SpellOut { range } | Self::Locale { range, .. } => range,
        }
    }

    /// Copies this annotation with a different UTF-16 range.
    #[must_use]
    pub fn with_range(&self, range: Range<usize>) -> Self {
        match self {
            Self::SpellOut { .. } => Self::spell_out(range),
            Self::Locale { locale, .. } => Self::locale(range, locale.clone()),
        }
    }

    fn shift(&mut self, offset: usize) {
        let range = match self {
            Self::SpellOut { range } | Self::Locale { range, .. } => range,
        };
        range.start += offset;
        range.end += offset;
    }
}

impl fmt::Display for StringAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let range = self.range();
        let (start, end) = (range.start, range.end);
        match self {
            Self::SpellOut { .. } => write!(
                f,
                "SpellOutStringAttribute(TextRange(start: {start}, end: {end}))"
            ),
            Self::Locale { locale, .. } => write!(
                f,
                "LocaleStringAttribute(TextRange(start: {start}, end: {end}), {locale})"
            ),
        }
    }
}

/// An attribute range that cannot be applied to a given string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidStringAttribute {
    /// Index of the invalid attribute in input order.
    pub attribute_index: usize,
    /// Range supplied for that attribute, in UTF-16 units.
    pub range: Range<usize>,
    /// Length of the string in UTF-16 units.
    pub text_units: usize,
}

impl fmt::Display for InvalidStringAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "attribute {} has range {:?} outside the {} UTF-16-unit string",
            self.attribute_index, self.range, self.text_units
        )
    }
}
impl Error for InvalidStringAttribute {}

/// An accessibility string and its ordered pronunciation attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AttributedString {
    string: String,
    attributes: Vec<StringAttribute>,
}

impl AttributedString {
    /// Creates plain text without pronunciation attributes.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            string: text.into(),
            attributes: Vec::new(),
        }
    }

    /// Validates UTF-16 ranges and stores attributes in the supplied order.
    pub fn try_with_attributes(
        text: impl Into<String>,
        attributes: impl IntoIterator<Item = StringAttribute>,
    ) -> Result<Self, InvalidStringAttribute> {
        let string = text.into();
        let attributes: Vec<_> = attributes.into_iter().collect();
        if !attributes.is_empty() {
            let text_units = string.encode_utf16().count();
            for (attribute_index, attr) in attributes.iter().enumerate() {
                let range = attr.range();
                if text_units == 0 || range.start > range.end || range.end > text_units {
                    return Err(InvalidStringAttribute {
                        attribute_index,
                        range: range.clone(),
                        text_units,
                    });
                }
            }
        }
        Ok(Self { string, attributes })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.string
    }

    #[must_use]
    pub fn attributes(&self) -> &[StringAttribute] {
        &self.attributes
    }

    /// Splits the string and attributes without cloning.
    #[must_use]
    pub fn into_parts(self) -> (String, Vec<StringAttribute>) {
        (self.string, self.attributes)
    }

    /// Appends another string, shifting its ranges by the left operand's
    /// UTF-16 length and preserving the attribute order within each.
    #[must_use]
    pub fn concat(mut self, mut other: Self) -> Self {
        if self.string.is_empty() {
            return other;
        }
        if other.string.is_empty() {
            return self;
        }
        if !other.attributes.is_empty() {
            let offset = self.string.encode_utf16().count();
            for attr in &mut other.attributes {
                attr.shift(offset);
            }
            self.attributes.append(&mut other.attributes);
        }
        self.string.push_str(&other.string);
        self
    }
}

impl From<String> for AttributedString {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<&str> for AttributedString {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<&String> for AttributedString {
    fn from(text: &String) -> Self {
        Self::new(text)
    }
}

impl From<Box<str>> for AttributedString {
    fn from(text: Box<str>) -> Self {
        Self::new(text)
    }
}

impl From<std::borrow::Cow<'_, str>> for AttributedString {
    fn from(text: std::borrow::Cow<'_, str>) -> Self {
        Self::new(text)
    }
}

/// Speech attributes for the three text fields of a semantic node.
/// All ranges are relative to the corresponding plain text in UTF-16 units.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SemanticTextAttributes {
    /// Attributes of the accessible label.
    pub label: Vec<StringAttribute>,
    /// Attributes of the accessible value.
    pub value: Vec<StringAttribute>,
    /// Attributes of the description or hint.
    pub hint: Vec<StringAttribute>,
}

impl SemanticTextAttributes {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.label.is_empty() && self.value.is_empty() && self.hint.is_empty()
    }
}

impl Add for AttributedString {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        self.concat(other)
    }
}

impl fmt::Display for AttributedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AttributedString('{}', attributes: [", self.string)?;
        for (i, attr) in self.attributes.iter().enumerate() {
            if i != 0 {
                f.write_str(", ")?;
            }
            write!(f, "{attr}")?;
        }
        f.write_str("])")
    }
}
