//! Flutter's spell check service (`services/spell_check.dart`): misspelled
//! ranges of a text and their replacement suggestions, fetched from a
//! platform spell checker.
//!
//! Ranges are byte offsets into the checked text, as everywhere in
//! Incular's text APIs; Flutter's are UTF-16 code units.

use std::{cell::RefCell, fmt, ops::Range, rc::Rc};

use incular_config::Locale;

use crate::message_codecs::{MessageValue, MethodCall, StandardMethodCodec};

/// Platform channel used by Flutter's default spelling service.
pub const FLUTTER_SPELL_CHECK_CHANNEL: &str = "flutter/spellcheck";

/// The asynchronous reply from a standard method channel, if one was received.
pub type FlutterSpellCheckReply = Box<dyn FnOnce(Option<Vec<u8>>)>;


/// Flutter's `SuggestionSpan`: a misspelled range and the replacements the
/// spell checker suggests for it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SuggestionSpan {
    /// The misspelled byte range of the checked text.
    pub range: Range<usize>,
    /// Suggested replacements, best first.
    pub suggestions: Vec<String>,
}

impl SuggestionSpan {
    /// A span over `range` with `suggestions`.
    #[must_use]
    pub fn new(
        range: Range<usize>,
        suggestions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            range,
            suggestions: suggestions.into_iter().map(Into::into).collect(),
        }
    }
}

impl fmt::Display for SuggestionSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SuggestionSpan(range: TextRange(start: {}, end: {}), suggestions: [",
            self.range.start, self.range.end
        )?;
        display_list(f, &self.suggestions)?;
        f.write_str("])")
    }
}

/// Flutter's `SpellCheckResults`: the text that was checked and its
/// misspelled spans, in text order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpellCheckResults {
    /// The text the spans index.
    pub spell_checked_text: String,
    /// The misspelled spans, ordered by range.
    pub suggestion_spans: Vec<SuggestionSpan>,
}

impl SpellCheckResults {
    /// Results for `spell_checked_text`.
    #[must_use]
    pub fn new(
        spell_checked_text: impl Into<String>,
        suggestion_spans: Vec<SuggestionSpan>,
    ) -> Self {
        Self {
            spell_checked_text: spell_checked_text.into(),
            suggestion_spans,
        }
    }

    /// Flutter's `EditableTextState.findSuggestionSpanAtCursorIndex`: the
    /// span whose range contains `cursor_index`, ends included, found by
    /// binary search.
    #[must_use]
    pub fn suggestion_span_at(&self, cursor_index: usize) -> Option<&SuggestionSpan> {
        let spans = &self.suggestion_spans;
        if spans
            .last()
            .is_none_or(|last| last.range.end < cursor_index)
        {
            return None;
        }
        let (mut left, mut right) = (0usize, spans.len());
        while left < right {
            // Flutter calculates the midpoint with an inclusive upper bound.
            // Use the lower midpoint for even counts, observable at a shared end.
            let middle = left + (right - left - 1) / 2;
            let span = &spans[middle];
            if (span.range.start..=span.range.end).contains(&cursor_index) {
                return Some(span);
            }
            if cursor_index <= span.range.start {
                right = middle;
            } else {
                left = middle + 1;
            }
        }
        None
    }
}

impl fmt::Display for SpellCheckResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SpellCheckResults(spellCheckText: {}, suggestionSpans: [",
            self.spell_checked_text
        )?;
        display_list(f, &self.suggestion_spans)?;
        f.write_str("])")
    }
}

fn display_list<T: fmt::Display>(f: &mut fmt::Formatter<'_>, items: &[T]) -> fmt::Result {
    for (index, item) in items.iter().enumerate() {
        if index != 0 {
            f.write_str(", ")?;
        }
        write!(f, "{item}")?;
    }
    Ok(())
}

/// Receives the outcome of one spell check: the misspelled spans, or `None`
/// when the check was cancelled or failed. Called on the UI thread.
pub type SpellCheckCallback = Box<dyn FnOnce(Option<Vec<SuggestionSpan>>)>;

/// Flutter's `SpellCheckService`: checks `text` in `locale` and answers
/// through `done`, now or later on the UI thread.
pub trait SpellCheckService {
    /// Flutter `fetchSpellCheckSuggestions`.
    fn fetch_spell_check_suggestions(&self, locale: &Locale, text: &str, done: SpellCheckCallback);
}

/// Flutter's `DefaultSpellCheckService`: forwards checks to the platform's
/// native spell checker and merges repeated results for unchanged text.
///
/// Flutter talks to the native checker over the `flutter/spellcheck`
/// channel. A host can supply either that transport with
/// [`Self::with_flutter_method_channel`] or a checker with [`Self::with_native`];
/// without one, as on every platform Incular currently runs on, each check
/// answers `None`, as Flutter does when the channel call fails.
#[derive(Default)]
pub struct DefaultSpellCheckService {
    native: Option<Rc<dyn SpellCheckService>>,
    last_saved_results: Rc<RefCell<Option<SpellCheckResults>>>,
}

impl DefaultSpellCheckService {
    /// A service with no native checker: every check answers `None`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A service over the host's native checker.
    #[must_use]
    pub fn with_native(native: impl SpellCheckService + 'static) -> Self {
        Self {
            native: Some(Rc::new(native)),
            last_saved_results: Rc::default(),
        }
    }

