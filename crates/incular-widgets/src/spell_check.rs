//! Flutter's widget-side spell check (`widgets/spell_check.dart`): the
//! configuration an [`EditableText`](crate::EditableText) takes, and the
//! text span that styles misspelled words.

use std::{fmt, ops::Range, rc::Rc};

use incular_config::TargetPlatform;
use incular_platform::{SpellCheckResults, SpellCheckService, SuggestionSpan};
use incular_text::{TextDecoration, TextEditingValue, TextSpan, TextStyle};

/// Flutter's `SpellCheckConfiguration`: whether an editable text checks its
/// spelling, with which service, and how misspelled words look.
///
/// Spell checking runs only with a service and a misspelled text style.
/// Flutter falls back to the platform's native checker when no service is
/// given; no Incular host provides one, so a configuration without a
/// service leaves checking off, as Flutter does when
/// `nativeSpellCheckServiceDefined` is false.
///
/// A custom toolbar builder can display suggestions without choosing a design system.
#[derive(Clone, Default)]
pub struct SpellCheckConfiguration {
    spell_check_service: Option<Rc<dyn SpellCheckService>>,
    misspelled_text_style: Option<TextStyle>,
    toolbar_builder: Option<crate::EditableTextContextMenuBuilder>,
    enabled: bool,
}

impl SpellCheckConfiguration {
    /// An enabled configuration with no service or style yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }

    /// Flutter `SpellCheckConfiguration.disabled()`.
    #[must_use]
    pub fn disabled() -> Self {
        Self::default()
    }

    /// The service that checks the text.
    #[must_use]
    pub fn spell_check_service(mut self, service: impl SpellCheckService + 'static) -> Self {
        self.spell_check_service = Some(Rc::new(service));
        self
    }

    /// The service, shared with other configurations.
    #[must_use]
    pub fn shared_spell_check_service(mut self, service: Rc<dyn SpellCheckService>) -> Self {
        self.spell_check_service = Some(service);
        self
    }

    /// The style merged over the field's style for misspelled words.
    #[must_use]
    pub fn misspelled_text_style(mut self, style: TextStyle) -> Self {
        self.misspelled_text_style = Some(style);
        self
    }

    /// Flutter `spellCheckSuggestionsToolbarBuilder`: renders the custom
    /// menu for the current misspelling and its available replacements.
    #[must_use]
    pub fn spell_check_suggestions_toolbar_builder(
        mut self,
        builder: impl for<'a> Fn(&crate::BuildContext<'a>, &crate::TextContextMenu) -> crate::Widget
        + 'static,
    ) -> Self {
        self.toolbar_builder = Some(Rc::new(builder));
        self
    }

    /// The toolbar builder, if configured.
    #[must_use]
    pub fn suggestions_toolbar_builder(&self) -> Option<&crate::EditableTextContextMenuBuilder> {
        self.toolbar_builder.as_ref()
    }

    /// Flutter `spellCheckEnabled`: false only for
    /// [`Self::disabled`].
    #[must_use]
    pub fn spell_check_enabled(&self) -> bool {
        self.enabled
    }

    /// The service, if any.
    #[must_use]
    pub fn service(&self) -> Option<&Rc<dyn SpellCheckService>> {
        self.spell_check_service.as_ref()
    }

    /// The misspelled text style, if any.
    #[must_use]
    pub fn misspelled_style(&self) -> Option<&TextStyle> {
        self.misspelled_text_style.as_ref()
    }

    /// Flutter's `EditableTextState._inferSpellCheckConfiguration`: the
    /// service and style spell checking runs with, or `None` when it is off.
    pub(crate) fn resolve(&self) -> Option<(Rc<dyn SpellCheckService>, TextStyle)> {
        if !self.enabled {
            return None;
        }
        Some((
            self.spell_check_service.clone()?,
            self.misspelled_text_style.clone()?,
        ))
    }
}

impl PartialEq for SpellCheckConfiguration {
    fn eq(&self, other: &Self) -> bool {
        let same_service = match (&self.spell_check_service, &other.spell_check_service) {
            (Some(left), Some(right)) => Rc::ptr_eq(left, right),
            (None, None) => true,
            _ => false,
        };
        same_service
            && self.misspelled_text_style == other.misspelled_text_style
            && crate::util::same_rc(&self.toolbar_builder, &other.toolbar_builder)
            && self.enabled == other.enabled
    }
}

impl fmt::Debug for SpellCheckConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpellCheckConfiguration")
            .field("enabled", &self.enabled)
            .field("service", &self.spell_check_service.is_some())
            .field("misspelled_text_style", &self.misspelled_text_style)
            .field("toolbar_builder", &self.toolbar_builder.is_some())
            .finish()
    }
}

/// Dart's `\w`: ASCII letters, digits and underscore.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Dart's `\b` at byte `index` of `text`.
fn is_word_boundary(text: &str, index: usize) -> bool {
    let before = text[..index].chars().next_back().is_some_and(is_word);
    let after = text[index..].chars().next().is_some_and(is_word);
    before != after
}

