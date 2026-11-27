//! Plain and rich text descriptions.

use crate::{
    TextAffinity, TextAlign, TextEngine, TextLayout, TextLayoutOptions, TextOverflow,
    TextPlaceholder, TextScaler, TextStyle,
};
use incular_config::Locale;
use incular_core::Size;
use std::{fmt, ops::Range, sync::Arc};

/// A widget-independent inline child. The text crate stores a stable identity
/// and measured size rather than depending on `incular-widgets`, which keeps
/// the crate graph acyclic. Widgets can resolve the identity during layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetSpan {
    pub id: u64,
    pub size: Size,
    pub alignment: WidgetSpanAlignment,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WidgetSpanAlignment {
    #[default]
    Baseline,
    AboveBaseline,
    BelowBaseline,
    Middle,
    TextTop,
    TextBottom,
}

impl WidgetSpan {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self {
            id,
            size: Size::ZERO,
            alignment: WidgetSpanAlignment::Baseline,
        }
    }

    #[must_use]
    pub const fn sized(id: u64, size: Size) -> Self {
        Self {
            id,
            size,
            alignment: WidgetSpanAlignment::Baseline,
        }
    }

    #[must_use]
    pub const fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub const fn alignment(mut self, alignment: WidgetSpanAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

/// A node in a rich text tree.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum InlineSpan {
    Text(TextSpan),
    Widget(WidgetSpan),
}

impl From<TextSpan> for InlineSpan {
    fn from(value: TextSpan) -> Self {
        Self::Text(value)
    }
}

impl From<WidgetSpan> for InlineSpan {
    fn from(value: WidgetSpan) -> Self {
        Self::Widget(value)
    }
}

/// The smallest update scope needed after replacing a span tree.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum InlineSpanComparison {
    #[default]
    Identical,
    Metadata,
    Paint,
    Layout,
}

impl InlineSpan {
    #[must_use]
    pub fn compare_to(&self, other: &Self) -> InlineSpanComparison {
        use InlineSpanComparison::{Identical, Layout};
        match (self, other) {
            (Self::Text(a), Self::Text(b)) => a.compare_to(b),
            (Self::Widget(a), Self::Widget(b)) if a == b => Identical,
            _ => Layout,
        }
    }
}

/// A positioned inline span, borrowed from the original paragraph tree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InlineSpanRef<'a> {
    Text(&'a TextSpan),
    Widget(&'a WidgetSpan),
}

/// Controls the flattening of inline text for presentation or diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlainTextOptions {
    pub include_semantics_labels: bool,
    pub include_placeholders: bool,
}

impl Default for PlainTextOptions {
    fn default() -> Self {
        Self {
            include_semantics_labels: true,
            include_placeholders: true,
        }
    }
}

/// Speech metadata for one literal text span or one inline widget marker.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InlineSpanSemanticsInformation<'a> {
    pub text: &'a str,
    pub semantics_label: Option<&'a str>,
    pub semantics_identifier: Option<&'a str>,
    pub is_placeholder: bool,
    pub locale: Option<&'a Locale>,
    pub spell_out: bool,
}

/// A pronunciation property attached to a UTF-16 interval of spoken text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InlineSpanStringAttribute<'a> {
    SpellOut(Range<usize>),
    Locale {
        range: Range<usize>,
        locale: &'a Locale,
    },
}

impl InlineSpanStringAttribute<'_> {
    #[must_use]
    pub fn range(&self) -> &Range<usize> {
        match self {
            Self::SpellOut(range) | Self::Locale { range, .. } => range,
        }
    }

    fn shift(&mut self, units: usize) {
        let range = match self {
            Self::SpellOut(range) | Self::Locale { range, .. } => range,
        };
        range.start += units;
        range.end += units;
    }
}

impl<'a> InlineSpanSemanticsInformation<'a> {
    /// Attributes in dispatch order, measured against the label if present.
    pub fn string_attributes(self) -> impl Iterator<Item = InlineSpanStringAttribute<'a>> {
        let width = if self.spell_out || self.locale.is_some() {
            self.semantics_label
                .unwrap_or(self.text)
                .encode_utf16()
                .count()
        } else {
            0
        };
        [
            (width > 0 && self.spell_out).then_some(InlineSpanStringAttribute::SpellOut(0..width)),
            self.locale
                .filter(|_| width > 0)
                .map(|locale| InlineSpanStringAttribute::Locale {
                    range: 0..width,
                    locale,
                }),
        ]
        .into_iter()
        .flatten()
    }
}

