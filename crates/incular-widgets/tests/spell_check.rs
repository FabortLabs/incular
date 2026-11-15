//! Flutter's widget-side spell check (`widgets/spell_check.dart` and the
//! spell check paths of `EditableText`, support-004): misspelled words are
//! styled from the service's results, results follow edits, and a field
//! checks its user edits.

use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, Locale, TargetPlatform};
use incular_core::{Color, Size};
use incular_rendering::{DisplayList, PaintCommand};
use incular_text::{
    ComposingRange, TextDecoration, TextDecorationStyle, TextEditingController, TextEditingValue,
    TextSelection, TextStyle,
};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    EditableText, SpellCheckCallback, SpellCheckConfiguration, SpellCheckResults,
    SpellCheckService, SuggestionSpan, TextDirection, Widget,
    build_text_span_with_spell_check_suggestions,
};

const MISSPELLED: Color = Color::rgba(255, 0, 0, 255);

fn misspelled_style() -> TextStyle {
    TextStyle::new()
        .color(MISSPELLED)
        .decoration(TextDecoration::UNDERLINE)
        .decoration_color(MISSPELLED)
        .decoration_style(TextDecorationStyle::Wavy)
}

fn value(text: &str, caret: usize) -> TextEditingValue {
    let mut value = TextEditingValue::new(text);
    value.selection = TextSelection::collapsed(caret);
    value
}

fn results(text: &str, ranges: &[(usize, usize)]) -> SpellCheckResults {
    SpellCheckResults::new(
        text,
        ranges
            .iter()
            .map(|&(start, end)| SuggestionSpan::new(start..end, ["fix"]))
            .collect(),
    )
}

/// `(text, misspelled)` for each leaf of the built span.
fn pieces(
    value: &TextEditingValue,
    results: &SpellCheckResults,
    platform: TargetPlatform,
) -> Vec<(String, bool)> {
    let style = TextStyle::new().color(Color::BLACK);
    let span = build_text_span_with_spell_check_suggestions(
        value,
        true,
        Some(&style),
        &misspelled_style(),
        results,
        platform,
    );
    span.flatten(&style)
        .into_iter()
        .map(|run| (run.text, run.style.color == MISSPELLED))
        .collect()
}

fn owned(pieces: &[(&str, bool)]) -> Vec<(String, bool)> {
    pieces
        .iter()
        .map(|&(text, misspelled)| (text.to_owned(), misspelled))
        .collect()
}

#[test]
fn misspelled_words_take_the_misspelled_style_unless_they_hold_the_caret() {
    let checked = results("teh quik fox", &[(0, 3), (4, 8)]);
    assert_eq!(
        pieces(
            &value("teh quik fox", 12),
            &checked,
            TargetPlatform::Windows
        ),
        owned(&[("teh", true), (" ", false), ("quik", true), (" fox", false)])
    );
    // A caret inside (or at the edge of) a word keeps it plain.
    assert_eq!(
        pieces(&value("teh quik fox", 6), &checked, TargetPlatform::Windows),
        owned(&[
            ("teh", true),
            (" ", false),
            ("quik", false),
            (" fox", false)
        ])
    );
}

#[test]
fn results_for_an_older_text_follow_the_words() {
    let checked = results("teh quik fox", &[(0, 3), (4, 8)]);
    // Text inserted before both words pushes them along.
    assert_eq!(
        pieces(
            &value("a teh quik fox", 0),
            &checked,
            TargetPlatform::Windows
        ),
        owned(&[
            ("a ", false),
            ("teh", true),
            (" ", false),
            ("quik", true),
            (" fox", false)
        ])
    );
    // A corrected word drops its span; the other one stays.
    assert_eq!(
        pieces(&value("the quik fox", 0), &checked, TargetPlatform::Windows),
        owned(&[("the ", false), ("quik", true), (" fox", false)])
    );
    // Only whole words match: "tehx" does not contain the word "teh".
    assert_eq!(
        pieces(&value("tehx quik", 0), &checked, TargetPlatform::Windows),
        owned(&[("tehx ", false), ("quik", true)])
    );
}

