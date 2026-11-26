use incular_text::{TextEditingController, TextEditingValue, TextRange, TextSelection};

const TEXT: &str = "foo composing bar";
const COMPOSING: TextRange = TextRange::new(4, 12);

fn reset(controller: &TextEditingController) {
    let mut value = TextEditingValue::new(TEXT);
    value.selection = TextSelection::collapsed(4);
    value.composing = Some(COMPOSING);
    controller.set_value(value);
    assert_eq!(controller.composing(), Some(COMPOSING));
}

#[test]
fn moving_the_caret_within_composition_keeps_the_range() {
    let controller = TextEditingController::with_text(TEXT);
    controller.set_composing(Some(COMPOSING));
    assert_eq!(controller.selection_if_set(), None);
    for offset in [5, 4, 12] {
        controller.set_selection(TextSelection::collapsed(offset));
        assert_eq!(controller.composing(), Some(COMPOSING));
        assert_eq!(
            controller.selection_if_set(),
            Some(TextSelection::collapsed(offset))
        );
    }
}

#[test]
fn collapsing_before_or_after_composition_clears_the_range() {
    let controller = TextEditingController::new();
    for offset in [2, 14] {
        reset(&controller);
        let selection = TextSelection::collapsed(offset);
        controller.set_selection(selection);
        assert_eq!(controller.selection(), selection);
        assert_eq!(controller.text(), TEXT);
        assert_eq!(controller.composing(), None);
    }
}

#[test]
fn both_selection_endpoints_must_be_within_composition() {
    let controller = TextEditingController::new();
    for (base, extent, retained) in [
        (1, 2, false),
        (5, 7, true),
        (13, 15, false),
        (7, 5, true),  // A backward range preserves its anchor.
        (12, 4, true), // Exact endpoints are inclusive.
        (4, 13, false),
        (6, 2, false),
    ] {
        reset(&controller);
        let selection = TextSelection::new(base, extent);
        controller.set_selection(selection);
        assert_eq!(controller.selection(), selection);
        assert_eq!(
            controller.composing(),
            retained.then_some(COMPOSING),
            "{selection:?}"
        );
        assert_eq!(controller.text(), TEXT);
    }
}
