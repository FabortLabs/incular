use incular_config::{Constraints, Locale};
use incular_core::Size;
use incular_text::{TextDecoration, TextEditingController, TextStyle};
use incular_widgets::{
    EditableText, SpellCheckCallback, SpellCheckConfiguration, SpellCheckService, SuggestionSpan,
    TextDirection, Widget, internal::WidgetTree,
};

#[derive(Clone)]
struct Service(Vec<SuggestionSpan>);

impl SpellCheckService for Service {
    fn fetch_spell_check_suggestions(&self, _: &Locale, _: &str, done: SpellCheckCallback) {
        done(Some(self.0.clone()));
    }
}

#[test]
fn mounted_editor_queries_the_service_result_at_and_between_word_edges() {
    let controller = TextEditingController::with_text("0123456789abcworld  Hey  green");
    let expected = SuggestionSpan::new(20..23, ["Hey", "He"]);
    let mut tree = WidgetTree::new();
    let id = tree
        .mount(Widget::from(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .spell_check_configuration(
                    SpellCheckConfiguration::new()
                        .spell_check_service(Service(vec![
                            SuggestionSpan::new(13..18, ["world", "word", "old"]),
                            expected.clone(),
                            SuggestionSpan::new(25..30, ["green", "grey", "great"]),
                        ]))
                        .misspelled_text_style(
                            TextStyle::new().decoration(TextDecoration::UNDERLINE),
                        ),
                ),
        ))
        .unwrap();
    tree.layout(Constraints::loose(Size::new(300.0, 100.0)))
        .unwrap();
    assert_eq!(
        tree.text_field_suggestion_span_at_cursor_index(id, 21),
        None
    );
    controller.insert_text(".");

    for caret in [20, 21, 23] {
        assert_eq!(
            tree.text_field_suggestion_span_at_cursor_index(id, caret),
            Some(expected.clone()),
            "caret {caret}"
        );
    }
    for caret in [5, 19, 24, 33] {
        assert_eq!(
            tree.text_field_suggestion_span_at_cursor_index(id, caret),
            None
        );
    }
    tree.update(
        id,
        Widget::from(
            EditableText::new(controller)
                .text_direction(TextDirection::Ltr)
                .obscure_text(true),
        ),
    )
    .unwrap();
    tree.layout(Constraints::loose(Size::new(300.0, 100.0)))
        .unwrap();
    assert_eq!(
        tree.text_field_suggestion_span_at_cursor_index(id, 21),
        None
    );
}