#[test]
fn android_underlines_the_composing_region_instead() {
    let checked = results("teh quik fox", &[(0, 3)]);
    let mut composing = value("teh quik fox", 12);
    composing.composing = Some(ComposingRange::new(9, 12));
    let style = TextStyle::new().color(Color::BLACK);
    let span = build_text_span_with_spell_check_suggestions(
        &composing,
        false,
        Some(&style),
        &misspelled_style(),
        &checked,
        TargetPlatform::Android,
    );
    let runs = span.flatten(&style);
    let summary: Vec<_> = runs
        .iter()
        .map(|run| {
            (
                run.text.as_str(),
                run.style.color == MISSPELLED,
                run.style.decoration == Some(TextDecoration::UNDERLINE),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("teh", true, true),
            (" quik ", false, false),
            ("fox", false, true),
        ]
    );
}

type Requests = Rc<RefCell<Vec<(String, SpellCheckCallback)>>>;

/// A service that keeps each request until the test answers it.
#[derive(Clone, Default)]
struct Pending {
    requests: Requests,
}

impl SpellCheckService for Pending {
    fn fetch_spell_check_suggestions(&self, _: &Locale, text: &str, done: SpellCheckCallback) {
        self.requests.borrow_mut().push((text.to_owned(), done));
    }
}

fn field(controller: &TextEditingController, service: &Pending, obscure: bool) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(Widget::from(
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .size(Size::new(280., 40.))
            .obscure_text(obscure)
            .style(TextStyle::new().color(Color::BLACK).size(14.))
            .spell_check_configuration(
                SpellCheckConfiguration::new()
                    .spell_check_service(service.clone())
                    .misspelled_text_style(misspelled_style()),
            ),
    ))
    .expect("mount");
    layout(&mut tree);
    tree
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::loose(Size::new(300., 100.)))
        .expect("layout");
}

fn misspelled_glyphs(list: &DisplayList) -> usize {
    list.commands()
        .iter()
        .filter(|command| {
            matches!(command, PaintCommand::GlyphRun { color, .. } if *color == MISSPELLED)
        })
        .count()
}

fn wavy_lines(list: &DisplayList) -> usize {
    list.commands()
        .iter()
        .filter(|command| matches!(command, PaintCommand::StrokePath { .. }))
        .count()
}

#[test]
fn a_field_checks_user_edits_and_styles_the_answer() {
    let controller = TextEditingController::new();
    let service = Pending::default();
    let mut tree = field(&controller, &service, false);

    // Programmatic changes are not checked, as in Flutter.
    controller.set_text("teh");
    layout(&mut tree);
    assert!(service.requests.borrow().is_empty());

    controller.set_selection(TextSelection::collapsed(3));
    controller.insert_text(" cat");
    let (text, done) = service.requests.borrow_mut().pop().expect("a check");
    assert_eq!(text, "teh cat");
    layout(&mut tree);
    assert_eq!(misspelled_glyphs(&tree.paint()), 0, "no answer yet");

    done(Some(vec![SuggestionSpan::new(0..3, ["the"])]));
    layout(&mut tree);
    let list = tree.paint();
    assert!(misspelled_glyphs(&list) > 0, "the misspelled word is red");
    assert_eq!(wavy_lines(&list), 1, "one wavy underline");
}

#[test]
fn obscured_fields_are_never_styled() {
    let controller = TextEditingController::new();
    let service = Pending::default();
    let mut tree = field(&controller, &service, true);
    controller.insert_text("teh");
    if let Some((_, done)) = service.requests.borrow_mut().pop() {
        done(Some(vec![SuggestionSpan::new(0..3, ["the"])]));
    }
    layout(&mut tree);
    let list = tree.paint();
    assert_eq!(misspelled_glyphs(&list), 0);
    assert_eq!(wavy_lines(&list), 0);
}

#[test]
fn configurations_without_a_service_or_style_do_not_check() {
    assert!(!SpellCheckConfiguration::disabled().spell_check_enabled());
    assert!(SpellCheckConfiguration::new().spell_check_enabled());
    let controller = TextEditingController::new();
    let service = Pending::default();
    let mut tree = WidgetTree::new();
    tree.mount(Widget::from(
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .spell_check_configuration(
                SpellCheckConfiguration::new().spell_check_service(service.clone()),
            ),
    ))
    .expect("mount");
    layout(&mut tree);
    controller.insert_text("teh");
    assert!(
        service.requests.borrow().is_empty(),
        "no misspelled style: off"
    );
}
