use incular_text::{TextEditingValue, TextRange};

#[test]
fn composing_validity_requires_a_present_ordered_range_within_the_text() {
    assert!(!TextEditingValue::empty().is_composing_range_valid());
    for (range, expected) in [
        (TextRange::new(1, 0), false),
        (TextRange::new(1, 5), false),
        // Rust represents Flutter's negative or otherwise unrepresentable
        // IME endpoints as absent, or rejects them at the byte boundary.
        (TextRange::new(usize::MAX, 4), false),
        (TextRange::new(1, 4), true),
        (TextRange::collapsed(4), true),
    ] {
        let value = TextEditingValue {
            composing: Some(range),
            ..TextEditingValue::new("test")
        };
        assert_eq!(value.is_composing_range_valid(), expected, "{range:?}");
    }
}

#[test]
fn composition_requires_representable_byte_boundaries() {
    let text = "a😀b";
    for (range, valid) in [
        (TextRange::new(1, 5), true),
        (TextRange::collapsed(5), true),
        (TextRange::collapsed(6), true),
        (TextRange::new(1, 4), false),
        (TextRange::collapsed(2), false),
    ] {
        let value = TextEditingValue {
            composing: Some(range),
            ..TextEditingValue::new(text)
        };
        assert_eq!(value.is_composing_range_valid(), valid, "{range:?}");
        assert_eq!(range.is_valid_for(text), valid);
    }
}

#[test]
fn composition_only_updates_preserve_the_unset_selection_and_skip_redundant_notifications() {
    use incular_text::TextEditingController;
    use std::{cell::Cell, rc::Rc};

    let controller = TextEditingController::with_text("draft");
    let count = Rc::new(Cell::new(0));
    let notifications = count.clone();
    let _observer = controller.observe(move |_| notifications.set(notifications.get() + 1));
    assert_eq!(controller.selection_if_set(), None);
    controller.set_composing(Some(TextRange::new(1, 4)));
    assert_eq!(controller.selection_if_set(), None);
    assert_eq!(controller.composing(), Some(TextRange::new(1, 4)));
    controller.clear_composing();
    assert_eq!(controller.selection_if_set(), None);
    assert_eq!(controller.composing(), None);
    assert_eq!(controller.text(), "draft");
    controller.clear_composing();
    assert_eq!(count.get(), 2);
}