/// One independent semantic fragment with owned text and adjusted ranges.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CombinedInlineSpanSemantics<'a> {
    pub text: String,
    pub label: String,
    pub identifier: Option<&'a str>,
    pub is_placeholder: bool,
    pub string_attributes: Vec<InlineSpanStringAttribute<'a>>,
}

/// Concatenates neighboring speech spans and shifts attributes by the
/// preceding label's UTF-16 length. A placeholder or named span keeps
/// its own fragment and restarts range accounting for the next fragment.
#[must_use]
pub fn combine_inline_span_semantics<'a>(
    spans: &[InlineSpanSemanticsInformation<'a>],
) -> Vec<CombinedInlineSpanSemantics<'a>> {
    let mut result = Vec::new();
    let mut working = CombinedInlineSpanSemantics::default();
    let mut label_units = 0;
    for &span in spans {
        let label = span.semantics_label.unwrap_or(span.text);
        if span.is_placeholder || span.semantics_identifier.is_some() {
            result.push(std::mem::take(&mut working));
            result.push(CombinedInlineSpanSemantics {
                text: span.text.to_owned(),
                label: label.to_owned(),
                identifier: span.semantics_identifier,
                is_placeholder: span.is_placeholder,
                string_attributes: span.string_attributes().collect(),
            });
            label_units = 0;
            continue;
        }
        working.text.push_str(span.text);
        working.label.push_str(label);
        working
            .string_attributes
            .extend(span.string_attributes().map(|mut attribute| {
                attribute.shift(label_units);
                attribute
            }));
        label_units += label.encode_utf16().count();
    }
    result.push(working);
    result
}

/// The recoverable diagnostic emitted for an invalid UTF-16 span.
#[derive(Debug)]
pub struct InvalidUtf16Span {
    pub error: std::string::FromUtf16Error,
}

impl InvalidUtf16Span {
    /// The owner can observe the error without surfacing it as a fatal error.
    #[must_use]
    pub const fn is_silent(&self) -> bool {
        true
    }
}

impl std::fmt::Display for InvalidUtf16Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid UTF-16 while building a TextSpan: {}", self.error)
    }
}

impl std::error::Error for InvalidUtf16Span {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// A recursively nestable text node. Styles are inherited by descendants;
/// inline widget spans occupy one object-replacement character in plain text.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpan {
    pub text: String,
    pub style: Option<TextStyle>,
    pub children: Vec<InlineSpan>,
    /// Flutter `TextSpan.semanticsLabel`: spoken in place of this span's text
    /// when the paragraph is announced.
    pub semantics_label: Option<String>,
    pub semantics_identifier: Option<String>,
    pub locale: Option<Locale>,
    pub spell_out: Option<bool>,
}