    /// Uses the Flutter spell-check standard method channel as the native
    /// provider. The embedder supplies the transport and completes each
    /// reply; this service encodes calls and validates UTF-16 reply ranges.
    #[must_use]
    pub fn with_flutter_method_channel(
        send: impl Fn(&'static str, Vec<u8>, FlutterSpellCheckReply) + 'static,
    ) -> Self {
        Self::with_native(FlutterSpellCheckTransport(send))
    }

    /// Flutter `lastSavedResults`: the last results this service answered.
    #[must_use]
    pub fn last_saved_results(&self) -> Option<SpellCheckResults> {
        self.last_saved_results.borrow().clone()
    }

    /// Flutter `DefaultSpellCheckService.mergeResults`: merges two ordered
    /// span lists, keeping the old span where both start at the same index.
    #[must_use]
    pub fn merge_results(
        old_results: &[SuggestionSpan],
        new_results: &[SuggestionSpan],
    ) -> Vec<SuggestionSpan> {
        let mut merged = Vec::with_capacity(old_results.len() + new_results.len());
        let (mut old, mut new) = (0, 0);
        while old < old_results.len() && new < new_results.len() {
            let (old_span, new_span) = (&old_results[old], &new_results[new]);
            if old_span.range.start == new_span.range.start {
                merged.push(old_span.clone());
                old += 1;
                new += 1;
            } else if old_span.range.start < new_span.range.start {
                merged.push(old_span.clone());
                old += 1;
            } else {
                merged.push(new_span.clone());
                new += 1;
            }
        }
        merged.extend_from_slice(&old_results[old..]);
        merged.extend_from_slice(&new_results[new..]);
        merged
    }
}

impl fmt::Debug for DefaultSpellCheckService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultSpellCheckService")
            .field("native", &self.native.is_some())
            .finish_non_exhaustive()
    }
}

impl SpellCheckService for DefaultSpellCheckService {
    fn fetch_spell_check_suggestions(&self, locale: &Locale, text: &str, done: SpellCheckCallback) {
        let Some(native) = &self.native else {
            done(None);
            return;
        };
        let last_saved = self.last_saved_results.clone();
        let checked = text.to_owned();
        native.fetch_spell_check_suggestions(
            locale,
            text,
            Box::new(move |spans| {
                let Some(mut spans) = spans else {
                    done(None);
                    return;
                };
                // Flutter merges only when the text is unchanged and the
                // span lists are equal (its `spansHaveChanged` is
                // `listEquals`), which keeps the saved spans.
                if let Some(saved) = &*last_saved.borrow()
                    && saved.spell_checked_text == checked
                    && saved.suggestion_spans == spans
                {
                    spans = Self::merge_results(&saved.suggestion_spans, &spans);
                }
                *last_saved.borrow_mut() = Some(SpellCheckResults::new(checked, spans.clone()));
                done(Some(spans));
            }),
        );
    }
}


struct FlutterSpellCheckTransport<F>(F);

impl<F> SpellCheckService for FlutterSpellCheckTransport<F>
where
    F: Fn(&'static str, Vec<u8>, FlutterSpellCheckReply),
{
    fn fetch_spell_check_suggestions(&self, locale: &Locale, text: &str, done: SpellCheckCallback) {
        let Ok(call) = StandardMethodCodec::encode_method_call(&MethodCall {
            method: "SpellCheck.initiateSpellCheck".into(),
            arguments: MessageValue::List(vec![
                MessageValue::String(locale.to_string()),
                MessageValue::String(text.to_owned()),
            ]),
        }) else {
            done(None);
            return;
        };
        let offsets = SpellCheckOffsets::new(text);
        (self.0)(
            FLUTTER_SPELL_CHECK_CHANNEL,
            call,
            Box::new(move |response| {
                let spans = response.and_then(|bytes| {
                    let MessageValue::List(results) =
                        StandardMethodCodec::decode_envelope(&bytes).ok()?
                    else {
                        return None;
                    };
                    results.into_iter().map(|span| offsets.decode_span(span)).collect()
                });
                done(spans);
            }),
        );
    }
}

enum SpellCheckOffsets {
    Ascii(usize),
    Utf16(Vec<usize>),
}

impl SpellCheckOffsets {
    fn new(text: &str) -> Self {
        if text.is_ascii() {
            return Self::Ascii(text.len());
        }
        let mut boundaries = vec![0];
        for (byte, ch) in text.char_indices() {
            if ch.len_utf16() == 2 {
                // An offset inside a surrogate pair has no UTF-8 boundary.
                boundaries.push(usize::MAX);
            }
            boundaries.push(byte + ch.len_utf8());
        }
        Self::Utf16(boundaries)
    }

    fn byte_offset(&self, value: Option<MessageValue>) -> Option<usize> {
        let MessageValue::Int(offset) = value? else {
            return None;
        };
        let offset = usize::try_from(offset).ok()?;
        match self {
            Self::Ascii(length) => (offset <= *length).then_some(offset),
            Self::Utf16(boundaries) => boundaries
                .get(offset)
                .copied()
                .filter(|&byte| byte != usize::MAX),
        }
    }

    fn decode_span(&self, span: MessageValue) -> Option<SuggestionSpan> {
        let MessageValue::Map(fields) = span else {
            return None;
        };
        let (mut start, mut end, mut suggestions) = (None, None, None);
        for (key, value) in fields {
            let MessageValue::String(key) = key else {
                continue;
            };
            match key.as_str() {
                "startIndex" => start = Some(value),
                "endIndex" => end = Some(value),
                "suggestions" => suggestions = Some(value),
                _ => {}
            }
        }
        let (start, end) = (self.byte_offset(start)?, self.byte_offset(end)?);
        if start > end {
            return None;
        }
        let MessageValue::List(suggestions) = suggestions? else {
            return None;
        };
        Some(SuggestionSpan {
            range: start..end,
            suggestions: suggestions
                .into_iter()
                .map(|suggestion| match suggestion {
                    MessageValue::String(suggestion) => Some(suggestion),
                    _ => None,
                })
                .collect::<Option<_>>()?,
        })
    }
}
