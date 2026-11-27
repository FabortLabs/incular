use incular_config::{Constraints, Locale, TextDirection};
use incular_core::Size;
use incular_text::{
    StrutStyle, TextEditingController, TextSelection, TextStyle, register_application_font_as,
};
use incular_widgets::{
    EditableText, SpellCheckCallback, SpellCheckConfiguration, SpellCheckService, SuggestionSpan,
    internal::WidgetTree,
};

struct TallWord;

impl SpellCheckService for TallWord {
    fn fetch_spell_check_suggestions(&self, _: &Locale, text: &str, respond: SpellCheckCallback) {
        assert_eq!(text, "TALL");
        respond(Some(vec![SuggestionSpan::new(0..4, ["tall"])]));
    }
}

#[test]
fn an_unset_caret_does_not_suppress_the_tall_styled_final_word() {
    let Some(path) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    const FONT: &str = "Pinned Ahem Spell Span";
    register_application_font_as(FONT, std::fs::read(path).unwrap());
    let controller = TextEditingController::new();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .style(TextStyle::new().font_family(FONT).font_size(10.))
                // Match the explicit low-level RenderEditable fixture,
                // which has no forced strut of its own.
                .strut_style(StrutStyle::new().font_size(0.))
                .spell_check_configuration(
                    SpellCheckConfiguration::new()
                        .spell_check_service(TallWord)
                        .misspelled_text_style(TextStyle::new().font_size(100.)),
                )
                .into(),
        )
        .unwrap();
    let size = |tree: &mut WidgetTree| {
        tree.layout(Constraints::loose(Size::new(600., 600.)))
            .unwrap();
        tree.element_bounds(root).unwrap().size
    };
    controller.insert_text("TALL");
    assert_eq!(
        controller.selection_if_set(),
        Some(TextSelection::collapsed(4))
    );
    assert_eq!(size(&mut tree).height, 10.);
    controller.clear_selection();
    assert!(controller.selection_if_set().is_none());
    assert_eq!(size(&mut tree).height, 100.);
    controller.set_selection(TextSelection::collapsed(4));
    assert_eq!(size(&mut tree).height, 10.);
}