impl TextSpan {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: None,
            children: Vec::new(),
            semantics_label: None,
            semantics_identifier: None,
            locale: None,
            spell_out: None,
        }
    }

    /// Imports a text run from a UTF-16 boundary such as a platform
    /// interface. A malformed run produces one silent diagnostic and one
    /// replacement character, not a partially decoded run.
    #[must_use]
    pub fn from_utf16(units: &[u16], report: impl FnOnce(InvalidUtf16Span)) -> Self {
        match String::from_utf16(units) {
            Ok(text) => Self::new(text),
            Err(error) => {
                report(InvalidUtf16Span { error });
                Self::new("\u{fffd}")
            }
        }
    }

    #[must_use]
    pub fn empty() -> Self {
        Self::new("")
    }

    /// Sets the spoken label for this span (Flutter `semanticsLabel`).
    #[must_use]
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }

    /// A platform-independent identifier for this span's semantics.
    #[must_use]
    pub fn semantics_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.semantics_identifier = Some(identifier.into());
        self
    }

    /// Overrides the inherited pronunciation locale for this subtree.
    #[must_use]
    pub fn locale(mut self, locale: Locale) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Overrides inherited character-by-character pronunciation.
    #[must_use]
    pub fn spell_out(mut self, spell_out: bool) -> Self {
        self.spell_out = Some(spell_out);
        self
    }

    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = Some(style);
        self
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<InlineSpan>) -> Self {
        self.children.push(child.into());
        self
    }

    #[must_use]
    pub fn children<I, S>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<InlineSpan>,
    {
        self.children = children.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn with_children<I, S>(text: impl Into<String>, children: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<InlineSpan>,
    {
        Self::new(text).children(children)
    }

    /// Compares span contents and inline reservations without shaping text.
    #[must_use]
    pub fn compare_to(&self, other: &Self) -> InlineSpanComparison {
        use InlineSpanComparison::{Identical, Layout, Metadata, Paint};
        use incular_core::ChangeImpact;

        if std::ptr::eq(self, other) {
            return Identical;
        }
        if self.text != other.text
            || self.children.len() != other.children.len()
            || self.style.is_some() != other.style.is_some()
        {
            return Layout;
        }
        let mut result = if self.semantics_label != other.semantics_label
            || self.semantics_identifier != other.semantics_identifier
            || self.locale != other.locale
            || self.spell_out != other.spell_out
        {
            Metadata
        } else {
            Identical
        };

        if let (Some(a), Some(b)) = (&self.style, &other.style) {
            match a.change_impact(b) {
                ChangeImpact::None => {}
                ChangeImpact::Composite | ChangeImpact::Paint => result = Paint,
                ChangeImpact::Layout | ChangeImpact::Build => return Layout,
            }
        }

        for (a, b) in self.children.iter().zip(&other.children) {
            result = result.max(a.compare_to(b));
            if result == Layout {
                break;
            }
        }
        result
    }

    /// The literal laid-out text, preserving widget positions and ignoring
    /// speech labels. Layout and editor byte offsets use this representation.
    #[must_use]
    pub fn plain_text(&self) -> String {
        self.to_plain_text(PlainTextOptions {
            include_semantics_labels: false,
            include_placeholders: true,
        })
    }

    /// Flutter-style presentation text, optionally using speech labels and
    /// object replacement characters. Use default options to include both.
    #[must_use]
    pub fn to_plain_text(&self, options: PlainTextOptions) -> String {
        let mut output = String::new();
        self.write_plain_text(&mut output, options);
        output
    }

    /// Visits just the immediate children in order, stopping on false.
    pub fn visit_direct_children<'a>(
        &'a self,
        visitor: impl FnMut(&'a InlineSpan) -> bool,
    ) -> bool {
        self.children.iter().all(visitor)
    }

    /// Finds the span covering a Flutter UTF-16 text position. At an
    /// ordinary boundary, upstream selects the preceding text and
    /// downstream selects the following text. A widget occupies one unit.
    #[must_use]
    pub fn span_for_utf16_position(
        &self,
        offset: usize,
        affinity: TextAffinity,
    ) -> Option<InlineSpanRef<'_>> {
        self.find_utf16_position(offset, affinity, &mut 0)
    }

    fn find_utf16_position(
        &self,
        offset: usize,
        affinity: TextAffinity,
        consumed: &mut usize,
    ) -> Option<InlineSpanRef<'_>> {
        if !self.text.is_empty() {
            let start = *consumed;
            let end = start + self.text.encode_utf16().count();
            if (start < offset && offset < end)
                || (start == offset && affinity == TextAffinity::Downstream)
                || (end == offset && affinity == TextAffinity::Upstream)
            {
                return Some(InlineSpanRef::Text(self));
            }
            *consumed = end;
        }
        for child in &self.children {
            match child {
                InlineSpan::Text(span) => {
                    if let Some(hit) = span.find_utf16_position(offset, affinity, consumed) {
                        return Some(hit);
                    }
                }
                InlineSpan::Widget(span) => {
                    if offset == *consumed {
                        return Some(InlineSpanRef::Widget(span));
                    }
                    *consumed += 1;
                }
            }
        }
        None
    }

    /// Returns literal text and speech metadata in paragraph traversal order.
    #[must_use]
    pub fn semantics_information(&self) -> Vec<InlineSpanSemanticsInformation<'_>> {
        let mut result = Vec::new();
        self.collect_semantics_information(None, false, &mut result);
        result
    }

    fn collect_semantics_information<'a>(
        &'a self,
        inherited_locale: Option<&'a Locale>,
        inherited_spell_out: bool,
        output: &mut Vec<InlineSpanSemanticsInformation<'a>>,
    ) {
        let locale = self.locale.as_ref().or(inherited_locale);
        let spell_out = self.spell_out.unwrap_or(inherited_spell_out);
        if !self.text.is_empty()
            || self.semantics_label.is_some()
            || self.semantics_identifier.is_some()
        {
            output.push(InlineSpanSemanticsInformation {
                text: &self.text,
                semantics_label: self.semantics_label.as_deref(),
                semantics_identifier: self.semantics_identifier.as_deref(),
                is_placeholder: false,
                locale,
                spell_out,
            });
        }
        for child in &self.children {
            match child {
                InlineSpan::Text(span) => {
                    span.collect_semantics_information(locale, spell_out, output)
                }
                InlineSpan::Widget(_) => output.push(InlineSpanSemanticsInformation {
                    text: "\u{fffc}",
                    semantics_label: None,
                    semantics_identifier: None,
                    is_placeholder: true,
                    locale: None,
                    spell_out: false,
                }),
            }
        }
    }

    /// The spoken text when any span sets a semantics label (Flutter
    /// `combineSemanticsInfo`): each span contributes its label, or its text
    /// when it has none. `None` when no span sets a label.
    #[must_use]
    pub fn semantics_text(&self) -> Option<String> {
        fn has_label(span: &TextSpan) -> bool {
            span.semantics_label.is_some()
                || span
                    .children
                    .iter()
                    .any(|child| matches!(child, InlineSpan::Text(inner) if has_label(inner)))
        }
        has_label(self).then(|| self.to_plain_text(PlainTextOptions::default()))
    }

    #[must_use]
    pub fn text_len(&self) -> usize {
        self.plain_text().len()
    }

    /// Returns leaf text runs with inherited styles and byte ranges into the
    /// flattened plain text. Widget spans are represented by `\u{fffc}`.
    #[must_use]
    pub fn flatten(&self, inherited: &TextStyle) -> Vec<StyledTextRun> {
        let mut runs = Vec::new();
        let mut offset = 0;
        self.flatten_into(inherited, &mut offset, &mut runs);
        runs
    }

    /// The inline boxes of this span's widget leaves, at the byte offsets of
    /// their U+FFFC in [`Self::plain_text`], sized and aligned as each
    /// [`WidgetSpan`] states.
    #[must_use]
    pub fn placeholders(&self) -> Vec<TextPlaceholder> {
        fn collect(span: &TextSpan, offset: &mut usize, output: &mut Vec<TextPlaceholder>) {
            *offset += span.text.len();
            for child in &span.children {
                match child {
                    InlineSpan::Text(inner) => collect(inner, offset, output),
                    InlineSpan::Widget(widget) => {
                        output.push(TextPlaceholder::new(
                            widget.id,
                            *offset,
                            widget.size,
                            widget.alignment,
                        ));
                        *offset += '\u{fffc}'.len_utf8();
                    }
                }
            }
        }
        let mut output = Vec::new();
        collect(self, &mut 0, &mut output);
        output
    }

    pub fn visit(&self, visitor: &mut impl TextSpanVisitor) {
        self.visit_with(&TextStyle::default(), visitor);
    }

    fn write_plain_text(&self, output: &mut String, options: PlainTextOptions) {
        let text = if options.include_semantics_labels {
            self.semantics_label.as_deref().unwrap_or(&self.text)
        } else {
            &self.text
        };
        output.push_str(text);
        for child in &self.children {
            match child {
                InlineSpan::Text(span) => span.write_plain_text(output, options),
                InlineSpan::Widget(_) if options.include_placeholders => output.push('\u{fffc}'),
                InlineSpan::Widget(_) => {}
            }
        }
    }

    fn flatten_into(
        &self,
        inherited: &TextStyle,
        offset: &mut usize,
        runs: &mut Vec<StyledTextRun>,
    ) {
        let style = self
            .style
            .as_ref()
            .map_or_else(|| inherited.clone(), |local| inherited.merge(local));
        if !self.text.is_empty() {
            let start = *offset;
            *offset += self.text.len();
            runs.push(StyledTextRun {
                text: self.text.clone(),
                style: style.clone(),
                range: start..*offset,
            });
        }
        for child in &self.children {
            match child {
                InlineSpan::Text(span) => span.flatten_into(&style, offset, runs),
                InlineSpan::Widget(widget) => {
                    let start = *offset;
                    *offset += '\u{fffc}'.len_utf8();
                    runs.push(StyledTextRun {
                        text: '\u{fffc}'.to_string(),
                        style: style.clone(),
                        range: start..*offset,
                    });
                    visitor_placeholder(widget, &style, start..*offset, runs);
                }
            }
        }
    }

    fn visit_with(&self, inherited: &TextStyle, visitor: &mut impl TextSpanVisitor) {
        let style = self
            .style
            .as_ref()
            .map_or_else(|| inherited.clone(), |local| inherited.merge(local));
        if !self.text.is_empty() {
            visitor.visit_text(&self.text, &style);
        }
        for child in &self.children {
            match child {
                InlineSpan::Text(span) => span.visit_with(&style, visitor),
                InlineSpan::Widget(widget) => visitor.visit_widget(*widget, &style),
            }
        }
    }
}

