use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, Locale, LocalizationCatalog};
use incular_core::Size;
use incular_runtime::{Runtime, Signal};
use incular_text::{TextEditingController, TextStyle};
use incular_widgets::{
    Builder, EditableText, FocusNode, Localizations, SpellCheckCallback, SpellCheckConfiguration,
    SpellCheckService, TextDirection,
};

struct Catalog;

impl LocalizationCatalog for Catalog {
    fn supported_locales(&self) -> &[Locale] {
        &[]
    }

    fn message(&self, _: &Locale, _: &str) -> Option<&str> {
        None
    }
}

type Requests = Rc<RefCell<Vec<(String, String)>>>;

struct Recorder(Requests);

impl SpellCheckService for Recorder {
    fn fetch_spell_check_suggestions(&self, locale: &Locale, text: &str, done: SpellCheckCallback) {
        self.0
            .borrow_mut()
            .push((locale.to_string(), text.to_owned()));
        done(None);
    }
}

fn locale(name: &str) -> Locale {
    name.parse().unwrap()
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::loose(Size::new(300., 80.)))
        .unwrap();
}

#[test]
fn checker_uses_the_editor_then_live_localizations_not_keyboard_language_hints() {
    let text = TextEditingController::new();
    let focus = FocusNode::new();
    let override_locale = Signal::new(Some(locale("tr")));
    let inherited = Signal::new(locale("en-GB"));
    let requests = Requests::default();
    let configuration = SpellCheckConfiguration::new()
        .spell_check_service(Recorder(requests.clone()))
        .misspelled_text_style(TextStyle::new());
    let mut runtime = Runtime::new(
        Builder::new({
            let text = text.clone();
            let focus = focus.clone();
            let override_locale = override_locale.clone();
            let inherited = inherited.clone();
            move |_| {
                let mut editor = EditableText::new(text.clone())
                    .text_direction(TextDirection::Ltr)
                    .focus_node(focus.clone())
                    .hint_locales(Some(vec![locale("de-DE")]))
                    .spell_check_configuration(configuration.clone());
                if let Some(locale) = override_locale.get() {
                    editor = editor.locale(locale);
                }
                Localizations::new(inherited.get(), Catalog, editor).into()
            }
        })
        .into(),
    )
    .unwrap();
    frame(&mut runtime);
    focus.request_focus();
    frame(&mut runtime);
    let field = runtime.focused_element().unwrap();

    text.insert_text("Istanbul");
    override_locale.set(None);
    frame(&mut runtime);
    text.set_text("");
    text.insert_text("colour");
    inherited.set(locale("fr-CA"));
    frame(&mut runtime);
    text.set_text("");
    text.insert_text("couleur");

    assert_eq!(runtime.focused_element(), Some(field));
    assert_eq!(
        &*requests.borrow(),
        &[
            ("tr".into(), "Istanbul".into()),
            ("en-GB".into(), "colour".into()),
            ("fr-CA".into(), "couleur".into()),
        ]
    );
}
