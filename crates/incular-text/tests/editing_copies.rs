//! Edits build the next value directly instead of copying the current one
//! and editing the copy, and selection changes never copy the text. Both
//! must behave exactly as the value-level operations they replace.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use incular_text::{TextEditingController, TextEditingValue, TextRange, TextSelection};

fn exactly_sized(controller: &TextEditingController) -> bool {
    controller.with_value(|value| value.text.capacity() == value.text.len())
}

#[test]
fn an_edit_leaves_the_text_exactly_sized() {
    let controller = TextEditingController::with_text("the quick brown fox".repeat(64));
    controller.set_selection(TextSelection::collapsed(40));
    controller.replace_selection("a");
    assert!(exactly_sized(&controller), "insertion");
    controller.backspace();
    assert!(exactly_sized(&controller), "backspace");
    controller.delete_forward();
    assert!(exactly_sized(&controller), "delete");
    controller.set_selection(TextSelection::new(10, 30));
    controller.replace_selection("replacement");
    assert!(exactly_sized(&controller), "replacement");
}

#[test]
fn edits_match_the_value_level_replace() {
    let text = "ab\u{e9}\u{1F600}cd\nef";
    let selections = [
        TextSelection::collapsed(0),
        TextSelection::collapsed(text.len()),
        TextSelection::new(1, 4),
        // Reversed, and an edge inside a multi-byte character.
        TextSelection::new(9, 2),
        TextSelection::new(3, 6),
    ];
    for selection in selections {
        for replacement in ["", "x", "\u{4e16}\u{754c}", "two\nlines"] {
            let controller = TextEditingController::with_text(text);
            controller.set_selection(selection);
            let expected = controller
                .value()
                .replace(controller.selection().range(), replacement);
            controller.replace_selection(replacement);
            assert_eq!(
                controller.value(),
                expected,
                "{selection:?} <- {replacement:?}"
            );
        }
    }
}

#[test]
fn deleting_matches_the_value_level_replace() {
    let text = "a\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}b";
    for caret in [0, 1, text.len() - 1, text.len()] {
        let backspaced = TextEditingController::with_text(text);
        backspaced.set_selection(TextSelection::collapsed(caret));
        backspaced.backspace();
        let deleted = TextEditingController::with_text(text);
        deleted.set_selection(TextSelection::collapsed(caret));
        deleted.delete_forward();
        // One grapheme goes, whole, and the caret lands on its edge.
        for controller in [&backspaced, &deleted] {
            let value = controller.value();
            assert!(value.text.is_char_boundary(value.selection.extent));
            assert!(value.selection.is_collapsed());
            assert!(text.len() - value.text.len() <= text.len() - 2);
        }
    }
    let ranged = TextEditingController::with_text("hello world");
    ranged.set_selection(TextSelection::new(2, 7));
    ranged.backspace();
    assert_eq!(
        ranged.value(),
        TextEditingValue::new("hello world").replace(TextRange::new(2, 7), "")
    );
}

#[test]
fn a_selection_change_notifies_once_and_only_when_it_changes() {
    let controller = TextEditingController::with_text("hello world");
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    let _token = controller.add_listener(move |value| log.borrow_mut().push(value.clone()));

    controller.set_selection(TextSelection::new(2, 5));
    assert_eq!(seen.borrow().len(), 1);
    assert_eq!(seen.borrow()[0].text, "hello world");
    assert_eq!(seen.borrow()[0].selection, TextSelection::new(2, 5));

    // The same selection again changes nothing.
    controller.set_selection(TextSelection::new(2, 5));
    assert_eq!(seen.borrow().len(), 1);

    // Out-of-range edges are clamped before the comparison.
    controller.set_selection(TextSelection::new(2, 500));
    assert_eq!(seen.borrow().len(), 2);
    assert_eq!(controller.selection(), TextSelection::new(2, 11));
    controller.set_selection(TextSelection::new(2, 900));
    assert_eq!(seen.borrow().len(), 2);

    controller.set_composing(Some(TextRange::new(1, 4)));
    assert_eq!(seen.borrow().len(), 3);
    assert_eq!(controller.composing(), Some(TextRange::new(1, 4)));
    controller.set_composing(Some(TextRange::new(1, 4)));
    assert_eq!(seen.borrow().len(), 3);
}

#[test]
fn a_selection_change_advances_only_the_visual_revision() {
    let controller = TextEditingController::with_text("hello");
    controller.set_preedit("pre", None);
    let (content, visual) = controller.revisions();
    controller.set_selection(TextSelection::collapsed(2));
    let (next_content, next_visual) = controller.revisions();
    assert_eq!(next_content, content);
    assert!(next_visual > visual);
    // As for every committed change, pending preedit text is dropped.
    assert_eq!(controller.preedit(), None);
}

#[test]
fn a_selection_change_still_passes_through_edit_transforms() {
    let controller = TextEditingController::with_text("hello world");
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let _transform = controller.register_edit_transform(move |old, next| {
        counted.set(counted.get() + 1);
        assert_eq!(old.text, next.text);
        // Keep the caret out of the first word.
        let mut next = next.clone();
        if next.selection.extent < 6 {
            next.selection = TextSelection::collapsed(6);
        }
        next
    });
    controller.set_selection(TextSelection::collapsed(2));
    assert_eq!(calls.get(), 1);
    assert_eq!(controller.selection(), TextSelection::collapsed(6));
    controller.set_selection(TextSelection::collapsed(8));
    assert_eq!(controller.selection(), TextSelection::collapsed(8));
}

#[test]
fn a_listener_may_edit_the_controller_it_observes() {
    let controller = TextEditingController::with_text("abc");
    let editor = controller.clone();
    let _token = controller.add_listener(move |value| {
        if value.selection.extent == 1 {
            editor.set_selection(TextSelection::collapsed(3));
        }
    });
    controller.set_selection(TextSelection::collapsed(1));
    assert_eq!(controller.selection(), TextSelection::collapsed(3));
}