/// A flattened leaf with inherited style information.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledTextRun {
    pub text: String,
    pub style: TextStyle,
    pub range: Range<usize>,
}

/// Visitor for rich text leaves. Both methods have defaults so callers can
/// collect only the kind of leaf they need.
pub trait TextSpanVisitor {
    fn visit_text(&mut self, _text: &str, _style: &TextStyle) {}
    fn visit_widget(&mut self, _widget: WidgetSpan, _style: &TextStyle) {}
}

// A widget leaf is already represented by a StyledTextRun. This helper keeps
// the flattening loop explicit without requiring a second public run variant.
fn visitor_placeholder(
    _widget: &WidgetSpan,
    _style: &TextStyle,
    _range: Range<usize>,
    _runs: &mut Vec<StyledTextRun>,
) {
}

/// A rich paragraph with paragraph-level alignment and scaling.
#[derive(Clone, Debug, PartialEq)]
pub struct RichText {
    pub text: TextSpan,
    pub text_align: TextAlign,
    pub text_scaler: TextScaler,
    pub max_lines: Option<usize>,
    pub soft_wrap: bool,
    pub overflow: TextOverflow,
}

impl RichText {
    #[must_use]
    pub fn new(text: TextSpan) -> Self {
        Self {
            text,
            text_align: TextAlign::Start,
            text_scaler: TextScaler::default(),
            max_lines: None,
            soft_wrap: true,
            overflow: TextOverflow::Clip,
        }
    }