/// The first index of `word` in `text` with a word boundary on both sides,
/// Dart's `text.indexOf(RegExp('\\b${RegExp.escape(word)}\\b'))`.
fn find_whole_word(text: &str, word: &str) -> Option<usize> {
    text.match_indices(word)
        .map(|(index, _)| index)
        .find(|&index| is_word_boundary(text, index) && is_word_boundary(text, index + word.len()))
}

fn span(range: Range<usize>, suggestions: &[String]) -> SuggestionSpan {
    SuggestionSpan {
        range,
        suggestions: suggestions.to_vec(),
    }
}

/// Flutter's `_correctSpellCheckResults`: moves the spans of results for
/// `results_text` onto `new_text`, following each misspelled word that was
/// pushed along and dropping words that changed.
fn correct_spell_check_results(
    new_text: &str,
    results_text: &str,
    results: &[SuggestionSpan],
) -> Vec<SuggestionSpan> {
    let mut corrected = Vec::with_capacity(results.len());
    let mut offset: isize = 0;
    let mut search_start = 0usize;
    for current in results {
        let Some(word) = results_text.get(current.range.clone()) else {
            continue;
        };
        let length = current.range.len();
        let found = new_text
            .get(search_start..)
            .and_then(|rest| find_whole_word(rest, word));
        let start = current.range.start as isize;
        let found_exactly = found.is_some_and(|index| current.range.start == index + search_start);
        let found_with_offset =
            found.is_some_and(|index| start + offset == (index + search_start) as isize);
        if found_exactly || found_with_offset {
            let adjusted_start = (start + offset).max(0) as usize;
            let adjusted = adjusted_start..adjusted_start + length;
            search_start = (adjusted.end + 1).min(new_text.len());
            corrected.push(span(adjusted, &current.suggestions));
        } else if let Some(index) = found {
            let adjusted_start = search_start + index;
            let adjusted_end = adjusted_start + length;
            search_start = (adjusted_end + 1).min(new_text.len());
            offset = adjusted_start as isize - start;
            corrected.push(span(adjusted_start..adjusted_end, &current.suggestions));
        }
        // Moves past a multi-byte character so later searches start on a
        // character boundary.
        while search_start < new_text.len() && !new_text.is_char_boundary(search_start) {
            search_start += 1;
        }
    }
    corrected
}

/// A slice of `text`, or nothing when the range is not on character
/// boundaries.
fn piece(text: &str, range: Range<usize>, style: Option<&TextStyle>) -> Option<TextSpan> {
    let slice = text.get(range)?;
    let mut span = TextSpan::new(slice);
    span.style = style.cloned();
    Some(span)
}

/// Keep the rendering cursor monotonic and on character boundaries even if a
/// user-provided checker returns malformed or overlapping byte ranges.
fn suggestion_range(text: &str, range: &Range<usize>, pointer: usize) -> Option<Range<usize>> {
    let start = range.start;
    let end = range.end.min(text.len());
    (pointer <= start && start < end && text.is_char_boundary(start) && text.is_char_boundary(end))
        .then_some(start..end)
}

/// Flutter's `_buildSubtreesWithoutComposingRegion`.
fn subtrees_without_composing_region(
    spans: &[SuggestionSpan],
    value: &TextEditingValue,
    style: Option<&TextStyle>,
    misspelled: &TextStyle,
    cursor_index: usize,
) -> Vec<TextSpan> {
    let text = value.text.as_str();
    let joint = style.map_or_else(|| misspelled.clone(), |style| style.merge(misspelled));
    // Each span adds at most a gap piece and its own piece, plus a tail.
    let mut children = Vec::with_capacity(spans.len() * 2 + 1);
    let mut pointer = 0;
    for current in spans {
        let Some(range) = suggestion_range(text, &current.range, pointer) else {
            continue;
        };
        if range.start > pointer {
            children.extend(piece(text, pointer..range.start, style));
        }
        let cursor_inside = range.start <= cursor_index && current.range.end >= cursor_index;
        let piece_style = if cursor_inside { style } else { Some(&joint) };
        pointer = range.end;
        children.extend(piece(text, range, piece_style));
    }
    if pointer < text.len() {
        children.extend(piece(text, pointer..text.len(), style));
    }
    children
}

