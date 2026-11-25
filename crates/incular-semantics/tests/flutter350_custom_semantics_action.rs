use incular_semantics::{CustomSemanticsAction, CustomSemanticsActionId, SemanticActionKind};

#[test]
fn a_label_canonicalizes_separately_allocated_actions() {
    let a = CustomSemanticsAction::new(String::from("test"));
    let b = CustomSemanticsAction::new(String::from("test"));
    let different = CustomSemanticsAction::new(String::from("not test"));
    let a_id = a.get_identifier();
    let b_id = b.get_identifier();
    let different_id = different.get_identifier();

    assert_eq!(a_id, b_id);
    assert_ne!(b_id, different_id);
    assert_eq!(CustomSemanticsAction::get_action(a_id), Some(a.clone()));
    assert_eq!(CustomSemanticsAction::get_action(b_id), Some(a));
    assert_eq!(
        CustomSemanticsAction::get_action(different_id),
        Some(different)
    );
}

#[test]
fn overridden_actions_are_canonical_by_both_hint_and_standard_action() {
    use SemanticActionKind::{Activate, LongPress};

    let activate = CustomSemanticsAction::overriding_action(Activate, "Select");
    let equivalent = CustomSemanticsAction::overriding_action(Activate, String::from("Select"));
    let other_kind = CustomSemanticsAction::overriding_action(LongPress, "Select");
    let other_hint = CustomSemanticsAction::overriding_action(Activate, "Choose");
    let label = CustomSemanticsAction::new("Select");
    let id = activate.get_identifier();

    assert_eq!(id, equivalent.get_identifier());
    for other in [other_kind, other_hint, label.clone()] {
        assert_ne!(id, other.get_identifier());
    }
    let recovered = CustomSemanticsAction::get_action(id).unwrap();
    assert_eq!(recovered.label(), None);
    assert_eq!(recovered.hint(), Some("Select"));
    assert_eq!(recovered.action(), Some(Activate));
    assert_eq!(label.label(), Some("Select"));
    assert_eq!(label.hint(), None);
    assert_eq!(label.action(), None);
    assert_eq!(
        CustomSemanticsAction::get_action(CustomSemanticsActionId(u32::MAX)),
        None
    );
}
