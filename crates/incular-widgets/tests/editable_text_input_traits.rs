//! Flutter `EditableText.textInputConfiguration`: the keyboard hints beyond
//! the input type and action, with Flutter's defaults.

use incular_config::{Brightness, Constraints};
use incular_core::Size;
use incular_platform::{TextCapitalization, TextInputTraits};
use incular_text::TextEditingController;
use incular_widgets::internal::*;

fn snapshot_traits(build: impl FnOnce(EditableText) -> EditableText) -> TextInputTraits {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Widget::from(build(
            EditableText::new(TextEditingController::new())
                .text_direction(incular_widgets::TextDirection::Ltr)
                .size(Size::new(180., 32.)),
        )))
        .expect("mount field");
    tree.layout(Constraints::loose(Size::new(300., 300.)))
        .expect("layout");
    tree.text_field_input_snapshot(root)
        .expect("snapshot")
        .traits
}

#[test]
fn defaults_match_editable_text() {
    let traits = snapshot_traits(|field| field);
    assert!(traits.autocorrect);
    assert!(traits.enable_suggestions);
    assert!(traits.enable_interactive_selection);
    assert_eq!(traits.text_capitalization, TextCapitalization::None);
    assert_eq!(traits.keyboard_appearance, Brightness::Light);
    // `autofillHints = const <String>[]`: autofill on, without a hint.
    assert_eq!(traits.autofill_hints, Some(Vec::new()));
    assert!(traits.enable_ime_personalized_learning);
}

#[test]
fn the_platform_default_disables_autofill() {
    // `AutofillConfiguration.disabled` when no client supplies hints.
    let traits = TextInputTraits::default();
    assert_eq!(traits.autofill_hints, None);
    assert!(traits.autocorrect && traits.enable_suggestions);
}

#[test]
fn setters_reach_the_snapshot() {
    let traits = snapshot_traits(|field| {
        field
            .autocorrect(false)
            .enable_suggestions(false)
            .text_capitalization(TextCapitalization::Words)
            .keyboard_appearance(Brightness::Dark)
            .autofill_hints(Some(["email", "username"]))
            .enable_ime_personalized_learning(false)
    });
    assert!(!traits.autocorrect);
    assert!(!traits.enable_suggestions);
    assert_eq!(traits.text_capitalization, TextCapitalization::Words);
    assert_eq!(traits.keyboard_appearance, Brightness::Dark);
    assert_eq!(
        traits.autofill_hints,
        Some(vec!["email".to_owned(), "username".to_owned()])
    );
    assert!(!traits.enable_ime_personalized_learning);
    let disabled = snapshot_traits(|field| field.autofill_hints(None::<[&str; 0]>));
    assert_eq!(disabled.autofill_hints, None);
}

#[test]
fn password_hints_infer_autocorrect_only_on_ios() {
    // `_inferAutocorrect` turns autocorrect off for password related hints
    // on iOS only.
    let traits = snapshot_traits(|field| field.autofill_hints(Some(["password"])));
    assert_eq!(traits.autocorrect, !cfg!(target_os = "ios"));
    assert!(snapshot_traits(|field| field.autofill_hints(Some(["email"]))).autocorrect);
}

#[test]
fn read_only_obscured_fields_disable_interactive_selection() {
    // `_userSelectionEnabled`: enableInteractiveSelection && (!readOnly ||
    // !obscureText).
    assert!(
        !snapshot_traits(|field| field.read_only(true).obscure_text(true))
            .enable_interactive_selection
    );
    assert!(snapshot_traits(|field| field.read_only(true)).enable_interactive_selection);
    assert!(
        !snapshot_traits(|field| field.enable_interactive_selection(false))
            .enable_interactive_selection
    );
}
