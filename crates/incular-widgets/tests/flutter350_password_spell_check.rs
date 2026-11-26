use incular_config::{Constraints, Locale};
use incular_core::Size;
use incular_text::{TextDecoration, TextEditingController, TextStyle};
use incular_widgets::{
    EditableText, SpellCheckCallback, SpellCheckConfiguration, SpellCheckService, SuggestionSpan,
    TextDirection, TextInputTypeHint, Widget, internal::WidgetTree,
};
use std::{cell::RefCell, rc::Rc};

type Request = (String, SpellCheckCallback);

#[derive(Clone, Default)]
struct Service(Rc<RefCell<Vec<Request>>>);

impl SpellCheckService for Service {
    fn fetch_spell_check_suggestions(
        &self,
        _locale: &Locale,
        text: &str,
        done: SpellCheckCallback,
    ) {
        self.0.borrow_mut().push((text.to_owned(), done));
    }
}

fn field(controller: &TextEditingController, service: &Service) -> EditableText {
    EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(180.0, 30.0))
        .spell_check_configuration(
            SpellCheckConfiguration::new()
                .spell_check_service(service.clone())
                .misspelled_text_style(TextStyle::new().decoration(TextDecoration::UNDERLINE)),
        )
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::loose(Size::new(300.0, 100.0)))
        .unwrap();
}

#[test]
fn hidden_and_password_inputs_never_send_plaintext_to_the_spell_service() {
    for case in 0..6 {
        let controller = TextEditingController::with_text("A");
        let service = Service::default();
        let input = field(&controller, &service).autofill_hints(None::<[&str; 0]>);
        let input = match case {
            0 => input.obscure_text(true),
            1 => input.input_type(TextInputTypeHint::Password),
            2 => input.input_type(TextInputTypeHint::number_password(false, false)),
            3 => input.autofill_hints(Some(["password"])),
            4 => input.autofill_hints(Some(["unrecognized", "newPassword"])),
            _ => input.autofill_hints(Some(["username"])),
        };
        let mut tree = WidgetTree::new();
        let id = tree.mount(Widget::from(input)).unwrap();
        layout(&mut tree);
        assert_eq!(tree.text_field_spell_check_enabled(id), case == 5);
        controller.insert_text(" input");
        layout(&mut tree);
        assert_eq!(
            service.0.borrow().len(),
            usize::from(case == 5),
            "case {case}: only username may check"
        );
    }
}

#[test]
fn password_transition_clears_pending_and_visible_results_and_only_checks_after_reenabled() {
    let controller = TextEditingController::with_text("A");
    let service = Service::default();
    let setup = |obscure| -> Widget { field(&controller, &service).obscure_text(obscure).into() };
    let mut tree = WidgetTree::new();
    let id = tree.mount(setup(false)).unwrap();
    layout(&mut tree);
    assert!(tree.text_field_spell_check_enabled(id));
    assert!(service.0.borrow().is_empty());
    controller.insert_text("x");
    let (first, done) = service.0.borrow_mut().pop().unwrap();
    assert_eq!(first, "Ax");
    done(Some(vec![SuggestionSpan::new(0..2, ["a"])]));
    layout(&mut tree);
    assert!(tree.text_field_spell_check_results(id).is_some());
    controller.insert_text("y");
    let (_, stale) = service.0.borrow_mut().pop().unwrap();

    tree.update(id, setup(true)).unwrap();
    layout(&mut tree);
    assert!(!tree.text_field_spell_check_enabled(id));
    assert!(tree.text_field_spell_check_results(id).is_none());
    stale(Some(vec![SuggestionSpan::new(0..3, ["b"])]));
    controller.set_text("A");
    assert!(service.0.borrow().is_empty());

    tree.update(id, setup(false)).unwrap();
    layout(&mut tree);
    assert!(tree.text_field_spell_check_enabled(id));
    assert!(
        tree.text_field_spell_check_results(id).is_none(),
        "old results cannot repopulate"
    );
    let (current, done) = service
        .0
        .borrow_mut()
        .pop()
        .expect("reenabling should check the current text once");
    assert_eq!(current, "A");
    assert!(service.0.borrow().is_empty());
    done(Some(vec![SuggestionSpan::new(0..1, ["a"])]));
    layout(&mut tree);
    assert_eq!(
        tree.text_field_spell_check_results(id)
            .unwrap()
            .spell_checked_text,
        "A"
    );
}

#[test]
fn visible_password_stays_unchecked_when_obscuring_is_turned_off() {
    let controller = TextEditingController::with_text("A");
    let service = Service::default();
    let input = |obscure| {
        Widget::from(
            field(&controller, &service)
                .input_type(TextInputTypeHint::Password)
                .obscure_text(obscure),
        )
    };
    let mut tree = WidgetTree::new();
    let id = tree.mount(input(true)).unwrap();
    layout(&mut tree);
    assert!(!tree.text_field_spell_check_enabled(id));

    tree.update(id, input(false)).unwrap();
    layout(&mut tree);
    assert!(!tree.text_field_spell_check_enabled(id));
    assert!(tree.text_field_spell_check_results(id).is_none());
    assert!(service.0.borrow().is_empty());
}
