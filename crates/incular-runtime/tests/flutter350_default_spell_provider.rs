use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::{Constraints, Locale};
use incular_core::{Color, Offset, Size};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_text::{TextDecoration, TextEditingController, TextStyle};
use incular_widgets::{
    Column, DefaultSpellCheckProvider, EditableText, SpellCheckCallback, SpellCheckConfiguration,
    SpellCheckService, SuggestionSpan, TextDirection, Widget,
};

type Requests = Rc<RefCell<Vec<(usize, String, SpellCheckCallback)>>>;

struct Checker {
    id: usize,
    requests: Requests,
}

impl SpellCheckService for Checker {
    fn fetch_spell_check_suggestions(&self, locale: &Locale, text: &str, done: SpellCheckCallback) {
        assert_eq!(locale.to_string(), "en-GB");
        self.requests
            .borrow_mut()
            .push((self.id, text.to_owned(), done));
    }
}

const MISSPELLING: Color = Color::rgba(155, 33, 122, 255);

fn field(
    controller: &TextEditingController,
    is_password: bool,
    config: SpellCheckConfiguration,
) -> Widget {
    EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .locale("en-GB".parse().unwrap())
        .size(Size::new(220., 32.))
        .obscure_text(is_password)
        .spell_check_configuration(config)
        .into()
}

fn frame(runtime: &mut Runtime) -> incular_rendering::DisplayList {
    runtime
        .run_frame(Constraints::tight(Size::new(240., 128.)))
        .unwrap()
        .0
}

#[test]
fn a_platform_default_is_per_editor_and_never_replaces_an_explicit_checker() {
    let [first, second, user, password] = std::array::from_fn(|_| TextEditingController::new());
    let native_calls: Requests = Rc::default();
    let override_calls: Requests = Rc::default();
    let services_created: Rc<Cell<usize>> = Rc::default();
    let opted_in = SpellCheckConfiguration::new().misspelled_text_style(
        TextStyle::new()
            .color(MISSPELLING)
            .decoration(TextDecoration::UNDERLINE),
    );
    let override_config = opted_in.clone().spell_check_service(Checker {
        id: 100,
        requests: override_calls.clone(),
    });
    let mut runtime = Runtime::new(
        Column::new([
            field(&first, false, opted_in.clone()),
            field(&second, false, opted_in.clone()),
            field(&user, false, override_config),
            field(&password, true, opted_in),
        ])
        .into(),
    )
    .unwrap();
    frame(&mut runtime);
    let first_field = runtime.tree().text_field_at(Offset::new(10., 12.)).unwrap();
    assert!(!runtime.tree().text_field_spell_check_enabled(first_field));
    first.insert_text("off");
    assert!(native_calls.borrow().is_empty());

    let factory: DefaultSpellCheckProvider = Rc::new({
        let native_calls = native_calls.clone();
        let services_created = services_created.clone();
        move || {
            let id = services_created.get();
            services_created.set(id + 1);
            Rc::new(Checker {
                id,
                requests: native_calls.clone(),
            })
        }
    });
    runtime.set_default_spell_check_provider(Some(factory.clone()));
    assert_eq!(
        services_created.get(),
        2,
        "only fallback editors need a provider"
    );
    runtime.set_default_spell_check_provider(Some(factory));
    assert_eq!(
        services_created.get(),
        2,
        "same provider does not replace sessions"
    );

    first.set_text("");
    first.insert_text("firts!");
    second.insert_text("secodn");
    user.insert_text("custom");
    password.insert_text("hidden");
    assert_eq!(override_calls.borrow().len(), 1);
    assert_eq!(override_calls.borrow()[0].1, "custom");
    assert_eq!(native_calls.borrow().len(), 2);
    let (second_id, second_text, late) = native_calls.borrow_mut().pop().unwrap();
    let (first_id, first_text, reply) = native_calls.borrow_mut().pop().unwrap();
    assert_ne!(first_id, second_id, "native caches are field-specific");
    assert_eq!(
        (first_text.as_str(), second_text.as_str()),
        ("firts!", "secodn")
    );

    reply(Some(vec![SuggestionSpan::new(0..5, ["first"])]));
    let styled = frame(&mut runtime);
    assert!(
        runtime
            .tree()
            .text_field_spell_check_results(first_field)
            .is_some()
    );
    assert!(styled.commands().iter().any(|cmd| {
        matches!(cmd, PaintCommand::GlyphRun { color, .. } if *color == MISSPELLING)
    }));

    runtime.set_default_spell_check_provider(None);
    late(Some(vec![SuggestionSpan::new(0..6, ["second"])]));
    let restored = frame(&mut runtime);
    assert!(!runtime.tree().text_field_spell_check_enabled(first_field));
    assert!(
        runtime
            .tree()
            .text_field_spell_check_results(first_field)
            .is_none()
    );
    assert!(!restored.commands().iter().any(|cmd| {
        matches!(cmd, PaintCommand::GlyphRun { color, .. } if *color == MISSPELLING)
    }));
    user.insert_text(" remains");
    assert_eq!(override_calls.borrow().len(), 2);
}
