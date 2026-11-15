//! Flutter's spell check service (`services/spell_check.dart`,
//! support-004): suggestion spans, the default service's merge of repeated
//! results, and its behavior without a native checker.

use std::{cell::RefCell, rc::Rc};

use incular_config::Locale;
use incular_platform::{
    DefaultSpellCheckService, SpellCheckCallback, SpellCheckResults, SpellCheckService,
    SuggestionSpan,
};

fn spans(ranges: &[(usize, usize)]) -> Vec<SuggestionSpan> {
    ranges
        .iter()
        .map(|&(start, end)| SuggestionSpan::new(start..end, ["fix"]))
        .collect()
}

#[test]
fn the_span_at_a_cursor_includes_both_ends() {
    let results = SpellCheckResults::new("teh quik fox", spans(&[(0, 3), (4, 8)]));
    assert_eq!(
        results.suggestion_span_at(0).map(|s| s.range.clone()),
        Some(0..3)
    );
    assert_eq!(
        results.suggestion_span_at(3).map(|s| s.range.clone()),
        Some(0..3)
    );
    assert_eq!(
        results.suggestion_span_at(6).map(|s| s.range.clone()),
        Some(4..8)
    );
    assert_eq!(
        results.suggestion_span_at(8).map(|s| s.range.clone()),
        Some(4..8)
    );
    assert_eq!(results.suggestion_span_at(9), None, "past the last span");
    assert_eq!(SpellCheckResults::default().suggestion_span_at(0), None);
}

#[test]
fn merge_keeps_old_spans_that_start_where_new_ones_do() {
    let old = vec![
        SuggestionSpan::new(0..3, ["old"]),
        SuggestionSpan::new(10..12, ["old"]),
    ];
    let new = vec![
        SuggestionSpan::new(0..4, ["new"]),
        SuggestionSpan::new(5..7, ["new"]),
        SuggestionSpan::new(20..22, ["new"]),
    ];
    let merged = DefaultSpellCheckService::merge_results(&old, &new);
    let summary: Vec<_> = merged
        .iter()
        .map(|span| (span.range.clone(), span.suggestions[0].as_str()))
        .collect();
    assert_eq!(
        summary,
        [
            (0..3, "old"),
            (5..7, "new"),
            (10..12, "old"),
            (20..22, "new")
        ]
    );
}

type Answers = Rc<RefCell<Vec<Option<Vec<SuggestionSpan>>>>>;

fn record(answers: &Answers) -> SpellCheckCallback {
    let answers = answers.clone();
    Box::new(move |spans| answers.borrow_mut().push(spans))
}

/// A native checker that flags every word starting with `x`.
struct Native {
    calls: Rc<RefCell<Vec<(String, String)>>>,
}

impl SpellCheckService for Native {
    fn fetch_spell_check_suggestions(&self, locale: &Locale, text: &str, done: SpellCheckCallback) {
        self.calls
            .borrow_mut()
            .push((locale.to_string(), text.to_owned()));
        let mut found = Vec::new();
        let mut start = 0;
        for word in text.split(' ') {
            if word.starts_with('x') {
                found.push(SuggestionSpan::new(start..start + word.len(), ["ok"]));
            }
            start += word.len() + 1;
        }
        done(Some(found));
    }
}

#[test]
fn the_default_service_answers_none_without_a_native_checker() {
    let answers = Answers::default();
    let service = DefaultSpellCheckService::new();
    service.fetch_spell_check_suggestions(&"en-US".parse().unwrap(), "xx", record(&answers));
    assert_eq!(*answers.borrow(), [None]);
    assert_eq!(service.last_saved_results(), None);
}

#[test]
fn the_default_service_forwards_to_the_native_checker_and_saves_results() {
    let calls = Rc::default();
    let service = DefaultSpellCheckService::with_native(Native {
        calls: Rc::clone(&calls),
    });
    let answers = Answers::default();
    let locale: Locale = "en-US".parse().unwrap();
    service.fetch_spell_check_suggestions(&locale, "ok xyz", record(&answers));
    service.fetch_spell_check_suggestions(&locale, "ok xyz", record(&answers));
    assert_eq!(calls.borrow().len(), 2);
    assert_eq!(calls.borrow()[0], ("en-US".to_owned(), "ok xyz".to_owned()));
    let expected = vec![SuggestionSpan::new(3..6, ["ok"])];
    assert_eq!(
        *answers.borrow(),
        [Some(expected.clone()), Some(expected.clone())]
    );
    assert_eq!(
        service.last_saved_results(),
        Some(SpellCheckResults::new("ok xyz", expected))
    );
}