    #[must_use]
    pub fn text_align(mut self, text_align: TextAlign) -> Self {
        self.text_align = text_align;
        self
    }

    #[must_use]
    pub fn align(self, text_align: TextAlign) -> Self {
        self.text_align(text_align)
    }

    #[must_use]
    pub fn text_scaler(mut self, text_scaler: TextScaler) -> Self {
        self.text_scaler = text_scaler;
        self
    }

    #[must_use]
    pub fn max_lines(mut self, max_lines: Option<usize>) -> Self {
        self.max_lines = max_lines;
        self
    }

    #[must_use]
    pub fn soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.soft_wrap = soft_wrap;
        self
    }

    #[must_use]
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }

    #[must_use]
    pub fn plain_text(&self) -> String {
        self.text.plain_text()
    }

    #[must_use]
    pub fn flatten(&self) -> Vec<StyledTextRun> {
        self.text
            .flatten(&TextStyle::default().scaled(self.text_scaler))
    }

    #[must_use]
    pub fn layout(&self, engine: &mut TextEngine, max_width: Option<f32>) -> Arc<TextLayout> {
        engine.layout_rich_with_options(
            &self.plain_text(),
            &self.flatten(),
            &self.text.placeholders(),
            TextLayoutOptions::new(max_width, self.text_align)
                .soft_wrap(self.soft_wrap)
                .max_lines(self.max_lines)
                .overflow(self.overflow),
        )
    }
}

/// A plain paragraph description. Rendering code can turn it into a
/// [`TextSpan`] without losing the familiar `Text::new("...")` API.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub text: String,
    pub style: TextStyle,
    pub text_align: TextAlign,
    pub text_scaler: TextScaler,
    pub max_lines: Option<usize>,
    pub soft_wrap: bool,
    pub overflow: TextOverflow,
}

impl Text {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::default(),
            text_align: TextAlign::Start,
            text_scaler: TextScaler::default(),
            max_lines: None,
            soft_wrap: true,
            overflow: TextOverflow::Clip,
        }
    }

    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }

    #[must_use]
    pub fn align(mut self, align: TextAlign) -> Self {
        self.text_align = align;
        self
    }

    #[must_use]
    pub fn text_align(self, align: TextAlign) -> Self {
        self.align(align)
    }

    #[must_use]
    pub fn text_scaler(mut self, scaler: TextScaler) -> Self {
        self.text_scaler = scaler;
        self
    }

    #[must_use]
    pub fn max_lines(mut self, max_lines: Option<usize>) -> Self {
        self.max_lines = max_lines;
        self
    }

    #[must_use]
    pub fn soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.soft_wrap = soft_wrap;
        self
    }

    #[must_use]
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }

    #[must_use]
    pub fn span(&self) -> TextSpan {
        TextSpan::new(self.text.clone()).style(self.style.clone())
    }

    #[must_use]
    pub fn rich_text(&self) -> RichText {
        RichText::new(self.span())
            .align(self.text_align)
            .text_scaler(self.text_scaler)
            .max_lines(self.max_lines)
            .soft_wrap(self.soft_wrap)
            .overflow(self.overflow)
    }

    #[must_use]
    pub fn layout(&self, engine: &mut TextEngine, max_width: Option<f32>) -> Arc<TextLayout> {
        engine.layout_with_options(
            &self.text,
            &self.style.scaled(self.text_scaler),
            TextLayoutOptions::new(max_width, self.text_align)
                .soft_wrap(self.soft_wrap)
                .max_lines(self.max_lines)
                .overflow(self.overflow),
        )
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for Text {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}