/// Flutter's `_buildSubtreesWithComposingRegion`, used on Android: the
/// composing region is underlined instead of a misspelled style.
fn subtrees_with_composing_region(
    spans: &[SuggestionSpan],
    value: &TextEditingValue,
    style: Option<&TextStyle>,
    misspelled: &TextStyle,
    composing_within_current_text_range: bool,
) -> Vec<TextSpan> {
    let text = value.text.as_str();
    // Flutter represents an absent composing range with negative endpoints.
    // Do not turn that into a real collapsed range at the start of the text.
    let composing = value
        .composing
        .filter(|range| !composing_within_current_text_range && range.is_valid_for(text))
        .map(|range| range.start..range.end);
    let underline = TextStyle::new().decoration(TextDecoration::UNDERLINE);
    let composing_style = style.map_or_else(|| underline.clone(), |style| style.merge(&underline));
    let joint = style.map_or_else(|| misspelled.clone(), |style| style.merge(misspelled));
    // A gap can add three pieces (composing region split) before each span.
    let mut children = Vec::with_capacity(spans.len() * 4 + 3);
    let add_composing = |children: &mut Vec<TextSpan>, pointer: usize, composing: &Range<usize>| {
        // Flutter's `_addComposingRegionTextSpans`.
        children.extend(piece(text, pointer..composing.start, style));
        children.extend(piece(text, composing.clone(), Some(&composing_style)));
    };
    let mut pointer = 0;
    for current in spans {
        let Some(region) = suggestion_range(text, &current.range, pointer) else {
            continue;
        };
        if region.start > pointer {
            let end = region.start;
            if let Some(composing) = &composing
                && composing.start >= pointer
                && composing.end <= end
            {
                add_composing(&mut children, pointer, composing);
                children.extend(piece(text, composing.end..end, style));
            } else {
                children.extend(piece(text, pointer..end, style));
            }
        }
        let is_composing = composing
            .as_ref()
            .is_some_and(|range| region.start >= range.start && region.end <= range.end);
        let piece_style = if is_composing {
            &composing_style
        } else {
            &joint
        };
        pointer = region.end;
        children.extend(piece(text, region, Some(piece_style)));
    }
    if pointer < text.len() {
        if let Some(composing) = &composing
            && pointer < composing.start
        {
            add_composing(&mut children, pointer, composing);
            if composing.end != text.len() {
                children.extend(piece(text, composing.end..text.len(), style));
            }
        } else {
            children.extend(piece(text, pointer..text.len(), style));
        }
    }
    children
}

/// Flutter's `buildTextSpanWithSpellCheckSuggestions`: the editable text's
/// span with each misspelled word in `style` merged with
/// `misspelled_text_style`. Results for an older text are first moved onto
/// the current text. A misspelled word holding the caret keeps the plain
/// style. On Android the composing region is underlined instead, as in
/// Flutter; `composing_within_current_text_range` is Flutter's
/// `composingRegionOutOfRange` flag that turns that off.
#[must_use]
pub fn build_text_span_with_spell_check_suggestions(
    value: &TextEditingValue,
    composing_within_current_text_range: bool,
    style: Option<&TextStyle>,
    misspelled_text_style: &TextStyle,
    results: &SpellCheckResults,
    platform: TargetPlatform,
) -> TextSpan {
    let corrected;
    let spans = if results.spell_checked_text == value.text {
        results.suggestion_spans.as_slice()
    } else {
        corrected = correct_spell_check_results(
            &value.text,
            &results.spell_checked_text,
            &results.suggestion_spans,
        );
        corrected.as_slice()
    };
    let children = if platform == TargetPlatform::Android {
        subtrees_with_composing_region(
            spans,
            value,
            style,
            misspelled_text_style,
            composing_within_current_text_range,
        )
    } else {
        subtrees_without_composing_region(
            spans,
            value,
            style,
            misspelled_text_style,
            value.selection.base,
        )
    };
    let mut root = TextSpan::new("");
    root.style = style.cloned();
    root.children = children.into_iter().map(Into::into).collect();
    root
}

/// One text field's spell check results, Flutter's
/// `EditableTextState.spellCheckResults`. Checks answer asynchronously; a
/// session dropped with its field ignores late answers, as Flutter's
/// `mounted` check does.
#[derive(Debug, Default)]
pub(crate) struct SpellCheckSession {
    results: std::cell::RefCell<Option<SpellCheckResults>>,
    revision: std::cell::Cell<u64>,
}

impl SpellCheckSession {
    /// Flutter `_performSpellCheck`: asks `service` to check `text` and keeps
    /// the answer. A cancelled or failed check keeps the previous results.
    pub(crate) fn request(
        self: &Rc<Self>,
        service: &Rc<dyn SpellCheckService>,
        locale: &incular_config::Locale,
        text: &str,
    ) {
        let session = Rc::downgrade(self);
        let checked = text.to_owned();
        service.fetch_spell_check_suggestions(
            locale,
            text,
            Box::new(move |spans| {
                let (Some(spans), Some(session)) = (spans, session.upgrade()) else {
                    return;
                };
                *session.results.borrow_mut() = Some(SpellCheckResults::new(checked, spans));
                session.revision.set(session.revision.get() + 1);
                // The field lays out again with the new spans.
                incular_animation::scheduler::schedule_frame();
            }),
        );
    }

    /// Flutter `_spellCheckResultsReceived`: the results, when they have at
    /// least one span.
    pub(crate) fn received(&self) -> Option<std::cell::Ref<'_, SpellCheckResults>> {
        std::cell::Ref::filter_map(self.results.borrow(), |results| {
            results
                .as_ref()
                .filter(|results| !results.suggestion_spans.is_empty())
        })
        .ok()
    }

    /// The latest results, spans or not.
    pub(crate) fn results(&self) -> Option<SpellCheckResults> {
        self.results.borrow().clone()
    }

    /// Bumped by each answer.
    pub(crate) fn revision(&self) -> u64 {
        self.revision.get()
    }
}
